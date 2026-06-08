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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Show details of an eventstream
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,
    },
    /// Create a new eventstream
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Permanently delete (cannot be recovered)
        #[arg(long)]
        hard_delete: bool,
    },

    // ── Definitions ──────────────────────────────────────────────────────
    /// Get the definition of an eventstream
    #[command(display_order = 6)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition of an eventstream
    #[command(display_order = 7)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,
    },
    /// Resume the entire eventstream
    #[command(display_order = 12)]
    Resume {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Source ID
        #[arg(long)]
        source_id: String,
    },

    // ── High-level helpers ───────────────────────────────────────────────
    /// Add a source to an eventstream (fetches current definition, merges, and updates)
    #[command(display_order = 40)]
    AddSource {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Source name (unique within the eventstream)
        #[arg(long)]
        name: String,

        /// Source type (e.g., `CustomEndpoint`, `AzureEventHub`, `SampleData`)
        #[arg(long, visible_alias = "type")]
        source_type: String,

        /// Source properties as JSON string (default: {})
        #[arg(long)]
        properties: Option<String>,
    },

    /// Add a destination to an eventstream (fetches current definition, merges, and updates)
    #[command(display_order = 41)]
    AddDestination {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Eventstream ID
        #[arg(long)]
        id: String,

        /// Destination name (unique within the eventstream)
        #[arg(long)]
        name: String,

        /// Destination type (e.g., `Eventhouse`, `Lakehouse`, `CustomEndpoint`, `Activator`)
        #[arg(long, visible_alias = "type")]
        destination_type: String,

        /// Destination properties as JSON string
        #[arg(long)]
        properties: Option<String>,

        /// Input node name (the stream or operator this destination reads from)
        #[arg(long)]
        input_node: String,
    },
}

#[allow(clippy::too_many_lines)]
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
        EventstreamCommand::Delete {
            workspace,
            id,
            hard_delete,
        } => delete(cli, client, workspace, id, *hard_delete).await,
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
        EventstreamCommand::AddSource {
            workspace,
            id,
            name,
            source_type,
            properties,
        } => {
            add_source(
                cli,
                client,
                workspace,
                id,
                name,
                source_type,
                properties.as_deref(),
            )
            .await
        }
        EventstreamCommand::AddDestination {
            workspace,
            id,
            name,
            destination_type,
            properties,
            input_node,
        } => {
            add_destination(
                cli,
                client,
                workspace,
                id,
                name,
                destination_type,
                properties.as_deref(),
                input_node,
            )
            .await
        }
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

async fn delete(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    hard_delete: bool,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "eventstream delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id, "hardDelete": hard_delete
        }),
    ) {
        return Ok(());
    }

    let url = if hard_delete {
        format!("/workspaces/{workspace}/eventstreams/{id}?hardDelete=true")
    } else {
        format!("/workspaces/{workspace}/eventstreams/{id}")
    };

    client
        .delete(&url)
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

// ─── High-level helpers ──────────────────────────────────────────────────────

/// Fetches the current eventstream definition, decodes it, returns the parsed JSON.
async fn fetch_current_definition(
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<Value> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/eventstreams/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await?;

    // Extract the eventstream.json part
    let parts = data["definition"]["parts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No definition parts returned"))?;

    for part in parts {
        if part["path"].as_str() == Some("eventstream.json") {
            let payload = part["payload"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing payload in eventstream.json part"))?;
            let decoded = base64::engine::general_purpose::STANDARD.decode(payload)?;
            let json_str = String::from_utf8(decoded)?;
            let parsed: Value = serde_json::from_str(&json_str)?;
            return Ok(parsed);
        }
    }

    Err(anyhow::anyhow!(
        "eventstream.json not found in definition parts"
    ))
}

/// Pushes updated definition back to the eventstream.
async fn push_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    definition: &Value,
) -> Result<Value> {
    let json_str = serde_json::to_string(definition)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(json_str.as_bytes());

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

    let data = client
        .post(
            &format!("/workspaces/{workspace}/eventstreams/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "eventstream update-definition", "Contributor"))?;

    // After update, fetch the topology to return the new source/destination with its server-assigned ID
    let topology = client
        .get(&format!(
            "/workspaces/{workspace}/eventstreams/{id}/topology"
        ))
        .await;

    if let Ok(topo) = topology {
        output::render_object(cli, &topo, "id");
        return Ok(topo);
    }

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
        Ok(obj)
    } else {
        output::render_object(cli, &data, "id");
        Ok(data)
    }
}

#[allow(clippy::too_many_arguments)]
async fn add_source(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: &str,
    source_type: &str,
    properties: Option<&str>,
) -> Result<()> {
    let props: Value = match properties {
        Some(p) => serde_json::from_str(p).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in --properties: {e}"),
                "Example: --properties '{}'".to_string(),
            )
        })?,
        None => serde_json::json!({}),
    };

    if output::dry_run_guard(
        cli,
        "eventstream add-source",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "source": { "name": name, "type": source_type, "properties": props }
        }),
    ) {
        return Ok(());
    }

    // 1. Fetch current definition
    let mut def = fetch_current_definition(client, workspace, id).await?;

    // 2. Add the new source
    let new_source = serde_json::json!({
        "name": name,
        "type": source_type,
        "properties": props,
    });

    let sources = def["sources"]
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("Definition missing sources array"))?;
    sources.push(new_source);

    // 3. Add a default stream for this source if no stream references it yet
    let streams = def["streams"]
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("Definition missing streams array"))?;
    let has_stream = streams.iter().any(|s| {
        s["inputNodes"]
            .as_array()
            .is_some_and(|nodes| nodes.iter().any(|n| n["name"].as_str() == Some(name)))
    });
    if !has_stream {
        let stream_name = format!("{name}-stream");
        streams.push(serde_json::json!({
            "name": stream_name,
            "type": "DefaultStream",
            "properties": {},
            "inputNodes": [{"name": name}]
        }));
    }

    // 4. Push updated definition
    push_definition(cli, client, workspace, id, &def).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn add_destination(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: &str,
    destination_type: &str,
    properties: Option<&str>,
    input_node: &str,
) -> Result<()> {
    let props: Value = match properties {
        Some(p) => serde_json::from_str(p).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in --properties: {e}"),
                "Example: --properties '{{\"workspaceId\":\"...\",\"itemId\":\"...\"}}'"
                    .to_string(),
            )
        })?,
        None => serde_json::json!({}),
    };

    if output::dry_run_guard(
        cli,
        "eventstream add-destination",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "destination": {
                "name": name,
                "type": destination_type,
                "properties": props,
                "inputNodes": [{"name": input_node}]
            }
        }),
    ) {
        return Ok(());
    }

    // 1. Fetch current definition
    let mut def = fetch_current_definition(client, workspace, id).await?;

    // 2. Add the new destination
    let new_dest = serde_json::json!({
        "name": name,
        "type": destination_type,
        "properties": props,
        "inputNodes": [{"name": input_node}]
    });

    let destinations = def["destinations"]
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("Definition missing destinations array"))?;
    destinations.push(new_dest);

    // 3. Push updated definition
    push_definition(cli, client, workspace, id, &def).await?;
    Ok(())
}
