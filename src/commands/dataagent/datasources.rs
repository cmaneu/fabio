use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

use super::{decode_part_payload, find_datasource_dir, get_definition_parts};

/// List configured data sources by parsing the agent's definition.
pub(super) async fn list_datasources(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let parts = get_definition_parts(client, workspace, id).await?;
    let datasources = extract_datasources_from_parts(&parts);

    output::render_list_with_token(
        cli,
        &datasources,
        &["displayName", "type", "artifactId", "workspaceId"],
        &["NAME", "TYPE", "ARTIFACT ID", "WORKSPACE ID"],
        "displayName",
        None,
    );
    Ok(())
}

/// Show details of a specific data source from the definition.
pub(super) async fn show_datasource(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
) -> Result<()> {
    let parts = get_definition_parts(client, workspace, id).await?;
    let datasources = extract_datasources_from_parts(&parts);

    let ds = datasources
        .iter()
        .find(|d| {
            let name = d.get("displayName").and_then(Value::as_str).unwrap_or("");
            let ds_id = d.get("artifactId").and_then(Value::as_str).unwrap_or("");
            name.eq_ignore_ascii_case(datasource) || ds_id == datasource
        })
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::NotFound,
                format!("Data source '{datasource}' not found"),
                "List available data sources: fabio data-agent list-datasources -w <workspace> --id <id>",
            )
        })?;

    output::render_object(cli, ds, "displayName");
    Ok(())
}

/// Add a data source to the agent's definition.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) async fn add_datasource(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    artifact: &str,
    artifact_workspace: Option<&str>,
    artifact_type: Option<&str>,
    instructions: Option<&str>,
) -> Result<()> {
    let ds_workspace = artifact_workspace.unwrap_or(workspace);

    // Auto-detect artifact type if not provided
    let resolved_type = if let Some(t) = artifact_type {
        t.to_string()
    } else {
        // Try to find the artifact in the workspace items list
        let items = client
            .get_list(
                &format!("/workspaces/{ds_workspace}/items"),
                "value",
                true,
                None,
            )
            .await?;

        let found = items.items.iter().find(|item| {
            let item_name = item
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or("");
            let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");
            item_name.eq_ignore_ascii_case(artifact) || item_id == artifact
        });

        match found {
            Some(item) => item
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            None => {
                return Err(FabioError::with_hint(
                    ErrorCode::NotFound,
                    format!("Artifact '{artifact}' not found in workspace '{ds_workspace}'"),
                    "Specify the artifact type with --artifact-type, or check the workspace items: fabio item list -w <workspace>",
                ).into());
            }
        }
    };

    // Map Fabric item type to data agent datasource type
    let ds_type = map_item_type_to_datasource_type(&resolved_type)?;

    // Resolve artifact ID
    let items = client
        .get_list(
            &format!("/workspaces/{ds_workspace}/items?type={resolved_type}"),
            "value",
            true,
            None,
        )
        .await?;

    let artifact_item = items
        .items
        .iter()
        .find(|item| {
            let item_name = item.get("displayName").and_then(Value::as_str).unwrap_or("");
            let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");
            item_name.eq_ignore_ascii_case(artifact) || item_id == artifact
        })
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::NotFound,
                format!("Artifact '{artifact}' of type '{resolved_type}' not found"),
                format!("List items of this type: fabio item list -w {ds_workspace} --type {resolved_type}"),
            )
        })?;

    let artifact_id = artifact_item
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let artifact_name = artifact_item
        .get("displayName")
        .and_then(Value::as_str)
        .unwrap_or(artifact);

    if output::dry_run_guard(
        cli,
        "data-agent add-datasource",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "artifactId": artifact_id,
            "artifactName": artifact_name,
            "artifactWorkspace": ds_workspace,
            "datasourceType": ds_type,
        }),
    ) {
        return Ok(());
    }

    // Build datasource definition
    let mut datasource_json = serde_json::json!({
        "artifactId": artifact_id,
        "workspaceId": ds_workspace,
        "displayName": artifact_name,
        "type": ds_type,
    });
    if let Some(instr) = instructions {
        datasource_json["dataSourceInstructions"] = Value::from(instr);
    }

    // Fetch current definition and append the new datasource part
    let parts = get_definition_parts(client, workspace, id).await?;
    let mut new_parts = parts;

    // Determine path prefix based on type
    let path_prefix = format!("Files/Config/draft/{ds_type}-{artifact_name}");
    let ds_encoded = BASE64.encode(serde_json::to_string(&datasource_json)?.as_bytes());

    new_parts.push(serde_json::json!({
        "path": format!("{path_prefix}/datasource.json"),
        "payload": ds_encoded,
        "payloadType": "InlineBase64"
    }));

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "status": "datasource_added",
        "artifactId": artifact_id,
        "displayName": artifact_name,
        "type": ds_type,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Remove a data source from the agent's definition.
pub(super) async fn remove_datasource(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent remove-datasource",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;

    // Find datasource parts matching the name or ID
    let new_parts: Vec<Value> = parts
        .iter()
        .filter(|part| {
            let path = part.get("path").and_then(Value::as_str).unwrap_or("");
            if !path.starts_with("Files/Config/draft/") || !path.contains('/') {
                return true; // keep non-datasource parts
            }
            // Check if this part belongs to the datasource being removed
            if let Some(payload) = part.get("payload").and_then(Value::as_str)
                && path.ends_with("/datasource.json")
                && let Some(decoded) = decode_part_payload(payload)
                && let Ok(parsed) = serde_json::from_str::<Value>(&decoded)
            {
                let name = parsed
                    .get("displayName")
                    .or_else(|| parsed.get("display_name"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let art_id = parsed
                    .get("artifactId")
                    .or_else(|| parsed.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if name.eq_ignore_ascii_case(datasource) || art_id == datasource {
                    return false; // remove this datasource part
                }
            }
            // Also remove associated fewshots file in the same directory
            if path.contains(datasource) {
                return false;
            }
            true
        })
        .cloned()
        .collect();

    if new_parts.len() == parts.len() {
        return Err(FabioError::with_hint(
            ErrorCode::NotFound,
            format!("Data source '{datasource}' not found in agent definition"),
            "List available data sources: fabio data-agent list-datasources -w <workspace> --id <id>",
        )
        .into());
    }

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "id": id,
        "status": "datasource_removed",
        "datasource": datasource,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Select or unselect tables in a data source.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) async fn select_tables(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
    tables: Option<&str>,
    all_tables: bool,
    unselect: bool,
) -> Result<()> {
    if tables.is_none() && !all_tables {
        return Err(
            FabioError::invalid_input("Either --tables or --all-tables must be provided").into(),
        );
    }

    if output::dry_run_guard(
        cli,
        "data-agent select-tables",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
            "tables": tables,
            "allTables": all_tables,
            "unselect": unselect,
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;
    let ds_dir = find_datasource_dir(&parts, datasource)?;
    let ds_path = format!("{ds_dir}/datasource.json");

    // Find and parse the datasource
    let ds_payload = parts
        .iter()
        .find_map(|part| {
            let path = part.get("path").and_then(Value::as_str)?;
            if path == ds_path {
                part.get("payload")
                    .and_then(Value::as_str)
                    .map(String::from)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::NotFound,
                format!("Datasource file not found at '{ds_path}'"),
            )
        })?;

    let mut ds_json: Value = decode_part_payload(&ds_payload)
        .and_then(|s| serde_json::from_str(&s).ok())
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "Failed to decode datasource definition",
            )
        })?;

    let table_names: Vec<&str> = tables
        .map(|t| t.split(',').map(str::trim).collect())
        .unwrap_or_default();
    let target_selected = !unselect;

    // Recursively set is_selected on matching table elements
    let modified = ds_json
        .get_mut("elements")
        .and_then(Value::as_array_mut)
        .map_or(0, |elements| {
            set_table_selection(elements, &table_names, all_tables, target_selected)
        });

    if modified == 0 && !all_tables {
        return Err(FabioError::with_hint(
            ErrorCode::NotFound,
            format!("No matching tables found: {}", table_names.join(", ")),
            "List available tables: fabio data-agent show-datasource -w <workspace> --id <id> --datasource <ds>",
        )
        .into());
    }

    // Re-encode and update definition
    let encoded = BASE64.encode(serde_json::to_string(&ds_json)?.as_bytes());

    let new_parts: Vec<Value> = parts
        .iter()
        .map(|p| {
            if p.get("path").and_then(Value::as_str) == Some(&ds_path) {
                serde_json::json!({
                    "path": ds_path,
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                })
            } else {
                p.clone()
            }
        })
        .collect();

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "status": if unselect { "tables_unselected" } else { "tables_selected" },
        "modified": modified,
        "allTables": all_tables,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

// ─── Private Helpers ─────────────────────────────────────────────────────────

/// Extract datasource information from definition parts.
fn extract_datasources_from_parts(parts: &[Value]) -> Vec<Value> {
    let mut datasources = Vec::new();
    for part in parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if path.starts_with("Files/Config/draft/") && path.ends_with("/datasource.json") {
            let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");
            if let Some(decoded) = decode_part_payload(payload)
                && let Ok(parsed) = serde_json::from_str::<Value>(&decoded)
            {
                datasources.push(parsed);
            }
        }
    }
    datasources
}

/// Map a Fabric item type to the data agent datasource type string.
fn map_item_type_to_datasource_type(item_type: &str) -> Result<String> {
    let ds_type = match item_type.to_lowercase().as_str() {
        "lakehouse" => "lakehouse_tables",
        "warehouse" => "data_warehouse",
        "kqldatabase" => "kusto",
        "semanticmodel" => "semantic_model",
        "ontology" => "ontology",
        "graphmodel" => "graph",
        "mirroreddatabase" => "mirrored_database",
        "sqldatabase" => "sql_database",
        _ => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Unsupported artifact type '{item_type}' for data agent datasource"),
                "Supported types: Lakehouse, Warehouse, KQLDatabase, SemanticModel, Ontology, GraphModel, MirroredDatabase, SQLDatabase",
            )
            .into());
        }
    };
    Ok(ds_type.to_string())
}

/// Recursively set `is_selected` on table elements matching the given names.
/// Returns the number of elements modified.
fn set_table_selection(
    elements: &mut [Value],
    table_names: &[&str],
    all_tables: bool,
    selected: bool,
) -> usize {
    let selectable_types = [
        "semantic_model.table",
        "lakehouse_tables.table",
        "warehouse_tables.table",
        "kusto.table",
        "mirrored_database.table",
        "sql_database.table",
    ];

    let mut count = 0;
    for elem in elements.iter_mut() {
        let elem_type = elem.get("type").and_then(Value::as_str).unwrap_or_default();
        let display_name = elem
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or_default();

        // Check if this is a selectable table element
        if selectable_types.contains(&elem_type) {
            let should_modify = all_tables
                || table_names
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(display_name));
            if should_modify {
                elem["is_selected"] = Value::Bool(selected);
                count += 1;
            }
        }

        // Recurse into children
        if let Some(children) = elem.get_mut("children").and_then(Value::as_array_mut) {
            count += set_table_selection(children, table_names, all_tables, selected);
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use serde_json::json;

    use super::*;

    #[test]
    fn map_item_type_lakehouse() {
        assert_eq!(
            map_item_type_to_datasource_type("Lakehouse").unwrap(),
            "lakehouse_tables"
        );
    }

    #[test]
    fn map_item_type_warehouse() {
        assert_eq!(
            map_item_type_to_datasource_type("Warehouse").unwrap(),
            "data_warehouse"
        );
    }

    #[test]
    fn map_item_type_kql_database() {
        assert_eq!(
            map_item_type_to_datasource_type("KQLDatabase").unwrap(),
            "kusto"
        );
    }

    #[test]
    fn map_item_type_semantic_model() {
        assert_eq!(
            map_item_type_to_datasource_type("SemanticModel").unwrap(),
            "semantic_model"
        );
    }

    #[test]
    fn map_item_type_mirrored_database() {
        assert_eq!(
            map_item_type_to_datasource_type("MirroredDatabase").unwrap(),
            "mirrored_database"
        );
    }

    #[test]
    fn map_item_type_sql_database() {
        assert_eq!(
            map_item_type_to_datasource_type("SQLDatabase").unwrap(),
            "sql_database"
        );
    }

    #[test]
    fn map_item_type_unsupported() {
        let err = map_item_type_to_datasource_type("Notebook").unwrap_err();
        assert!(err.to_string().contains("Unsupported"));
    }

    #[test]
    fn map_item_type_case_insensitive() {
        assert_eq!(
            map_item_type_to_datasource_type("lakehouse").unwrap(),
            "lakehouse_tables"
        );
        assert_eq!(
            map_item_type_to_datasource_type("WAREHOUSE").unwrap(),
            "data_warehouse"
        );
    }

    #[test]
    fn extract_datasources_from_parts_finds_datasource() {
        let ds_json = json!({
            "artifactId": "aaa",
            "displayName": "TestLH",
            "type": "lakehouse_tables"
        });
        let payload = BASE64.encode(ds_json.to_string().as_bytes());
        let parts = vec![
            json!({
                "path": "Files/Config/data_agent.json",
                "payload": "e30=",
                "payloadType": "InlineBase64"
            }),
            json!({
                "path": "Files/Config/draft/lakehouse_tables-TestLH/datasource.json",
                "payload": payload,
                "payloadType": "InlineBase64"
            }),
        ];

        let datasources = extract_datasources_from_parts(&parts);
        assert_eq!(datasources.len(), 1);
        assert_eq!(datasources[0]["displayName"], "TestLH");
        assert_eq!(datasources[0]["type"], "lakehouse_tables");
    }

    #[test]
    fn extract_datasources_from_parts_empty() {
        let parts = vec![json!({
            "path": "Files/Config/data_agent.json",
            "payload": "e30=",
            "payloadType": "InlineBase64"
        })];
        let datasources = extract_datasources_from_parts(&parts);
        assert!(datasources.is_empty());
    }

    #[test]
    fn set_table_selection_selects_by_name() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "is_selected": false,
            "children": [
                {"display_name": "orders", "type": "lakehouse_tables.table", "is_selected": false, "children": []},
                {"display_name": "products", "type": "lakehouse_tables.table", "is_selected": false, "children": []}
            ]
        })];

        let count = set_table_selection(&mut elements, &["orders"], false, true);
        assert_eq!(count, 1);
        let children = elements[0]["children"].as_array().unwrap();
        assert_eq!(children[0]["is_selected"], true);
        assert_eq!(children[1]["is_selected"], false);
    }

    #[test]
    fn set_table_selection_selects_all() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "is_selected": false,
            "children": [
                {"display_name": "orders", "type": "lakehouse_tables.table", "is_selected": false, "children": []},
                {"display_name": "products", "type": "lakehouse_tables.table", "is_selected": false, "children": []}
            ]
        })];

        let count = set_table_selection(&mut elements, &[], true, true);
        assert_eq!(count, 2);
        let children = elements[0]["children"].as_array().unwrap();
        assert_eq!(children[0]["is_selected"], true);
        assert_eq!(children[1]["is_selected"], true);
    }

    #[test]
    fn set_table_selection_unselects() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "is_selected": true,
            "children": [
                {"display_name": "orders", "type": "lakehouse_tables.table", "is_selected": true, "children": []}
            ]
        })];

        let count = set_table_selection(&mut elements, &["orders"], false, false);
        assert_eq!(count, 1);
        let children = elements[0]["children"].as_array().unwrap();
        assert_eq!(children[0]["is_selected"], false);
    }

    #[test]
    fn set_table_selection_case_insensitive() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "children": [
                {"display_name": "Orders", "type": "lakehouse_tables.table", "is_selected": false, "children": []}
            ]
        })];

        let count = set_table_selection(&mut elements, &["orders"], false, true);
        assert_eq!(count, 1);
    }
}
