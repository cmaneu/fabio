use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum SqlEndpointCommand {
    /// List SQL endpoints in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Show details of a SQL endpoint
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// SQL endpoint ID
        #[arg(long)]
        id: String,
    },
    /// Get the SQL connection string for a SQL endpoint
    #[command(display_order = 3)]
    ConnectionString {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// SQL endpoint ID
        #[arg(long)]
        id: String,

        /// Guest tenant ID (if different from the SQL endpoint's tenant)
        #[arg(long)]
        guest_tenant_id: Option<String>,

        /// Private link type: `None` or `Workspace`
        #[arg(long)]
        private_link_type: Option<String>,
    },
    /// Refresh metadata for all tables in a SQL endpoint (LRO)
    #[command(display_order = 4)]
    RefreshMetadata {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// SQL endpoint ID
        #[arg(long)]
        id: String,
    },
    /// Get SQL audit settings for the endpoint
    #[command(display_order = 10)]
    GetAuditSettings {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// SQL endpoint ID
        #[arg(long)]
        id: String,
    },
    /// Update SQL audit settings for the endpoint
    #[command(display_order = 11)]
    UpdateAuditSettings {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// SQL endpoint ID
        #[arg(long)]
        id: String,

        /// Audit state: Enabled or Disabled
        #[arg(long)]
        state: Option<String>,

        /// Retention days (0 = indefinite)
        #[arg(long)]
        retention_days: Option<i64>,

        /// Audit actions and groups (comma-separated)
        #[arg(long, value_delimiter = ',')]
        audit_actions: Option<Vec<String>>,

        /// Predicate expression for filtering audit logs
        #[arg(long)]
        predicate_expression: Option<String>,
    },
    /// Set audit actions and groups for the endpoint
    #[command(display_order = 12)]
    SetAuditActions {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// SQL endpoint ID
        #[arg(long)]
        id: String,

        /// Audit actions and groups (comma-separated)
        #[arg(long, value_delimiter = ',')]
        actions: Vec<String>,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &SqlEndpointCommand) -> Result<()> {
    match command {
        SqlEndpointCommand::List { workspace } => list(cli, client, workspace).await,
        SqlEndpointCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        SqlEndpointCommand::ConnectionString {
            workspace,
            id,
            guest_tenant_id,
            private_link_type,
        } => {
            connection_string(
                cli,
                client,
                workspace,
                id,
                guest_tenant_id.as_deref(),
                private_link_type.as_deref(),
            )
            .await
        }
        SqlEndpointCommand::RefreshMetadata { workspace, id } => {
            refresh_metadata(cli, client, workspace, id).await
        }
        SqlEndpointCommand::GetAuditSettings { workspace, id } => {
            get_audit_settings(cli, client, workspace, id).await
        }
        SqlEndpointCommand::UpdateAuditSettings {
            workspace,
            id,
            state,
            retention_days,
            audit_actions,
            predicate_expression,
        } => {
            update_audit_settings(
                cli,
                client,
                workspace,
                id,
                state.as_deref(),
                *retention_days,
                audit_actions.as_deref(),
                predicate_expression.as_deref(),
            )
            .await
        }
        SqlEndpointCommand::SetAuditActions {
            workspace,
            id,
            actions,
        } => set_audit_actions(cli, client, workspace, id, actions).await,
    }
}

// ─── Queries ─────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/sqlEndpoints"),
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
        .get(&format!("/workspaces/{workspace}/sqlEndpoints/{id}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn connection_string(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    guest_tenant_id: Option<&str>,
    private_link_type: Option<&str>,
) -> Result<()> {
    let mut url = format!("/workspaces/{workspace}/sqlEndpoints/{id}/connectionString");
    let mut params = Vec::new();
    if let Some(tenant) = guest_tenant_id {
        params.push(format!("guestTenantId={tenant}"));
    }
    if let Some(plt) = private_link_type {
        params.push(format!("privateLinkType={plt}"));
    }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    let data = client.get(&url).await?;
    output::render_object(cli, &data, "connectionString");
    Ok(())
}

// ─── Mutations ───────────────────────────────────────────────────────────────

async fn refresh_metadata(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "sql-endpoint refresh-metadata",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/sqlEndpoints/{id}/refreshMetadata"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "sql-endpoint refresh-metadata", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "metadata_refreshed" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Audit settings ──────────────────────────────────────────────────────────

async fn get_audit_settings(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/sqlEndpoints/{id}/settings/sqlAudit"
        ))
        .await?;
    output::render_object(cli, &data, "state");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn update_audit_settings(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    state: Option<&str>,
    retention_days: Option<i64>,
    audit_actions: Option<&[String]>,
    predicate_expression: Option<&str>,
) -> Result<()> {
    if state.is_none()
        && retention_days.is_none()
        && audit_actions.is_none()
        && predicate_expression.is_none()
    {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one audit setting must be provided".to_string(),
            "Options: --state Enabled|Disabled, --retention-days N, --audit-actions GROUP1,GROUP2, --predicate-expression EXPR".to_string(),
        )
        .into());
    }

    let mut body = serde_json::json!({});
    if let Some(s) = state {
        body["state"] = Value::String(s.to_string());
    }
    if let Some(days) = retention_days {
        body["retentionDays"] = Value::Number(serde_json::Number::from(days));
    }
    if let Some(actions) = audit_actions {
        body["auditActionsAndGroups"] =
            Value::Array(actions.iter().map(|a| Value::String(a.clone())).collect());
    }
    if let Some(pred) = predicate_expression {
        body["predicateExpression"] = Value::String(pred.to_string());
    }

    if output::dry_run_guard(cli, "sql-endpoint update-audit-settings", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/sqlEndpoints/{id}/settings/sqlAudit"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "sql-endpoint update-audit-settings", "Audit"))?;
    output::render_object(cli, &data, "state");
    Ok(())
}

async fn set_audit_actions(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    actions: &[String],
) -> Result<()> {
    let body = Value::Array(actions.iter().map(|a| Value::String(a.clone())).collect());

    if output::dry_run_guard(cli, "sql-endpoint set-audit-actions", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/sqlEndpoints/{id}/settings/sqlAudit/setAuditActionsAndGroups"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "sql-endpoint set-audit-actions", "Audit"))?;

    let obj = serde_json::json!({ "id": id, "status": "audit_actions_updated" });
    output::render_object(cli, &obj, "status");
    Ok(())
}
