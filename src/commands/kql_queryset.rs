use anyhow::Result;
use base64::Engine;
use clap::Subcommand;
use reqwest::header::AUTHORIZATION;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum KqlQuerysetCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List KQL querysets in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// Show details of a KQL queryset
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// KQL queryset ID
        #[arg(long)]
        id: String,
    },
    /// Create a new KQL queryset
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Queryset display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update KQL queryset properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// KQL queryset ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a KQL queryset
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// KQL queryset ID
        #[arg(long)]
        id: String,
    },

    // ── Definitions ──────────────────────────────────────────────────────
    /// Get the definition of a KQL queryset
    #[command(display_order = 6)]
    GetDefinition {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// KQL queryset ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition of a KQL queryset
    #[command(display_order = 7)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// KQL queryset ID
        #[arg(long)]
        id: String,

        /// KQL queryset file path (reads file content)
        #[arg(long)]
        file: Option<String>,

        /// KQL queryset content (inline)
        #[arg(long)]
        content: Option<String>,
    },

    // ── Query Execution ──────────────────────────────────────────────────
    /// Run a saved query tab from the queryset against its configured data source
    #[command(display_order = 8)]
    Run {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// KQL queryset ID
        #[arg(long)]
        id: String,

        /// Tab name or zero-based index to execute (default: first tab)
        #[arg(long)]
        tab: Option<String>,

        /// Override the Kusto query URI (default: from queryset data source)
        #[arg(long)]
        query_uri: Option<String>,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &KqlQuerysetCommand) -> Result<()> {
    match command {
        KqlQuerysetCommand::List { workspace } => list(cli, client, workspace).await,
        KqlQuerysetCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        KqlQuerysetCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref()).await,
        KqlQuerysetCommand::Update {
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
        KqlQuerysetCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        KqlQuerysetCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        KqlQuerysetCommand::UpdateDefinition {
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
        KqlQuerysetCommand::Run {
            workspace,
            id,
            tab,
            query_uri,
        } => {
            run(
                cli,
                client,
                workspace,
                id,
                tab.as_deref(),
                query_uri.as_deref(),
            )
            .await
        }
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/kqlQuerysets"),
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
        .get(&format!("/workspaces/{workspace}/kqlQuerysets/{id}"))
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
        "displayName": name
    });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if output::dry_run_guard(cli, "kql-queryset create", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/kqlQuerysets"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-queryset create", "Member"))?;
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
            "Example: fabio kql-queryset update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "kql-queryset update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/kqlQuerysets/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "kql-queryset update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "kql-queryset delete",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/kqlQuerysets/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "kql-queryset delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Definitions ─────────────────────────────────────────────────────────────

async fn get_definition(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/kqlQuerysets/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-queryset get-definition", "Contributor"))?;
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
                "Example: fabio kql-queryset update-definition --workspace <WS> --id <ID> --file query.kql".to_string(),
            ).into());
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(script.as_bytes());

    let body = serde_json::json!({
        "definition": {
            "parts": [
                {
                    "path": "RealTimeQueryset.json",
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    if output::dry_run_guard(
        cli,
        "kql-queryset update-definition",
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
            &format!("/workspaces/{workspace}/kqlQuerysets/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-queryset update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Run (Query Execution) ───────────────────────────────────────────────────

/// Run a saved query tab from the queryset definition against its configured data source.
#[allow(clippy::too_many_lines)]
async fn run(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    tab_selector: Option<&str>,
    query_uri_override: Option<&str>,
) -> Result<()> {
    // 1. Fetch queryset definition (LRO)
    let def_data = client
        .post(
            &format!("/workspaces/{workspace}/kqlQuerysets/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-queryset run", "Viewer"))?;

    // 2. Find RealTimeQueryset.json part and decode it
    let queryset = decode_queryset_definition(&def_data)?;

    // 3. Extract data sources and tabs
    let qs = queryset.get("queryset").ok_or_else(|| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Queryset definition missing 'queryset' root object.".to_string(),
            "The queryset may be empty. Use 'kql-queryset update-definition' to save queries."
                .to_string(),
        )
    })?;

    let data_sources = qs
        .get("dataSources")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Queryset has no data sources configured.".to_string(),
                "Update the queryset definition with data source info (clusterUri, databaseName)."
                    .to_string(),
            )
        })?;

    let tabs = qs.get("tabs").and_then(Value::as_array).ok_or_else(|| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Queryset has no tabs (saved queries).".to_string(),
            "Update the queryset definition to add tabs with KQL queries.".to_string(),
        )
    })?;

    if tabs.is_empty() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Queryset has no tabs (saved queries).".to_string(),
            "Update the queryset definition to add tabs with KQL queries.".to_string(),
        )
        .into());
    }

    // 4. Select tab by name or index
    let tab = select_tab(tabs, tab_selector)?;

    // 5. Get the KQL content from the tab
    let kql_text = tab.get("content").and_then(Value::as_str).ok_or_else(|| {
        FabioError::new(
            ErrorCode::InvalidInput,
            "Selected tab has no 'content' field (KQL query text).".to_string(),
        )
    })?;

    if kql_text.trim().is_empty() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Selected tab has empty KQL query content.".to_string(),
            "Update the queryset definition with a non-empty query in the tab.".to_string(),
        )
        .into());
    }

    // 6. Resolve data source for this tab
    let ds_id = tab.get("dataSourceId").and_then(Value::as_str);
    let data_source = resolve_data_source(data_sources, ds_id)?;

    let cluster_uri = query_uri_override
        .map(|u| u.trim_end_matches('/').to_string())
        .or_else(|| {
            data_source
                .get("clusterUri")
                .and_then(Value::as_str)
                .map(|u| u.trim_end_matches('/').to_string())
        })
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Could not determine Kusto query URI from queryset data source.".to_string(),
                "Provide --query-uri manually or update the queryset definition with clusterUri."
                    .to_string(),
            )
        })?;

    let db_name = data_source
        .get("databaseName")
        .and_then(Value::as_str)
        .unwrap_or_default();

    // 7. Acquire token scoped to the Kusto query URI
    let scope = format!("{cluster_uri}/.default");
    let token = client.require_token_for_scope(&scope).await?;

    // 8. Execute KQL query (management commands starting with '.' use v1/mgmt, else v2/query)
    let is_mgmt = kql_text.trim_start().starts_with('.');
    let url = if is_mgmt {
        format!("{cluster_uri}/v1/rest/mgmt")
    } else {
        format!("{cluster_uri}/v2/rest/query")
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
            "Verify the data source clusterUri and databaseName in the queryset are correct."
                .to_string(),
        )
        .into());
    }

    // 9. Parse response
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

    // 10. Render output
    if rows.is_empty() {
        let obj = serde_json::json!({
            "rows_returned": 0,
            "tab": tab.get("title").and_then(Value::as_str).unwrap_or(""),
            "message": "Query executed successfully (no results returned)."
        });
        output::render_object(cli, &obj, "message");
    } else {
        let col_refs: Vec<&str> = columns.iter().map(String::as_str).collect();
        output::render_list(cli, &rows, &col_refs, &col_refs, &columns[0]);
    }

    Ok(())
}

/// Decode RealTimeQueryset.json from the getDefinition response.
fn decode_queryset_definition(def_data: &Value) -> Result<Value> {
    let parts = def_data
        .get("definition")
        .and_then(|d| d.get("parts"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "Unexpected definition response: missing 'definition.parts' array.".to_string(),
            )
        })?;

    let queryset_part = parts
        .iter()
        .find(|p| p.get("path").and_then(Value::as_str) == Some("RealTimeQueryset.json"))
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::NotFound,
                "No 'RealTimeQueryset.json' part found in queryset definition.".to_string(),
                "The queryset may be empty or in an unexpected format.".to_string(),
            )
        })?;

    let payload = queryset_part
        .get("payload")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "RealTimeQueryset.json part has no payload.".to_string(),
            )
        })?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|e| {
            FabioError::new(
                ErrorCode::ApiError,
                format!("Failed to decode RealTimeQueryset.json base64 payload: {e}"),
            )
        })?;

    let decoded_str = String::from_utf8(decoded_bytes).map_err(|e| {
        FabioError::new(
            ErrorCode::ApiError,
            format!("RealTimeQueryset.json payload is not valid UTF-8: {e}"),
        )
    })?;

    // Handle empty queryset (just "{}")
    let trimmed = decoded_str.trim();
    if trimmed == "{}" || trimmed.is_empty() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Queryset definition is empty (no saved queries).".to_string(),
            "Use 'fabio kql-queryset update-definition' to save queries into the queryset."
                .to_string(),
        )
        .into());
    }

    serde_json::from_str(&decoded_str).map_err(|e| {
        FabioError::new(
            ErrorCode::ApiError,
            format!("Failed to parse RealTimeQueryset.json content: {e}"),
        )
        .into()
    })
}

/// Select a tab from the queryset by name (title) or zero-based index.
fn select_tab<'a>(tabs: &'a [Value], selector: Option<&str>) -> Result<&'a Value> {
    match selector {
        None => {
            // Default: first tab
            Ok(&tabs[0])
        }
        Some(s) => {
            // Try as zero-based index first
            if let Ok(idx) = s.parse::<usize>() {
                return tabs.get(idx).ok_or_else(|| {
                    let tab_names: Vec<&str> = tabs
                        .iter()
                        .filter_map(|t| t.get("title").and_then(Value::as_str))
                        .collect();
                    FabioError::with_hint(
                        ErrorCode::NotFound,
                        format!(
                            "Tab index {idx} out of range (queryset has {} tabs).",
                            tabs.len()
                        ),
                        format!("Available tabs: {}", tab_names.join(", ")),
                    )
                    .into()
                });
            }

            // Try by title (case-insensitive match)
            let found = tabs.iter().find(|t| {
                t.get("title")
                    .and_then(Value::as_str)
                    .is_some_and(|title| title.eq_ignore_ascii_case(s))
            });

            found.ok_or_else(|| {
                let tab_names: Vec<&str> = tabs
                    .iter()
                    .filter_map(|t| t.get("title").and_then(Value::as_str))
                    .collect();
                FabioError::with_hint(
                    ErrorCode::NotFound,
                    format!("Tab '{s}' not found in queryset."),
                    format!("Available tabs: {}", tab_names.join(", ")),
                )
                .into()
            })
        }
    }
}

/// Resolve the data source from the queryset for a given tab.
fn resolve_data_source<'a>(data_sources: &'a [Value], ds_id: Option<&str>) -> Result<&'a Value> {
    if data_sources.is_empty() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Queryset has no data sources configured.".to_string(),
            "Update the queryset definition with data source info (clusterUri, databaseName)."
                .to_string(),
        )
        .into());
    }

    ds_id.map_or_else(
        || Ok(&data_sources[0]),
        |id| {
            data_sources
                .iter()
                .find(|ds| ds.get("id").and_then(Value::as_str) == Some(id))
                .ok_or_else(|| {
                    FabioError::with_hint(
                        ErrorCode::NotFound,
                        format!("Data source '{id}' referenced by tab not found in queryset."),
                        "Verify the queryset definition has matching dataSourceId entries."
                            .to_string(),
                    )
                    .into()
                })
        },
    )
}

// ─── Kusto Response Parsing ──────────────────────────────────────────────────

/// Parse Kusto v1 response format (management commands via `/v1/rest/mgmt`).
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

/// Parse Kusto v2 response format (queries via `/v2/rest/query`).
fn parse_kusto_v2_response(frames: &Value) -> Result<(Vec<Value>, Vec<String>)> {
    let frame_array = frames.as_array().ok_or_else(|| {
        FabioError::new(
            ErrorCode::ApiError,
            "Unexpected Kusto response format: expected JSON array of frames.".to_string(),
        )
    })?;

    let primary_frame = frame_array
        .iter()
        .find(|f| {
            f.get("FrameType").and_then(Value::as_str) == Some("DataTable")
                && f.get("TableKind").and_then(Value::as_str) == Some("PrimaryResult")
        })
        .or_else(|| {
            frame_array
                .iter()
                .find(|f| f.get("FrameType").and_then(Value::as_str) == Some("DataTable"))
        });

    let Some(frame) = primary_frame else {
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

// ─── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_queryset_definition_success() {
        let payload = r#"{"queryset":{"version":"1.0.0","dataSources":[{"id":"ds1","clusterUri":"https://test.kusto.fabric.microsoft.com","type":"AzureDataExplorer","databaseName":"TestDb"}],"tabs":[{"id":"t1","content":"T | count","title":"CountTab","dataSourceId":"ds1"}]}}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(payload.as_bytes());
        let def_data = serde_json::json!({
            "definition": {
                "parts": [{
                    "path": "RealTimeQueryset.json",
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }]
            }
        });

        let result = decode_queryset_definition(&def_data).unwrap();
        assert_eq!(
            result["queryset"]["dataSources"][0]["databaseName"],
            "TestDb"
        );
        assert_eq!(result["queryset"]["tabs"][0]["content"], "T | count");
    }

    #[test]
    fn test_decode_queryset_definition_empty() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(b"{}");
        let def_data = serde_json::json!({
            "definition": {
                "parts": [{
                    "path": "RealTimeQueryset.json",
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }]
            }
        });

        let result = decode_queryset_definition(&def_data);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("empty"));
    }

    #[test]
    fn test_decode_queryset_definition_missing_part() {
        let def_data = serde_json::json!({
            "definition": {
                "parts": [{
                    "path": "other.json",
                    "payload": "e30=",
                    "payloadType": "InlineBase64"
                }]
            }
        });

        let result = decode_queryset_definition(&def_data);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("RealTimeQueryset.json"));
    }

    #[test]
    fn test_select_tab_default_first() {
        let tabs = vec![
            serde_json::json!({"id": "t1", "title": "First", "content": "Q1"}),
            serde_json::json!({"id": "t2", "title": "Second", "content": "Q2"}),
        ];
        let tab = select_tab(&tabs, None).unwrap();
        assert_eq!(tab["title"], "First");
    }

    #[test]
    fn test_select_tab_by_index() {
        let tabs = vec![
            serde_json::json!({"id": "t1", "title": "First", "content": "Q1"}),
            serde_json::json!({"id": "t2", "title": "Second", "content": "Q2"}),
        ];
        let tab = select_tab(&tabs, Some("1")).unwrap();
        assert_eq!(tab["title"], "Second");
    }

    #[test]
    fn test_select_tab_by_name() {
        let tabs = vec![
            serde_json::json!({"id": "t1", "title": "SalesByType", "content": "Q1"}),
            serde_json::json!({"id": "t2", "title": "HighValue", "content": "Q2"}),
        ];
        let tab = select_tab(&tabs, Some("HighValue")).unwrap();
        assert_eq!(tab["id"], "t2");
    }

    #[test]
    fn test_select_tab_by_name_case_insensitive() {
        let tabs = vec![serde_json::json!({"id": "t1", "title": "SalesByType", "content": "Q1"})];
        let tab = select_tab(&tabs, Some("salesbytype")).unwrap();
        assert_eq!(tab["id"], "t1");
    }

    #[test]
    fn test_select_tab_not_found() {
        let tabs = vec![serde_json::json!({"id": "t1", "title": "First", "content": "Q1"})];
        let result = select_tab(&tabs, Some("NonExistent"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("NonExistent"));
    }

    #[test]
    fn test_select_tab_index_out_of_range() {
        let tabs = vec![serde_json::json!({"id": "t1", "title": "First", "content": "Q1"})];
        let result = select_tab(&tabs, Some("5"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("out of range"));
    }

    #[test]
    fn test_resolve_data_source_by_id() {
        let sources = vec![
            serde_json::json!({"id": "ds1", "clusterUri": "https://a.kusto.fabric.microsoft.com", "databaseName": "Db1"}),
            serde_json::json!({"id": "ds2", "clusterUri": "https://b.kusto.fabric.microsoft.com", "databaseName": "Db2"}),
        ];
        let ds = resolve_data_source(&sources, Some("ds2")).unwrap();
        assert_eq!(ds["databaseName"], "Db2");
    }

    #[test]
    fn test_resolve_data_source_default_first() {
        let sources = vec![
            serde_json::json!({"id": "ds1", "clusterUri": "https://a.kusto.fabric.microsoft.com", "databaseName": "Db1"}),
        ];
        let ds = resolve_data_source(&sources, None).unwrap();
        assert_eq!(ds["databaseName"], "Db1");
    }

    #[test]
    fn test_resolve_data_source_not_found() {
        let sources = vec![
            serde_json::json!({"id": "ds1", "clusterUri": "https://a.kusto.fabric.microsoft.com", "databaseName": "Db1"}),
        ];
        let result = resolve_data_source(&sources, Some("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_data_source_empty() {
        let sources: Vec<Value> = vec![];
        let result = resolve_data_source(&sources, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("no data sources"));
    }

    #[test]
    fn test_parse_kusto_v1_response() {
        let resp = serde_json::json!({
            "Tables": [{
                "TableName": "Table_0",
                "Columns": [
                    {"ColumnName": "Count", "DataType": "Int64"}
                ],
                "Rows": [[42]]
            }]
        });
        let (rows, columns) = parse_kusto_v1_response(&resp).unwrap();
        assert_eq!(columns, vec!["Count"]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["Count"], 42);
    }

    #[test]
    fn test_parse_kusto_v2_response() {
        let frames = serde_json::json!([
            {"FrameType": "DataSetHeader", "IsProgressive": false},
            {
                "FrameType": "DataTable",
                "TableKind": "PrimaryResult",
                "TableName": "PrimaryResult",
                "Columns": [{"ColumnName": "event_type", "ColumnType": "string"}],
                "Rows": [["purchase"], ["refund"]]
            },
            {"FrameType": "DataSetCompletion", "HasErrors": false}
        ]);
        let (rows, columns) = parse_kusto_v2_response(&frames).unwrap();
        assert_eq!(columns, vec!["event_type"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["event_type"], "purchase");
        assert_eq!(rows[1]["event_type"], "refund");
    }

    #[test]
    fn test_parse_kusto_v2_response_with_error() {
        let frames = serde_json::json!([
            {"FrameType": "DataSetHeader", "IsProgressive": false},
            {"FrameType": "DataSetCompletion", "HasErrors": true, "OneApiErrors": "Syntax error"}
        ]);
        let result = parse_kusto_v2_response(&frames);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Syntax error"));
    }
}
