use std::fmt::Write;
use std::fs;
use std::path::Path;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

// ─── Get Definition ──────────────────────────────────────────────────────────

pub(super) async fn get_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    format: Option<&str>,
    decode: bool,
) -> Result<()> {
    let mut path = format!("/workspaces/{workspace}/items/{id}/getDefinition");
    if let Some(f) = format {
        let _ = write!(path, "?format={f}");
    }

    let data = client
        .post(&path, &serde_json::json!({}), true)
        .await
        .map_err(|e| enrich_forbidden(e, "item get-definition", "ReadWrite"))?;
    if decode {
        let decoded = output::decode_definition_parts(data);
        output::render_object(cli, &decoded, "definition");
    } else {
        output::render_object(cli, &data, "definition");
    }
    Ok(())
}

// ─── Update Definition ───────────────────────────────────────────────────────

pub(super) async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    definition: Option<&str>,
    update_metadata: bool,
) -> Result<()> {
    if file.is_none() && definition.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Either --file or --definition must be provided".to_string(),
            "Example: fabio item update-definition --workspace <WS> --id <ID> --file ./notebook.ipynb"
                .to_string(),
        )
        .into());
    }

    let body = if let Some(def_json) = definition {
        // Inline JSON definition payload
        serde_json::from_str::<Value>(def_json).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in --definition: {e}"),
                "Provide valid JSON: {\"definition\":{\"parts\":[{\"path\":\"...\",\"payload\":\"base64...\",\"payloadType\":\"InlineBase64\"}]}}"
                    .to_string(),
            )
        })?
    } else if let Some(file_path) = file {
        // Read file and encode as base64
        let path = Path::new(file_path);
        let content = fs::read(path).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Failed to read file '{file_path}': {e}"),
                "Provide a valid file path.".to_string(),
            )
        })?;

        let encoded = BASE64.encode(&content);
        let filename = path
            .file_name()
            .map_or("definition", |f| f.to_str().unwrap_or("definition"));

        serde_json::json!({
            "definition": {
                "parts": [{
                    "path": filename,
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }]
            }
        })
    } else {
        unreachable!()
    };

    if output::dry_run_guard(cli, "item update-definition", &body) {
        return Ok(());
    }

    let mut path = format!("/workspaces/{workspace}/items/{id}/updateDefinition");
    if update_metadata {
        path.push_str("?updateMetadata=true");
    }

    client
        .post(&path, &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "item update-definition", "ReadWrite"))?;

    let obj = serde_json::json!({
        "id": id,
        "workspace": workspace,
        "status": "definition_updated"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}
