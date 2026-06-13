//! Extract runtime context from Fabric workspaces as a relationship graph.
//!
//! Builds a graph of workspace items (nodes) and their relationships (edges)
//! by inspecting item properties, definitions, and connections. Designed to
//! provide structured context for coding agents and external applications.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Result, bail};
use base64::prelude::{BASE64_STANDARD, Engine as _};
use clap::Subcommand;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;
use crate::verbose;

// ── CLI definition ──────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum ContextCommand {
    /// Extract a graph of items and relationships from workspace(s)
    #[command(display_order = 1)]
    Extract {
        /// Workspace ID(s) or name(s) to scan (repeatable)
        #[arg(short, long, env = "FABIO_WORKSPACE", num_args = 1..)]
        workspace: Vec<String>,

        /// Fetch item definitions to discover embedded references (slower)
        #[arg(long)]
        deep: bool,

        /// Also fetch item connections
        #[arg(long)]
        include_connections: bool,

        /// Filter to specific item types (comma-separated, case-insensitive)
        #[arg(long)]
        item_types: Option<String>,

        /// Max concurrency for API calls
        #[arg(long, default_value = "8")]
        concurrency: usize,
    },
}

// ── Graph data model ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphNode {
    id: String,
    #[serde(rename = "type")]
    item_type: String,
    name: String,
    workspace_id: String,
    workspace_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
struct GraphEdge {
    source: String,
    target: String,
    relationship: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceInfo {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    capacity_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphSummary {
    total_nodes: usize,
    total_edges: usize,
    workspaces_scanned: usize,
    item_types: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    relationship_types: Option<BTreeMap<String, usize>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContextGraph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    workspaces: Vec<WorkspaceInfo>,
    summary: GraphSummary,
}

// ── Dispatch ────────────────────────────────────────────────────────────────

pub async fn execute(cli: &Cli, client: &FabricClient, command: &ContextCommand) -> Result<()> {
    match command {
        ContextCommand::Extract {
            workspace,
            deep,
            include_connections,
            item_types,
            concurrency,
        } => {
            extract(
                cli,
                client,
                workspace,
                *deep,
                *include_connections,
                item_types.as_deref(),
                *concurrency,
            )
            .await
        }
    }
}

// ── Main extraction logic ───────────────────────────────────────────────────

async fn extract(
    cli: &Cli,
    client: &FabricClient,
    workspaces: &[String],
    deep: bool,
    include_connections: bool,
    item_types_filter: Option<&str>,
    concurrency: usize,
) -> Result<()> {
    if workspaces.is_empty() {
        bail!("At least one --workspace is required");
    }

    // Parse item type filter
    let type_filter: Option<HashSet<String>> =
        item_types_filter.map(|s| s.split(',').map(|t| t.trim().to_lowercase()).collect());

    // Dry-run: show what would be scanned
    if output::dry_run_guard(
        cli,
        "context extract",
        &serde_json::json!({
            "workspaces": workspaces,
            "deep": deep,
            "includeConnections": include_connections,
            "itemTypes": item_types_filter,
            "concurrency": concurrency,
        }),
    ) {
        return Ok(());
    }

    let semaphore = Arc::new(Semaphore::new(concurrency));

    // Phase 1: Resolve workspaces
    let workspace_infos = resolve_workspaces(client, workspaces).await?;

    // Phase 2: List and filter items
    let all_items = list_workspace_items(client, &workspace_infos, type_filter.as_ref()).await?;

    // Phase 3-7: Build graph (nodes + edges)
    let graph = build_graph(
        client,
        &workspace_infos,
        &all_items,
        deep,
        include_connections,
        &semaphore,
    )
    .await?;

    let json_value = serde_json::to_value(&graph)?;
    output::render_object(cli, &json_value, "summary");
    Ok(())
}

async fn resolve_workspaces(
    client: &FabricClient,
    workspaces: &[String],
) -> Result<Vec<WorkspaceInfo>> {
    verbose::trace_category(
        "context",
        &format!("resolving {} workspace(s)", workspaces.len()),
    );

    // Resolve all workspaces concurrently
    let mut join_set = JoinSet::new();
    for (i, ws) in workspaces.iter().enumerate() {
        let client = client.clone();
        let ws = ws.clone();
        join_set.spawn(async move { (i, resolve_workspace(&client, &ws).await) });
    }

    let mut infos: Vec<Option<WorkspaceInfo>> = vec![None; workspaces.len()];
    while let Some(Ok((i, result))) = join_set.join_next().await {
        infos[i] = Some(result?);
    }

    Ok(infos.into_iter().flatten().collect())
}

async fn list_workspace_items(
    client: &FabricClient,
    workspace_infos: &[WorkspaceInfo],
    type_filter: Option<&HashSet<String>>,
) -> Result<Vec<(Value, String, String)>> {
    verbose::trace_category(
        "context",
        &format!("listing items in {} workspace(s)", workspace_infos.len()),
    );

    // List all workspaces concurrently
    let mut join_set = JoinSet::new();
    for (i, ws) in workspace_infos.iter().enumerate() {
        let client = client.clone();
        let ws_id = ws.id.clone();
        join_set.spawn(async move {
            let resp = client
                .get_list(&format!("/workspaces/{ws_id}/items"), "value", true, None)
                .await;
            (i, resp)
        });
    }

    let mut per_workspace_items: Vec<Vec<Value>> = vec![Vec::new(); workspace_infos.len()];
    while let Some(Ok((i, result))) = join_set.join_next().await {
        per_workspace_items[i] = result?.items;
    }

    // Flatten and filter
    let mut all_items: Vec<(Value, String, String)> = Vec::new();
    for (i, items) in per_workspace_items.into_iter().enumerate() {
        let ws_id = &workspace_infos[i].id;
        let ws_name = &workspace_infos[i].name;
        for item in items {
            if let Some(filter) = type_filter {
                let item_type = item
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_lowercase();
                if !filter.contains(&item_type) {
                    continue;
                }
            }
            all_items.push((item, ws_id.clone(), ws_name.clone()));
        }
    }

    verbose::trace_category("context", &format!("found {} items total", all_items.len()));
    Ok(all_items)
}

#[allow(clippy::too_many_lines)]
async fn build_graph(
    client: &FabricClient,
    workspace_infos: &[WorkspaceInfo],
    all_items: &[(Value, String, String)],
    deep: bool,
    include_connections: bool,
    semaphore: &Arc<Semaphore>,
) -> Result<ContextGraph> {
    // Build known-item ID set for cross-referencing
    let known_ids: HashSet<String> = all_items
        .iter()
        .filter_map(|(item, _, _)| item.get("id").and_then(Value::as_str).map(String::from))
        .collect();

    let known_workspace_ids: HashSet<String> =
        workspace_infos.iter().map(|ws| ws.id.clone()).collect();

    // Fetch item details (properties) concurrently
    verbose::trace_category(
        "context",
        "fetching item details for property-based relationships",
    );
    let item_details = fetch_item_details(client, all_items, semaphore).await;

    // Build nodes
    let nodes: Vec<GraphNode> = build_nodes(all_items, &item_details);

    // Discover edges from properties
    verbose::trace_category("context", "discovering relationships from item properties");
    let mut edges: HashSet<GraphEdge> = HashSet::new();

    for (i, detail) in item_details.iter().enumerate() {
        extract_property_edges(
            detail,
            &nodes[i].id,
            &nodes[i].item_type,
            &known_ids,
            &known_workspace_ids,
            &mut edges,
        );
    }

    // Deep mode: fetch definitions & GUID-scan
    if deep {
        verbose::trace_category(
            "context",
            "deep mode: fetching definitions for GUID scanning",
        );
        let definition_edges =
            fetch_definitions_and_scan(client, &nodes, &known_ids, &known_workspace_ids, semaphore)
                .await;
        edges.extend(definition_edges);
    }

    // Fetch connections
    if include_connections {
        verbose::trace_category("context", "fetching item connections");
        let connection_edges = fetch_connections(client, &nodes, &known_ids, semaphore).await;
        edges.extend(connection_edges);
    }

    // Build summary
    Ok(assemble_graph(nodes, edges, workspace_infos))
}

fn build_nodes(all_items: &[(Value, String, String)], item_details: &[Value]) -> Vec<GraphNode> {
    all_items
        .iter()
        .enumerate()
        .map(|(i, (item, ws_id, ws_name))| {
            let properties = item_details
                .get(i)
                .and_then(|detail| detail.get("properties").cloned());

            GraphNode {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                item_type: item
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                name: item
                    .get("displayName")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                workspace_id: ws_id.clone(),
                workspace_name: ws_name.clone(),
                description: item
                    .get("description")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(String::from),
                properties,
            }
        })
        .collect()
}

fn assemble_graph(
    nodes: Vec<GraphNode>,
    edges: HashSet<GraphEdge>,
    workspace_infos: &[WorkspaceInfo],
) -> ContextGraph {
    let mut item_type_counts: BTreeMap<String, usize> = BTreeMap::new();
    for node in &nodes {
        *item_type_counts.entry(node.item_type.clone()).or_insert(0) += 1;
    }

    let mut rel_type_counts: BTreeMap<String, usize> = BTreeMap::new();
    for edge in &edges {
        *rel_type_counts
            .entry(edge.relationship.clone())
            .or_insert(0) += 1;
    }

    let edges_vec: Vec<GraphEdge> = edges.into_iter().collect();

    ContextGraph {
        summary: GraphSummary {
            total_nodes: nodes.len(),
            total_edges: edges_vec.len(),
            workspaces_scanned: workspace_infos.len(),
            item_types: item_type_counts,
            relationship_types: if rel_type_counts.is_empty() {
                None
            } else {
                Some(rel_type_counts)
            },
        },
        nodes,
        edges: edges_vec,
        workspaces: workspace_infos.to_vec(),
    }
}

// ── Workspace resolution ────────────────────────────────────────────────────

async fn resolve_workspace(client: &FabricClient, ws: &str) -> Result<WorkspaceInfo> {
    // GUID detection: 36 chars, hex + dashes, exactly 4 dashes
    if is_guid(ws) {
        let data = client.get(&format!("/workspaces/{ws}")).await?;
        let name = data
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let capacity_id = data
            .get("capacityId")
            .and_then(Value::as_str)
            .map(String::from);
        return Ok(WorkspaceInfo {
            id: ws.to_string(),
            name,
            capacity_id,
        });
    }

    // Name resolution: list workspaces and find by name
    let resp = client.get_list("/workspaces", "value", true, None).await?;
    for item in &resp.items {
        let name = item
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if name.eq_ignore_ascii_case(ws) {
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let capacity_id = item
                .get("capacityId")
                .and_then(Value::as_str)
                .map(String::from);
            return Ok(WorkspaceInfo {
                id,
                name: name.to_string(),
                capacity_id,
            });
        }
    }

    bail!("Workspace not found: {ws}. Use `fabio workspace list` to see available workspaces.")
}

fn is_guid(s: &str) -> bool {
    s.len() == 36
        && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        && s.chars().filter(|&c| c == '-').count() == 4
}

// ── Fetch item details concurrently ─────────────────────────────────────────

async fn fetch_item_details(
    client: &FabricClient,
    items: &[(Value, String, String)],
    semaphore: &Arc<Semaphore>,
) -> Vec<Value> {
    let mut results: Vec<Value> = vec![Value::Null; items.len()];
    let mut join_set = JoinSet::new();

    // Collect items that have type-specific endpoints for richer detail
    let detail_requests: Vec<(usize, String, String, String)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, (item, ws_id, _))| {
            let id = item.get("id").and_then(Value::as_str)?;
            let item_type = item.get("type").and_then(Value::as_str)?;
            let endpoint = type_specific_endpoint(item_type)?;
            Some((i, ws_id.clone(), id.to_string(), endpoint.to_string()))
        })
        .collect();

    for (index, ws_id, item_id, endpoint) in detail_requests {
        let client = client.clone();
        let sem = semaphore.clone();
        let path = format!("/workspaces/{ws_id}/{endpoint}/{item_id}");
        join_set.spawn(async move {
            let _permit = sem.acquire().await.unwrap_or_else(|_| unreachable!());
            let result = client.get(&path).await.unwrap_or(Value::Null);
            (index, result)
        });
    }

    while let Some(Ok((index, value))) = join_set.join_next().await {
        results[index] = value;
    }

    // Fill in items without type-specific endpoints using the basic item data
    for (i, (item, _, _)) in items.iter().enumerate() {
        if results[i].is_null() {
            results[i] = item.clone();
        }
    }

    results
}

/// Returns the type-specific REST endpoint segment for item types that expose
/// properties with relationship data in their GET response.
fn type_specific_endpoint(item_type: &str) -> Option<&'static str> {
    match item_type {
        "KQLDatabase" => Some("kqlDatabases"),
        "Lakehouse" => Some("lakehouses"),
        "Warehouse" => Some("warehouses"),
        "SQLDatabase" => Some("sqlDatabases"),
        "SQLEndpoint" => Some("sqlEndpoints"),
        "Eventhouse" => Some("eventhouses"),
        "KQLDashboard" => Some("kqlDashboards"),
        "KQLQueryset" => Some("kqlQuerysets"),
        "SemanticModel" => Some("semanticModels"),
        "Report" => Some("reports"),
        "Notebook" => Some("notebooks"),
        "Eventstream" => Some("eventstreams"),
        "DataPipeline" => Some("dataPipelines"),
        "Dataflow" => Some("dataflows"),
        "GraphQLApi" => Some("graphqlApis"),
        "MirroredDatabase" => Some("mirroredDatabases"),
        "CopyJob" => Some("copyJobs"),
        "SparkJobDefinition" => Some("sparkJobDefinitions"),
        "Environment" => Some("environments"),
        "MirroredAzureDatabricksCatalog" => Some("mirroredAzureDatabricksCatalogs"),
        "GraphModel" => Some("graphModels"),
        _ => None,
    }
}

// ── Extract edges from item properties ──────────────────────────────────────

fn extract_property_edges(
    detail: &Value,
    source_id: &str,
    source_type: &str,
    known_ids: &HashSet<String>,
    _known_workspace_ids: &HashSet<String>,
    edges: &mut HashSet<GraphEdge>,
) {
    let properties = match detail.get("properties") {
        Some(p) if p.is_object() => p,
        _ => return,
    };

    // KQL Database → Eventhouse (parent)
    if source_type == "KQLDatabase" {
        if let Some(parent_id) = properties
            .get("parentEventhouseItemId")
            .and_then(Value::as_str)
        {
            if known_ids.contains(parent_id) {
                edges.insert(GraphEdge {
                    source: source_id.to_string(),
                    target: parent_id.to_string(),
                    relationship: "child_of".to_string(),
                    metadata: Some(serde_json::json!({"parentType": "Eventhouse"})),
                });
            }
        }
    }

    // Lakehouse → SQL Endpoint (companion relationship via sqlEndpointProperties)
    if source_type == "Lakehouse" {
        if let Some(sql_props) = properties.get("sqlEndpointProperties") {
            if let Some(ep_id) = sql_props.get("id").and_then(Value::as_str) {
                if known_ids.contains(ep_id) {
                    edges.insert(GraphEdge {
                        source: source_id.to_string(),
                        target: ep_id.to_string(),
                        relationship: "has_endpoint".to_string(),
                        metadata: Some(serde_json::json!({"endpointType": "SQLEndpoint"})),
                    });
                }
            }
        }
    }

    // Graph Model → properties.queryReadiness, lastDataLoadingStatus
    // (informational — not a cross-item edge)

    // Generic: scan all property string values for known item IDs
    scan_value_for_ids(properties, source_id, "references", known_ids, edges);
}

/// Recursively scan a JSON value for strings matching known item IDs.
fn scan_value_for_ids(
    value: &Value,
    source_id: &str,
    relationship: &str,
    known_ids: &HashSet<String>,
    edges: &mut HashSet<GraphEdge>,
) {
    match value {
        Value::String(s) => {
            if s.len() == 36 && is_guid(s) && known_ids.contains(s.as_str()) && s != source_id {
                edges.insert(GraphEdge {
                    source: source_id.to_string(),
                    target: s.clone(),
                    relationship: relationship.to_string(),
                    metadata: None,
                });
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                scan_value_for_ids(v, source_id, relationship, known_ids, edges);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                scan_value_for_ids(v, source_id, relationship, known_ids, edges);
            }
        }
        _ => {}
    }
}

// ── Deep mode: definition scanning ──────────────────────────────────────────

async fn fetch_definitions_and_scan(
    client: &FabricClient,
    nodes: &[GraphNode],
    known_ids: &HashSet<String>,
    known_workspace_ids: &HashSet<String>,
    _semaphore: &Arc<Semaphore>,
) -> HashSet<GraphEdge> {
    let uuid_re =
        Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
            .expect("valid regex");

    let all_known: HashSet<String> = known_ids.union(known_workspace_ids).cloned().collect();

    // Fetch all definitions via bulk API (one LRO per workspace)
    let item_parts = bulk_fetch_definitions(client, nodes).await;

    verbose::trace_category(
        "context",
        &format!("scanning definitions for {} items", item_parts.len()),
    );

    // Scan each item's parts for GUID references
    let mut edges: HashSet<GraphEdge> = HashSet::new();
    for node in nodes {
        let Some(parts) = item_parts.get(&node.id) else {
            continue;
        };
        for (path, payload) in parts {
            if path == ".platform" {
                continue;
            }
            let Ok(decoded) = BASE64_STANDARD.decode(payload) else {
                continue;
            };
            let Ok(content) = String::from_utf8(decoded) else {
                continue;
            };
            let mut discovered: HashSet<GraphEdge> = HashSet::new();
            scan_content_for_refs(
                &content,
                path,
                &node.id,
                &uuid_re,
                &all_known,
                known_ids,
                &mut discovered,
            );
            edges.extend(discovered);
        }
    }

    edges
}

/// Fetch definitions for all items using bulk export (one LRO per workspace).
async fn bulk_fetch_definitions(
    client: &FabricClient,
    nodes: &[GraphNode],
) -> HashMap<String, Vec<(String, String)>> {
    let mut ws_ids: Vec<String> = nodes.iter().map(|n| n.workspace_id.clone()).collect();
    ws_ids.sort();
    ws_ids.dedup();

    verbose::trace_category(
        "context",
        &format!(
            "bulk-exporting definitions from {} workspace(s) (single LRO each)",
            ws_ids.len()
        ),
    );

    let mut join_set = JoinSet::new();
    for ws_id in ws_ids {
        let client = client.clone();
        join_set.spawn(async move {
            let result = client
                .post(
                    &format!("/workspaces/{ws_id}/items/bulkExportDefinitions?beta=True"),
                    &serde_json::json!({"mode": "All"}),
                    true,
                )
                .await;
            (ws_id, result)
        });
    }

    let mut item_parts: HashMap<String, Vec<(String, String)>> = HashMap::new();
    while let Some(Ok((ws_id, result))) = join_set.join_next().await {
        let Ok(data) = result else {
            verbose::trace_category(
                "context",
                &format!("bulk export failed for workspace {ws_id}"),
            );
            continue;
        };
        parse_bulk_export_response(&data, &mut item_parts);
    }

    item_parts
}

/// Parse a bulk export response and populate the `item_parts` map.
fn parse_bulk_export_response(
    data: &Value,
    item_parts: &mut HashMap<String, Vec<(String, String)>>,
) {
    let index = data.get("itemDefinitionsIndex").and_then(Value::as_array);
    let parts = data.get("definitionParts").and_then(Value::as_array);

    let (Some(index), Some(parts)) = (index, parts) else {
        return;
    };

    // Build rootPath → item_id mapping
    let mut root_to_id: Vec<(String, String)> = Vec::new();
    for entry in index {
        let id = entry.get("id").and_then(Value::as_str).unwrap_or_default();
        let root = entry
            .get("rootPath")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !id.is_empty() && !root.is_empty() {
            root_to_id.push((root.to_string(), id.to_string()));
        }
    }
    root_to_id.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));

    // Assign each part to an item by matching its path prefix
    for part in parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or_default();
        let payload = part
            .get("payload")
            .and_then(Value::as_str)
            .unwrap_or_default();

        let matched = root_to_id
            .iter()
            .find(|(root, _)| path.starts_with(root.as_str()))
            .map(|(root, id)| {
                let rel_path = path
                    .strip_prefix(root.as_str())
                    .unwrap_or(path)
                    .trim_start_matches('/');
                (id.clone(), rel_path.to_string())
            });

        if let Some((id, rel_path)) = matched {
            item_parts
                .entry(id)
                .or_default()
                .push((rel_path, payload.to_string()));
        }
    }
}

/// Scan decoded content for UUID references to known items.
fn scan_content_for_refs(
    content: &str,
    path: &str,
    source_id: &str,
    uuid_re: &Regex,
    all_known: &HashSet<String>,
    known_items: &HashSet<String>,
    discovered: &mut HashSet<GraphEdge>,
) {
    for mat in uuid_re.find_iter(content) {
        let found_id = mat.as_str().to_lowercase();
        if found_id == source_id.to_lowercase() {
            continue;
        }
        if is_well_known_guid(&found_id) {
            continue;
        }
        let is_known = all_known.contains(&found_id)
            || all_known.iter().any(|k| k.eq_ignore_ascii_case(&found_id));
        if !is_known {
            continue;
        }

        let rel = if known_items
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&found_id))
        {
            classify_definition_reference(path, content, &found_id)
        } else {
            "workspace_ref".to_string()
        };

        let target = known_items
            .iter()
            .find(|k| k.eq_ignore_ascii_case(&found_id))
            .cloned()
            .unwrap_or(found_id);

        discovered.insert(GraphEdge {
            source: source_id.to_string(),
            target,
            relationship: rel,
            metadata: Some(serde_json::json!({"discoveredIn": path})),
        });
    }
}

/// Classify a definition reference based on the file path and content context.
fn classify_definition_reference(path: &str, content: &str, _found_id: &str) -> String {
    let path_lower = path.to_lowercase();

    // Report → Semantic Model
    if path_lower.contains("definition.pbir") || path_lower.contains("pbir") {
        return "bound_to_model".to_string();
    }

    // Notebook → Lakehouse (trident metadata)
    if content.contains("default_lakehouse") || content.contains("trident") {
        return "default_lakehouse".to_string();
    }

    // Eventstream topology references
    if path_lower.contains("eventstream") && content.contains("destinations") {
        return "streams_to".to_string();
    }

    // Semantic model → data source
    if path_lower.contains("model.tmdl") || path_lower.contains(".bim") {
        return "reads_from".to_string();
    }

    // Data pipeline → referenced items
    if path_lower.contains("pipeline") && content.contains("ExecutePipeline") {
        return "executes".to_string();
    }

    // Data agent → data source
    if path_lower.contains("datasource") {
        return "queries".to_string();
    }

    // Ontology → data binding (lakehouse reference)
    if path_lower.contains("databinding") || path_lower.contains("data_binding") {
        return "bound_to_data".to_string();
    }

    // Generic reference
    "definition_ref".to_string()
}

/// Well-known GUIDs that should not be treated as item references.
fn is_well_known_guid(id: &str) -> bool {
    let lower = id.to_lowercase();
    // All zeros
    if lower == "00000000-0000-0000-0000-000000000000" {
        return true;
    }
    // All f's
    if lower == "ffffffff-ffff-ffff-ffff-ffffffffffff" {
        return true;
    }
    // Near-zero (placeholder GUIDs)
    if lower.starts_with("00000000-0000-0000-0000-00000000000") {
        return true;
    }
    false
}

// ── Connection fetching ─────────────────────────────────────────────────────

async fn fetch_connections(
    client: &FabricClient,
    nodes: &[GraphNode],
    _known_ids: &HashSet<String>,
    semaphore: &Arc<Semaphore>,
) -> HashSet<GraphEdge> {
    let mut edges: HashSet<GraphEdge> = HashSet::new();
    let mut join_set = JoinSet::new();

    for node in nodes {
        let client = client.clone();
        let sem = semaphore.clone();
        let ws_id = node.workspace_id.clone();
        let item_id = node.id.clone();
        let source_id = node.id.clone();

        join_set.spawn(async move {
            let _permit = sem.acquire().await.unwrap_or_else(|_| unreachable!());
            let result = client
                .get_list(
                    &format!("/workspaces/{ws_id}/items/{item_id}/connections"),
                    "value",
                    true,
                    None,
                )
                .await;

            let mut discovered: HashSet<GraphEdge> = HashSet::new();
            if let Ok(resp) = result {
                for conn in &resp.items {
                    if let Some(conn_id) = conn.get("id").and_then(Value::as_str) {
                        let conn_name = conn
                            .get("displayName")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        let conn_type = conn
                            .get("connectivityType")
                            .and_then(Value::as_str)
                            .unwrap_or_default();

                        discovered.insert(GraphEdge {
                            source: source_id.clone(),
                            target: conn_id.to_string(),
                            relationship: "connected_via".to_string(),
                            metadata: Some(serde_json::json!({
                                "connectionName": conn_name,
                                "connectivityType": conn_type,
                            })),
                        });
                    }
                }
            }
            discovered
        });
    }

    while let Some(Ok(discovered)) = join_set.join_next().await {
        edges.extend(discovered);
    }

    edges
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn test_is_guid() {
        assert!(is_guid("12345678-1234-1234-1234-123456789abc"));
        assert!(is_guid("ABCDEF01-2345-6789-abcd-ef0123456789"));
        assert!(!is_guid("not-a-guid"));
        assert!(!is_guid("12345678-1234-1234-1234-123456789ab")); // too short
        assert!(!is_guid("12345678-1234-1234-1234-123456789abcd")); // too long
        assert!(!is_guid("1234567g-1234-1234-1234-123456789abc")); // invalid char
    }

    #[test]
    fn test_is_well_known_guid() {
        assert!(is_well_known_guid("00000000-0000-0000-0000-000000000000"));
        assert!(is_well_known_guid("00000000-0000-0000-0000-000000000001"));
        assert!(is_well_known_guid("ffffffff-ffff-ffff-ffff-ffffffffffff"));
        assert!(!is_well_known_guid("12345678-1234-1234-1234-123456789abc"));
    }

    #[test]
    fn test_scan_value_for_ids() {
        let known_ids: HashSet<String> = [
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
            "11111111-2222-3333-4444-555555555555".to_string(),
        ]
        .into();

        let value = serde_json::json!({
            "parentId": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "nested": {
                "ref": "11111111-2222-3333-4444-555555555555"
            },
            "unrelated": "not-a-guid"
        });

        let source_id = "source00-0000-0000-0000-000000000000";
        let mut edges: HashSet<GraphEdge> = HashSet::new();
        scan_value_for_ids(&value, source_id, "references", &known_ids, &mut edges);

        assert_eq!(edges.len(), 2);
        let targets: BTreeSet<String> = edges.iter().map(|e| e.target.clone()).collect();
        assert!(targets.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"));
        assert!(targets.contains("11111111-2222-3333-4444-555555555555"));
    }

    #[test]
    fn test_scan_value_excludes_self() {
        let source_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        let known_ids: HashSet<String> = [source_id.to_string()].into();

        let value = serde_json::json!({"id": source_id});
        let mut edges: HashSet<GraphEdge> = HashSet::new();
        scan_value_for_ids(&value, source_id, "references", &known_ids, &mut edges);

        assert_eq!(edges.len(), 0);
    }

    #[test]
    fn test_classify_definition_reference() {
        assert_eq!(
            classify_definition_reference("definition.pbir", "{}", "id"),
            "bound_to_model"
        );
        assert_eq!(
            classify_definition_reference(
                "notebook-content.py",
                r#"{"trident":{"lakehouse":{"default_lakehouse":"x"}}}"#,
                "id"
            ),
            "default_lakehouse"
        );
        assert_eq!(
            classify_definition_reference("pipeline.json", r#"{"ExecutePipeline": {}}"#, "id"),
            "executes"
        );
        assert_eq!(
            classify_definition_reference("model.tmdl", "expression stuff", "id"),
            "reads_from"
        );
        assert_eq!(
            classify_definition_reference("some-file.json", "random content", "id"),
            "definition_ref"
        );
    }

    #[test]
    fn test_type_specific_endpoint() {
        assert_eq!(type_specific_endpoint("KQLDatabase"), Some("kqlDatabases"));
        assert_eq!(type_specific_endpoint("Lakehouse"), Some("lakehouses"));
        assert_eq!(type_specific_endpoint("UnknownType"), None);
    }

    #[test]
    fn test_graph_serialization() {
        let graph = ContextGraph {
            nodes: vec![GraphNode {
                id: "aaa".to_string(),
                item_type: "Notebook".to_string(),
                name: "MyNB".to_string(),
                workspace_id: "ws1".to_string(),
                workspace_name: "TestWS".to_string(),
                description: None,
                properties: None,
            }],
            edges: vec![GraphEdge {
                source: "aaa".to_string(),
                target: "bbb".to_string(),
                relationship: "default_lakehouse".to_string(),
                metadata: None,
            }],
            workspaces: vec![WorkspaceInfo {
                id: "ws1".to_string(),
                name: "TestWS".to_string(),
                capacity_id: Some("cap1".to_string()),
            }],
            summary: GraphSummary {
                total_nodes: 1,
                total_edges: 1,
                workspaces_scanned: 1,
                item_types: BTreeMap::from([("Notebook".to_string(), 1)]),
                relationship_types: Some(BTreeMap::from([("default_lakehouse".to_string(), 1)])),
            },
        };

        let json = serde_json::to_value(&graph).unwrap();
        assert_eq!(json["summary"]["totalNodes"], 1);
        assert_eq!(json["summary"]["totalEdges"], 1);
        assert_eq!(json["nodes"][0]["name"], "MyNB");
        assert_eq!(json["edges"][0]["relationship"], "default_lakehouse");
        assert_eq!(json["workspaces"][0]["capacityId"], "cap1");
    }
}
