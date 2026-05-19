use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum WorkspaceCommand {
    /// List all workspaces
    List,
    /// Show details of a workspace
    Show {
        /// Workspace ID
        #[arg(long)]
        id: String,
    },
    /// Create a new workspace
    Create {
        /// Display name for the workspace
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a workspace
    Delete {
        /// Workspace ID
        #[arg(long)]
        id: String,
    },
    /// Assign a workspace to a capacity
    AssignCapacity {
        /// Workspace ID
        #[arg(long)]
        id: String,

        /// Target capacity ID
        #[arg(short, long)]
        capacity: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &WorkspaceCommand) -> Result<()> {
    match command {
        WorkspaceCommand::List => list(cli, client).await,
        WorkspaceCommand::Show { id } => show(cli, client, id).await,
        WorkspaceCommand::Create { name, description } => {
            create(cli, client, name, description.as_deref()).await
        }
        WorkspaceCommand::Delete { id } => delete(cli, client, id).await,
        WorkspaceCommand::AssignCapacity { id, capacity } => {
            assign_capacity(cli, client, id, capacity).await
        }
    }
}

async fn list(cli: &Cli, client: &FabricClient) -> Result<()> {
    let data = client.get("/workspaces").await?;
    let items = data
        .get("value")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    output::render_list(
        cli,
        &items,
        &["displayName", "id", "type"],
        &["NAME", "ID", "TYPE"],
        "id",
    );
    Ok(())
}

async fn show(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    let data = client.get(&format!("/workspaces/{id}")).await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn create(
    cli: &Cli,
    client: &FabricClient,
    name: &str,
    description: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if output::dry_run_guard(cli, "workspace create", &body) {
        return Ok(());
    }

    let data = client.post("/workspaces", &body, false).await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    if output::dry_run_guard(cli, "workspace delete", &serde_json::json!({ "id": id })) {
        return Ok(());
    }

    client.delete(&format!("/workspaces/{id}")).await?;
    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn assign_capacity(cli: &Cli, client: &FabricClient, id: &str, capacity: &str) -> Result<()> {
    let body = serde_json::json!({ "capacityId": capacity });

    if output::dry_run_guard(
        cli,
        "workspace assign-capacity",
        &serde_json::json!({ "workspaceId": id, "capacityId": capacity }),
    ) {
        return Ok(());
    }

    client
        .post(&format!("/workspaces/{id}/assignToCapacity"), &body, false)
        .await?;

    let obj = serde_json::json!({
        "workspaceId": id,
        "capacityId": capacity,
        "status": "assigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}
