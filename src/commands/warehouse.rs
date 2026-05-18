use std::io::{self, Read};
use std::sync::Arc;

use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;
use tiberius::{AuthMethod, Client, ColumnData, Config, EncryptionLevel, Row};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::{self, ClientConfig, RootCertStore, ServerName};
use tokio_util::compat::TokioAsyncWriteCompatExt;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError};
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
        #[arg(short, long)]
        sql: Option<String>,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &WarehouseCommand) -> Result<()> {
    match command {
        WarehouseCommand::List { workspace } => list(cli, client, workspace).await,
        WarehouseCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        WarehouseCommand::Query { workspace, id, sql } => {
            query(cli, client, workspace, id, sql.as_deref()).await
        }
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/warehouses"))
        .await?;
    let items = data
        .get("value")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    output::render_list(cli, &items, &["displayName", "id"], &["NAME", "ID"], "id");
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

    // Get connection info from warehouse or lakehouse metadata
    let (server, database) = get_connection_info(client, workspace, id).await?;

    // Get a token scoped for SQL (database.windows.net)
    let token = client.require_sql_auth().await?;

    // Execute query via TDS
    let rows = execute_sql(&server, &database, &token, &sql_text).await?;

    output::render_list(
        cli,
        &rows,
        &[], // dynamic columns - render_list handles empty cols as "render all fields"
        &[],
        "",
    );
    Ok(())
}

/// Execute SQL via TDS protocol using tiberius with AAD token auth.
/// Handles Azure SQL redirect (Routing) by reconnecting to the target server
/// using TDS 8.0 (raw TLS transport + TDS inside).
async fn execute_sql(server: &str, database: &str, token: &str, sql: &str) -> Result<Vec<Value>> {
    // Try TDS 8.0 first: raw TLS to server, then TDS inside.
    // Fabric endpoints use TDS 8.0 (transport-level TLS, no TDS-level TLS wrapping).
    execute_sql_tds8(server, 1433, database, token, sql).await
}

/// Connect using TDS 8.0: raw TLS on transport, then TDS inside.
/// Fabric SQL endpoints (*.datawarehouse.fabric.microsoft.com and
/// *.pbidedicated.windows.net) require TLS at the transport level.
/// TDS runs inside TLS without additional TDS-level encryption negotiation.
///
/// Handles Azure SQL Routing: the gateway redirects to a backend server.
/// On redirect, we establish a new TDS 8.0 connection to the target.
async fn execute_sql_tds8(
    host: &str,
    port: u16,
    database: &str,
    token: &str,
    sql: &str,
) -> Result<Vec<Value>> {
    match connect_tds8(host, port, database, token).await? {
        TdsConnection::Connected(mut client) => execute_query(&mut client, sql).await,
        TdsConnection::Routing {
            host: redir_host,
            port: redir_port,
        } => {
            // Strip instance name (after \) for DNS resolution
            let target_host = redir_host.split('\\').next().unwrap_or(&redir_host);
            match connect_tds8(target_host, redir_port, database, token).await? {
                TdsConnection::Connected(mut client) => execute_query(&mut client, sql).await,
                TdsConnection::Routing { host: h, .. } => Err(FabioError::new(
                    ErrorCode::ApiError,
                    format!("Double redirect not supported (redirected to {h})"),
                )
                .into()),
            }
        }
    }
}

/// Result of a TDS 8.0 connection attempt.
enum TdsConnection {
    Connected(Box<Client<tokio_util::compat::Compat<tokio_rustls::client::TlsStream<TcpStream>>>>),
    Routing { host: String, port: u16 },
}

/// Establish a TDS 8.0 connection (raw TLS + TDS inside).
async fn connect_tds8(host: &str, port: u16, database: &str, token: &str) -> Result<TdsConnection> {
    // Build rustls config with native root certificates
    let mut root_store = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().unwrap_or_default() {
        root_store.add(&rustls::Certificate(cert.0)).ok();
    }

    let mut tls_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // TDS 8.0: advertise ALPN to indicate transport-level TLS
    tls_config.alpn_protocols = vec![b"tds/8.0".to_vec()];

    let connector = TlsConnector::from(Arc::new(tls_config));

    // Establish raw TLS
    let addr = format!("{host}:{port}");
    let tcp = TcpStream::connect(&addr).await.map_err(|e| {
        FabioError::new(
            ErrorCode::NetworkError,
            format!("TCP connect to {addr} failed: {e}"),
        )
    })?;
    tcp.set_nodelay(true).ok();

    let server_name = ServerName::try_from(host).map_err(|e| {
        FabioError::new(
            ErrorCode::ApiError,
            format!("Invalid server name for TLS: {e}"),
        )
    })?;

    let tls_stream = connector.connect(server_name, tcp).await.map_err(|e| {
        FabioError::new(
            ErrorCode::ApiError,
            format!("TLS handshake to {addr} failed: {e}"),
        )
    })?;

    // Configure tiberius for TDS 8.0: set encryption to Required so the PRELOGIN
    // tells the server we support encryption (via transport TLS). Without the rustls
    // feature, tiberius's negotiated_encryption() always returns NotSupported,
    // so no inner TLS handshake is attempted - exactly what TDS 8.0 needs.
    let mut config = Config::new();
    config.host(host);
    config.port(port);
    config.database(database);
    config.authentication(AuthMethod::aad_token(token));
    config.encryption(EncryptionLevel::Required);
    config.trust_cert();

    match Client::connect(config, tls_stream.compat_write()).await {
        Ok(client) => Ok(TdsConnection::Connected(Box::new(client))),
        Err(tiberius::error::Error::Routing { host: h, port: p }) => {
            Ok(TdsConnection::Routing { host: h, port: p })
        }
        Err(e) => Err(FabioError::new(
            ErrorCode::ApiError,
            format!("TDS connection to {addr} failed: {e}"),
        )
        .into()),
    }
}

/// Execute a SQL query on an established TDS connection.
async fn execute_query(
    client: &mut Client<tokio_util::compat::Compat<tokio_rustls::client::TlsStream<TcpStream>>>,
    sql: &str,
) -> Result<Vec<Value>> {
    let stream = client
        .simple_query(sql)
        .await
        .map_err(|e| FabioError::new(ErrorCode::ApiError, format!("SQL execution failed: {e}")))?;
    let result_set = stream.into_first_result().await.map_err(|e| {
        FabioError::new(
            ErrorCode::ApiError,
            format!("Failed to read query results: {e}"),
        )
    })?;
    Ok(rows_to_json(&result_set))
}

/// Convert TDS result rows to JSON values.
fn rows_to_json(rows: &[Row]) -> Vec<Value> {
    rows.iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            for (col, data) in row.cells() {
                obj.insert(col.name().to_string(), column_to_json(data));
            }
            Value::Object(obj)
        })
        .collect()
}

/// Convert a TDS column value to a JSON value.
fn column_to_json(data: &ColumnData<'_>) -> Value {
    match data {
        ColumnData::U8(v) => v.map_or(Value::Null, Value::from),
        ColumnData::I16(v) => v.map_or(Value::Null, Value::from),
        ColumnData::I32(v) => v.map_or(Value::Null, Value::from),
        ColumnData::I64(v) => v.map_or(Value::Null, Value::from),
        ColumnData::F32(v) => v.map_or(Value::Null, |n| {
            serde_json::Number::from_f64(f64::from(n)).map_or(Value::Null, Value::Number)
        }),
        ColumnData::F64(v) => v.map_or(Value::Null, |n| {
            serde_json::Number::from_f64(n).map_or(Value::Null, Value::Number)
        }),
        ColumnData::Bit(v) => v.map_or(Value::Null, Value::from),
        ColumnData::String(v) => v.as_ref().map_or(Value::Null, |s| Value::from(s.as_ref())),
        ColumnData::Guid(v) => v.map_or(Value::Null, |g| Value::from(g.to_string())),
        ColumnData::Numeric(v) => v.map_or(Value::Null, |n| Value::from(n.to_string())),
        ColumnData::DateTime(v) => v.map_or(Value::Null, |d| Value::from(format!("{d:?}"))),
        ColumnData::SmallDateTime(v) => v.map_or(Value::Null, |d| Value::from(format!("{d:?}"))),
        ColumnData::DateTime2(v) => v.map_or(Value::Null, |d| Value::from(format!("{d:?}"))),
        ColumnData::DateTimeOffset(v) => v.map_or(Value::Null, |d| Value::from(format!("{d:?}"))),
        ColumnData::Date(v) => v.map_or(Value::Null, |d| Value::from(format!("{d:?}"))),
        ColumnData::Time(v) => v.map_or(Value::Null, |t| Value::from(format!("{t:?}"))),
        ColumnData::Binary(v) => v.as_ref().map_or(Value::Null, |b| {
            use std::fmt::Write;
            let mut hex = String::with_capacity(b.len() * 2);
            for byte in b.iter() {
                write!(hex, "{byte:02x}").unwrap();
            }
            Value::from(hex)
        }),
        ColumnData::Xml(v) => v
            .as_ref()
            .map_or(Value::Null, |x| Value::from(x.as_ref().to_string())),
    }
}

/// Get SQL server and database name from warehouse or lakehouse metadata.
async fn get_connection_info(
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
                let server = parse_server(conn);
                let db = data
                    .get("displayName")
                    .and_then(Value::as_str)
                    .unwrap_or("master")
                    .to_string();
                return Ok((server, db));
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
                let server = parse_server(conn);
                let db = data
                    .get("displayName")
                    .and_then(Value::as_str)
                    .unwrap_or("master")
                    .to_string();
                return Ok((server, db));
            }
        }
    }

    Err(FabioError::new(
        ErrorCode::NotFound,
        "Could not determine SQL endpoint. Verify the item is a warehouse or lakehouse with a SQL endpoint.",
    ).into())
}

/// Parse server hostname from a connection string or raw hostname.
fn parse_server(connection_string: &str) -> String {
    connection_string
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
        .to_string()
}
