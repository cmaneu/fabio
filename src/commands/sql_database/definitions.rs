use anyhow::Result;
use base64::Engine;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

pub(super) async fn get_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    format: Option<&str>,
) -> Result<()> {
    let url = format.map_or_else(
        || format!("/workspaces/{workspace}/sqlDatabases/{id}/getDefinition"),
        |f| format!("/workspaces/{workspace}/sqlDatabases/{id}/getDefinition?format={f}"),
    );

    let data = client
        .post(&url, &serde_json::json!({}), true)
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database get-definition", "Contributor"))?;
    output::render_object(cli, &data, "definition");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
    format: Option<&str>,
    update_metadata: bool,
) -> Result<()> {
    let payload_bytes = match (file, content) {
        (Some(path), _) => {
            std::fs::read(path).map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?
        }
        (_, Some(c)) => c.as_bytes().to_vec(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio sql-database update-definition --workspace <WS> --id <ID> --file schema.dacpac".to_string(),
            ).into());
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(&payload_bytes);
    let fmt = format.unwrap_or("dacpac");
    let extension = match fmt {
        "sqlproj" => "sqlproj",
        _ => "dacpac",
    };

    let body = serde_json::json!({
        "definition": {
            "format": fmt,
            "parts": [
                {
                    "path": format!("definition.{extension}"),
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    if output::dry_run_guard(
        cli,
        "sql-database update-definition",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "format": fmt,
            "contentLength": payload_bytes.len()
        }),
    ) {
        return Ok(());
    }

    let mut url = format!("/workspaces/{workspace}/sqlDatabases/{id}/updateDefinition");
    if update_metadata {
        url.push_str("?updateMetadata=true");
    }

    let data = client
        .post(&url, &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}
