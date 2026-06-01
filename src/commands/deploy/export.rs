use anyhow::Result;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;

use super::platform::{DefinitionPart, PlatformMetadata, write_source_directory};

/// Export a workspace's item definitions to a local `.platform` directory.
///
/// For each item in the workspace:
/// 1. Fetches the item metadata (type, name)
/// 2. Fetches the definition via `getDefinition` LRO
/// 3. Writes the directory structure with `.platform` + definition files
pub async fn export_workspace(
    cli: &Cli,
    client: &FabricClient,
    workspace_id: &str,
    output_dir: &std::path::Path,
    item_types: Option<&[String]>,
    overwrite: bool,
) -> Result<ExportResult> {
    // List all items in workspace
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace_id}/items"),
            "value",
            true,
            None,
        )
        .await?;

    let mut items_to_export: Vec<(String, String, String, Option<String>)> = Vec::new(); // (id, type, name, description)

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
        if let Some(types) = item_types {
            if !types.iter().any(|t| t.eq_ignore_ascii_case(item_type)) {
                continue;
            }
        }

        items_to_export.push((
            id.to_owned(),
            item_type.to_owned(),
            name.to_owned(),
            description,
        ));
    }

    let total = items_to_export.len();
    let mut exported = Vec::new();
    let mut skipped = Vec::new();

    for (id, item_type, name, description) in &items_to_export {
        // Fetch definition via LRO
        let path = format!("/workspaces/{workspace_id}/items/{id}/getDefinition");
        let result = client.post(&path, &serde_json::json!({}), true).await;

        match result {
            Ok(data) => {
                let parts = extract_definition_parts(&data);
                if parts.is_empty() {
                    skipped.push(format!("{item_type} \"{name}\" (no definition parts)"));
                    continue;
                }

                let definition_format = data
                    .get("definition")
                    .and_then(|d| d.get("format"))
                    .and_then(|f| f.as_str())
                    .map(str::to_owned);

                let metadata = PlatformMetadata {
                    item_type: item_type.clone(),
                    display_name: name.clone(),
                    logical_id: extract_logical_id(&data),
                    description: description.clone(),
                    definition_format,
                };

                exported.push((metadata, parts));
            }
            Err(_) => {
                skipped.push(format!(
                    "{item_type} \"{name}\" (getDefinition not supported)"
                ));
            }
        }
    }

    // Write to disk
    let count = if cli.dry_run {
        exported.len()
    } else {
        write_source_directory(output_dir, &exported, overwrite)?
    };

    Ok(ExportResult {
        total_items: total,
        exported: count,
        skipped,
    })
}

/// Result of an export operation.
#[derive(Debug, serde::Serialize)]
pub struct ExportResult {
    pub total_items: usize,
    pub exported: usize,
    pub skipped: Vec<String>,
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
