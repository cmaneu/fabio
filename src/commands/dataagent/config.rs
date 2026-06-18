use anyhow::Result;
use base64::Engine;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::FabioError;
use crate::output;

use super::{decode_part_payload, get_definition_parts};

/// Get agent configuration by parsing the definition's `stage_config.json`.
pub(super) async fn get_config(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let definition_resp = client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await?;

    let parts = definition_resp
        .get("definition")
        .and_then(|d| d.get("parts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut config = serde_json::json!({
        "instructions": null,
        "enablePreviewRuntime": false,
        "dataSources": [],
    });

    for part in &parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");

        if path == "Files/Config/draft/stage_config.json" {
            if let Some(decoded) = decode_part_payload(payload) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&decoded) {
                    config["instructions"] = parsed
                        .get("aiInstructions")
                        .or_else(|| parsed.get("additionalInstructions"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    if let Some(experimental) = parsed.get("experimental") {
                        let enabled = experimental
                            .get("enableExperimentalFeatures")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        config["enablePreviewRuntime"] = Value::Bool(enabled);
                    }
                }
            }
        }

        // Collect datasource names
        if path.starts_with("Files/Config/draft/") && path.ends_with("/datasource.json") {
            if let Some(decoded) = decode_part_payload(payload) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&decoded) {
                    let ds_info = serde_json::json!({
                        "displayName": parsed.get("displayName").or_else(|| parsed.get("display_name")),
                        "type": parsed.get("type"),
                        "artifactId": parsed.get("artifactId").or_else(|| parsed.get("id")),
                    });
                    if let Some(arr) = config["dataSources"].as_array_mut() {
                        arr.push(ds_info);
                    }
                }
            }
        }
    }

    output::render_object(cli, &config, "instructions");
    Ok(())
}

/// Update agent configuration by modifying `stage_config.json` in the definition.
#[allow(
    clippy::fn_params_excessive_bools,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]
pub(super) async fn update_config(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    instructions: Option<&str>,
    instructions_file: Option<&str>,
    enable_preview_runtime: bool,
    disable_preview_runtime: bool,
) -> Result<()> {
    // Resolve instructions from --instructions or --instructions-file
    let resolved_instructions = match (instructions, instructions_file) {
        (Some(instr), _) => Some(instr.to_string()),
        (_, Some(path)) => {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read instructions file '{path}': {e}"))?;
            Some(content)
        }
        _ => None,
    };

    if resolved_instructions.is_none() && !enable_preview_runtime && !disable_preview_runtime {
        return Err(FabioError::invalid_input(
            "At least one of --instructions, --instructions-file, --enable-preview-runtime, or --disable-preview-runtime must be provided",
        )
        .into());
    }

    if output::dry_run_guard(
        cli,
        "data-agent update-config",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "instructions": resolved_instructions.as_deref().map(|s| if s.len() > 100 { format!("{}...", &s[..100]) } else { s.to_string() }),
            "instructionsFile": instructions_file,
            "enablePreviewRuntime": enable_preview_runtime,
            "disablePreviewRuntime": disable_preview_runtime,
        }),
    ) {
        return Ok(());
    }

    // Fetch current definition
    let parts = get_definition_parts(client, workspace, id).await?;

    // Find and modify stage_config.json
    let mut new_parts: Vec<Value> = Vec::new();
    let mut found_config = false;

    for part in &parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if path == "Files/Config/draft/stage_config.json" {
            found_config = true;
            let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");
            let mut config = decode_part_payload(payload)
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| serde_json::json!({}));

            if let Some(instr) = &resolved_instructions {
                config["aiInstructions"] = Value::String(instr.clone());
            }
            if enable_preview_runtime || disable_preview_runtime {
                let experimental = config.as_object_mut().map(|o| {
                    o.entry("experimental")
                        .or_insert_with(|| serde_json::json!({}))
                });
                if let Some(exp) = experimental {
                    if let Some(obj) = exp.as_object_mut() {
                        obj.insert(
                            "enableExperimentalFeatures".to_string(),
                            Value::Bool(enable_preview_runtime),
                        );
                    }
                }
            }

            let encoded =
                base64::engine::general_purpose::STANDARD.encode(config.to_string().as_bytes());
            new_parts.push(serde_json::json!({
                "path": path,
                "payload": encoded,
                "payloadType": "InlineBase64"
            }));
        } else {
            new_parts.push(part.clone());
        }
    }

    // If no stage_config exists yet, create one
    if !found_config {
        let mut config = serde_json::json!({
            "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/stageConfiguration/1.0.0/schema.json"
        });
        if let Some(instr) = &resolved_instructions {
            config["aiInstructions"] = Value::String(instr.clone());
        }
        if enable_preview_runtime {
            config["experimental"] = serde_json::json!({"enableExperimentalFeatures": true});
        }
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(config.to_string().as_bytes());
        new_parts.push(serde_json::json!({
            "path": "Files/Config/draft/stage_config.json",
            "payload": encoded,
            "payloadType": "InlineBase64"
        }));
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
        "status": "config_updated",
        "instructions": resolved_instructions.as_deref(),
        "enablePreviewRuntime": enable_preview_runtime,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}
