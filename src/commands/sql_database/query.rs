use std::io::{self, Read};

use anyhow::Result;
use mssql_tds::connection::client_context::{ClientContext, TdsAuthenticationMethod};
use mssql_tds::connection::tds_client::{ResultSet, ResultSetClient};
use mssql_tds::connection_provider::tds_connection_provider::TdsConnectionProvider;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::commands::tds_utils::column_value_to_json;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

/// Resolve SQL database connection info: returns (`server_host`, `port`, `database_name`).
pub(super) async fn resolve_sql_connection(
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

#[allow(clippy::too_many_lines)]
pub(super) async fn query(
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
            let msg = format!("{e}");
            let hint = if msg.contains("18456") {
                ". Hint: Fabric SQL Database requires F4+ capacity. On F2, TDS connections \
                 fail with 'Validation of user's permissions failed' due to insufficient \
                 compute, not actual permissions issues. Scale your capacity to F4 or higher."
            } else {
                ""
            };
            FabioError::new(
                ErrorCode::ApiError,
                format!("TDS connection failed: {e}{hint}"),
            )
        })?;

    // Execute SQL
    tds_client
        .execute(sql_text, Some(60), None)
        .await
        .map_err(|e| {
            let msg = format!("{e}");
            let hint = if msg.contains("40515") {
                ". Hint: Fabric SQL Database does not support cross-database queries via \
                 three-part naming ([Database].[schema].[table]). Use a lakehouse SQL \
                 endpoint or warehouse instead — they support cross-database queries to \
                 SQL Databases in the same workspace."
            } else if msg.contains("Invalid object name") && msg.contains("sys.") {
                ". Hint: Fabric SQL Database does not support all SQL Server system views. \
                 Supported: sys.tables, sys.columns, sys.schemas, sys.types, \
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

pub(super) async fn connection_string(
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
