use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

/// Known job types for the Fabric Job Scheduler API.
#[allow(dead_code)]
pub const KNOWN_JOB_TYPES: &[&str] = &[
    "DefaultJob",
    "RunNotebook",
    "Pipeline",
    "TableMaintenance",
    "SparkJob",
];

#[derive(Debug, Subcommand)]
pub enum JobSchedulerCommand {
    // ── Instances ────────────────────────────────────────────────────────
    /// List job instances for an item
    #[command(display_order = 1)]
    ListInstances {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,
    },
    /// Show details of a job instance
    #[command(display_order = 2)]
    GetInstance {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Job instance ID
        #[arg(long)]
        job_instance_id: String,
    },
    /// Run an on-demand job for an item
    #[command(display_order = 3)]
    RunOnDemand {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Job type (e.g., `DefaultJob`, `RunNotebook`, `Pipeline`, `TableMaintenance`, `SparkJob`)
        #[arg(long, default_value = "DefaultJob")]
        job_type: String,

        /// Execution data as JSON (optional, depends on job type)
        #[arg(long)]
        execution_data: Option<String>,
    },
    /// Cancel a running job instance
    #[command(display_order = 4)]
    CancelInstance {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Job instance ID
        #[arg(long)]
        job_instance_id: String,
    },

    // ── Schedules ────────────────────────────────────────────────────────
    /// List schedules for an item job type
    #[command(display_order = 10)]
    ListSchedules {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Job type (e.g., `DefaultJob`, `RunNotebook`, `Pipeline`)
        #[arg(long, default_value = "DefaultJob")]
        job_type: String,
    },
    /// Show details of a specific schedule
    #[command(display_order = 11)]
    GetSchedule {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Job type
        #[arg(long, default_value = "DefaultJob")]
        job_type: String,

        /// Schedule ID
        #[arg(long)]
        schedule_id: String,
    },
    /// Create a schedule for an item job type
    #[command(display_order = 12)]
    CreateSchedule {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Job type
        #[arg(long, default_value = "DefaultJob")]
        job_type: String,

        /// Whether the schedule is enabled
        #[arg(long, default_value_t = true)]
        enabled: bool,

        /// Schedule configuration as JSON (cron or recurrence)
        /// Example: '{"type":"Cron","expression":"0 0 * * *","timezone":"UTC"}'
        #[arg(long)]
        config: String,
    },
    /// Update an existing schedule
    #[command(display_order = 13)]
    UpdateSchedule {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Job type
        #[arg(long, default_value = "DefaultJob")]
        job_type: String,

        /// Schedule ID
        #[arg(long)]
        schedule_id: String,

        /// Whether the schedule is enabled
        #[arg(long)]
        enabled: Option<bool>,

        /// Updated schedule configuration as JSON
        #[arg(long)]
        config: Option<String>,
    },
    /// Delete a schedule
    #[command(display_order = 14)]
    DeleteSchedule {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Job type
        #[arg(long, default_value = "DefaultJob")]
        job_type: String,

        /// Schedule ID
        #[arg(long)]
        schedule_id: String,
    },
}

pub async fn execute(
    cli: &Cli,
    client: &FabricClient,
    command: &JobSchedulerCommand,
) -> Result<()> {
    match command {
        // ── Instances ────────────────────────────────────────────────────
        JobSchedulerCommand::ListInstances { workspace, id } => {
            list_instances(cli, client, workspace, id).await
        }
        JobSchedulerCommand::GetInstance {
            workspace,
            id,
            job_instance_id,
        } => get_instance(cli, client, workspace, id, job_instance_id).await,
        JobSchedulerCommand::RunOnDemand {
            workspace,
            id,
            job_type,
            execution_data,
        } => {
            run_on_demand(
                cli,
                client,
                workspace,
                id,
                job_type,
                execution_data.as_deref(),
            )
            .await
        }
        JobSchedulerCommand::CancelInstance {
            workspace,
            id,
            job_instance_id,
        } => cancel_instance(cli, client, workspace, id, job_instance_id).await,
        // ── Schedules ────────────────────────────────────────────────────
        JobSchedulerCommand::ListSchedules {
            workspace,
            id,
            job_type,
        } => list_schedules(cli, client, workspace, id, job_type).await,
        JobSchedulerCommand::GetSchedule {
            workspace,
            id,
            job_type,
            schedule_id,
        } => get_schedule(cli, client, workspace, id, job_type, schedule_id).await,
        JobSchedulerCommand::CreateSchedule {
            workspace,
            id,
            job_type,
            enabled,
            config,
        } => create_schedule(cli, client, workspace, id, job_type, *enabled, config).await,
        JobSchedulerCommand::UpdateSchedule {
            workspace,
            id,
            job_type,
            schedule_id,
            enabled,
            config,
        } => {
            update_schedule(
                cli,
                client,
                workspace,
                id,
                job_type,
                schedule_id,
                *enabled,
                config.as_deref(),
            )
            .await
        }
        JobSchedulerCommand::DeleteSchedule {
            workspace,
            id,
            job_type,
            schedule_id,
        } => delete_schedule(cli, client, workspace, id, job_type, schedule_id).await,
    }
}

// ─── Instances ───────────────────────────────────────────────────────────────

async fn list_instances(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/items/{item_id}/jobs/instances"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "jobType", "status", "startTimeUtc", "endTimeUtc"],
        &["ID", "JOB TYPE", "STATUS", "START", "END"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn get_instance(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    job_instance_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/items/{item_id}/jobs/instances/{job_instance_id}"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn run_on_demand(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    job_type: &str,
    execution_data: Option<&str>,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "job-scheduler run-on-demand",
        &serde_json::json!({
            "workspace": workspace,
            "itemId": item_id,
            "jobType": job_type
        }),
    ) {
        return Ok(());
    }

    let body = if let Some(ed) = execution_data {
        let exec_value: Value = serde_json::from_str(ed)
            .map_err(|e| anyhow::anyhow!("Invalid --execution-data JSON: {e}"))?;
        serde_json::json!({ "executionData": exec_value })
    } else {
        serde_json::json!({})
    };

    let data = client
        .post(
            &format!("/workspaces/{workspace}/items/{item_id}/jobs/instances?jobType={job_type}"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "job-scheduler run-on-demand", "Contributor"))?;

    // The API typically returns 202 with Location header; data may be empty
    let obj = if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        serde_json::json!({
            "itemId": item_id,
            "jobType": job_type,
            "status": "accepted"
        })
    } else {
        data
    };
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn cancel_instance(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    job_instance_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "job-scheduler cancel-instance",
        &serde_json::json!({
            "workspace": workspace,
            "itemId": item_id,
            "jobInstanceId": job_instance_id
        }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!(
                "/workspaces/{workspace}/items/{item_id}/jobs/instances/{job_instance_id}/cancel"
            ),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "job-scheduler cancel-instance", "Contributor"))?;

    let obj = serde_json::json!({
        "jobInstanceId": job_instance_id,
        "status": "cancelled"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Schedules ───────────────────────────────────────────────────────────────

async fn list_schedules(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    job_type: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/items/{item_id}/jobs/{job_type}/schedules"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "enabled", "createdDateTime"],
        &["ID", "ENABLED", "CREATED"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn get_schedule(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    job_type: &str,
    schedule_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/items/{item_id}/jobs/{job_type}/schedules/{schedule_id}"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn create_schedule(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    job_type: &str,
    enabled: bool,
    config: &str,
) -> Result<()> {
    let config_value: Value = serde_json::from_str(config)
        .map_err(|e| anyhow::anyhow!("Invalid --config JSON: {e}. Expected schedule configuration, e.g.: {{\"type\":\"Cron\",\"expression\":\"0 0 * * *\",\"timezone\":\"UTC\"}}"))?;

    let mut body = serde_json::json!({
        "enabled": enabled,
    });
    // Merge config into body at top-level (Fabric API expects flat schedule object)
    if let Some(config_obj) = config_value.as_object() {
        for (k, v) in config_obj {
            body[k] = v.clone();
        }
    } else {
        body["configuration"] = config_value;
    }

    if output::dry_run_guard(cli, "job-scheduler create-schedule", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/items/{item_id}/jobs/{job_type}/schedules"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "job-scheduler create-schedule", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn update_schedule(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    job_type: &str,
    schedule_id: &str,
    enabled: Option<bool>,
    config: Option<&str>,
) -> Result<()> {
    if enabled.is_none() && config.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --enabled or --config must be provided".to_string(),
            "Example: fabio job-scheduler update-schedule --workspace <WS> --id <ID> --schedule-id <SID> --enabled false".to_string(),
        )
        .into());
    }

    let mut body = serde_json::json!({});
    if let Some(e) = enabled {
        body["enabled"] = Value::Bool(e);
    }
    if let Some(c) = config {
        let config_value: Value =
            serde_json::from_str(c).map_err(|e| anyhow::anyhow!("Invalid --config JSON: {e}"))?;
        if let Some(config_obj) = config_value.as_object() {
            for (k, v) in config_obj {
                body[k] = v.clone();
            }
        } else {
            body["configuration"] = config_value;
        }
    }

    if output::dry_run_guard(cli, "job-scheduler update-schedule", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!(
                "/workspaces/{workspace}/items/{item_id}/jobs/{job_type}/schedules/{schedule_id}"
            ),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "job-scheduler update-schedule", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete_schedule(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    job_type: &str,
    schedule_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "job-scheduler delete-schedule",
        &serde_json::json!({
            "workspace": workspace,
            "itemId": item_id,
            "jobType": job_type,
            "scheduleId": schedule_id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!(
            "/workspaces/{workspace}/items/{item_id}/jobs/{job_type}/schedules/{schedule_id}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "job-scheduler delete-schedule", "Admin"))?;

    let obj = serde_json::json!({
        "scheduleId": schedule_id,
        "status": "deleted"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_job_types_are_non_empty() {
        assert!(!KNOWN_JOB_TYPES.is_empty());
        for t in KNOWN_JOB_TYPES {
            assert!(!t.is_empty());
        }
    }
}
