use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum DataPipelineCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List data pipelines in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a data pipeline
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Data pipeline ID
        #[arg(long)]
        id: String,
    },
    /// Create a new data pipeline
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Pipeline display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update data pipeline properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Data pipeline ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a data pipeline
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Data pipeline ID
        #[arg(long)]
        id: String,
    },

    // ── Execution ────────────────────────────────────────────────────────
    /// Run a data pipeline
    #[command(display_order = 10)]
    Run {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Data pipeline ID
        #[arg(long)]
        id: String,
    },
}

pub async fn execute(
    cli: &Cli,
    client: &FabricClient,
    command: &DataPipelineCommand,
) -> Result<()> {
    match command {
        DataPipelineCommand::List { workspace } => list(cli, client, workspace).await,
        DataPipelineCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        DataPipelineCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref()).await,
        DataPipelineCommand::Update {
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
        DataPipelineCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        DataPipelineCommand::Run { workspace, id } => run(cli, client, workspace, id).await,
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/dataPipelines"),
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
        .get(&format!("/workspaces/{workspace}/dataPipelines/{id}"))
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
        "data-pipeline create",
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
            &format!("/workspaces/{workspace}/dataPipelines"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-pipeline create", "Member"))?;
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
            "Example: fabio data-pipeline update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "data-pipeline update", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/dataPipelines/{id}"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-pipeline update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-pipeline delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/dataPipelines/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "data-pipeline delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Execution ───────────────────────────────────────────────────────────────

async fn run(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-pipeline run",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/jobs/instances?jobType=Pipeline"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-pipeline run", "Contributor"))?;

    // The API returns 202 with job info or empty; construct response
    let obj = if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        serde_json::json!({
            "itemId": id,
            "status": "started"
        })
    } else {
        data
    };
    output::render_object(cli, &obj, "status");
    Ok(())
}
