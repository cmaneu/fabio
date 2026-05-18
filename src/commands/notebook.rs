use anyhow::Result;
use clap::Subcommand;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum NotebookCommand {
    /// Create a new notebook
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Notebook display name
        #[arg(long)]
        name: String,

        /// Notebook content (Python/PySpark code)
        #[arg(long)]
        content: Option<String>,
    },
    /// Get the definition (source code) of a notebook
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,
    },
    /// Run a notebook
    Run {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,
    },
    /// Check the status of a notebook run
    Status {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,

        /// Job instance ID
        #[arg(long)]
        job_id: String,
    },
    /// Stop a running notebook
    Stop {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,

        /// Job instance ID
        #[arg(long)]
        job_id: String,
    },
    /// Delete a notebook
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Notebook item ID
        #[arg(long)]
        id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &NotebookCommand) -> Result<()> {
    match command {
        NotebookCommand::Create {
            workspace,
            name,
            content,
        } => create(cli, client, workspace, name, content.as_deref()).await,
        NotebookCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        NotebookCommand::Run { workspace, id } => run(cli, client, workspace, id).await,
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
        NotebookCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
    }
}

async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    content: Option<&str>,
) -> Result<()> {
    let code = content.unwrap_or("# New notebook\nprint('Hello from Fabric!')");

    // Build ipynb structure (source must be list of strings per spec)
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
        .await?;
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

async fn run(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let job_id = client.run_notebook(workspace, id).await?;

    let obj = serde_json::json!({
        "itemId": id,
        "jobId": job_id,
        "status": "started"
    });

    output::render_object(cli, &obj, "jobId");
    Ok(())
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
        .await?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}
