use std::io::{self, Read};

use anyhow::Result;
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
pub enum WarehouseCommand {
    /// List warehouses in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// Show details of a warehouse
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,
    },
    /// Create a new warehouse
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update warehouse properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a warehouse
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,
    },
    /// Execute a SQL query against a warehouse or SQL endpoint
    #[command(display_order = 10)]
    Query {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse or Lakehouse item ID
        #[arg(long)]
        id: String,

        /// SQL query to execute (prefix with @ to read from file, omit to read from stdin)
        #[arg(long)]
        sql: Option<String>,
    },
    /// Get the connection string for a warehouse
    #[command(display_order = 15)]
    ConnectionString {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,
    },
    /// Get SQL pools configuration for a workspace
    #[command(display_order = 20)]
    GetSqlPoolsConfig {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// Update SQL pools configuration for a workspace
    #[command(display_order = 21)]
    UpdateSqlPoolsConfig {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Path to JSON file with configuration (prefix with @)
        #[arg(long, group = "input")]
        file: Option<String>,

        /// Inline JSON content
        #[arg(long, group = "input")]
        content: Option<String>,
    },
    /// Get SQL audit settings for a warehouse
    #[command(display_order = 25)]
    GetAuditSettings {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,
    },
    /// Update SQL audit settings for a warehouse
    #[command(display_order = 26)]
    UpdateAuditSettings {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,

        /// Audit state (e.g. Enabled, Disabled)
        #[arg(long)]
        state: Option<String>,

        /// Retention period in days
        #[arg(long)]
        retention_days: Option<u32>,

        /// Comma-separated list of audit actions
        #[arg(long)]
        audit_actions: Option<String>,
    },
    /// Set audit actions and groups for a warehouse
    #[command(display_order = 27)]
    SetAuditActions {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,

        /// Comma-separated list of audit actions and groups
        #[arg(long, value_delimiter = ',')]
        actions: Vec<String>,
    },
    /// List restore points for a warehouse
    #[command(display_order = 30)]
    ListRestorePoints {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,
    },
    /// Create a restore point for a warehouse
    #[command(display_order = 31)]
    CreateRestorePoint {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,

        /// Optional label for the restore point
        #[arg(long)]
        name: Option<String>,
    },
    /// Show details of a restore point
    #[command(display_order = 32)]
    ShowRestorePoint {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,

        /// Restore point ID
        #[arg(long)]
        restore_point_id: String,
    },
    /// Update a restore point
    #[command(display_order = 33)]
    UpdateRestorePoint {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,

        /// Restore point ID
        #[arg(long)]
        restore_point_id: String,

        /// New label for the restore point
        #[arg(long)]
        name: Option<String>,
    },
    /// Delete a restore point
    #[command(display_order = 34)]
    DeleteRestorePoint {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,

        /// Restore point ID
        #[arg(long)]
        restore_point_id: String,
    },
    /// Restore a warehouse to a restore point
    #[command(display_order = 36)]
    RestoreToPoint {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,

        /// Restore point ID
        #[arg(long)]
        restore_point_id: String,

        /// Name for the restored warehouse
        #[arg(long)]
        name: String,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &WarehouseCommand) -> Result<()> {
    match command {
        WarehouseCommand::List { workspace } => list(cli, client, workspace).await,
        WarehouseCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        WarehouseCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref()).await,
        WarehouseCommand::Update {
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
        WarehouseCommand::Delete { workspace, id } => {
            delete_warehouse(cli, client, workspace, id).await
        }
        WarehouseCommand::Query { workspace, id, sql } => {
            query(cli, client, workspace, id, sql.as_deref())
                .await
                .map_err(|e| enrich_forbidden(e, "warehouse query", "Viewer"))
        }
        WarehouseCommand::ConnectionString { workspace, id } => {
            connection_string(cli, client, workspace, id).await
        }
        WarehouseCommand::GetSqlPoolsConfig { workspace } => {
            get_sql_pools_config(cli, client, workspace).await
        }
        WarehouseCommand::UpdateSqlPoolsConfig {
            workspace,
            file,
            content,
        } => {
            update_sql_pools_config(cli, client, workspace, file.as_deref(), content.as_deref())
                .await
        }
        WarehouseCommand::GetAuditSettings { workspace, id } => {
            get_audit_settings(cli, client, workspace, id).await
        }
        WarehouseCommand::UpdateAuditSettings {
            workspace,
            id,
            state,
            retention_days,
            audit_actions,
        } => {
            update_audit_settings(
                cli,
                client,
                workspace,
                id,
                state.as_deref(),
                *retention_days,
                audit_actions.as_deref(),
            )
            .await
        }
        WarehouseCommand::SetAuditActions {
            workspace,
            id,
            actions,
        } => set_audit_actions(cli, client, workspace, id, actions).await,
        WarehouseCommand::ListRestorePoints { workspace, id } => {
            list_restore_points(cli, client, workspace, id).await
        }
        WarehouseCommand::CreateRestorePoint {
            workspace,
            id,
            name,
        } => create_restore_point(cli, client, workspace, id, name.as_deref()).await,
        WarehouseCommand::ShowRestorePoint {
            workspace,
            id,
            restore_point_id,
        } => show_restore_point(cli, client, workspace, id, restore_point_id).await,
        WarehouseCommand::UpdateRestorePoint {
            workspace,
            id,
            restore_point_id,
            name,
        } => {
            update_restore_point(
                cli,
                client,
                workspace,
                id,
                restore_point_id,
                name.as_deref(),
            )
            .await
        }
        WarehouseCommand::DeleteRestorePoint {
            workspace,
            id,
            restore_point_id,
        } => delete_restore_point(cli, client, workspace, id, restore_point_id).await,
        WarehouseCommand::RestoreToPoint {
            workspace,
            id,
            restore_point_id,
            name,
        } => restore_to_point(cli, client, workspace, id, restore_point_id, name).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/warehouses"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id"],
        &["NAME", "ID"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn show(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/warehouses/{id}"))
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
        "warehouse create",
        &serde_json::json!({
            "workspace": workspace,
            "displayName": name,
            "description": description
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(&format!("/workspaces/{workspace}/warehouses"), &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse create", "Member"))?;
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
            "Example: fabio warehouse update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "warehouse update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/warehouses/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete_warehouse(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "warehouse delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/warehouses/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

#[allow(clippy::too_many_lines)]
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

    // Get connection string from warehouse or lakehouse
    let (connection_string, item_name) = get_connection_string(client, workspace, id).await?;
    let (server, parsed_db) = parse_connection_string(&connection_string);
    // Use item displayName as database catalog; fall back to parsed value from connection string
    let database = if item_name.is_empty() {
        parsed_db
    } else {
        item_name
    };

    // Acquire AAD token for SQL scope
    let token = client.require_sql_auth().await?;

    // Build TDS connection
    let data_source = format!("tcp:{server},1433");
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
        .map_err(|e| FabioError::new(ErrorCode::ApiError, format!("TDS connection failed: {e}")))?;

    // Execute SQL
    tds_client
        .execute(sql_text, Some(60), None)
        .await
        .map_err(|e| {
            let msg = format!("{e}");
            let hint = if msg.contains("Invalid object name") && msg.contains("sys.") {
                ". Hint: Fabric Warehouse/Lakehouse SQL does not support all SQL Server \
                 system views. Supported: sys.tables, sys.columns, sys.schemas, \
                 INFORMATION_SCHEMA.TABLES, INFORMATION_SCHEMA.COLUMNS"
            } else {
                ""
            };
            FabioError::new(
                ErrorCode::ApiError,
                format!("SQL execution failed: {e}{hint}"),
            )
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
        while let Some(row) = rs
            .next_row()
            .await
            .map_err(|e| FabioError::new(ErrorCode::ApiError, format!("Failed to read row: {e}")))?
        {
            let mut obj = serde_json::Map::new();
            for (i, val) in row.into_iter().enumerate() {
                let col_name = columns
                    .get(i)
                    .map_or_else(|| format!("column{i}"), std::clone::Clone::clone);
                obj.insert(col_name, column_value_to_json(&val));
            }
            all_rows.push(Value::Object(obj));
        }
    }

    tds_client
        .close_query()
        .await
        .map_err(|e| FabioError::new(ErrorCode::ApiError, format!("Failed to close query: {e}")))?;

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

/// Get SQL connection string from warehouse or lakehouse metadata.
/// Returns (`server_hostname`, `database_name`).
async fn get_connection_string(
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<(String, String)> {
    // Try warehouse endpoint first
    if let Ok(data) = client
        .get(&format!("/workspaces/{workspace}/warehouses/{id}"))
        .await
    {
        if let Some(conn) = data
            .get("properties")
            .and_then(|p| p.get("connectionString"))
            .and_then(Value::as_str)
        {
            if !conn.is_empty() {
                let db_name = data
                    .get("displayName")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                return Ok((conn.to_string(), db_name));
            }
        }
    }

    // Fall back to lakehouse SQL endpoint
    if let Ok(data) = client
        .get(&format!("/workspaces/{workspace}/lakehouses/{id}"))
        .await
    {
        if let Some(conn) = data
            .get("properties")
            .and_then(|p| p.get("sqlEndpointProperties"))
            .and_then(|s| s.get("connectionString"))
            .and_then(Value::as_str)
        {
            if !conn.is_empty() {
                let db_name = data
                    .get("displayName")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                return Ok((conn.to_string(), db_name));
            }
        }
    }

    Err(FabioError {
        code: ErrorCode::NotFound,
        message: "Could not determine SQL connection string. Verify the item is a warehouse or lakehouse with a SQL endpoint.".into(),
        hint: Some(
            "Only Warehouse and Lakehouse items support SQL queries via this command.\n\
             For SQL Databases, use: fabio sql-database query\n\
             For lakehouses, pass the lakehouse ID (not the SQL endpoint ID).\n\
             List items: fabio item list --workspace <WS> --type Warehouse"
                .into(),
        ),
    }.into())
}

/// Parse connection string into server and database components.
/// Fabric connection strings come in formats like:
///   - `<guid>.datawarehouse.fabric.microsoft.com`  (server only)
///   - `jdbc:sqlserver://<server>;database=<db>;...`  (JDBC style)
///   - `<server>,1433;Initial Catalog=<db>;...`  (ADO.NET style)
fn parse_connection_string(connection_string: &str) -> (String, String) {
    let cleaned = connection_string
        .trim()
        .trim_start_matches("jdbc:sqlserver://")
        .trim_start_matches("jdbc:");

    // Extract server: everything before the first ';' or ','
    let server = cleaned
        .split(';')
        .next()
        .unwrap_or(cleaned)
        .split(',')
        .next()
        .unwrap_or(cleaned)
        .to_string();

    // Extract database from key-value pairs (case-insensitive)
    let database = cleaned
        .split(';')
        .find_map(|part| {
            let lower = part.trim().to_lowercase();
            if lower.starts_with("database=") || lower.starts_with("initial catalog=") {
                part.trim().split('=').nth(1).map(str::to_string)
            } else {
                None
            }
        })
        .unwrap_or_default();

    (server, database)
}

async fn connection_string(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/warehouses/{id}/connectionString"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse connection-string", "Viewer"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn get_sql_pools_config(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/warehouses/sqlPoolsConfiguration?beta=true"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse get-sql-pools-config", "Viewer"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update_sql_pools_config(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body: Value = match (file, content) {
        (Some(f), _) => {
            let text = std::fs::read_to_string(f).map_err(|e| {
                FabioError::not_found(format!("Configuration file not found: {f}: {e}"))
            })?;
            serde_json::from_str(&text)?
        }
        (_, Some(c)) => serde_json::from_str(c)?,
        _ => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio warehouse update-sql-pools-config --workspace <WS> --content '{...}'"
                    .to_string(),
            )
            .into());
        }
    };

    if output::dry_run_guard(cli, "warehouse update-sql-pools-config", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/warehouses/sqlPoolsConfiguration?beta=true"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse update-sql-pools-config", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn get_audit_settings(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/warehouses/{id}/settings/sqlAudit"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse get-audit-settings", "Viewer"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update_audit_settings(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    state: Option<&str>,
    retention_days: Option<u32>,
    audit_actions: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({});
    if let Some(s) = state {
        body["state"] = Value::String(s.to_string());
    }
    if let Some(days) = retention_days {
        body["retentionDays"] = Value::from(days);
    }
    if let Some(actions) = audit_actions {
        let list: Vec<&str> = actions.split(',').map(str::trim).collect();
        body["auditActionsAndGroups"] = serde_json::json!(list);
    }

    if output::dry_run_guard(cli, "warehouse update-audit-settings", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/warehouses/{id}/settings/sqlAudit"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse update-audit-settings", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn set_audit_actions(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    actions: &[String],
) -> Result<()> {
    let body = serde_json::json!({
        "auditActionsAndGroups": actions,
    });

    if output::dry_run_guard(cli, "warehouse set-audit-actions", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/warehouses/{id}/settings/sqlAudit/setAuditActionsAndGroups"
            ),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse set-audit-actions", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn list_restore_points(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/warehouses/{id}/restorePoints"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse list-restore-points", "Viewer"))?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["restorePointLabel", "id", "createdDateTime"],
        &["LABEL", "ID", "CREATED"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn create_restore_point(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: Option<&str>,
) -> Result<()> {
    let body = name.map_or_else(
        || serde_json::json!({}),
        |n| serde_json::json!({ "restorePointLabel": n }),
    );

    if output::dry_run_guard(cli, "warehouse create-restore-point", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/warehouses/{id}/restorePoints"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse create-restore-point", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn show_restore_point(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    restore_point_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/warehouses/{id}/restorePoints/{restore_point_id}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse show-restore-point", "Viewer"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update_restore_point(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    restore_point_id: &str,
    name: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({});
    if let Some(n) = name {
        body["restorePointLabel"] = Value::String(n.to_string());
    }

    if output::dry_run_guard(cli, "warehouse update-restore-point", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/warehouses/{id}/restorePoints/{restore_point_id}"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse update-restore-point", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete_restore_point(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    restore_point_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "warehouse delete-restore-point",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "restorePointId": restore_point_id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!(
            "/workspaces/{workspace}/warehouses/{id}/restorePoints/{restore_point_id}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse delete-restore-point", "Contributor"))?;

    let obj = serde_json::json!({ "id": restore_point_id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn restore_to_point(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    restore_point_id: &str,
    name: &str,
) -> Result<()> {
    let body = serde_json::json!({
        "restoreToWarehouseName": name,
    });

    if output::dry_run_guard(cli, "warehouse restore-to-point", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/warehouses/{id}/restorePoints/{restore_point_id}/restore"
            ),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse restore-to-point", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_hostname() {
        let (server, db) = parse_connection_string("abc123.datawarehouse.fabric.microsoft.com");
        assert_eq!(server, "abc123.datawarehouse.fabric.microsoft.com");
        assert_eq!(db, "");
    }

    #[test]
    fn parse_hostname_with_port() {
        let (server, db) =
            parse_connection_string("abc123.datawarehouse.fabric.microsoft.com,1433");
        assert_eq!(server, "abc123.datawarehouse.fabric.microsoft.com");
        assert_eq!(db, "");
    }

    #[test]
    fn parse_jdbc_with_database() {
        let (server, db) = parse_connection_string(
            "jdbc:sqlserver://myserver.fabric.microsoft.com;database=MyDB;encrypt=true",
        );
        assert_eq!(server, "myserver.fabric.microsoft.com");
        assert_eq!(db, "MyDB");
    }

    #[test]
    fn parse_adonet_initial_catalog() {
        let (server, db) = parse_connection_string(
            "myserver.database.windows.net,1433;Initial Catalog=SalesDB;Encrypt=True",
        );
        assert_eq!(server, "myserver.database.windows.net");
        assert_eq!(db, "SalesDB");
    }

    #[test]
    fn parse_trims_whitespace() {
        let (server, db) = parse_connection_string("  abc.fabric.microsoft.com  ");
        assert_eq!(server, "abc.fabric.microsoft.com");
        assert_eq!(db, "");
    }

    #[test]
    fn parse_case_insensitive_database_key() {
        let (server, db) = parse_connection_string("host.com;DATABASE=TestDb;encrypt=true");
        assert_eq!(server, "host.com");
        assert_eq!(db, "TestDb");
    }
}
