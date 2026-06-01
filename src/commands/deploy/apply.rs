use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};
use tokio::sync::Semaphore;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::FabioError;

use super::changeset::{Change, ChangeAction, Changeset, DeployFailure, DeployResult};
use super::ordering::{delete_priority, deploy_priority, topological_sort};
use super::platform::SourceWorkspace;

/// Write a progress line to stderr (diagnostics channel).
/// Only emits when stderr is connected (non-quiet mode).
fn emit_progress(quiet: bool, msg: &str) {
    if !quiet {
        eprintln!("[deploy] {msg}");
    }
}

/// Execute the deployment changeset.
///
/// Items are deployed in dependency order (by type), with parallelism within each type batch.
/// Deletes happen last, in reverse dependency order.
#[allow(clippy::too_many_lines)]
pub async fn execute_changeset(
    cli: &Cli,
    client: &FabricClient,
    workspace_id: &str,
    changeset: &Changeset,
    source: &SourceWorkspace,
    concurrency: usize,
    fail_fast: bool,
) -> Result<DeployResult> {
    let start = Instant::now();

    let mut succeeded: Vec<Change> = Vec::new();
    let mut failed: Vec<DeployFailure> = Vec::new();
    let mut skipped: Vec<Change> = Vec::new();

    // Separate changes by action
    let mut creates_updates: Vec<&Change> = Vec::new();
    let mut deletes: Vec<&Change> = Vec::new();

    for change in &changeset.changes {
        match change.action {
            ChangeAction::Create | ChangeAction::Update | ChangeAction::Rename => {
                creates_updates.push(change);
            }
            ChangeAction::Delete => deletes.push(change),
            ChangeAction::Skip => skipped.push(change.clone()),
        }
    }

    // Group creates/updates by type and sort by deploy priority
    let mut type_groups: HashMap<&str, Vec<&Change>> = HashMap::new();
    for change in &creates_updates {
        type_groups
            .entry(change.item_type.as_str())
            .or_default()
            .push(change);
    }

    let mut sorted_types: Vec<&str> = type_groups.keys().copied().collect();
    sorted_types.sort_by_key(|t| deploy_priority(t));

    // Build lookup for source items by (type, name) for definition access
    let source_map: HashMap<(&str, &str), usize> = source
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            (
                (
                    item.metadata.item_type.as_str(),
                    item.metadata.display_name.as_str(),
                ),
                i,
            )
        })
        .collect();

    // Map to track created items: (type, name) → deployed GUID
    // Used for logical ID resolution in subsequent batches
    let mut created_ids: HashMap<(String, String), String> = HashMap::new();

    // Execute creates/updates in type order
    let total_changes = creates_updates.len();
    let completed = AtomicUsize::new(0);

    for item_type in &sorted_types {
        let group = &type_groups[item_type];

        emit_progress(
            cli.quiet,
            &format!(
                "deploying {} {} item(s) [{}/{}]",
                group.len(),
                item_type,
                completed.load(Ordering::Relaxed),
                total_changes
            ),
        );

        // For DataPipeline, do topological sort within the group
        let ordered_changes = if *item_type == "DataPipeline" {
            order_pipelines(group, source, &source_map)?
        } else {
            group.clone()
        };

        // Execute items in this type batch with bounded concurrency
        let batch_concurrency = concurrency.max(1);

        if batch_concurrency == 1 || ordered_changes.len() <= 1 {
            // Sequential execution (preserves ordering for pipelines or single items)
            for change in &ordered_changes {
                if fail_fast && !failed.is_empty() {
                    break;
                }

                let result = execute_single_change(
                    cli,
                    client,
                    workspace_id,
                    change,
                    source,
                    &source_map,
                    &created_ids,
                )
                .await;

                match result {
                    Ok(deployed_id) => {
                        let mut change_result = (*change).clone();
                        if let Some(ref id) = deployed_id {
                            created_ids
                                .insert((change.item_type.clone(), change.name.clone()), id.clone());
                            change_result.deployed_id = Some(id.clone());
                        }
                        succeeded.push(change_result);
                        let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                        emit_progress(
                            cli.quiet,
                            &format!(
                                "  {} {} \"{}\" [{}/{}]",
                                match change.action {
                                    ChangeAction::Create => "created",
                                    ChangeAction::Rename => "renamed",
                                    _ => "updated",
                                },
                                change.item_type,
                                change.name,
                                done,
                                total_changes
                            ),
                        );
                    }
                    Err(e) => {
                        completed.fetch_add(1, Ordering::Relaxed);
                        emit_progress(
                            cli.quiet,
                            &format!(
                                "  FAILED {} \"{}\" : {}",
                                change.item_type,
                                change.name,
                                e.root_cause()
                            ),
                        );
                        failed.push(DeployFailure {
                            change: (*change).clone(),
                            error: e.to_string(),
                            code: extract_error_code(&e),
                        });
                    }
                }
            }
        } else {
            // Parallel execution within type batch (non-pipeline types)
            let semaphore = Arc::new(Semaphore::new(batch_concurrency));
            let mut handles = Vec::with_capacity(ordered_changes.len());
            let dry_run = cli.dry_run;

            for change in &ordered_changes {
                let sem = Arc::clone(&semaphore);
                let change_owned = (*change).clone();
                let ws_id = workspace_id.to_owned();
                let client_clone = client.clone();

                // Find source item index for this change
                let src_idx = source_map
                    .get(&(change.item_type.as_str(), change.name.as_str()))
                    .copied();

                let source_item = src_idx.map(|idx| source.items[idx].clone());

                // Build resolution map snapshot for this batch
                let res_map = build_resolution_map(source, &created_ids);

                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    let result = execute_single_change_owned(
                        dry_run,
                        &client_clone,
                        &ws_id,
                        &change_owned,
                        source_item.as_ref(),
                        &res_map,
                    )
                    .await;
                    (change_owned, result)
                }));
            }

            // Collect results
            for handle in handles {
                let (mut change_owned, result) = handle.await?;
                match result {
                    Ok(deployed_id) => {
                        if let Some(ref id) = deployed_id {
                            created_ids.insert(
                                (change_owned.item_type.clone(), change_owned.name.clone()),
                                id.clone(),
                            );
                            change_owned.deployed_id = Some(id.clone());
                        }
                        let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                        emit_progress(
                            cli.quiet,
                            &format!(
                                "  {} {} \"{}\" [{}/{}]",
                                if change_owned.action == ChangeAction::Create {
                                    "created"
                                } else {
                                    "updated"
                                },
                                change_owned.item_type,
                                change_owned.name,
                                done,
                                total_changes
                            ),
                        );
                        succeeded.push(change_owned);
                    }
                    Err(e) => {
                        completed.fetch_add(1, Ordering::Relaxed);
                        emit_progress(
                            cli.quiet,
                            &format!(
                                "  FAILED {} \"{}\" : {}",
                                change_owned.item_type,
                                change_owned.name,
                                e.root_cause()
                            ),
                        );
                        if fail_fast {
                            failed.push(DeployFailure {
                                change: change_owned,
                                error: e.to_string(),
                                code: extract_error_code(&e),
                            });
                            break;
                        }
                        failed.push(DeployFailure {
                            change: change_owned,
                            error: e.to_string(),
                            code: extract_error_code(&e),
                        });
                    }
                }
            }
        }
    }

    // Execute deletes in reverse dependency order (always sequential for safety)
    let mut deletes_sorted = deletes;
    deletes_sorted.sort_by_key(|c| delete_priority(&c.item_type));

    if !deletes_sorted.is_empty() {
        emit_progress(
            cli.quiet,
            &format!("deleting {} orphaned item(s)", deletes_sorted.len()),
        );
    }

    for (i, change) in deletes_sorted.iter().enumerate() {
        if fail_fast && !failed.is_empty() {
            break;
        }

        let result = execute_delete(client, workspace_id, change).await;
        match result {
            Ok(()) => {
                emit_progress(
                    cli.quiet,
                    &format!(
                        "  deleted {} \"{}\" [{}/{}]",
                        change.item_type,
                        change.name,
                        i + 1,
                        deletes_sorted.len()
                    ),
                );
                succeeded.push((*change).clone());
            }
            Err(e) => {
                emit_progress(
                    cli.quiet,
                    &format!(
                        "  FAILED delete {} \"{}\" : {}",
                        change.item_type,
                        change.name,
                        e.root_cause()
                    ),
                );
                failed.push(DeployFailure {
                    change: (*change).clone(),
                    error: e.to_string(),
                    code: extract_error_code(&e),
                });
            }
        }
    }

    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    Ok(DeployResult {
        succeeded,
        failed,
        skipped,
        duration_ms,
    })
}

/// Execute post-deploy hooks for items that were successfully deployed.
///
/// Hooks:
/// - **Semantic Model**: Triggers `POST /datasets/{id}/refreshes` (framing refresh for Direct Lake)
/// - **Environment**: Triggers `POST /environments/{id}/staging/publish` (publishes staged changes)
///
/// Returns a list of hook result objects (for inclusion in the deploy output).
pub async fn execute_post_hooks(
    cli: &Cli,
    client: &FabricClient,
    workspace_id: &str,
    succeeded: &[Change],
) -> Vec<Value> {
    let mut results: Vec<Value> = Vec::new();

    for change in succeeded {
        match change.action {
            ChangeAction::Create | ChangeAction::Update | ChangeAction::Rename => {}
            _ => continue,
        }

        let Some(ref item_id) = change.deployed_id else {
            continue;
        };

        match change.item_type.as_str() {
            "SemanticModel" => {
                emit_progress(
                    cli.quiet,
                    &format!("post-hook: refreshing semantic model \"{}\"", change.name),
                );
                let path = format!(
                    "/workspaces/{workspace_id}/semanticModels/{item_id}/refreshes"
                );
                let body = json!({"type": "Full"});
                match client.post(&path, &body, false).await {
                    Ok(_) => {
                        results.push(json!({
                            "hook": "refresh",
                            "item_type": "SemanticModel",
                            "item_name": change.name,
                            "status": "triggered"
                        }));
                    }
                    Err(e) => {
                        emit_progress(
                            cli.quiet,
                            &format!(
                                "  post-hook FAILED: refresh semantic model \"{}\": {}",
                                change.name,
                                e.root_cause()
                            ),
                        );
                        results.push(json!({
                            "hook": "refresh",
                            "item_type": "SemanticModel",
                            "item_name": change.name,
                            "status": "failed",
                            "error": e.to_string()
                        }));
                    }
                }
            }
            "Environment" => {
                emit_progress(
                    cli.quiet,
                    &format!("post-hook: publishing environment \"{}\"", change.name),
                );
                let path = format!(
                    "/workspaces/{workspace_id}/environments/{item_id}/staging/publish"
                );
                let body = json!({});
                match client.post(&path, &body, false).await {
                    Ok(_) => {
                        results.push(json!({
                            "hook": "publish",
                            "item_type": "Environment",
                            "item_name": change.name,
                            "status": "triggered"
                        }));
                    }
                    Err(e) => {
                        emit_progress(
                            cli.quiet,
                            &format!(
                                "  post-hook FAILED: publish environment \"{}\": {}",
                                change.name,
                                e.root_cause()
                            ),
                        );
                        results.push(json!({
                            "hook": "publish",
                            "item_type": "Environment",
                            "item_name": change.name,
                            "status": "failed",
                            "error": e.to_string()
                        }));
                    }
                }
            }
            _ => {}
        }
    }

    results
}

/// Execute a single create or update operation.
///
/// Returns the deployed item GUID on success.
async fn execute_single_change(
    cli: &Cli,
    client: &FabricClient,
    workspace_id: &str,
    change: &Change,
    source: &SourceWorkspace,
    source_map: &HashMap<(&str, &str), usize>,
    created_ids: &HashMap<(String, String), String>,
) -> Result<Option<String>> {
    let source_idx = source_map
        .get(&(change.item_type.as_str(), change.name.as_str()))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Source item not found for {} \"{}\"",
                change.item_type,
                change.name
            )
        })?;

    let source_item = &source.items[*source_idx];

    // Build logical ID resolution map from created_ids + source workspace info
    let resolution_map = build_resolution_map(source, created_ids);

    deploy_change(
        cli.dry_run,
        client,
        workspace_id,
        change,
        source_item,
        &resolution_map,
    )
    .await
}

/// Execute a single create or update with owned data (for parallel spawned tasks).
async fn execute_single_change_owned(
    dry_run: bool,
    client: &FabricClient,
    workspace_id: &str,
    change: &Change,
    source_item: Option<&super::platform::SourceItem>,
    resolution_map: &HashMap<String, String>,
) -> Result<Option<String>> {
    let source_item = source_item.ok_or_else(|| {
        anyhow::anyhow!(
            "Source item not found for {} \"{}\"",
            change.item_type,
            change.name
        )
    })?;
    deploy_change(
        dry_run,
        client,
        workspace_id,
        change,
        source_item,
        resolution_map,
    )
    .await
}

/// Core deploy logic shared by sequential and parallel paths.
#[allow(clippy::option_if_let_else, clippy::too_many_lines)]
async fn deploy_change(
    dry_run: bool,
    client: &FabricClient,
    workspace_id: &str,
    change: &Change,
    source_item: &super::platform::SourceItem,
    resolution_map: &HashMap<String, String>,
) -> Result<Option<String>> {
    // Build definition parts for API, applying logical ID resolution
    let parts: Vec<Value> = source_item
        .parts
        .iter()
        .map(|p| {
            let payload = resolve_logical_ids_in_payload(&p.payload, resolution_map);
            json!({
                "path": p.path,
                "payload": payload,
                "payloadType": p.payload_type,
            })
        })
        .collect();

    match change.action {
        ChangeAction::Create => {
            // Omit definition entirely when there are no parts (e.g. Lakehouse, MLModel)
            let mut body = if parts.is_empty() {
                json!({
                    "displayName": change.name,
                    "type": change.item_type
                })
            } else {
                let definition =
                    if let Some(ref fmt) = source_item.metadata.definition_format {
                        json!({ "format": fmt, "parts": parts })
                    } else {
                        json!({ "parts": parts })
                    };
                json!({
                    "displayName": change.name,
                    "type": change.item_type,
                    "definition": definition
                })
            };

            // Include creationPayload if present (e.g. KQLDatabase eventhouse-id)
            if let Some(ref payload) = source_item.creation_payload {
                body.as_object_mut()
                    .unwrap()
                    .insert("creationPayload".to_owned(), payload.clone());
            }

            // Include description if present in source metadata
            if let Some(ref desc) = source_item.metadata.description {
                body.as_object_mut()
                    .unwrap()
                    .insert("description".to_owned(), Value::String(desc.clone()));
            }

            if dry_run {
                return Ok(None);
            }

            let result = client
                .post(&format!("/workspaces/{workspace_id}/items"), &body, true)
                .await?;

            let id = result.get("id").and_then(|v| v.as_str()).map(str::to_owned);

            Ok(id)
        }
        ChangeAction::Update => {
            let deployed_id = change.deployed_id.as_deref().ok_or_else(|| {
                anyhow::anyhow!(
                    "No deployed ID for update of {} \"{}\"",
                    change.item_type,
                    change.name
                )
            })?;

            // Skip updateDefinition when there are no parts (nothing to update)
            if parts.is_empty() {
                return Ok(Some(deployed_id.to_owned()));
            }

            let definition = if let Some(ref fmt) = source_item.metadata.definition_format {
                json!({ "format": fmt, "parts": parts })
            } else {
                json!({ "parts": parts })
            };

            let body = json!({
                "definition": definition
            });

            if dry_run {
                return Ok(Some(deployed_id.to_owned()));
            }

            client
                .post(
                    &format!("/workspaces/{workspace_id}/items/{deployed_id}/updateDefinition"),
                    &body,
                    true,
                )
                .await?;

            Ok(Some(deployed_id.to_owned()))
        }
        ChangeAction::Rename => {
            let deployed_id = change.deployed_id.as_deref().ok_or_else(|| {
                anyhow::anyhow!(
                    "No deployed ID for rename of {} \"{}\"",
                    change.item_type,
                    change.name
                )
            })?;

            if dry_run {
                return Ok(Some(deployed_id.to_owned()));
            }

            // Step 1: Rename the item via PATCH
            let patch_body = if let Some(ref desc) = source_item.metadata.description {
                json!({ "displayName": change.name, "description": desc })
            } else {
                json!({ "displayName": change.name })
            };

            client
                .patch(
                    &format!("/workspaces/{workspace_id}/items/{deployed_id}"),
                    &patch_body,
                )
                .await?;

            // Step 2: Update definition if there are parts
            if !parts.is_empty() {
                let definition =
                    if let Some(ref fmt) = source_item.metadata.definition_format {
                        json!({ "format": fmt, "parts": parts })
                    } else {
                        json!({ "parts": parts })
                    };

                let body = json!({ "definition": definition });

                client
                    .post(
                        &format!(
                            "/workspaces/{workspace_id}/items/{deployed_id}/updateDefinition"
                        ),
                        &body,
                        true,
                    )
                    .await?;
            }

            Ok(Some(deployed_id.to_owned()))
        }
        _ => Ok(None),
    }
}

/// Execute a delete operation.
async fn execute_delete(client: &FabricClient, workspace_id: &str, change: &Change) -> Result<()> {
    let deployed_id = change.deployed_id.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "No deployed ID for delete of {} \"{}\"",
            change.item_type,
            change.name
        )
    })?;

    client
        .delete(&format!("/workspaces/{workspace_id}/items/{deployed_id}"))
        .await?;

    Ok(())
}

/// Order `DataPipeline` changes by their internal references (sub-pipeline invocations).
fn order_pipelines<'a>(
    changes: &[&'a Change],
    source: &SourceWorkspace,
    source_map: &HashMap<(&str, &str), usize>,
) -> Result<Vec<&'a Change>> {
    if changes.len() <= 1 {
        return Ok(changes.to_vec());
    }

    // Extract pipeline references from definitions
    let mut items_with_refs: Vec<(String, Vec<String>)> = Vec::new();

    for change in changes {
        let refs = source_map
            .get(&("DataPipeline", change.name.as_str()))
            .map_or_else(Vec::new, |idx| {
                extract_pipeline_references(&source.items[*idx])
            });
        items_with_refs.push((change.name.clone(), refs));
    }

    let sorted_names = topological_sort(&items_with_refs)?;

    // Reorder changes to match sorted order
    let change_map: HashMap<&str, &'a Change> =
        changes.iter().map(|c| (c.name.as_str(), *c)).collect();

    let mut ordered = Vec::with_capacity(changes.len());
    for name in &sorted_names {
        if let Some(change) = change_map.get(name.as_str()) {
            ordered.push(*change);
        }
    }

    Ok(ordered)
}

/// Extract names of other `DataPipelines` referenced by this pipeline's definition.
///
/// Looks for "Execute Pipeline" activity references in the pipeline JSON.
fn extract_pipeline_references(source_item: &super::platform::SourceItem) -> Vec<String> {
    let mut refs = Vec::new();

    for part in &source_item.parts {
        // Only check pipeline content files
        if !part.path.contains("pipeline")
            && !std::path::Path::new(&part.path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        {
            continue;
        }

        let Ok(decoded) = BASE64.decode(&part.payload) else {
            continue;
        };
        let Ok(content) = String::from_utf8(decoded) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<Value>(&content) else {
            continue;
        };

        // Look for ExecutePipeline activities
        if let Some(activities) = json
            .get("properties")
            .and_then(|p| p.get("activities"))
            .and_then(|a| a.as_array())
        {
            for activity in activities {
                let is_execute_pipeline = activity
                    .get("type")
                    .and_then(|t| t.as_str())
                    .is_some_and(|t| t == "ExecutePipeline");

                if is_execute_pipeline {
                    if let Some(name) = activity
                        .get("typeProperties")
                        .and_then(|tp| tp.get("pipeline"))
                        .and_then(|p| p.get("referenceName"))
                        .and_then(|n| n.as_str())
                    {
                        refs.push(name.to_owned());
                    }
                }
            }
        }
    }

    refs
}

/// Build a map from logical IDs to deployed (runtime) IDs.
///
/// This enables cross-item reference resolution: when item A's definition references
/// item B by its logical ID, we replace it with item B's actual deployed GUID.
///
/// Sources of resolution:
/// 1. `created_ids` — items created earlier in this apply session `(type, name)` → `deployed_id`
/// 2. Source workspace — items that have both a `logical_id` and a `deployed_id` in the changeset
fn build_resolution_map(
    source: &SourceWorkspace,
    created_ids: &HashMap<(String, String), String>,
) -> HashMap<String, String> {
    let mut map = HashMap::new();

    // Map logical_id → deployed_id from created_ids
    for ((item_type, name), deployed_id) in created_ids {
        // Find the source item's logical_id
        if let Some(&idx) = source
            .type_name_index
            .get(&(item_type.clone(), name.clone()))
        {
            if let Some(ref logical_id) = source.items[idx].metadata.logical_id {
                map.insert(logical_id.clone(), deployed_id.clone());
            }
        }
    }

    map
}

/// Replace logical IDs found in a base64-encoded definition payload with deployed IDs.
///
/// Decodes the payload, performs string replacement for any GUIDs in the resolution map,
/// and re-encodes. If the payload is not valid UTF-8 or contains no matches, returns
/// the original payload unchanged.
fn resolve_logical_ids_in_payload(
    payload: &str,
    resolution_map: &HashMap<String, String>,
) -> String {
    if resolution_map.is_empty() {
        return payload.to_owned();
    }

    let Ok(decoded) = BASE64.decode(payload) else {
        return payload.to_owned();
    };
    let Ok(mut content) = String::from_utf8(decoded) else {
        return payload.to_owned();
    };

    let mut replaced = false;
    for (logical_id, deployed_id) in resolution_map {
        if content.contains(logical_id.as_str()) {
            content = content.replace(logical_id.as_str(), deployed_id.as_str());
            replaced = true;
        }
    }

    if replaced {
        BASE64.encode(content.as_bytes())
    } else {
        payload.to_owned()
    }
}

fn extract_error_code(err: &anyhow::Error) -> String {
    err.downcast_ref::<FabioError>().map_or_else(
        || "UNKNOWN".to_owned(),
        |fabio_err| format!("{:?}", fabio_err.code),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;

    use super::super::changeset::ChangeAction;
    use super::super::platform::{DefinitionPart, SourceItem};

    #[test]
    fn test_extract_pipeline_references_empty() {
        let item = SourceItem {
            metadata: super::super::platform::PlatformMetadata {
                item_type: "DataPipeline".to_owned(),
                display_name: "Test".to_owned(),
                logical_id: None,
                description: None,
                definition_format: None,
            },
            parts: vec![],
            content_hash: "sha256:abc".to_owned(),
            source_path: std::path::PathBuf::from("/tmp"),
            creation_payload: None,
        };

        let refs = extract_pipeline_references(&item);
        assert!(refs.is_empty());
    }

    #[test]
    fn test_extract_pipeline_references_finds_execute_pipeline() {
        let pipeline_json = serde_json::json!({
            "properties": {
                "activities": [
                    {
                        "name": "Run Sub Pipeline",
                        "type": "ExecutePipeline",
                        "typeProperties": {
                            "pipeline": {
                                "referenceName": "ChildPipeline"
                            }
                        }
                    },
                    {
                        "name": "Copy Data",
                        "type": "Copy",
                        "typeProperties": {}
                    }
                ]
            }
        });
        let payload = BASE64.encode(serde_json::to_string(&pipeline_json).unwrap().as_bytes());

        let item = SourceItem {
            metadata: super::super::platform::PlatformMetadata {
                item_type: "DataPipeline".to_owned(),
                display_name: "ParentPipeline".to_owned(),
                logical_id: None,
                description: None,
                definition_format: None,
            },
            parts: vec![DefinitionPart {
                path: "pipeline-content.json".to_owned(),
                payload,
                payload_type: "InlineBase64".to_owned(),
            }],
            content_hash: "sha256:abc".to_owned(),
            source_path: std::path::PathBuf::from("/tmp"),
            creation_payload: None,
        };

        let refs = extract_pipeline_references(&item);
        assert_eq!(refs, vec!["ChildPipeline"]);
    }

    #[test]
    fn test_extract_pipeline_references_multiple() {
        let pipeline_json = serde_json::json!({
            "properties": {
                "activities": [
                    {
                        "name": "Step 1",
                        "type": "ExecutePipeline",
                        "typeProperties": {
                            "pipeline": {"referenceName": "PipeA"}
                        }
                    },
                    {
                        "name": "Step 2",
                        "type": "ExecutePipeline",
                        "typeProperties": {
                            "pipeline": {"referenceName": "PipeB"}
                        }
                    }
                ]
            }
        });
        let payload = BASE64.encode(serde_json::to_string(&pipeline_json).unwrap().as_bytes());

        let item = SourceItem {
            metadata: super::super::platform::PlatformMetadata {
                item_type: "DataPipeline".to_owned(),
                display_name: "Orchestrator".to_owned(),
                logical_id: None,
                description: None,
                definition_format: None,
            },
            parts: vec![DefinitionPart {
                path: "pipeline-content.json".to_owned(),
                payload,
                payload_type: "InlineBase64".to_owned(),
            }],
            content_hash: "sha256:abc".to_owned(),
            source_path: std::path::PathBuf::from("/tmp"),
            creation_payload: None,
        };

        let refs = extract_pipeline_references(&item);
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&"PipeA".to_owned()));
        assert!(refs.contains(&"PipeB".to_owned()));
    }

    #[test]
    fn test_deploy_priority_ordering() {
        // Lakehouse should deploy before Notebook, which should deploy before DataPipeline
        let lh = deploy_priority("Lakehouse");
        let nb = deploy_priority("Notebook");
        let dp = deploy_priority("DataPipeline");
        assert!(lh < nb);
        assert!(nb < dp);
    }

    #[test]
    fn test_delete_priority_is_reversed() {
        // Deletes happen in reverse: DataPipeline before Notebook before Lakehouse
        let lh = delete_priority("Lakehouse");
        let nb = delete_priority("Notebook");
        let dp = delete_priority("DataPipeline");
        assert!(dp < nb);
        assert!(nb < lh);
    }

    #[test]
    fn test_order_changes_by_type_priority() {
        let changes = vec![
            Change {
                name: "MyPipeline".to_owned(),
                item_type: "DataPipeline".to_owned(),
                action: ChangeAction::Create,
                reason: "new".to_owned(),
                logical_id: None,
                deployed_id: None,
                source_hash: None,
                previous_name: None,
            },
            Change {
                name: "MyNotebook".to_owned(),
                item_type: "Notebook".to_owned(),
                action: ChangeAction::Create,
                reason: "new".to_owned(),
                logical_id: None,
                deployed_id: None,
                source_hash: None,
                previous_name: None,
            },
            Change {
                name: "MyLH".to_owned(),
                item_type: "Lakehouse".to_owned(),
                action: ChangeAction::Create,
                reason: "new".to_owned(),
                logical_id: None,
                deployed_id: None,
                source_hash: None,
                previous_name: None,
            },
        ];

        // Group by type and sort by deploy_priority
        let mut groups: Vec<(&str, Vec<&Change>)> = Vec::new();
        let mut type_groups: HashMap<&str, Vec<&Change>> = HashMap::new();
        for c in &changes {
            type_groups.entry(c.item_type.as_str()).or_default().push(c);
        }
        let mut sorted_types: Vec<&str> = type_groups.keys().copied().collect();
        sorted_types.sort_by_key(|t| deploy_priority(t));
        for t in sorted_types {
            groups.push((t, type_groups.remove(t).unwrap()));
        }

        assert_eq!(groups[0].0, "Lakehouse");
        assert_eq!(groups[1].0, "Notebook");
        assert_eq!(groups[2].0, "DataPipeline");
    }

    #[test]
    fn test_resolve_logical_ids_no_match() {
        let payload = BASE64.encode(b"some content without any guids");
        let map = HashMap::from([(
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_owned(),
            "11111111-2222-3333-4444-555555555555".to_owned(),
        )]);

        let result = resolve_logical_ids_in_payload(&payload, &map);
        assert_eq!(result, payload); // unchanged
    }

    #[test]
    fn test_resolve_logical_ids_with_match() {
        let content =
            r#"{"lakehouseId": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "other": "value"}"#;
        let payload = BASE64.encode(content.as_bytes());
        let map = HashMap::from([(
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_owned(),
            "11111111-2222-3333-4444-555555555555".to_owned(),
        )]);

        let result = resolve_logical_ids_in_payload(&payload, &map);
        let decoded = String::from_utf8(BASE64.decode(&result).unwrap()).unwrap();
        assert!(decoded.contains("11111111-2222-3333-4444-555555555555"));
        assert!(!decoded.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"));
    }

    #[test]
    fn test_resolve_logical_ids_multiple_occurrences() {
        let content =
            "ref1=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee ref2=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        let payload = BASE64.encode(content.as_bytes());
        let map = HashMap::from([(
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_owned(),
            "99999999-0000-1111-2222-333333333333".to_owned(),
        )]);

        let result = resolve_logical_ids_in_payload(&payload, &map);
        let decoded = String::from_utf8(BASE64.decode(&result).unwrap()).unwrap();
        // Both occurrences replaced
        assert_eq!(
            decoded
                .matches("99999999-0000-1111-2222-333333333333")
                .count(),
            2
        );
    }

    #[test]
    fn test_resolve_logical_ids_empty_map() {
        let payload = BASE64.encode(b"anything here");
        let map: HashMap<String, String> = HashMap::new();

        let result = resolve_logical_ids_in_payload(&payload, &map);
        assert_eq!(result, payload); // no-op
    }

    #[test]
    fn test_resolve_logical_ids_invalid_base64() {
        let payload = "not-valid-base64!!!";
        let map = HashMap::from([("foo".to_owned(), "bar".to_owned())]);

        let result = resolve_logical_ids_in_payload(payload, &map);
        assert_eq!(result, payload); // returns original
    }

    #[test]
    fn test_build_resolution_map_from_created_ids() {
        use super::super::platform::{PlatformMetadata, SourceWorkspace};

        let source = SourceWorkspace {
            items: vec![SourceItem {
                metadata: PlatformMetadata {
                    item_type: "Lakehouse".to_owned(),
                    display_name: "SalesLH".to_owned(),
                    logical_id: Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_owned()),
                    description: None,
                    definition_format: None,
                },
                parts: vec![],
                content_hash: "sha256:abc".to_owned(),
                source_path: std::path::PathBuf::from("/tmp"),
            creation_payload: None,
            }],
            logical_id_index: HashMap::from([(
                "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_owned(),
                0,
            )]),
            type_name_index: HashMap::from([(("Lakehouse".to_owned(), "SalesLH".to_owned()), 0)]),
        };

        let created_ids = HashMap::from([(
            ("Lakehouse".to_owned(), "SalesLH".to_owned()),
            "11111111-2222-3333-4444-555555555555".to_owned(),
        )]);

        let map = build_resolution_map(&source, &created_ids);
        assert_eq!(
            map.get("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
            Some(&"11111111-2222-3333-4444-555555555555".to_owned())
        );
    }

    #[test]
    fn test_build_resolution_map_no_logical_id() {
        use super::super::platform::{PlatformMetadata, SourceWorkspace};

        let source = SourceWorkspace {
            items: vec![SourceItem {
                metadata: PlatformMetadata {
                    item_type: "Notebook".to_owned(),
                    display_name: "MyNB".to_owned(),
                    logical_id: None, // no logical ID
                    description: None,
                    definition_format: None,
                },
                parts: vec![],
                content_hash: "sha256:abc".to_owned(),
                source_path: std::path::PathBuf::from("/tmp"),
            creation_payload: None,
            }],
            logical_id_index: HashMap::new(),
            type_name_index: HashMap::from([(("Notebook".to_owned(), "MyNB".to_owned()), 0)]),
        };

        let created_ids = HashMap::from([(
            ("Notebook".to_owned(), "MyNB".to_owned()),
            "22222222-3333-4444-5555-666666666666".to_owned(),
        )]);

        let map = build_resolution_map(&source, &created_ids);
        // No entry — item has no logical_id so it can't be referenced
        assert!(map.is_empty());
    }

    #[test]
    fn test_build_resolution_map_multiple_items() {
        use super::super::platform::{PlatformMetadata, SourceWorkspace};

        let source = SourceWorkspace {
            items: vec![
                SourceItem {
                    metadata: PlatformMetadata {
                        item_type: "Lakehouse".to_owned(),
                        display_name: "LH1".to_owned(),
                        logical_id: Some("lid-lh1".to_owned()),
                        description: None,
                        definition_format: None,
                    },
                    parts: vec![],
                    content_hash: "sha256:abc".to_owned(),
                    source_path: std::path::PathBuf::from("/tmp"),
            creation_payload: None,
                },
                SourceItem {
                    metadata: PlatformMetadata {
                        item_type: "Lakehouse".to_owned(),
                        display_name: "LH2".to_owned(),
                        logical_id: Some("lid-lh2".to_owned()),
                        description: None,
                        definition_format: None,
                    },
                    parts: vec![],
                    content_hash: "sha256:def".to_owned(),
                    source_path: std::path::PathBuf::from("/tmp"),
            creation_payload: None,
                },
                SourceItem {
                    metadata: PlatformMetadata {
                        item_type: "Notebook".to_owned(),
                        display_name: "NB1".to_owned(),
                        logical_id: Some("lid-nb1".to_owned()),
                        description: None,
                        definition_format: None,
                    },
                    parts: vec![],
                    content_hash: "sha256:ghi".to_owned(),
                    source_path: std::path::PathBuf::from("/tmp"),
            creation_payload: None,
                },
            ],
            logical_id_index: HashMap::from([
                ("lid-lh1".to_owned(), 0),
                ("lid-lh2".to_owned(), 1),
                ("lid-nb1".to_owned(), 2),
            ]),
            type_name_index: HashMap::from([
                (("Lakehouse".to_owned(), "LH1".to_owned()), 0),
                (("Lakehouse".to_owned(), "LH2".to_owned()), 1),
                (("Notebook".to_owned(), "NB1".to_owned()), 2),
            ]),
        };

        let created_ids = HashMap::from([
            (
                ("Lakehouse".to_owned(), "LH1".to_owned()),
                "deployed-id-lh1".to_owned(),
            ),
            (
                ("Notebook".to_owned(), "NB1".to_owned()),
                "deployed-id-nb1".to_owned(),
            ),
        ]);

        let map = build_resolution_map(&source, &created_ids);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("lid-lh1"), Some(&"deployed-id-lh1".to_owned()));
        assert_eq!(map.get("lid-nb1"), Some(&"deployed-id-nb1".to_owned()));
        // LH2 not in created_ids → not in resolution map
        assert!(!map.contains_key("lid-lh2"));
    }

    #[test]
    fn test_resolve_logical_ids_multiple_different_ids() {
        let content = r#"{"lh": "lid-aaa", "nb": "lid-bbb", "other": "no-match"}"#;
        let payload = BASE64.encode(content.as_bytes());
        let map = HashMap::from([
            ("lid-aaa".to_owned(), "deployed-aaa".to_owned()),
            ("lid-bbb".to_owned(), "deployed-bbb".to_owned()),
        ]);

        let result = resolve_logical_ids_in_payload(&payload, &map);
        let decoded = String::from_utf8(BASE64.decode(&result).unwrap()).unwrap();
        assert!(decoded.contains("deployed-aaa"));
        assert!(decoded.contains("deployed-bbb"));
        assert!(!decoded.contains("lid-aaa"));
        assert!(!decoded.contains("lid-bbb"));
        assert!(decoded.contains("no-match")); // untouched
    }

    #[test]
    fn test_resolve_logical_ids_non_utf8_payload() {
        // Binary payload that is not valid UTF-8
        let binary = vec![0xFF, 0xFE, 0x00, 0x01, 0x80, 0x90];
        let payload = BASE64.encode(&binary);
        let map = HashMap::from([("whatever".to_owned(), "replaced".to_owned())]);

        let result = resolve_logical_ids_in_payload(&payload, &map);
        assert_eq!(result, payload); // returned unchanged
    }

    #[test]
    fn test_resolve_logical_ids_partial_match_substring() {
        // Ensure that a logical ID that is a substring of another doesn't cause issues
        let content = r#"{"id1": "abc-123", "id2": "abc-123-extended"}"#;
        let payload = BASE64.encode(content.as_bytes());
        let map = HashMap::from([("abc-123".to_owned(), "REPLACED".to_owned())]);

        let result = resolve_logical_ids_in_payload(&payload, &map);
        let decoded = String::from_utf8(BASE64.decode(&result).unwrap()).unwrap();
        // Both occurrences of "abc-123" get replaced (including substring within longer string)
        assert!(decoded.contains("REPLACED"));
        assert!(!decoded.contains("abc-123\""));
    }

    #[test]
    fn test_build_resolution_map_ignores_items_not_in_source() {
        use super::super::platform::{PlatformMetadata, SourceWorkspace};

        let source = SourceWorkspace {
            items: vec![SourceItem {
                metadata: PlatformMetadata {
                    item_type: "Lakehouse".to_owned(),
                    display_name: "LH1".to_owned(),
                    logical_id: Some("lid-lh1".to_owned()),
                    description: None,
                    definition_format: None,
                },
                parts: vec![],
                content_hash: "sha256:abc".to_owned(),
                source_path: std::path::PathBuf::from("/tmp"),
            creation_payload: None,
            }],
            logical_id_index: HashMap::from([("lid-lh1".to_owned(), 0)]),
            type_name_index: HashMap::from([(("Lakehouse".to_owned(), "LH1".to_owned()), 0)]),
        };

        // created_ids has an item that doesn't exist in source type_name_index
        let created_ids = HashMap::from([
            (
                ("Lakehouse".to_owned(), "LH1".to_owned()),
                "deployed-lh1".to_owned(),
            ),
            (
                ("Notebook".to_owned(), "Ghost".to_owned()),
                "deployed-ghost".to_owned(),
            ),
        ]);

        let map = build_resolution_map(&source, &created_ids);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("lid-lh1"), Some(&"deployed-lh1".to_owned()));
    }

    #[test]
    fn test_extract_error_code_fabio_error() {
        use crate::errors::{ErrorCode, FabioError};
        let err = FabioError {
            code: ErrorCode::NotFound,
            message: "Not found".to_owned(),
            hint: None,
        };
        let anyhow_err: anyhow::Error = err.into();
        let code = extract_error_code(&anyhow_err);
        assert_eq!(code, "NotFound");
    }

    #[test]
    fn test_extract_error_code_unknown_error() {
        let err = anyhow::anyhow!("some random error");
        let code = extract_error_code(&err);
        assert_eq!(code, "UNKNOWN");
    }
}
