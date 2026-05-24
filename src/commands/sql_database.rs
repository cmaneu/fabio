use std::io::{self, Read};

use anyhow::Result;
use base64::Engine;
use clap::Subcommand;
use mssql_tds::connection::client_context::{ClientContext, TdsAuthenticationMethod};
use mssql_tds::connection::tds_client::{ResultSet, ResultSetClient};
use mssql_tds::connection_provider::tds_connection_provider::TdsConnectionProvider;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::commands::tds_utils::column_value_to_json;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum SqlDatabaseCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List SQL databases in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a SQL database
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database ID
        #[arg(long)]
        id: String,
    },
    /// Create a new SQL database
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Creation mode: `New`, `Restore`, or `RestoreDeletedDatabase`
        #[arg(long, default_value = "New")]
        creation_mode: Option<String>,

        /// Backup retention period in days (1-35, for mode=New)
        #[arg(long)]
        backup_retention_days: Option<i32>,

        /// Database collation (for mode=New)
        #[arg(long)]
        collation: Option<String>,

        /// Source database workspace ID (for mode=Restore)
        #[arg(long)]
        source_workspace: Option<String>,

        /// Source database item ID (for mode=Restore)
        #[arg(long)]
        source_database: Option<String>,

        /// Point-in-time to restore (ISO 8601, for mode=Restore or `RestoreDeletedDatabase`)
        #[arg(long)]
        restore_point: Option<String>,

        /// Name of the restorable deleted database (for mode=RestoreDeletedDatabase)
        #[arg(long)]
        restorable_deleted_database_name: Option<String>,
    },
    /// Update SQL database properties
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database ID
        #[arg(long)]
        id: String,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a SQL database
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database ID
        #[arg(long)]
        id: String,

        /// Permanently delete (cannot be recovered)
        #[arg(long)]
        hard_delete: bool,
    },

    // ── Definitions ──────────────────────────────────────────────────────
    /// Get the definition of a SQL database (dacpac or sqlproj format)
    #[command(display_order = 10)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database ID
        #[arg(long)]
        id: String,

        /// Definition format: dacpac or sqlproj (default: dacpac)
        #[arg(long)]
        format: Option<String>,
    },
    /// Update the definition of a SQL database
    #[command(display_order = 11)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database ID
        #[arg(long)]
        id: String,

        /// Definition file path (.dacpac or .sqlproj)
        #[arg(long)]
        file: Option<String>,

        /// Definition as inline base64 content
        #[arg(long)]
        content: Option<String>,

        /// Definition format: dacpac or sqlproj (default: dacpac)
        #[arg(long)]
        format: Option<String>,

        /// Also update item metadata from the definition
        #[arg(long)]
        update_metadata: bool,
    },

    // ── Mirroring ────────────────────────────────────────────────────────
    /// Start mirroring for the SQL database
    #[command(display_order = 20)]
    StartMirroring {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database ID
        #[arg(long)]
        id: String,
    },
    /// Stop mirroring for the SQL database
    #[command(display_order = 21)]
    StopMirroring {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database ID
        #[arg(long)]
        id: String,
    },

    // ── CMK ──────────────────────────────────────────────────────────────
    /// Revalidate Customer-Managed Key (CMK) for the SQL database
    #[command(display_order = 30)]
    RevalidateCmk {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database ID
        #[arg(long)]
        id: String,
    },

    // ── Audit settings ───────────────────────────────────────────────────
    /// Get SQL audit settings for the database
    #[command(display_order = 40)]
    GetAuditSettings {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database ID
        #[arg(long)]
        id: String,
    },
    /// Update SQL audit settings for the database
    #[command(display_order = 41)]
    UpdateAuditSettings {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database ID
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

    // ── Restorable deleted databases ─────────────────────────────────────
    /// List restorable deleted SQL databases in a workspace
    #[command(display_order = 50)]
    ListDeleted {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },

    // ── Query & connectivity ─────────────────────────────────────────────
    /// Execute a SQL query against a SQL database via TDS
    #[command(display_order = 60)]
    Query {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database item ID
        #[arg(long)]
        id: String,

        /// SQL query to execute (prefix with @ to read from file, omit to read from stdin)
        #[arg(long)]
        sql: Option<String>,
    },
    /// Show the TDS connection string for a SQL database
    #[command(display_order = 61)]
    ConnectionString {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// SQL database item ID
        #[arg(long)]
        id: String,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &SqlDatabaseCommand) -> Result<()> {
    match command {
        SqlDatabaseCommand::List { workspace } => list(cli, client, workspace).await,
        SqlDatabaseCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        SqlDatabaseCommand::Create {
            workspace,
            name,
            description,
            creation_mode,
            backup_retention_days,
            collation,
            source_workspace,
            source_database,
            restore_point,
            restorable_deleted_database_name,
        } => {
            create(
                cli,
                client,
                workspace,
                name,
                description.as_deref(),
                creation_mode.as_deref(),
                *backup_retention_days,
                collation.as_deref(),
                source_workspace.as_deref(),
                source_database.as_deref(),
                restore_point.as_deref(),
                restorable_deleted_database_name.as_deref(),
            )
            .await
        }
        SqlDatabaseCommand::Update {
            workspace,
            id,
            description,
        } => update(cli, client, workspace, id, description.as_deref()).await,
        SqlDatabaseCommand::Delete {
            workspace,
            id,
            hard_delete,
        } => delete(cli, client, workspace, id, *hard_delete).await,
        SqlDatabaseCommand::GetDefinition {
            workspace,
            id,
            format,
        } => get_definition(cli, client, workspace, id, format.as_deref()).await,
        SqlDatabaseCommand::UpdateDefinition {
            workspace,
            id,
            file,
            content,
            format,
            update_metadata,
        } => {
            update_definition(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
                format.as_deref(),
                *update_metadata,
            )
            .await
        }
        SqlDatabaseCommand::StartMirroring { workspace, id } => {
            start_mirroring(cli, client, workspace, id).await
        }
        SqlDatabaseCommand::StopMirroring { workspace, id } => {
            stop_mirroring(cli, client, workspace, id).await
        }
        SqlDatabaseCommand::RevalidateCmk { workspace, id } => {
            revalidate_cmk(cli, client, workspace, id).await
        }
        SqlDatabaseCommand::GetAuditSettings { workspace, id } => {
            get_audit_settings(cli, client, workspace, id).await
        }
        SqlDatabaseCommand::UpdateAuditSettings {
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
        SqlDatabaseCommand::ListDeleted { workspace } => list_deleted(cli, client, workspace).await,
        SqlDatabaseCommand::Query { workspace, id, sql } => {
            query(cli, client, workspace, id, sql.as_deref()).await
        }
        SqlDatabaseCommand::ConnectionString { workspace, id } => {
            connection_string(cli, client, workspace, id).await
        }
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/sqlDatabases"),
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
        .get(&format!("/workspaces/{workspace}/sqlDatabases/{id}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    description: Option<&str>,
    creation_mode: Option<&str>,
    backup_retention_days: Option<i32>,
    collation: Option<&str>,
    source_workspace: Option<&str>,
    source_database: Option<&str>,
    restore_point: Option<&str>,
    restorable_deleted_database_name: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    // Build creationPayload based on mode
    let mode = creation_mode.unwrap_or("New");
    match mode {
        "Restore" => {
            let src_ws = source_workspace.ok_or_else(|| {
                FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "--source-workspace is required for Restore mode".to_string(),
                    "Example: fabio sql-database create --workspace <WS> --name <NAME> --creation-mode Restore --source-workspace <SRC_WS> --source-database <SRC_ID> --restore-point 2024-01-01T00:00:00Z".to_string(),
                )
            })?;
            let src_db = source_database.ok_or_else(|| {
                FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "--source-database is required for Restore mode".to_string(),
                    "Provide the item ID of the source database to restore from".to_string(),
                )
            })?;
            let rp = restore_point.ok_or_else(|| {
                FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "--restore-point is required for Restore mode".to_string(),
                    "Provide an ISO 8601 timestamp (e.g., 2024-01-01T00:00:00Z)".to_string(),
                )
            })?;
            body["creationPayload"] = serde_json::json!({
                "creationMode": "Restore",
                "restorePointInTime": rp,
                "sourceDatabaseReference": {
                    "workspaceId": src_ws,
                    "id": src_db
                }
            });
        }
        "RestoreDeletedDatabase" => {
            let deleted_name = restorable_deleted_database_name.ok_or_else(|| {
                FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "--restorable-deleted-database-name is required for RestoreDeletedDatabase mode".to_string(),
                    "Use 'fabio sql-database list-deleted' to find available names".to_string(),
                )
            })?;
            let rp = restore_point.ok_or_else(|| {
                FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "--restore-point is required for RestoreDeletedDatabase mode".to_string(),
                    "Provide an ISO 8601 timestamp (e.g., 2024-01-01T00:00:00Z)".to_string(),
                )
            })?;
            body["creationPayload"] = serde_json::json!({
                "creationMode": "RestoreDeletedDatabase",
                "restorePointInTime": rp,
                "restorableDeletedDatabaseName": deleted_name
            });
        }
        _ => {
            // "New" mode or default
            let mut payload = serde_json::json!({ "creationMode": "New" });
            if let Some(days) = backup_retention_days {
                payload["backupRetentionDays"] = Value::Number(serde_json::Number::from(days));
            }
            if let Some(c) = collation {
                payload["collation"] = Value::String(c.to_string());
            }
            // Only include creationPayload if there are extra settings
            if backup_retention_days.is_some() || collation.is_some() {
                body["creationPayload"] = payload;
            }
        }
    }

    if output::dry_run_guard(cli, "sql-database create", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/sqlDatabases"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database create", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    description: Option<&str>,
) -> Result<()> {
    if description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least --description must be provided".to_string(),
            "Example: fabio sql-database update --workspace <WS> --id <ID> --description \"New desc\""
                .to_string(),
        )
        .into());
    }

    let mut body = serde_json::json!({});
    if let Some(d) = description {
        body["description"] = Value::String(d.to_string());
    }

    if output::dry_run_guard(cli, "sql-database update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/sqlDatabases/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database update", "Contributor"))?;
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
        "sql-database delete",
        &serde_json::json!({ "workspace": workspace, "id": id, "hardDelete": hard_delete }),
    ) {
        return Ok(());
    }

    let url = if hard_delete {
        format!("/workspaces/{workspace}/sqlDatabases/{id}?hardDelete=true")
    } else {
        format!("/workspaces/{workspace}/sqlDatabases/{id}")
    };

    client
        .delete(&url)
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database delete", "Member"))?;

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
    format: Option<&str>,
) -> Result<()> {
    let url = format.map_or_else(
        || format!("/workspaces/{workspace}/sqlDatabases/{id}/getDefinition"),
        |f| format!("/workspaces/{workspace}/sqlDatabases/{id}/getDefinition?format={f}"),
    );

    let data = client
        .post(&url, &serde_json::json!({}), true)
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database get-definition", "Contributor"))?;
    output::render_object(cli, &data, "definition");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
    format: Option<&str>,
    update_metadata: bool,
) -> Result<()> {
    let payload_bytes = match (file, content) {
        (Some(path), _) => {
            std::fs::read(path).map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?
        }
        (_, Some(c)) => c.as_bytes().to_vec(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio sql-database update-definition --workspace <WS> --id <ID> --file schema.dacpac".to_string(),
            ).into());
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(&payload_bytes);
    let fmt = format.unwrap_or("dacpac");
    let extension = match fmt {
        "sqlproj" => "sqlproj",
        _ => "dacpac",
    };

    let body = serde_json::json!({
        "definition": {
            "format": fmt,
            "parts": [
                {
                    "path": format!("definition.{extension}"),
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    if output::dry_run_guard(
        cli,
        "sql-database update-definition",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "format": fmt,
            "contentLength": payload_bytes.len()
        }),
    ) {
        return Ok(());
    }

    let mut url = format!("/workspaces/{workspace}/sqlDatabases/{id}/updateDefinition");
    if update_metadata {
        url.push_str("?updateMetadata=true");
    }

    let data = client
        .post(&url, &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Mirroring ───────────────────────────────────────────────────────────────

async fn start_mirroring(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "sql-database start-mirroring",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/sqlDatabases/{id}/startMirroring"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database start-mirroring", "Contributor"))?;

    let obj = serde_json::json!({ "id": id, "status": "mirroring_started" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn stop_mirroring(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "sql-database stop-mirroring",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/sqlDatabases/{id}/stopMirroring"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database stop-mirroring", "Contributor"))?;

    let obj = serde_json::json!({ "id": id, "status": "mirroring_stopped" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── CMK ─────────────────────────────────────────────────────────────────────

async fn revalidate_cmk(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "sql-database revalidate-cmk",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/sqlDatabases/{id}/revalidateCMK"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database revalidate-cmk", "Contributor"))?;

    let obj = serde_json::json!({ "id": id, "status": "cmk_revalidated" });
    output::render_object(cli, &obj, "status");
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
            "/workspaces/{workspace}/sqlDatabases/{id}/settings/sqlAudit"
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

    if output::dry_run_guard(cli, "sql-database update-audit-settings", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/sqlDatabases/{id}/settings/sqlAudit"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database update-audit-settings", "Contributor"))?;
    output::render_object(cli, &data, "state");
    Ok(())
}

// ─── Restorable deleted databases ────────────────────────────────────────────

async fn list_deleted(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/sqlDatabases/restorableDeletedDatabases"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &[
            "displayName",
            "properties.restorableDeletedDatabaseName",
            "properties.deletionTimestamp",
        ],
        &["NAME", "RESTORABLE_NAME", "DELETED_AT"],
        "displayName",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

// ─── Query & connectivity ────────────────────────────────────────────────────

/// Resolve SQL database connection info: returns (`server_host`, `port`, `database_name`).
async fn resolve_sql_connection(
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<(String, u16, String)> {
    let data = client
        .get(&format!("/workspaces/{workspace}/sqlDatabases/{id}"))
        .await?;

    let raw_server = data
        .get("properties")
        .and_then(|p| p.get("serverFqdn"))
        .and_then(Value::as_str)
        .or_else(|| {
            data.get("properties")
                .and_then(|p| p.get("connectionString"))
                .and_then(Value::as_str)
        })
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::NotFound,
                "Could not determine SQL server for this database. Verify it is provisioned.",
            )
        })?;

    // serverFqdn may include port (e.g., "host.database.fabric.microsoft.com,1433")
    let (host, port) = if let Some((h, p)) = raw_server.rsplit_once(',') {
        (h.to_string(), p.parse::<u16>().unwrap_or(1433))
    } else {
        (raw_server.to_string(), 1433)
    };

    let database = data
        .get("properties")
        .and_then(|p| p.get("databaseName"))
        .and_then(Value::as_str)
        .or_else(|| data.get("displayName").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();

    Ok((host, port, database))
}

async fn query(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    sql: Option<&str>,
) -> Result<()> {
    // Resolve SQL text: --sql flag, @file prefix, or stdin
    let sql_text = match sql {
        Some(s) if s.starts_with('@') => {
            let file_path = &s[1..];
            std::fs::read_to_string(file_path).map_err(|e| {
                FabioError::not_found(format!("SQL file not found: {file_path}: {e}"))
            })?
        }
        Some(s) => s.to_string(),
        None => {
            // Read from stdin
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).map_err(|e| {
                FabioError::new(
                    ErrorCode::ApiError,
                    format!("Failed to read SQL from stdin: {e}"),
                )
            })?;
            if buf.trim().is_empty() {
                return Err(FabioError::new(
                    ErrorCode::ApiError,
                    "No SQL provided. Use --sql, @file, or pipe SQL via stdin.",
                )
                .into());
            }
            buf
        }
    };

    // Resolve connection details
    let (server, port, database) = resolve_sql_connection(client, workspace, id).await?;

    // Acquire AAD token for SQL scope
    let token = client.require_sql_auth().await?;

    // Build TDS connection
    let data_source = format!("tcp:{server},{port}");
    let mut context = ClientContext::with_data_source(&data_source);
    context.database = database;
    context.tds_authentication_method = TdsAuthenticationMethod::AccessToken;
    context.access_token = Some(token);
    context.application_name = "fabio".to_string();
    context.connect_timeout = 30;

    let provider = TdsConnectionProvider {};
    let mut tds_client = provider
        .create_client(context, &data_source, None)
        .await
        .map_err(|e| {
            FabioError::new(
                ErrorCode::ApiError,
                format!("TDS connection failed: {e}"),
            )
        })?;

    // Execute SQL
    tds_client
        .execute(sql_text, Some(60), None)
        .await
        .map_err(|e| {
            FabioError::new(ErrorCode::ApiError, format!("SQL execution failed: {e}"))
        })?;

    // Collect results
    let mut all_rows: Vec<Value> = Vec::new();
    let mut columns: Vec<String> = Vec::new();

    if let Some(rs) = tds_client.get_current_resultset() {
        // Get column names from metadata
        columns = rs
            .get_metadata()
            .iter()
            .map(|col| col.column_name.clone())
            .collect();

        // Read all rows
        while let Some(row) = rs.next_row().await.map_err(|e| {
            FabioError::new(ErrorCode::ApiError, format!("Failed to read row: {e}"))
        })? {
            let mut obj = serde_json::Map::new();
            for (i, val) in row.into_iter().enumerate() {
                let col_name = columns.get(i).map_or_else(
                    || format!("column{i}"),
                    std::clone::Clone::clone,
                );
                obj.insert(col_name, column_value_to_json(&val));
            }
            all_rows.push(Value::Object(obj));
        }
    }

    tds_client.close_query().await.map_err(|e| {
        FabioError::new(ErrorCode::ApiError, format!("Failed to close query: {e}"))
    })?;

    // Render output
    if all_rows.is_empty() {
        let obj = serde_json::json!({
            "rows_affected": 0,
            "message": "Query executed successfully (no result set returned)."
        });
        output::render_object(cli, &obj, "message");
    } else {
        let col_refs: Vec<&str> = columns.iter().map(String::as_str).collect();
        output::render_list(cli, &all_rows, &col_refs, &col_refs, &columns[0]);
    }

    Ok(())
}

async fn connection_string(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let (server, port, database) = resolve_sql_connection(client, workspace, id).await?;
    let conn_str = format!(
        "Server=tcp:{server},{port};Initial Catalog={database};Encrypt=True;TrustServerCertificate=False;Authentication=ActiveDirectoryDefault"
    );
    let obj = serde_json::json!({
        "server": server,
        "port": port,
        "database": database,
        "connectionString": conn_str
    });
    output::render_object(cli, &obj, "connectionString");
    Ok(())
}
