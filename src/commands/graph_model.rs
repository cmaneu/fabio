use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use clap::Subcommand;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
#[command(
    after_help = "For complete flag reference, run: fabio context agent\nReturns machine-readable JSON schema of all commands, flags, and types."
)]
pub enum GraphModelCommand {
    /// List graph models in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Show details of a graph model
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Graph model ID
        #[arg(long)]
        id: String,
    },
    /// Create a new graph model
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Ontology ID to link the graph model to
        #[arg(long)]
        ontology: Option<String>,

        /// Sensitivity label ID to apply on creation
        #[arg(long)]
        sensitivity_label: Option<String>,
    },
    /// Update graph model properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Graph model ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a graph model
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Graph model ID
        #[arg(long)]
        id: String,

        /// Permanently delete (cannot be recovered)
        #[arg(long)]
        hard_delete: bool,
    },
    /// Get the definition of a graph model
    #[command(display_order = 6)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Graph model ID
        #[arg(long)]
        id: String,

        /// Decode base64 payloads inline (adds decodedPayload field)
        #[arg(long)]
        decode: bool,
    },
    /// Update the definition of a graph model
    #[command(display_order = 7)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Graph model ID
        #[arg(long)]
        id: String,

        /// Path to definition file
        #[arg(long)]
        file: Option<String>,

        /// Inline definition content
        #[arg(long)]
        content: Option<String>,
    },
    /// Trigger a graph refresh job
    #[command(display_order = 10)]
    RefreshGraph {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Graph model ID
        #[arg(long)]
        id: String,

        /// Wait for the refresh to complete
        #[arg(long)]
        wait: bool,

        /// Timeout in seconds when using --wait (default: 600)
        #[arg(long, default_value_t = 600)]
        timeout: u64,
    },
    /// Execute a graph query
    #[command(display_order = 11)]
    ExecuteQuery {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Graph model ID
        #[arg(long)]
        id: String,

        /// Graph query string
        #[arg(long)]
        query: String,
    },
    /// Get the queryable graph type
    #[command(display_order = 12)]
    GetQueryableGraphType {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Graph model ID
        #[arg(long)]
        id: String,
    },
    /// Initialize a graph model for querying (portal-only operation)
    #[command(display_order = 20)]
    Initialize {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Graph model ID
        #[arg(long)]
        id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &GraphModelCommand) -> Result<()> {
    match command {
        GraphModelCommand::List { workspace } => list(cli, client, workspace).await,
        GraphModelCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        GraphModelCommand::Create {
            workspace,
            name,
            description,
            ontology,
            sensitivity_label,
        } => {
            create(
                cli,
                client,
                workspace,
                name,
                description.as_deref(),
                ontology.as_deref(),
                sensitivity_label.as_deref(),
            )
            .await
        }
        GraphModelCommand::Update {
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
        GraphModelCommand::Delete { workspace, id, hard_delete } => delete(cli, client, workspace, id, *hard_delete).await,
        GraphModelCommand::GetDefinition {
            workspace,
            id,
            decode,
        } => get_definition(cli, client, workspace, id, *decode).await,
        GraphModelCommand::UpdateDefinition {
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
        GraphModelCommand::RefreshGraph {
            workspace,
            id,
            wait,
            timeout,
        } => refresh_graph(cli, client, workspace, id, *wait, *timeout).await,
        GraphModelCommand::ExecuteQuery {
            workspace,
            id,
            query,
        } => execute_query(cli, client, workspace, id, query).await,
        GraphModelCommand::GetQueryableGraphType { workspace, id } => {
            get_queryable_graph_type(cli, client, workspace, id).await
        }
        GraphModelCommand::Initialize { .. } => {
            Err(crate::errors::FabioError::with_hint(
                crate::errors::ErrorCode::InvalidInput,
                "Graph model initialization is a portal-only operation.",
                "Open the graph model in the Fabric portal to initialize it. \
                 The REST API refresh fails with 'VersionConfig does not exist' \
                 until the portal provisions the internal loading infrastructure. \
                 After portal initialization, use: fabio graph-model refresh-graph --workspace <WS> --id <ID>",
            ).into())
        }
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/graphModels"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    let has_labels = resp
        .items
        .iter()
        .any(|item| item.get("sensitivityLabel").is_some_and(|v| !v.is_null()));

    if has_labels {
        output::render_list_with_token(
            cli,
            &resp.items,
            &["displayName", "id", "description", "sensitivityLabel.id"],
            &["NAME", "ID", "DESCRIPTION", "SENSITIVITY LABEL"],
            "id",
            resp.continuation_token.as_deref(),
        );
    } else {
        output::render_list_with_token(
            cli,
            &resp.items,
            &["displayName", "id", "description"],
            &["NAME", "ID", "DESCRIPTION"],
            "id",
            resp.continuation_token.as_deref(),
        );
    }
    Ok(())
}

async fn show(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/graphModels/{id}"))
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
    ontology: Option<&str>,
    sensitivity_label: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(desc) = description {
        body["description"] = Value::from(desc);
    }

    // If an ontology ID is provided, include it in the definition
    if let Some(ont_id) = ontology {
        let ont_json = serde_json::json!({ "ontologyId": ont_id });
        let encoded = BASE64.encode(ont_json.to_string().as_bytes());
        body["definition"] = serde_json::json!({
            "parts": [{
                "path": "GraphModel.json",
                "payload": encoded,
                "payloadType": "InlineBase64"
            }]
        });
    }
    if let Some(label_id) = sensitivity_label {
        body["sensitivityLabelSettings"] = serde_json::json!({
            "sensitivityLabelId": label_id
        });
    }

    if output::dry_run_guard(
        cli,
        "graph-model create",
        &serde_json::json!({
            "workspace": workspace,
            "displayName": name,
            "description": description,
            "ontology": ontology,
            "sensitivityLabel": sensitivity_label
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(&format!("/workspaces/{workspace}/graphModels"), &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "graph-model create", "Member"))?;
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
            "Example: fabio graph-model update --workspace <WS> --id <ID> --name \"New Name\""
                .to_string(),
        )
        .into());
    }

    let mut body = serde_json::json!({});
    if let Some(n) = name {
        body["displayName"] = Value::from(n);
    }
    if let Some(d) = description {
        body["description"] = Value::from(d);
    }

    if output::dry_run_guard(cli, "graph-model update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/graphModels/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "graph-model update", "Contributor"))?;
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
        "graph-model delete",
        &serde_json::json!({ "workspace": workspace, "id": id, "hardDelete": hard_delete }),
    ) {
        return Ok(());
    }

    let url = if hard_delete {
        format!("/workspaces/{workspace}/graphModels/{id}?hardDelete=true")
    } else {
        format!("/workspaces/{workspace}/graphModels/{id}")
    };

    client
        .delete(&url)
        .await
        .map_err(|e| enrich_forbidden(e, "graph-model delete", "Member"))?;

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
            &format!("/workspaces/{workspace}/graphModels/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "graph-model get-definition", "Contributor"))?;
    if decode {
        let decoded = output::decode_definition_parts(data);
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
                "Example: fabio graph-model update-definition --workspace <WS> --id <ID> --file definition.json".to_string(),
            ).into());
        }
    };

    let encoded = BASE64.encode(definition_json.as_bytes());

    let body = serde_json::json!({
        "definition": {
            "parts": [
                {
                    "path": "GraphModel.json",
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    if output::dry_run_guard(
        cli,
        "graph-model update-definition",
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
            &format!("/workspaces/{workspace}/graphModels/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "graph-model update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Extra operations ────────────────────────────────────────────────────────

async fn refresh_graph(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    wait: bool,
    timeout_secs: u64,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "graph-model refresh-graph",
        &serde_json::json!({ "workspace": workspace, "id": id, "wait": wait, "timeout": timeout_secs }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/graphModels/{id}/jobs/instances?jobType=RefreshGraph"
            ),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "graph-model refresh-graph", "Contributor"))?;

    if !wait {
        if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
            let obj = serde_json::json!({ "id": id, "status": "refresh_triggered" });
            output::render_object(cli, &obj, "status");
        } else {
            output::render_object(cli, &data, "id");
        }
        return Ok(());
    }

    // Poll graph model status until refresh completes
    let poll_interval = Duration::from_secs(5);
    let max_wait = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > max_wait {
            return Err(FabioError::new(
                ErrorCode::Timeout,
                format!(
                    "Graph refresh timed out after {timeout_secs}s. Use 'graph-model show' to check status."
                ),
            )
            .into());
        }

        sleep(poll_interval).await;

        let model_data = client
            .get(&format!("/workspaces/{workspace}/graphModels/{id}"))
            .await?;

        let status_str = model_data
            .pointer("/properties/lastDataLoadingStatus/status")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        match status_str {
            "Completed" => {
                let obj = serde_json::json!({
                    "id": id,
                    "status": "Completed",
                    "queryReadiness": model_data.pointer("/properties/queryReadiness").and_then(|v| v.as_str()).unwrap_or("Unknown")
                });
                output::render_object(cli, &obj, "status");
                return Ok(());
            }
            "Failed" => {
                return Err(FabioError::new(
                    ErrorCode::ApiError,
                    format!("Graph refresh failed for model {id}"),
                )
                .into());
            }
            _ => {} // Continue polling (NotStarted, InProgress)
        }
    }
}

async fn execute_query(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    query: &str,
) -> Result<()> {
    let body = serde_json::json!({ "query": query });

    let data = client
        .post(
            &format!("/workspaces/{workspace}/graphModels/{id}/executeQuery?preview=true"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "graph-model execute-query", "Contributor"))?;
    output::render_object(cli, &data, "data");
    Ok(())
}

async fn get_queryable_graph_type(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/graphModels/{id}/getQueryableGraphType?preview=true"
        ))
        .await?;
    output::render_object(cli, &data, "data");
    Ok(())
}
