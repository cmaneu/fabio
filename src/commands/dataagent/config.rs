use anyhow::Result;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::FabioError;
use crate::output;

/// Get agent configuration via the staging settings API.
///
/// Uses: `GET /workspaces/{ws}/dataAgents/{id}/staging/settings`
pub(super) async fn get_config(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let settings = client
        .get(&format!(
            "/workspaces/{workspace}/dataAgents/{id}/staging/settings"
        ))
        .await?;

    // Also fetch datasources list to include summary in config output
    let ds_resp = client
        .get_list(
            &format!("/workspaces/{workspace}/dataAgents/{id}/staging/datasources"),
            "value",
            true,
            None,
        )
        .await?;

    let ai_instructions = settings
        .get("aiInstructions")
        .cloned()
        .unwrap_or(Value::Null);

    let datasources: Vec<Value> = ds_resp
        .items
        .iter()
        .map(|ds| {
            serde_json::json!({
                "id": ds.get("id").and_then(Value::as_str),
                "displayName": ds.get("displayName").and_then(Value::as_str),
                "type": ds.get("type").and_then(Value::as_str),
            })
        })
        .collect();

    let config = serde_json::json!({
        "instructions": ai_instructions,
        "dataSources": datasources,
    });

    output::render_object(cli, &config, "instructions");
    Ok(())
}

/// Update agent configuration via the staging settings API.
///
/// Uses: `PATCH /workspaces/{ws}/dataAgents/{id}/staging/settings`
#[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
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
            "instructions": resolved_instructions.as_deref().map(|s| if s.len() > 100 { format!("{}...", &s[..s.floor_char_boundary(100)]) } else { s.to_string() }),
            "instructionsFile": instructions_file,
            "enablePreviewRuntime": enable_preview_runtime,
            "disablePreviewRuntime": disable_preview_runtime,
        }),
    ) {
        return Ok(());
    }

    // Build PATCH body — only include provided fields (partial update)
    let mut body = serde_json::Map::new();
    if let Some(instr) = &resolved_instructions {
        body.insert("aiInstructions".to_string(), Value::from(instr.as_str()));
    }

    let resp = client
        .patch(
            &format!("/workspaces/{workspace}/dataAgents/{id}/staging/settings"),
            &Value::Object(body),
        )
        .await?;

    let result = if resp.is_null() || resp.as_object().is_some_and(serde_json::Map::is_empty) {
        serde_json::json!({
            "id": id,
            "status": "config_updated",
            "instructions": resolved_instructions.as_deref(),
        })
    } else {
        let mut r = serde_json::json!({
            "id": id,
            "status": "config_updated",
        });
        if let Some(instr) = resp.get("aiInstructions") {
            r["instructions"] = instr.clone();
        }
        r
    };
    output::render_object(cli, &result, "status");
    Ok(())
}
