use std::time::Duration;

use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;
use tokio::time::sleep;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::commands::jobs::{JobEntry, JobLedger};
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum NotebookCommand {
    // ── Lifecycle ────────────────────────────────────────────────────────
    /// List notebooks in a workspace
    #[command(display_order = 0)]
    List {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// Show details of a notebook
    #[command(display_order = 0)]
    Show {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,
    },
    /// Create a new notebook
    #[command(display_order = 1)]
    Create {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook display name
        #[arg(long)]
        name: String,

        /// Notebook content (Python/PySpark code)
        #[arg(long, visible_alias = "source")]
        content: Option<String>,

        /// Default lakehouse ID (binds the notebook so relative paths like Files/ and Tables/ work)
        #[arg(long)]
        lakehouse: Option<String>,
    },
    /// Update notebook properties (name and/or description)
    #[command(display_order = 2)]
    Update {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Get the definition (source code) of a notebook
    #[command(display_order = 3)]
    GetDefinition {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition (source code) of a notebook
    #[command(display_order = 4)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,

        /// Python/PySpark code content (replaces entire notebook)
        #[arg(long)]
        content: Option<String>,

        /// Path to .ipynb or .py file
        #[arg(long)]
        file: Option<String>,
    },
    /// Delete a notebook
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,
    },

    // ── Execution ────────────────────────────────────────────────────────
    /// Run a notebook
    #[command(display_order = 10)]
    Run {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,

        /// Wait for the notebook run to complete (polls until finished)
        #[arg(long)]
        wait: bool,

        /// Maximum time to wait in seconds (default: 600)
        #[arg(long, default_value = "600")]
        timeout: u64,
    },
    /// Check the status of a notebook run
    #[command(display_order = 11)]
    Status {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,

        /// Job instance ID
        #[arg(long)]
        job_id: String,
    },
    /// Get details of a specific job instance
    #[command(name = "get-job-instance", display_order = 12)]
    GetJobInstance {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,

        /// Job instance ID
        #[arg(long)]
        job_instance_id: String,
    },
    /// Stop a running notebook
    #[command(display_order = 13)]
    Stop {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,

        /// Job instance ID
        #[arg(long)]
        job_id: String,
    },

    // ── Livy Sessions ────────────────────────────────────────────────────
    /// List Livy sessions for a notebook
    #[command(name = "list-livy-sessions", display_order = 15)]
    ListLivySessions {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,
    },
    /// Get details of a Livy session
    #[command(name = "get-livy-session", display_order = 16)]
    GetLivySession {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,

        /// Livy session ID
        #[arg(long)]
        livy_id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &NotebookCommand) -> Result<()> {
    match command {
        NotebookCommand::List { workspace } => list(cli, client, workspace).await,
        NotebookCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        NotebookCommand::Create {
            workspace,
            name,
            content,
            lakehouse,
        } => {
            create(
                cli,
                client,
                workspace,
                name,
                content.as_deref(),
                lakehouse.as_deref(),
            )
            .await
        }
        NotebookCommand::Update {
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
        NotebookCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        NotebookCommand::UpdateDefinition {
            workspace,
            id,
            content,
            file,
        } => {
            update_definition(
                cli,
                client,
                workspace,
                id,
                content.as_deref(),
                file.as_deref(),
            )
            .await
        }
        NotebookCommand::Run {
            workspace,
            id,
            wait,
            timeout,
        } => run(cli, client, workspace, id, *wait, *timeout).await,
        NotebookCommand::Status {
            workspace,
            id,
            job_id,
        } => status(cli, client, workspace, id, job_id).await,
        NotebookCommand::Stop {
            workspace,
            id,
            job_id,
        } => stop(cli, client, workspace, id, job_id).await,
        NotebookCommand::GetJobInstance {
            workspace,
            id,
            job_instance_id,
        } => get_job_instance(cli, client, workspace, id, job_instance_id).await,
        NotebookCommand::ListLivySessions { workspace, id } => {
            list_livy_sessions(cli, client, workspace, id).await
        }
        NotebookCommand::GetLivySession {
            workspace,
            id,
            livy_id,
        } => get_livy_session(cli, client, workspace, id, livy_id).await,
        NotebookCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/notebooks"),
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
        .get(&format!("/workspaces/{workspace}/notebooks/{id}"))
        .await?;
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
            "Example: fabio notebook update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "notebook update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/notebooks/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "notebook update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    content: Option<&str>,
    file: Option<&str>,
) -> Result<()> {
    if content.is_none() && file.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Either --content or --file must be provided".to_string(),
            "Example: fabio notebook update-definition --workspace <WS> --id <ID> --content \"print('hello')\""
                .to_string(),
        )
        .into());
    }

    // Build the definition payload
    let encoded = if let Some(file_path) = file {
        let file_content = std::fs::read(file_path).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Failed to read file '{file_path}': {e}"),
                "Provide a valid .py or .ipynb file path.".to_string(),
            )
        })?;
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &file_content)
    } else {
        // Build ipynb from content
        let code = content.unwrap();
        let notebook_json = serde_json::json!({
            "nbformat": 4,
            "nbformat_minor": 5,
            "metadata": {
                "language_info": { "name": "python" }
            },
            "cells": [{
                "cell_type": "code",
                "metadata": {},
                "source": code.lines().map(|l| format!("{l}\n")).collect::<Vec<_>>(),
                "outputs": []
            }]
        });
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            serde_json::to_string(&notebook_json)?,
        )
    };

    let body = serde_json::json!({
        "definition": {
            "format": "ipynb",
            "parts": [{
                "path": "notebook-content.py",
                "payload": encoded,
                "payloadType": "InlineBase64"
            }]
        }
    });

    if output::dry_run_guard(cli, "notebook update-definition", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "notebook update-definition", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "workspace": workspace,
        "status": "definition_updated"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Create ──────────────────────────────────────────────────────────────────

async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    content: Option<&str>,
    lakehouse: Option<&str>,
) -> Result<()> {
    let code = content.unwrap_or("# New notebook\nprint('Hello from Fabric!')");

    // Build ipynb structure (source must be list of strings per spec)
    let metadata = lakehouse.map_or_else(
        || {
            serde_json::json!({
                "language_info": { "name": "python" }
            })
        },
        |lh_id| {
            // Include Fabric trident metadata to bind the default lakehouse.
            // Without this, relative paths (Files/, Tables/) and saveAsTable() won't work.
            serde_json::json!({
                "language_info": { "name": "python" },
                "trident": {
                    "lakehouse": {
                        "default_lakehouse": lh_id,
                        "default_lakehouse_name": "",
                        "default_lakehouse_workspace_id": workspace,
                        "known_lakehouses": [{ "id": lh_id }]
                    }
                }
            })
        },
    );

    let notebook_json = serde_json::json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": metadata,
        "cells": [{
            "cell_type": "code",
            "metadata": {},
            "source": code.lines().map(|l| format!("{l}\n")).collect::<Vec<_>>(),
            "outputs": []
        }]
    });

    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&notebook_json)?,
    );

    let body = serde_json::json!({
        "displayName": name,
        "type": "Notebook",
        "definition": {
            "format": "ipynb",
            "parts": [{
                "path": "notebook-content.py",
                "payload": encoded,
                "payloadType": "InlineBase64"
            }]
        }
    });

    let data = client
        .post(&format!("/workspaces/{workspace}/items"), &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "notebook create", "Member"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn get_definition(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await?;
    output::render_object(cli, &data, "definition");
    Ok(())
}

async fn run(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    wait: bool,
    timeout_secs: u64,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "notebook run",
        &serde_json::json!({"workspace": workspace, "id": id, "wait": wait, "timeout": timeout_secs}),
    ) {
        return Ok(());
    }

    let job_id = client
        .run_notebook(workspace, id)
        .await
        .map_err(|e| enrich_forbidden(e, "notebook run", "Contributor"))?;

    // Record job in local ledger
    let entry = JobEntry::new(&job_id, "notebook-run", workspace, id);
    let _ = JobLedger::append(&entry);

    if !wait {
        let obj = serde_json::json!({
            "itemId": id,
            "jobId": job_id,
            "status": "started"
        });
        output::render_object(cli, &obj, "jobId");
        return Ok(());
    }

    // Poll until completion
    let poll_interval = Duration::from_secs(5);
    let max_wait = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > max_wait {
            let _ = JobLedger::update(&job_id, "timeout", None);
            return Err(FabioError::new(
                ErrorCode::Timeout,
                format!(
                    "Notebook run timed out after {timeout_secs}s. Job ID: {job_id}. Use 'notebook status' to check progress."
                ),
            )
            .into());
        }

        sleep(poll_interval).await;

        let data = client
            .get(&format!(
                "/workspaces/{workspace}/items/{id}/jobs/instances/{job_id}"
            ))
            .await?;

        let status_str = data
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        match status_str {
            "Completed" => {
                let _ = JobLedger::update(&job_id, "completed", None);
                let obj = serde_json::json!({
                    "itemId": id,
                    "jobId": job_id,
                    "status": "Completed"
                });
                output::render_object(cli, &obj, "status");
                return Ok(());
            }
            "Failed" => {
                let message = data
                    .get("failureReason")
                    .and_then(|r| r.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("Notebook run failed");
                let _ = JobLedger::update(&job_id, "failed", Some(message));
                return Err(FabioError::new(ErrorCode::ApiError, message).into());
            }
            "Cancelled" => {
                let _ = JobLedger::update(&job_id, "cancelled", None);
                return Err(
                    FabioError::new(ErrorCode::ApiError, "Notebook run was cancelled").into(),
                );
            }
            // NotStarted, InProgress, Deduped - keep polling
            _ => {}
        }
    }
}

async fn status(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    job_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/items/{id}/jobs/instances/{job_id}"
        ))
        .await?;
    output::render_object(cli, &data, "status");
    Ok(())
}

async fn stop(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    job_id: &str,
) -> Result<()> {
    client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/jobs/instances/{job_id}/cancel"),
            &serde_json::json!({}),
            false,
        )
        .await?;

    let obj = serde_json::json!({
        "jobId": job_id,
        "status": "cancelled"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    client
        .delete(&format!("/workspaces/{workspace}/items/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "notebook delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Job Instance & Livy Sessions ────────────────────────────────────────────

async fn get_job_instance(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    job_instance_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/notebooks/{id}/jobs/execute/instances/{job_instance_id}?beta=true"
        ))
        .await?;
    output::render_object(cli, &data, "status");
    Ok(())
}

async fn list_livy_sessions(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/notebooks/{id}/livySessions"
        ))
        .await?;

    if let Some(arr) = data.get("value").and_then(|v| v.as_array()) {
        output::render_list_with_token(
            cli,
            arr,
            &["id", "state", "appId"],
            &["ID", "STATE", "APP_ID"],
            "id",
            None,
        );
    } else {
        output::render_object(cli, &data, "sessions");
    }
    Ok(())
}

async fn get_livy_session(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    livy_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/notebooks/{id}/livySessions/{livy_id}"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}
