use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use serde_json::{Value, json};
use tokio::sync::Semaphore;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::FabioError;

use super::changeset::{Change, ChangeAction, Changeset, DeployFailure, DeployResult};
use super::ordering::{delete_priority, deploy_priority, topological_sort};
use super::platform::SourceWorkspace;

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
            ChangeAction::Create | ChangeAction::Update => creates_updates.push(change),
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
    for item_type in &sorted_types {
        let group = &type_groups[item_type];

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
                        if let Some(id) = deployed_id {
                            created_ids.insert((change.item_type.clone(), change.name.clone()), id);
                        }
                        succeeded.push((*change).clone());
                    }
                    Err(e) => {
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

                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    let result = execute_single_change_owned(
                        dry_run,
                        &client_clone,
                        &ws_id,
                        &change_owned,
                        source_item.as_ref(),
                    )
                    .await;
                    (change_owned, result)
                }));
            }

            // Collect results
            for handle in handles {
                let (change_owned, result) = handle.await?;
                match result {
                    Ok(deployed_id) => {
                        if let Some(id) = deployed_id {
                            created_ids.insert(
                                (change_owned.item_type.clone(), change_owned.name.clone()),
                                id,
                            );
                        }
                        succeeded.push(change_owned);
                    }
                    Err(e) => {
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

    for change in &deletes_sorted {
        if fail_fast && !failed.is_empty() {
            break;
        }

        let result = execute_delete(client, workspace_id, change).await;
        match result {
            Ok(()) => succeeded.push((*change).clone()),
            Err(e) => {
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
    _created_ids: &HashMap<(String, String), String>,
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
    deploy_change(cli.dry_run, client, workspace_id, change, source_item).await
}

/// Execute a single create or update with owned data (for parallel spawned tasks).
async fn execute_single_change_owned(
    dry_run: bool,
    client: &FabricClient,
    workspace_id: &str,
    change: &Change,
    source_item: Option<&super::platform::SourceItem>,
) -> Result<Option<String>> {
    let source_item = source_item.ok_or_else(|| {
        anyhow::anyhow!(
            "Source item not found for {} \"{}\"",
            change.item_type,
            change.name
        )
    })?;
    deploy_change(dry_run, client, workspace_id, change, source_item).await
}

/// Core deploy logic shared by sequential and parallel paths.
#[allow(clippy::option_if_let_else)]
async fn deploy_change(
    dry_run: bool,
    client: &FabricClient,
    workspace_id: &str,
    change: &Change,
    source_item: &super::platform::SourceItem,
) -> Result<Option<String>> {
    // Build definition parts for API
    let parts: Vec<Value> = source_item
        .parts
        .iter()
        .map(|p| {
            json!({
                "path": p.path,
                "payload": p.payload,
                "payloadType": p.payload_type,
            })
        })
        .collect();

    match change.action {
        ChangeAction::Create => {
            // Build definition with optional format
            let definition = if let Some(ref fmt) = source_item.metadata.definition_format {
                json!({ "format": fmt, "parts": parts })
            } else {
                json!({ "parts": parts })
            };

            let body = json!({
                "displayName": change.name,
                "type": change.item_type,
                "definition": definition
            });

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

            // Build definition with optional format
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
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;

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
            },
            Change {
                name: "MyNotebook".to_owned(),
                item_type: "Notebook".to_owned(),
                action: ChangeAction::Create,
                reason: "new".to_owned(),
                logical_id: None,
                deployed_id: None,
                source_hash: None,
            },
            Change {
                name: "MyLH".to_owned(),
                item_type: "Lakehouse".to_owned(),
                action: ChangeAction::Create,
                reason: "new".to_owned(),
                logical_id: None,
                deployed_id: None,
                source_hash: None,
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
}
