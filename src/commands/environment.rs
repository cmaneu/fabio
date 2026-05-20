use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum EnvironmentCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List environments in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of an environment
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Create a new environment
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update environment properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete an environment
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },

    // ── Publish ──────────────────────────────────────────────────────────
    /// Publish staged changes to an environment
    #[command(display_order = 10)]
    Publish {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Cancel a pending publish operation
    #[command(display_order = 11)]
    CancelPublish {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Get the published Spark settings (compute/pool/driver/executor)
    #[command(display_order = 12)]
    GetSparkSettings {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Get the staging (draft) Spark settings
    #[command(display_order = 13)]
    GetStagingSparkSettings {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &EnvironmentCommand) -> Result<()> {
    match command {
        EnvironmentCommand::List { workspace } => list(cli, client, workspace).await,
        EnvironmentCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        EnvironmentCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref()).await,
        EnvironmentCommand::Update {
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
        EnvironmentCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        EnvironmentCommand::Publish { workspace, id } => publish(cli, client, workspace, id).await,
        EnvironmentCommand::CancelPublish { workspace, id } => {
            cancel_publish(cli, client, workspace, id).await
        }
        EnvironmentCommand::GetSparkSettings { workspace, id } => {
            get_spark_settings(cli, client, workspace, id).await
        }
        EnvironmentCommand::GetStagingSparkSettings { workspace, id } => {
            get_staging_spark_settings(cli, client, workspace, id).await
        }
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/environments"),
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
        .get(&format!("/workspaces/{workspace}/environments/{id}"))
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
        "environment create",
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
            &format!("/workspaces/{workspace}/environments"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "environment create", "Member"))?;
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
            "Example: fabio environment update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "environment update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/environments/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "environment update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "environment delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/environments/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "environment delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Publish ─────────────────────────────────────────────────────────────────

async fn publish(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "environment publish",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/environments/{id}/staging/publish"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "environment publish", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "status": "publish_started"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn cancel_publish(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    client
        .post(
            &format!("/workspaces/{workspace}/environments/{id}/staging/cancelPublish"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "environment cancel-publish", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "status": "publish_cancelled"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Spark Settings ──────────────────────────────────────────────────────────

async fn get_spark_settings(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/environments/{id}/sparkcompute"
        ))
        .await?;
    output::render_object(cli, &data, "instancePool");
    Ok(())
}

async fn get_staging_spark_settings(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/environments/{id}/staging/sparkcompute"
        ))
        .await?;
    output::render_object(cli, &data, "instancePool");
    Ok(())
}
