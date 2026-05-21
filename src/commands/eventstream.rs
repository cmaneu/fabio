use anyhow::Result;
use base64::Engine;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum EventstreamCommand {
    /// List eventstreams in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of an eventstream
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,
    },
    /// Create a new eventstream
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update eventstream properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete an eventstream
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,
    },

    // ── Definitions ──────────────────────────────────────────────────────
    /// Get the definition of an eventstream
    #[command(display_order = 6)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition of an eventstream
    #[command(display_order = 7)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Definition file path (reads file content)
        #[arg(long)]
        file: Option<String>,

        /// Definition content (inline)
        #[arg(long)]
        content: Option<String>,
    },

    // ── Topology ─────────────────────────────────────────────────────────
    /// Get the topology of an eventstream
    #[command(display_order = 10)]
    GetTopology {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,
    },

    // ── Stream Control ───────────────────────────────────────────────────
    /// Pause the entire eventstream
    #[command(display_order = 11)]
    Pause {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,
    },
    /// Resume the entire eventstream
    #[command(display_order = 12)]
    Resume {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,
    },

    // ── Destinations ─────────────────────────────────────────────────────
    /// Get details of a destination
    #[command(display_order = 20)]
    GetDestination {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Destination ID
        #[arg(long)]
        destination_id: String,
    },
    /// Get the connection of a destination
    #[command(display_order = 21)]
    GetDestinationConnection {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Destination ID
        #[arg(long)]
        destination_id: String,
    },
    /// Pause a destination
    #[command(display_order = 22)]
    PauseDestination {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Destination ID
        #[arg(long)]
        destination_id: String,
    },
    /// Resume a destination
    #[command(display_order = 23)]
    ResumeDestination {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Destination ID
        #[arg(long)]
        destination_id: String,
    },

    // ── Sources ──────────────────────────────────────────────────────────
    /// Get details of a source
    #[command(display_order = 30)]
    GetSource {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Source ID
        #[arg(long)]
        source_id: String,
    },
    /// Get the connection of a source
    #[command(display_order = 31)]
    GetSourceConnection {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Source ID
        #[arg(long)]
        source_id: String,
    },
    /// Pause a source
    #[command(display_order = 32)]
    PauseSource {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Source ID
        #[arg(long)]
        source_id: String,
    },
    /// Resume a source
    #[command(display_order = 33)]
    ResumeSource {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Source ID
        #[arg(long)]
        source_id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &EventstreamCommand) -> Result<()> {
    match command {
        EventstreamCommand::List { workspace } => list(cli, client, workspace).await,
        EventstreamCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        EventstreamCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref()).await,
        EventstreamCommand::Update {
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
        EventstreamCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        EventstreamCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        EventstreamCommand::UpdateDefinition {
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
        EventstreamCommand::GetTopology { workspace, id } => {
            get_topology(cli, client, workspace, id).await
        }
        EventstreamCommand::Pause { workspace, id } => {
            pause_stream(cli, client, workspace, id).await
        }
        EventstreamCommand::Resume { workspace, id } => {
            resume_stream(cli, client, workspace, id).await
        }
        EventstreamCommand::GetDestination {
            workspace,
            id,
            destination_id,
        } => get_destination(cli, client, workspace, id, destination_id).await,
        EventstreamCommand::GetDestinationConnection {
            workspace,
            id,
            destination_id,
        } => get_destination_connection(cli, client, workspace, id, destination_id).await,
        EventstreamCommand::PauseDestination {
            workspace,
            id,
            destination_id,
        } => pause_destination(cli, client, workspace, id, destination_id).await,
        EventstreamCommand::ResumeDestination {
            workspace,
            id,
            destination_id,
        } => resume_destination(cli, client, workspace, id, destination_id).await,
        EventstreamCommand::GetSource {
            workspace,
            id,
            source_id,
        } => get_source(cli, client, workspace, id, source_id).await,
        EventstreamCommand::GetSourceConnection {
            workspace,
            id,
            source_id,
        } => get_source_connection(cli, client, workspace, id, source_id).await,
        EventstreamCommand::PauseSource {
            workspace,
            id,
            source_id,
        } => pause_source(cli, client, workspace, id, source_id).await,
        EventstreamCommand::ResumeSource {
            workspace,
            id,
            source_id,
        } => resume_source(cli, client, workspace, id, source_id).await,
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/eventstreams"),
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
        .get(&format!("/workspaces/{workspace}/eventstreams/{id}"))
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
    let mut body = serde_json::json!({
        "displayName": name,
    });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if output::dry_run_guard(
        cli,
        "eventstream create",
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
            &format!("/workspaces/{workspace}/eventstreams"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream create", "Member"))?;
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
            "Example: fabio eventstream update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "eventstream update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/eventstreams/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "eventstream delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/eventstreams/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Definitions ─────────────────────────────────────────────────────────────

async fn get_definition(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/eventstreams/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream get-definition", "Contributor"))?;
    output::render_object(cli, &data, "definition");
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
    let script = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(c)) => c.to_string(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio eventstream update-definition --workspace <WS> --id <ID> --file definition.json".to_string(),
            ).into());
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(script.as_bytes());

    let body = serde_json::json!({
        "definition": {
            "parts": [
                {
                    "path": "eventstream.json",
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    if output::dry_run_guard(
        cli,
        "eventstream update-definition",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "contentLength": script.len()
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/eventstreams/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Topology ────────────────────────────────────────────────────────────────

async fn get_topology(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/eventstreams/{id}/topology"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

// ─── Stream Control ──────────────────────────────────────────────────────────

async fn pause_stream(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "eventstream pause",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/eventstreams/{id}/pause"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream pause", "Contributor"))?;

    let obj = serde_json::json!({ "id": id, "status": "paused" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn resume_stream(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "eventstream resume",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/eventstreams/{id}/resume"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream resume", "Contributor"))?;

    let obj = serde_json::json!({ "id": id, "status": "resumed" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Destinations ────────────────────────────────────────────────────────────

async fn get_destination(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    destination_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/eventstreams/{id}/destinations/{destination_id}"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn get_destination_connection(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    destination_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/eventstreams/{id}/destinations/{destination_id}/connection"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn pause_destination(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    destination_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "eventstream pause-destination",
        &serde_json::json!({ "workspace": workspace, "id": id, "destinationId": destination_id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!(
                "/workspaces/{workspace}/eventstreams/{id}/destinations/{destination_id}/pause"
            ),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream pause-destination", "Contributor"))?;

    let obj = serde_json::json!({ "id": destination_id, "status": "paused" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn resume_destination(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    destination_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "eventstream resume-destination",
        &serde_json::json!({ "workspace": workspace, "id": id, "destinationId": destination_id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!(
                "/workspaces/{workspace}/eventstreams/{id}/destinations/{destination_id}/resume"
            ),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream resume-destination", "Contributor"))?;

    let obj = serde_json::json!({ "id": destination_id, "status": "resumed" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Sources ─────────────────────────────────────────────────────────────────

async fn get_source(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    source_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/eventstreams/{id}/sources/{source_id}"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn get_source_connection(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    source_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/eventstreams/{id}/sources/{source_id}/connection"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn pause_source(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    source_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "eventstream pause-source",
        &serde_json::json!({ "workspace": workspace, "id": id, "sourceId": source_id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/eventstreams/{id}/sources/{source_id}/pause"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream pause-source", "Contributor"))?;

    let obj = serde_json::json!({ "id": source_id, "status": "paused" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn resume_source(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    source_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "eventstream resume-source",
        &serde_json::json!({ "workspace": workspace, "id": id, "sourceId": source_id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/eventstreams/{id}/sources/{source_id}/resume"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream resume-source", "Contributor"))?;

    let obj = serde_json::json!({ "id": source_id, "status": "resumed" });
    output::render_object(cli, &obj, "status");
    Ok(())
}
