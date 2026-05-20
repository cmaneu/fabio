use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum DeploymentPipelineCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List deployment pipelines
    #[command(display_order = 1)]
    List,
    /// Show details of a deployment pipeline
    #[command(display_order = 2)]
    Show {
        /// Deployment pipeline ID
        #[arg(long)]
        id: String,
    },
    /// Create a new deployment pipeline
    #[command(display_order = 3)]
    Create {
        /// Display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update a deployment pipeline
    #[command(display_order = 4)]
    Update {
        /// Deployment pipeline ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a deployment pipeline
    #[command(display_order = 5)]
    Delete {
        /// Deployment pipeline ID
        #[arg(long)]
        id: String,
    },

    // ── Stages ───────────────────────────────────────────────────────────
    /// List stages in a deployment pipeline
    #[command(display_order = 10)]
    ListStages {
        /// Deployment pipeline ID
        #[arg(long)]
        id: String,
    },
    /// List items in a deployment pipeline stage
    #[command(display_order = 11)]
    ListStageItems {
        /// Deployment pipeline ID
        #[arg(long)]
        id: String,

        /// Stage ID
        #[arg(long)]
        stage_id: String,
    },
    /// Assign a workspace to a deployment pipeline stage
    #[command(display_order = 12)]
    AssignWorkspace {
        /// Deployment pipeline ID
        #[arg(long)]
        id: String,

        /// Stage ID to assign the workspace to
        #[arg(long)]
        stage_id: String,

        /// Workspace ID to assign
        #[arg(long)]
        workspace: String,
    },
    /// Unassign the workspace from a deployment pipeline stage
    #[command(display_order = 13)]
    UnassignWorkspace {
        /// Deployment pipeline ID
        #[arg(long)]
        id: String,

        /// Stage ID to unassign
        #[arg(long)]
        stage_id: String,
    },

    // ── Deploy ───────────────────────────────────────────────────────────
    /// Deploy items from one stage to another
    #[command(display_order = 20)]
    Deploy {
        /// Deployment pipeline ID
        #[arg(long)]
        id: String,

        /// Source stage ID
        #[arg(long)]
        source_stage_id: String,

        /// Target stage ID (if omitted, deploys to the next stage)
        #[arg(long)]
        target_stage_id: Option<String>,

        /// Items to deploy as JSON array (if omitted, all items are deployed).
        /// Example: '[{"itemId":"...","itemType":"Notebook"}]'
        #[arg(long)]
        items: Option<String>,

        /// Optional note for this deployment
        #[arg(long)]
        note: Option<String>,
    },
}

pub async fn execute(
    cli: &Cli,
    client: &FabricClient,
    command: &DeploymentPipelineCommand,
) -> Result<()> {
    match command {
        DeploymentPipelineCommand::List => list(cli, client).await,
        DeploymentPipelineCommand::Show { id } => show(cli, client, id).await,
        DeploymentPipelineCommand::Create { name, description } => {
            create(cli, client, name, description.as_deref()).await
        }
        DeploymentPipelineCommand::Update {
            id,
            name,
            description,
        } => update(cli, client, id, name.as_deref(), description.as_deref()).await,
        DeploymentPipelineCommand::Delete { id } => delete(cli, client, id).await,
        DeploymentPipelineCommand::ListStages { id } => list_stages(cli, client, id).await,
        DeploymentPipelineCommand::ListStageItems { id, stage_id } => {
            list_stage_items(cli, client, id, stage_id).await
        }
        DeploymentPipelineCommand::AssignWorkspace {
            id,
            stage_id,
            workspace,
        } => assign_workspace(cli, client, id, stage_id, workspace).await,
        DeploymentPipelineCommand::UnassignWorkspace { id, stage_id } => {
            unassign_workspace(cli, client, id, stage_id).await
        }
        DeploymentPipelineCommand::Deploy {
            id,
            source_stage_id,
            target_stage_id,
            items,
            note,
        } => {
            deploy(
                cli,
                client,
                id,
                source_stage_id,
                target_stage_id.as_deref(),
                items.as_deref(),
                note.as_deref(),
            )
            .await
        }
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list("/deploymentPipelines", "value", cli.all)
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

async fn show(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    let data = client.get(&format!("/deploymentPipelines/{id}")).await?;
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

    if output::dry_run_guard(cli, "deployment-pipeline create", &body) {
        return Ok(());
    }

    let data = client
        .post("/deploymentPipelines", &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "deployment-pipeline create", "Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --name or --description must be provided".to_string(),
            "Example: fabio deployment-pipeline update --id <ID> --name \"New Name\"".to_string(),
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

    if output::dry_run_guard(cli, "deployment-pipeline update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/deploymentPipelines/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "deployment-pipeline update", "Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "deployment-pipeline delete",
        &serde_json::json!({ "id": id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/deploymentPipelines/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "deployment-pipeline delete", "Admin"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Stages ──────────────────────────────────────────────────────────────────

async fn list_stages(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/deploymentPipelines/{id}/stages"),
            "value",
            cli.all,
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id", "order", "workspaceId"],
        &["NAME", "ID", "ORDER", "WORKSPACE"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn list_stage_items(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    stage_id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/deploymentPipelines/{id}/stages/{stage_id}/items"),
            "value",
            cli.all,
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["itemDisplayName", "itemId", "itemType"],
        &["NAME", "ID", "TYPE"],
        "itemId",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn assign_workspace(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    stage_id: &str,
    workspace: &str,
) -> Result<()> {
    let body = serde_json::json!({ "workspaceId": workspace });

    if output::dry_run_guard(cli, "deployment-pipeline assign-workspace", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/deploymentPipelines/{id}/stages/{stage_id}/assignWorkspace"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "deployment-pipeline assign-workspace", "Admin"))?;

    let obj = serde_json::json!({
        "pipelineId": id,
        "stageId": stage_id,
        "workspaceId": workspace,
        "status": "assigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn unassign_workspace(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    stage_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "deployment-pipeline unassign-workspace",
        &serde_json::json!({ "pipelineId": id, "stageId": stage_id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/deploymentPipelines/{id}/stages/{stage_id}/unassignWorkspace"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "deployment-pipeline unassign-workspace", "Admin"))?;

    let obj = serde_json::json!({
        "pipelineId": id,
        "stageId": stage_id,
        "status": "unassigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Deploy ──────────────────────────────────────────────────────────────────

async fn deploy(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    source_stage_id: &str,
    target_stage_id: Option<&str>,
    items: Option<&str>,
    note: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({
        "sourceStageId": source_stage_id,
    });
    if let Some(target) = target_stage_id {
        body["targetStageId"] = Value::String(target.to_string());
    }
    if let Some(items_json) = items {
        let items_value: Value = serde_json::from_str(items_json)
            .map_err(|e| anyhow::anyhow!("Invalid --items JSON: {e}. Expected array, e.g.: [{{\"itemId\":\"...\",\"itemType\":\"Notebook\"}}]"))?;
        body["items"] = items_value;
    }
    if let Some(n) = note {
        body["note"] = Value::String(n.to_string());
    }

    if output::dry_run_guard(cli, "deployment-pipeline deploy", &body) {
        return Ok(());
    }

    let data = client
        .post(&format!("/deploymentPipelines/{id}/deploy"), &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "deployment-pipeline deploy", "Contributor"))?;

    // API may return LRO or immediate result
    let obj = if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        serde_json::json!({
            "pipelineId": id,
            "status": "accepted"
        })
    } else {
        data
    };
    output::render_object(cli, &obj, "status");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deploy_command_can_parse_items_json() {
        let items = r#"[{"itemId":"abc-123","itemType":"Notebook"}]"#;
        let val: Value = serde_json::from_str(items).unwrap();
        assert!(val.is_array());
        assert_eq!(val.as_array().unwrap().len(), 1);
    }
}
