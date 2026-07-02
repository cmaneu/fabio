use std::sync::Arc;

use anyhow::Result;
use serde_json::Value;
use tokio::sync::Semaphore;

use crate::cli::Cli;
use crate::client::FabricClient;

use super::platform::{DefinitionPart, PlatformMetadata, write_source_directory};

/// Export a workspace's item definitions to a local `.platform` directory.
///
/// For each item in the workspace:
/// 1. Fetches the item metadata (type, name)
/// 2. Fetches the definition via `getDefinition` LRO (in parallel)
/// 3. Writes the directory structure with `.platform` + definition files
pub async fn export_workspace(
    cli: &Cli,
    client: &FabricClient,
    workspace_id: &str,
    output_dir: &std::path::Path,
    item_types: Option<&[String]>,
    overwrite: bool,
    concurrency: usize,
) -> Result<ExportResult> {
    let items_to_export = collect_exportable_items(client, workspace_id, item_types).await?;
    let total = items_to_export.len();

    // Fetch definitions in parallel with bounded concurrency
    let (exported, skipped) = fetch_definitions_parallel(
        client,
        workspace_id,
        &items_to_export,
        concurrency,
        cli.quiet,
    )
    .await?;

    // Write to disk
    let count = if cli.dry_run {
        exported.len()
    } else {
        write_source_directory(output_dir, &exported, overwrite)?
    };

    // Export shortcuts for Lakehouse items
    if !cli.dry_run {
        export_lakehouse_shortcuts(client, workspace_id, output_dir, &items_to_export).await;
    }

    Ok(ExportResult {
        total_items: total,
        exported: count,
        skipped,
    })
}

/// Collect all exportable items from the workspace, optionally filtering by type.
async fn collect_exportable_items(
    client: &FabricClient,
    workspace_id: &str,
    item_types: Option<&[String]>,
) -> Result<Vec<(String, String, String, Option<String>)>> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace_id}/items"),
            "value",
            true,
            None,
        )
        .await?;

    let mut items: Vec<(String, String, String, Option<String>)> = Vec::new();

    for item in &resp.items {
        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or_default();
        let item_type = item
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let name = item
            .get("displayName")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_owned);

        if id.is_empty() || item_type.is_empty() || name.is_empty() {
            continue;
        }

        // Filter by item types if specified
        if let Some(types) = item_types
            && !types.iter().any(|t| t.eq_ignore_ascii_case(item_type))
        {
            continue;
        }

        items.push((
            id.to_owned(),
            item_type.to_owned(),
            name.to_owned(),
            description,
        ));
    }

    Ok(items)
}

/// Fetch item definitions in parallel using bounded concurrency.
///
/// Returns `(exported_items, skipped_messages)`.
async fn fetch_definitions_parallel(
    client: &FabricClient,
    workspace_id: &str,
    items: &[(String, String, String, Option<String>)],
    concurrency: usize,
    quiet: bool,
) -> Result<(Vec<(PlatformMetadata, Vec<DefinitionPart>)>, Vec<String>)> {
    let batch_concurrency = concurrency.max(1);
    let semaphore = Arc::new(Semaphore::new(batch_concurrency));
    let mut handles = Vec::with_capacity(items.len());

    for (id, item_type, name, description) in items {
        let sem = Arc::clone(&semaphore);
        let client_clone = client.clone();
        let ws_id = workspace_id.to_owned();
        let item_id = id.clone();
        let item_type_owned = item_type.clone();
        let name_owned = name.clone();
        let description_owned = description.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let path = format!("/workspaces/{ws_id}/items/{item_id}/getDefinition");
            let result = client_clone.post(&path, &serde_json::json!({}), true).await;

            if !quiet {
                eprintln!(
                    "[deploy export] fetched definition for {item_type_owned} \"{name_owned}\""
                );
            }

            (item_type_owned, name_owned, description_owned, result)
        }));
    }

    let mut exported = Vec::new();
    let mut skipped = Vec::new();

    for handle in handles {
        let (item_type, name, description, result) = handle.await?;

        match result {
            Ok(data) => {
                let parts = extract_definition_parts(&data);
                if parts.is_empty() && !is_shell_only_type(&item_type) {
                    skipped.push(format!("{item_type} \"{name}\" (no definition parts)"));
                    continue;
                }

                let definition_format = data
                    .get("definition")
                    .and_then(|d| d.get("format"))
                    .and_then(|f| f.as_str())
                    .map(str::to_owned);

                let metadata = PlatformMetadata {
                    item_type,
                    display_name: name,
                    logical_id: extract_logical_id(&data),
                    description,
                    definition_format,
                    platform_creation_payload: None,
                };

                exported.push((metadata, parts));
            }
            Err(_) => {
                // Shell-only items (Warehouse, SQLDatabase, etc.) don't support
                // getDefinition but should still be exported with just a .platform
                // file so deploy apply can recreate the container.
                if is_shell_only_type(&item_type) {
                    let metadata = PlatformMetadata {
                        item_type,
                        display_name: name,
                        logical_id: None,
                        description,
                        definition_format: None,
                        platform_creation_payload: None,
                    };
                    exported.push((metadata, Vec::new()));
                } else {
                    skipped.push(format!(
                        "{item_type} \"{name}\" (getDefinition not supported)"
                    ));
                }
            }
        }
    }

    Ok((exported, skipped))
}

/// Result of an export operation.
#[derive(Debug, serde::Serialize)]
pub struct ExportResult {
    pub total_items: usize,
    pub exported: usize,
    pub skipped: Vec<String>,
}

/// Item types that are exported as metadata-only (`.platform` file, no definition parts).
///
/// These types don't support `getDefinition` but are still valid deployment targets:
/// - `deploy apply` creates them with just `displayName` + `type` (no definition body)
/// - `deploy apply` skips `updateDefinition` when parts are empty
///
/// Matches fabric-cicd's `SHELL_ONLY_PUBLISH` concept.
const SHELL_ONLY_TYPES: &[&str] = &["Warehouse", "SQLDatabase", "MLExperiment", "MLModel"];

/// Returns true if the item type should be exported as shell-only (just `.platform`).
fn is_shell_only_type(item_type: &str) -> bool {
    SHELL_ONLY_TYPES
        .iter()
        .any(|t| t.eq_ignore_ascii_case(item_type))
}

/// Extract definition parts from a `getDefinition` API response.
fn extract_definition_parts(data: &Value) -> Vec<DefinitionPart> {
    let Some(parts) = data
        .get("definition")
        .and_then(|d| d.get("parts"))
        .and_then(|p| p.as_array())
    else {
        return Vec::new();
    };

    parts
        .iter()
        .filter_map(|p| {
            let path = p.get("path")?.as_str()?.to_owned();
            let payload = p.get("payload")?.as_str()?.to_owned();
            let payload_type = p
                .get("payloadType")
                .and_then(|v| v.as_str())
                .unwrap_or("InlineBase64")
                .to_owned();

            // Skip .platform files from export (we generate our own)
            if path == ".platform" {
                return None;
            }

            Some(DefinitionPart {
                path,
                payload,
                payload_type,
            })
        })
        .collect()
}

/// Try to extract a logical ID from the definition response.
///
/// Some definitions include `.platform` as a part with `config.logicalId`.
fn extract_logical_id(data: &Value) -> Option<String> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;

    let parts = data.get("definition")?.get("parts")?.as_array()?;

    for part in parts {
        let path = part.get("path")?.as_str()?;
        if path == ".platform" {
            let payload = part.get("payload")?.as_str()?;
            let decoded = BASE64.decode(payload).ok()?;
            let json: Value = serde_json::from_slice(&decoded).ok()?;
            return json
                .get("config")
                .and_then(|c| c.get("logicalId"))
                .and_then(|v| v.as_str())
                .map(str::to_owned);
        }
    }

    None
}

/// Export shortcuts for all Lakehouse items in the workspace.
///
/// For each Lakehouse item, fetches deployed shortcuts via `GET /items/{id}/shortcuts`
/// and writes `shortcuts.metadata.json` to the item's export directory.
/// Failures are silently ignored (shortcuts are optional).
async fn export_lakehouse_shortcuts(
    client: &FabricClient,
    workspace_id: &str,
    output_dir: &std::path::Path,
    items: &[(String, String, String, Option<String>)], // (id, type, name, description)
) {
    for (id, item_type, name, _) in items {
        if !item_type.eq_ignore_ascii_case("Lakehouse") {
            continue;
        }

        let url = format!("/workspaces/{workspace_id}/items/{id}/shortcuts");
        let Ok(data) = client.get(&url).await else {
            continue;
        };

        let Some(shortcuts) = data.get("value").and_then(|v| v.as_array()) else {
            continue;
        };

        if shortcuts.is_empty() {
            continue;
        }

        let dir_name = format!("{name}.{item_type}");
        let item_dir = output_dir.join(&dir_name);

        // Only write if the item directory already exists (was exported successfully)
        if !item_dir.exists() {
            continue;
        }

        let content = serde_json::to_string_pretty(shortcuts).unwrap_or_default();
        let _ = std::fs::write(item_dir.join("shortcuts.metadata.json"), content);
    }
}
