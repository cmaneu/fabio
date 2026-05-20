use std::io::{self, Read};

use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{enrich_forbidden, ErrorCode, FabioError};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum WarehouseCommand {
    /// List warehouses in a workspace
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a warehouse
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Warehouse item ID
        #[arg(long)]
        id: String,
    },
    /// Execute a SQL query against a warehouse or SQL endpoint
    Query {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Warehouse or Lakehouse item ID
        #[arg(long)]
        id: String,

        /// SQL query to execute (prefix with @ to read from file, omit to read from stdin)
        #[arg(long)]
        sql: Option<String>,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &WarehouseCommand) -> Result<()> {
    match command {
        WarehouseCommand::List { workspace } => list(cli, client, workspace).await,
        WarehouseCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        WarehouseCommand::Query { workspace, id, sql } => {
            query(cli, client, workspace, id, sql.as_deref())
                .await
                .map_err(|e| enrich_forbidden(e, "warehouse query", "Viewer"))
        }
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/warehouses"),
            "value",
            cli.all,
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
    let connection_string = get_connection_string(client, workspace, id).await?;

    // For now, output a message about ODBC requirement
    // Full ODBC implementation would use odbc-api crate
    let _conn_info = parse_connection_string(&connection_string);

    // TODO: Implement ODBC connection with odbc-api crate
    // For now, return the query info as structured output
    let obj = serde_json::json!({
        "sql": sql_text,
        "endpoint": connection_string,
        "status": "not_implemented",
        "message": "SQL execution via ODBC not yet implemented. Use 'az sql query' or sqlcmd."
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

/// Get SQL connection string from warehouse or lakehouse metadata.
async fn get_connection_string(client: &FabricClient, workspace: &str, id: &str) -> Result<String> {
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
                return Ok(conn.to_string());
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
                return Ok(conn.to_string());
            }
        }
    }

    Err(FabioError::new(
        ErrorCode::NotFound,
        "Could not determine SQL connection string. Verify the item is a warehouse or lakehouse with a SQL endpoint.",
    ).into())
}

/// Parse connection string into server and database components.
fn parse_connection_string(connection_string: &str) -> (String, String) {
    let server = connection_string
        .trim()
        .trim_start_matches("jdbc:")
        .split("//")
        .last()
        .unwrap_or(connection_string)
        .split(';')
        .next()
        .unwrap_or(connection_string)
        .split(',')
        .next()
        .unwrap_or(connection_string)
        .to_string();

    // Database name would come from item metadata
    (server, String::new())
}
