use std::io::{self, Read};

use anyhow::Result;
use base64::Engine;
use clap::Subcommand;
use reqwest::header::AUTHORIZATION;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::{self, FabricClient};
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum KqlDatabaseCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List KQL databases in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a KQL database
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,
    },
    /// Create a new KQL database
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Database display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Parent eventhouse item ID
        #[arg(long)]
        eventhouse_id: String,

        /// Database type: `ReadWrite` or `ReadOnlyFollowing`
        #[arg(long, default_value = "ReadWrite", value_parser = ["ReadWrite", "ReadOnlyFollowing"])]
        database_type: String,
    },
    /// Update KQL database properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a KQL database
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,
    },

    /// Execute a KQL query against a KQL database
    #[command(display_order = 6)]
    Query {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// KQL query text (use @file.kql to read from file, or pipe via stdin)
        #[arg(long)]
        kql: Option<String>,

        /// Override the Kusto query URI (auto-discovered from database properties if omitted)
        #[arg(long)]
        query_uri: Option<String>,
    },

    // ── Definitions ──────────────────────────────────────────────────────
    /// Get the definition of a KQL database (KQL script)
    #[command(name = "get-definition", display_order = 7)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition of a KQL database
    #[command(name = "update-definition", display_order = 11)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// KQL script file path (reads file content)
        #[arg(long)]
        file: Option<String>,

        /// KQL script content (inline)
        #[arg(long)]
        content: Option<String>,
    },

    // ── Shortcuts ────────────────────────────────────────────────────────
    /// List shortcuts in a KQL database
    #[command(name = "list-shortcuts", display_order = 10)]
    ListShortcuts {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,
    },
    /// Create a shortcut in a KQL database
    #[command(name = "create-shortcut", display_order = 11)]
    CreateShortcut {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// Shortcut name
        #[arg(long)]
        name: String,

        /// JSON file with shortcut configuration
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON shortcut configuration
        #[arg(long)]
        content: Option<String>,
    },
    /// Get a shortcut in a KQL database
    #[command(name = "get-shortcut", display_order = 12)]
    GetShortcut {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// Shortcut name
        #[arg(long)]
        shortcut_name: String,
    },
    /// Delete a shortcut in a KQL database
    #[command(name = "delete-shortcut", display_order = 13)]
    DeleteShortcut {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// Shortcut name
        #[arg(long)]
        shortcut_name: String,
    },
    /// Bulk-create multiple shortcuts (LRO)
    #[command(name = "bulk-create-shortcuts", display_order = 14)]
    BulkCreateShortcuts {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// Path to JSON file with array of shortcut requests
        #[arg(long, group = "input")]
        file: Option<String>,

        /// Inline JSON with array of shortcut requests
        #[arg(long, group = "input")]
        content: Option<String>,

        /// Conflict policy: `Abort`, `GenerateUniqueName`, `CreateOrOverwrite`, `OverwriteOnly`
        #[arg(long = "conflict-policy")]
        conflict_policy: Option<String>,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &KqlDatabaseCommand) -> Result<()> {
    match command {
        KqlDatabaseCommand::List { workspace } => list(cli, client, workspace).await,
        KqlDatabaseCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        KqlDatabaseCommand::Create {
            workspace,
            name,
            description,
            eventhouse_id,
            database_type,
        } => {
            create(
                cli,
                client,
                workspace,
                name,
                description.as_deref(),
                eventhouse_id,
                database_type,
            )
            .await
        }
        KqlDatabaseCommand::Update {
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
        KqlDatabaseCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        KqlDatabaseCommand::Query {
            workspace,
            id,
            kql,
            query_uri,
        } => {
            query(
                cli,
                client,
                workspace,
                id,
                kql.as_deref(),
                query_uri.as_deref(),
            )
            .await
        }
        KqlDatabaseCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        KqlDatabaseCommand::UpdateDefinition {
            workspace,
            id,
            file,
            content,
        } => {
            update_definition(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        KqlDatabaseCommand::ListShortcuts { workspace, id } => {
            list_shortcuts(cli, client, workspace, id).await
        }
        KqlDatabaseCommand::CreateShortcut {
            workspace,
            id,
            name,
            file,
            content,
        } => {
            create_shortcut(
                cli,
                client,
                workspace,
                id,
                name,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        KqlDatabaseCommand::GetShortcut {
            workspace,
            id,
            shortcut_name,
        } => get_shortcut(cli, client, workspace, id, shortcut_name).await,
        KqlDatabaseCommand::DeleteShortcut {
            workspace,
            id,
            shortcut_name,
        } => delete_shortcut(cli, client, workspace, id, shortcut_name).await,
        KqlDatabaseCommand::BulkCreateShortcuts {
            workspace,
            id,
            file,
            content,
            conflict_policy,
        } => {
            bulk_create_shortcuts(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
                conflict_policy.as_deref(),
            )
            .await
        }
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/kqlDatabases"),
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
        .get(&format!("/workspaces/{workspace}/kqlDatabases/{id}"))
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
    eventhouse_id: &str,
    database_type: &str,
) -> Result<()> {
    let mut body = serde_json::json!({
        "displayName": name,
        "creationPayload": {
            "databaseType": database_type,
            "parentEventhouseItemId": eventhouse_id
        }
    });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if output::dry_run_guard(cli, "kql-database create", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/kqlDatabases"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database create", "Member"))?;
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
            "Example: fabio kql-database update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "kql-database update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/kqlDatabases/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "kql-database delete",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/kqlDatabases/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Query ───────────────────────────────────────────────────────────────────

async fn query(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    kql: Option<&str>,
    query_uri_override: Option<&str>,
) -> Result<()> {
    // Resolve KQL text: --kql flag, @file prefix, or stdin
    let kql_text = match kql {
        Some(s) if s.starts_with('@') => {
            let file_path = &s[1..];
            std::fs::read_to_string(file_path).map_err(|e| {
                FabioError::not_found(format!("KQL file not found: {file_path}: {e}"))
            })?
        }
        Some(s) => s.to_string(),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).map_err(|e| {
                FabioError::new(
                    ErrorCode::ApiError,
                    format!("Failed to read KQL from stdin: {e}"),
                )
            })?;
            if buf.trim().is_empty() {
                return Err(FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "No KQL provided. Use --kql, @file, or pipe KQL via stdin.".to_string(),
                    "Example: fabio kql-database query --workspace <WS> --id <ID> --kql \"MyTable | take 10\"".to_string(),
                )
                .into());
            }
            buf
        }
    };

    // Resolve Query URI and database name
    let (kusto_uri, db_name) = resolve_query_uri(client, workspace, id, query_uri_override).await?;

    // Acquire token scoped to the Kusto query URI
    let scope = format!("{kusto_uri}/.default");
    let token = client.require_token_for_scope(&scope).await?;

    // Management commands (starting with '.') use /v1/rest/mgmt; queries use /v2/rest/query
    let is_mgmt = kql_text.trim_start().starts_with('.');
    let url = if is_mgmt {
        format!("{kusto_uri}/v1/rest/mgmt")
    } else {
        format!("{kusto_uri}/v2/rest/query")
    };
    let body = serde_json::json!({
        "db": db_name,
        "csl": kql_text,
    });

    let resp = client
        .http()
        .post(&url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            FabioError::new(
                ErrorCode::NetworkError,
                format!("Kusto request failed: {e}"),
            )
        })?;

    let status = resp.status();
    let resp_text = resp.text().await.map_err(|e| {
        FabioError::new(
            ErrorCode::ApiError,
            format!("Failed to read Kusto response: {e}"),
        )
    })?;

    if !status.is_success() {
        return Err(FabioError::with_hint(
            ErrorCode::ApiError,
            format!("Kusto query failed (HTTP {status}): {resp_text}"),
            "Verify the KQL database is accessible and the query syntax is valid.".to_string(),
        )
        .into());
    }

    // Parse response: v1 (mgmt) returns {"Tables":[...]}, v2 (query) returns array of frames
    let parsed: Value = serde_json::from_str(&resp_text).map_err(|e| {
        FabioError::new(
            ErrorCode::ApiError,
            format!("Failed to parse Kusto response: {e}"),
        )
    })?;

    let (rows, columns) = if is_mgmt {
        parse_kusto_v1_response(&parsed)?
    } else {
        parse_kusto_v2_response(&parsed)?
    };

    // Render output
    if rows.is_empty() {
        let obj = serde_json::json!({
            "rows_returned": 0,
            "message": "Query executed successfully (no results returned)."
        });
        output::render_object(cli, &obj, "message");
    } else {
        let col_refs: Vec<&str> = columns.iter().map(String::as_str).collect();
        output::render_list(cli, &rows, &col_refs, &col_refs, &columns[0]);
    }

    Ok(())
}

/// Resolve the Kusto query URI and database name for a KQL database.
/// Tries the item properties first; falls back to user-provided override.
async fn resolve_query_uri(
    client: &FabricClient,
    workspace: &str,
    id: &str,
    override_uri: Option<&str>,
) -> Result<(String, String)> {
    // Get the KQL database metadata
    let data = client
        .get(&format!("/workspaces/{workspace}/kqlDatabases/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database query", "Viewer"))?;

    let db_name = data
        .get("displayName")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    // If user provided a query URI override, validate and use it
    if let Some(uri) = override_uri {
        client::validate_trusted_url(uri, "--query-uri")?;
        let uri = uri.trim_end_matches('/').to_string();
        return Ok((uri, db_name));
    }

    // Try to extract query URI from properties
    let properties = data.get("properties");

    // Try known property paths
    let query_uri = properties
        .and_then(|p| p.get("queryServiceUri"))
        .and_then(Value::as_str)
        .or_else(|| {
            properties
                .and_then(|p| p.get("queryUri"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            properties
                .and_then(|p| p.get("databaseUrl"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            // Try parentEventhouseItemId-based URI construction
            properties
                .and_then(|p| p.get("parentEventhouseItemId"))
                .and_then(Value::as_str)
                .map(|_| {
                    // Cannot construct URI without region; fall through to error
                    ""
                })
                .filter(|s| !s.is_empty())
        });

    if let Some(uri) = query_uri {
        let uri = uri.trim_end_matches('/').to_string();
        if !uri.is_empty() {
            // Validate URI from API properties against trusted domains
            client::validate_trusted_url(&uri, "queryServiceUri (from database properties)")?;
            return Ok((uri, db_name));
        }
    }

    Err(FabioError::with_hint(
        ErrorCode::NotFound,
        "Could not determine Kusto query URI from database properties.".to_string(),
        "Provide the query URI manually with --query-uri. Find it in Fabric portal: \
         KQL Database → Database details → Query URI. \
         Example: fabio kql-database query --workspace <WS> --id <ID> --query-uri https://<id>.<region>.kusto.fabric.microsoft.com --kql \"T | take 10\""
            .to_string(),
    )
    .into())
}

/// Parse Kusto v1 response format (used by management commands via `/v1/rest/mgmt`).
///
/// The v1 format is: `{"Tables": [{"TableName": "...", "Columns": [...], "Rows": [[...], ...]}]}`
/// We take the first table as the primary result.
fn parse_kusto_v1_response(resp: &Value) -> Result<(Vec<Value>, Vec<String>)> {
    let tables = resp
        .get("Tables")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "Unexpected Kusto v1 response: missing 'Tables' array.".to_string(),
            )
        })?;

    // Use the first table as primary result
    let Some(table) = tables.first() else {
        return Ok((Vec::new(), Vec::new()));
    };

    let columns: Vec<String> =
        table
            .get("Columns")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |cols| {
                cols.iter()
                    .filter_map(|c| {
                        c.get("ColumnName")
                            .and_then(Value::as_str)
                            .map(String::from)
                    })
                    .collect()
            });

    if columns.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let rows: Vec<Value> =
        table
            .get("Rows")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |rows| {
                rows.iter()
                    .map(|row| {
                        let mut obj = serde_json::Map::new();
                        if let Some(row_arr) = row.as_array() {
                            for (i, val) in row_arr.iter().enumerate() {
                                let col_name = columns
                                    .get(i)
                                    .cloned()
                                    .unwrap_or_else(|| format!("column{i}"));
                                obj.insert(col_name, val.clone());
                            }
                        }
                        Value::Object(obj)
                    })
                    .collect()
            });

    Ok((rows, columns))
}

/// Parse Kusto v2 response format into rows and column names.
///
/// The v2 format is a JSON array of frames:
/// - `DataSetHeader` — dataset metadata
/// - `DataTable` — result table(s) (look for `TableKind: "PrimaryResult"`)
/// - `DataSetCompletion` — final status
fn parse_kusto_v2_response(frames: &Value) -> Result<(Vec<Value>, Vec<String>)> {
    let frame_array = frames.as_array().ok_or_else(|| {
        FabioError::new(
            ErrorCode::ApiError,
            "Unexpected Kusto response format: expected JSON array of frames.".to_string(),
        )
    })?;

    // Find the PrimaryResult frame
    let primary_frame = frame_array
        .iter()
        .find(|f| {
            f.get("FrameType").and_then(Value::as_str) == Some("DataTable")
                && f.get("TableKind").and_then(Value::as_str) == Some("PrimaryResult")
        })
        .or_else(|| {
            // Fallback: first DataTable frame
            frame_array
                .iter()
                .find(|f| f.get("FrameType").and_then(Value::as_str) == Some("DataTable"))
        });

    let Some(frame) = primary_frame else {
        // Check if there's an error in the completion frame
        if let Some(completion) = frame_array
            .iter()
            .find(|f| f.get("FrameType").and_then(Value::as_str) == Some("DataSetCompletion"))
        {
            if completion.get("HasErrors").and_then(Value::as_bool) == Some(true) {
                let error_msg = completion
                    .get("OneApiErrors")
                    .map_or("Unknown Kusto error", |e| {
                        e.as_str().unwrap_or("Unknown Kusto error")
                    });
                return Err(FabioError::new(
                    ErrorCode::ApiError,
                    format!("Kusto query error: {error_msg}"),
                )
                .into());
            }
        }
        return Ok((Vec::new(), Vec::new()));
    };

    // Extract column names
    let columns: Vec<String> =
        frame
            .get("Columns")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |cols| {
                cols.iter()
                    .filter_map(|c| {
                        c.get("ColumnName")
                            .and_then(Value::as_str)
                            .map(String::from)
                    })
                    .collect()
            });

    if columns.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    // Extract rows and convert to JSON objects
    let rows: Vec<Value> =
        frame
            .get("Rows")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |rows| {
                rows.iter()
                    .map(|row| {
                        let mut obj = serde_json::Map::new();
                        if let Some(row_arr) = row.as_array() {
                            for (i, val) in row_arr.iter().enumerate() {
                                let col_name = columns
                                    .get(i)
                                    .cloned()
                                    .unwrap_or_else(|| format!("column{i}"));
                                obj.insert(col_name, val.clone());
                            }
                        }
                        Value::Object(obj)
                    })
                    .collect()
            });

    Ok((rows, columns))
}

// ─── Definitions ─────────────────────────────────────────────────────────────

async fn get_definition(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/kqlDatabases/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database get-definition", "Contributor"))?;
    output::render_object(cli, &data, "definition");
    Ok(())
}

async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let script = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(c)) => c.to_string(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio kql-database update-definition --workspace <WS> --id <ID> --file schema.kql".to_string(),
            ).into());
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(script.as_bytes());

    let body = serde_json::json!({
        "definition": {
            "parts": [
                {
                    "path": "DatabaseProperties.kql",
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    if output::dry_run_guard(
        cli,
        "kql-database update-definition",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "contentLength": script.len()
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/kqlDatabases/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Shortcuts ───────────────────────────────────────────────────────────────

async fn list_shortcuts(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/kqlDatabases/{id}/shortcuts"
        ))
        .await?;

    if let Some(arr) = data.as_array() {
        output::render_list_with_token(
            cli,
            arr,
            &["name", "target"],
            &["NAME", "TARGET"],
            "name",
            None,
        );
    } else {
        output::render_object(cli, &data, "shortcuts");
    }
    Ok(())
}

async fn create_shortcut(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let config: Value = match (file, content) {
        (Some(path), _) => {
            let raw = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?;
            serde_json::from_str(&raw)?
        }
        (_, Some(c)) => serde_json::from_str(c)?,
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio kql-database create-shortcut --workspace <WS> --id <ID> --name my-shortcut --content '{...}'"
                    .to_string(),
            )
            .into());
        }
    };

    let mut body = config;
    if let Some(obj) = body.as_object_mut() {
        obj.insert("name".to_string(), Value::String(name.to_string()));
    }

    if output::dry_run_guard(cli, "kql-database create-shortcut", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/kqlDatabases/{id}/shortcuts"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database create-shortcut", "Contributor"))?;
    output::render_object(cli, &data, "name");
    Ok(())
}

async fn get_shortcut(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    shortcut_name: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/kqlDatabases/{id}/shortcuts/{shortcut_name}"
        ))
        .await?;
    output::render_object(cli, &data, "name");
    Ok(())
}

async fn delete_shortcut(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    shortcut_name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "kql-database delete-shortcut",
        &serde_json::json!({ "workspace": workspace, "id": id, "shortcutName": shortcut_name }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!(
            "/workspaces/{workspace}/kqlDatabases/{id}/shortcuts/{shortcut_name}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database delete-shortcut", "Contributor"))?;

    let obj = serde_json::json!({ "shortcutName": shortcut_name, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn bulk_create_shortcuts(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
    conflict_policy: Option<&str>,
) -> Result<()> {
    let input: Value = match (file, content) {
        (Some(path), _) => {
            let raw = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?;
            serde_json::from_str(&raw)?
        }
        (_, Some(c)) => serde_json::from_str(c)?,
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio kql-database bulk-create-shortcuts --workspace <WS> --id <ID> --file shortcuts.json"
                    .to_string(),
            )
            .into());
        }
    };

    // Wrap in the API envelope if user provided a raw array
    let body = if input.is_array() {
        serde_json::json!({ "createShortcutRequests": input })
    } else {
        input
    };

    if output::dry_run_guard(cli, "kql-database bulk-create-shortcuts", &body) {
        return Ok(());
    }

    let mut url = format!("/workspaces/{workspace}/items/{id}/shortcuts/bulkCreate");
    if let Some(policy) = conflict_policy {
        use std::fmt::Write;
        let _ = write!(url, "?shortcutConflictPolicy={policy}");
    }

    let data = client
        .post(&url, &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database bulk-create-shortcuts", "Contributor"))?;
    output::render_object(cli, &data, "value");
    Ok(())
}

// ─── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_kusto_v2_primary_result() {
        let frames = json!([
            {
                "FrameType": "DataSetHeader",
                "IsProgressive": false,
                "Version": "v2.0"
            },
            {
                "FrameType": "DataTable",
                "TableId": 0,
                "TableKind": "PrimaryResult",
                "TableName": "PrimaryResult",
                "Columns": [
                    {"ColumnName": "Name", "ColumnType": "string"},
                    {"ColumnName": "Age", "ColumnType": "int"},
                    {"ColumnName": "Score", "ColumnType": "real"}
                ],
                "Rows": [
                    ["Alice", 30, 95.5],
                    ["Bob", 25, 87.3]
                ]
            },
            {
                "FrameType": "DataSetCompletion",
                "HasErrors": false,
                "Cancelled": false
            }
        ]);

        let (rows, columns) = parse_kusto_v2_response(&frames).unwrap();
        assert_eq!(columns, vec!["Name", "Age", "Score"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["Name"], "Alice");
        assert_eq!(rows[0]["Age"], 30);
        assert_eq!(rows[1]["Name"], "Bob");
        assert_eq!(rows[1]["Score"], 87.3);
    }

    #[test]
    fn test_parse_kusto_v2_empty_result() {
        let frames = json!([
            {
                "FrameType": "DataSetHeader",
                "IsProgressive": false,
                "Version": "v2.0"
            },
            {
                "FrameType": "DataTable",
                "TableId": 0,
                "TableKind": "PrimaryResult",
                "TableName": "PrimaryResult",
                "Columns": [
                    {"ColumnName": "Count", "ColumnType": "long"}
                ],
                "Rows": []
            },
            {
                "FrameType": "DataSetCompletion",
                "HasErrors": false,
                "Cancelled": false
            }
        ]);

        let (rows, columns) = parse_kusto_v2_response(&frames).unwrap();
        assert_eq!(columns, vec!["Count"]);
        assert!(rows.is_empty());
    }

    #[test]
    fn test_parse_kusto_v2_no_primary_falls_back_to_first_datatable() {
        let frames = json!([
            {
                "FrameType": "DataSetHeader",
                "IsProgressive": false,
                "Version": "v2.0"
            },
            {
                "FrameType": "DataTable",
                "TableId": 0,
                "TableKind": "QueryCompletionInformation",
                "TableName": "@ExtendedProperties",
                "Columns": [
                    {"ColumnName": "Key", "ColumnType": "string"},
                    {"ColumnName": "Value", "ColumnType": "string"}
                ],
                "Rows": [
                    ["ServerExecutionTime", "00:00:00.001"]
                ]
            },
            {
                "FrameType": "DataSetCompletion",
                "HasErrors": false,
                "Cancelled": false
            }
        ]);

        let (rows, columns) = parse_kusto_v2_response(&frames).unwrap();
        assert_eq!(columns, vec!["Key", "Value"]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["Key"], "ServerExecutionTime");
    }

    #[test]
    fn test_parse_kusto_v2_null_values() {
        let frames = json!([
            {
                "FrameType": "DataTable",
                "TableId": 0,
                "TableKind": "PrimaryResult",
                "TableName": "PrimaryResult",
                "Columns": [
                    {"ColumnName": "Id", "ColumnType": "int"},
                    {"ColumnName": "Label", "ColumnType": "string"}
                ],
                "Rows": [
                    [1, null],
                    [2, "active"]
                ]
            }
        ]);

        let (rows, columns) = parse_kusto_v2_response(&frames).unwrap();
        assert_eq!(columns, vec!["Id", "Label"]);
        assert_eq!(rows.len(), 2);
        assert!(rows[0]["Label"].is_null());
        assert_eq!(rows[1]["Label"], "active");
    }

    #[test]
    fn test_parse_kusto_v2_no_frames_returns_empty() {
        let frames = json!([
            {
                "FrameType": "DataSetHeader",
                "IsProgressive": false,
                "Version": "v2.0"
            },
            {
                "FrameType": "DataSetCompletion",
                "HasErrors": false,
                "Cancelled": false
            }
        ]);

        let (rows, columns) = parse_kusto_v2_response(&frames).unwrap();
        assert!(rows.is_empty());
        assert!(columns.is_empty());
    }

    #[test]
    fn test_parse_kusto_v2_error_in_completion() {
        let frames = json!([
            {
                "FrameType": "DataSetHeader",
                "IsProgressive": false,
                "Version": "v2.0"
            },
            {
                "FrameType": "DataSetCompletion",
                "HasErrors": true,
                "OneApiErrors": "Syntax error in query"
            }
        ]);

        let result = parse_kusto_v2_response(&frames);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Syntax error in query"));
    }

    #[test]
    fn test_parse_kusto_v2_not_array_returns_error() {
        let frames = json!({"error": "unexpected"});

        let result = parse_kusto_v2_response(&frames);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("expected JSON array"));
    }
}
