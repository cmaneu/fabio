use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum EventhouseCommand {
    /// List eventhouses in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of an eventhouse
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventhouse ID
        #[arg(long)]
        id: String,
    },
    /// Create a new eventhouse
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventhouse display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update eventhouse properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventhouse ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete an eventhouse
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Eventhouse ID
        #[arg(long)]
        id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &EventhouseCommand) -> Result<()> {
    match command {
        EventhouseCommand::List { workspace } => list(cli, client, workspace).await,
        EventhouseCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        EventhouseCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref()).await,
        EventhouseCommand::Update {
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
        EventhouseCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/eventhouses"),
            "value",
            cli.all,
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
        .get(&format!("/workspaces/{workspace}/eventhouses/{id}"))
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
        "eventhouse create",
        &serde_json::json!({
            "workspace": workspace,
            "displayName": name,
            "description": description
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(&format!("/workspaces/{workspace}/eventhouses"), &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "eventhouse create", "Member"))?;
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
            "Example: fabio eventhouse update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "eventhouse update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/eventhouses/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "eventhouse update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "eventhouse delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/eventhouses/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "eventhouse delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}
