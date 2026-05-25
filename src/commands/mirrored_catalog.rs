use anyhow::Result;
use base64::Engine;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum MirroredCatalogCommand {
    /// List mirrored catalogs in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a mirrored catalog
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Mirrored catalog ID
        #[arg(long)]
        id: String,
    },
    /// Create a new mirrored catalog
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update mirrored catalog properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Mirrored catalog ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a mirrored catalog
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Mirrored catalog ID
        #[arg(long)]
        id: String,
    },
    /// Get the definition of a mirrored catalog
    #[command(display_order = 6)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Mirrored catalog ID
        #[arg(long)]
        id: String,

        /// Decode base64 payloads inline (adds decodedPayload field)
        #[arg(long)]
        decode: bool,
    },
    /// Update the definition of a mirrored catalog
    #[command(display_order = 7)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Mirrored catalog ID
        #[arg(long)]
        id: String,

        /// Path to definition file
        #[arg(long)]
        file: Option<String>,

        /// Inline definition content
        #[arg(long)]
        content: Option<String>,
    },
    /// Refresh catalog metadata
    #[command(display_order = 10)]
    RefreshMetadata {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Mirrored catalog ID
        #[arg(long)]
        id: String,
    },
    /// List catalog mirroring scopes (workspace-level)
    #[command(display_order = 11)]
    ListScopes {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// List catalog mirroring tables (workspace-level)
    #[command(display_order = 12)]
    ListTables {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Get mirroring status
    #[command(display_order = 13)]
    MirroringStatus {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Mirrored catalog ID
        #[arg(long)]
        id: String,
    },
    /// Get tables mirroring status
    #[command(display_order = 14)]
    TablesMirroringStatus {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Mirrored catalog ID
        #[arg(long)]
        id: String,
    },
}

pub async fn execute(
    cli: &Cli,
    client: &FabricClient,
    command: &MirroredCatalogCommand,
) -> Result<()> {
    match command {
        MirroredCatalogCommand::List { workspace } => list(cli, client, workspace).await,
        MirroredCatalogCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        MirroredCatalogCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref()).await,
        MirroredCatalogCommand::Update {
            workspace,
            id,
            name,
            description,
        } => {
            update(
                cli,
                client,
                workspace,
                id,
                name.as_deref(),
                description.as_deref(),
            )
            .await
        }
        MirroredCatalogCommand::Delete { workspace, id } => {
            delete(cli, client, workspace, id).await
        }
        MirroredCatalogCommand::GetDefinition {
            workspace,
            id,
            decode,
        } => get_definition(cli, client, workspace, id, *decode).await,
        MirroredCatalogCommand::UpdateDefinition {
            workspace,
            id,
            file,
            content,
        } => {
            update_definition(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        MirroredCatalogCommand::RefreshMetadata { workspace, id } => {
            refresh_metadata(cli, client, workspace, id).await
        }
        MirroredCatalogCommand::ListScopes { workspace } => {
            list_scopes(cli, client, workspace).await
        }
        MirroredCatalogCommand::ListTables { workspace } => {
            list_tables(cli, client, workspace).await
        }
        MirroredCatalogCommand::MirroringStatus { workspace, id } => {
            mirroring_status(cli, client, workspace, id).await
        }
        MirroredCatalogCommand::TablesMirroringStatus { workspace, id } => {
            tables_mirroring_status(cli, client, workspace, id).await
        }
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/mirroredCatalogs"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id", "description"],
        &["NAME", "ID", "DESCRIPTION"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn show(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/mirroredCatalogs/{id}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    description: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if output::dry_run_guard(
        cli,
        "mirrored-catalog create",
        &serde_json::json!({
            "workspace": workspace,
            "displayName": name,
            "description": description
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/mirroredCatalogs"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "mirrored-catalog create", "Member"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --name or --description must be provided".to_string(),
            "Example: fabio mirrored-catalog update --workspace <WS> --id <ID> --name \"New Name\""
                .to_string(),
        )
        .into());
    }

    let mut body = serde_json::json!({});
    if let Some(n) = name {
        body["displayName"] = Value::String(n.to_string());
    }
    if let Some(d) = description {
        body["description"] = Value::String(d.to_string());
    }

    if output::dry_run_guard(cli, "mirrored-catalog update", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/mirroredCatalogs/{id}"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "mirrored-catalog update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "mirrored-catalog delete",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/mirroredCatalogs/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "mirrored-catalog delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Definitions ─────────────────────────────────────────────────────────────

async fn get_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    decode: bool,
) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/mirroredCatalogs/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "mirrored-catalog get-definition", "Contributor"))?;
    if decode {
        let decoded = output::decode_definition_parts(&data);
        output::render_object(cli, &decoded, "definition");
    } else {
        output::render_object(cli, &data, "definition");
    }
    Ok(())
}

async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let definition_json = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(c)) => c.to_string(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio mirrored-catalog update-definition --workspace <WS> --id <ID> --file definition.json".to_string(),
            ).into());
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(definition_json.as_bytes());

    let body = serde_json::json!({
        "definition": {
            "parts": [
                {
                    "path": "mirroring.json",
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    if output::dry_run_guard(
        cli,
        "mirrored-catalog update-definition",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "contentLength": definition_json.len()
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/mirroredCatalogs/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "mirrored-catalog update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Extra operations ────────────────────────────────────────────────────────

async fn refresh_metadata(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "mirrored-catalog refresh-metadata",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/mirroredCatalogs/{id}/refreshCatalogMetadata?beta=true"
            ),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "mirrored-catalog refresh-metadata", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "refresh_triggered" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

async fn list_scopes(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/catalogmirroring/scopes?beta=true"
        ))
        .await?;
    output::render_object(cli, &data, "data");
    Ok(())
}

async fn list_tables(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/catalogmirroring/tables?beta=true"
        ))
        .await?;
    output::render_object(cli, &data, "data");
    Ok(())
}

async fn mirroring_status(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/mirroredCatalogs/{id}/mirroringStatus?beta=true"
        ))
        .await?;
    output::render_object(cli, &data, "status");
    Ok(())
}

async fn tables_mirroring_status(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/mirroredCatalogs/{id}/tablesMirroringStatus?beta=true"
        ))
        .await?;
    output::render_object(cli, &data, "data");
    Ok(())
}
