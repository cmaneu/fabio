mod config;
mod crud;
mod datasources;
mod definition;
mod elements;
mod fewshots;
mod query;

use anyhow::Result;
use base64::Engine;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};

#[derive(Debug, Subcommand)]
pub enum DataAgentCommand {
    /// List data agents in a workspace
    List {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Show details of a data agent
    Show {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,
    },
    /// Create a new data agent
    Create {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent display name
        #[arg(long)]
        name: String,

        /// Data agent description (max 256 characters)
        #[arg(long)]
        description: Option<String>,
    },
    /// Update a data agent (name and/or description)
    Update {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description (max 256 characters)
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a data agent
    Delete {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Permanently delete (cannot be recovered)
        #[arg(long)]
        hard_delete: bool,
    },
    /// Query (chat with) a published data agent using natural language
    Query {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Natural language question (omit to read from stdin)
        #[arg(short, long)]
        prompt: Option<String>,

        /// Published URL (from portal Settings page after publishing the agent)
        #[arg(long)]
        published_url: Option<String>,

        /// Include execution details (SQL queries, tool calls, run steps)
        #[arg(long)]
        show_steps: bool,

        /// Agent stage to query: sandbox (draft) or production (published)
        #[arg(long, default_value = "production")]
        stage: String,

        /// Maximum wait time in seconds for the query to complete (default: 300)
        #[arg(long, default_value = "300")]
        timeout: u64,
    },

    // ── Configuration ────────────────────────────────────────────────────
    /// Get the configuration of a data agent (instructions, data sources, preview runtime)
    #[command(display_order = 8)]
    GetConfig {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,
    },
    /// Update the configuration of a data agent (instructions, preview runtime)
    #[command(display_order = 9)]
    UpdateConfig {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// AI instructions for the agent (guides data source selection and query generation)
        #[arg(long)]
        instructions: Option<String>,

        /// Path to file containing AI instructions (alternative to --instructions)
        #[arg(long, conflicts_with = "instructions")]
        instructions_file: Option<String>,

        /// Enable preview runtime (agentic NL2SQL reasoning path)
        #[arg(long)]
        enable_preview_runtime: bool,

        /// Disable preview runtime
        #[arg(long, conflicts_with = "enable_preview_runtime")]
        disable_preview_runtime: bool,
    },

    // ── Datasource Management ────────────────────────────────────────────
    /// List configured data sources for a data agent
    #[command(display_order = 13)]
    ListDatasources {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,
    },
    /// Show details of a configured data source
    #[command(display_order = 14)]
    ShowDatasource {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Data source name or ID
        #[arg(long)]
        datasource: String,
    },
    /// Add a data source to the agent (auto-discovers schema from artifact)
    #[command(display_order = 15)]
    AddDatasource {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Artifact name or ID (lakehouse, warehouse, KQL database, semantic model, etc.)
        #[arg(long)]
        artifact: String,

        /// Workspace containing the artifact (defaults to same workspace as agent)
        #[arg(long)]
        artifact_workspace: Option<String>,

        /// Artifact type (auto-detected if omitted). Values: `Lakehouse`, `Warehouse`, `KQLDatabase`, `SemanticModel`, `Ontology`, `GraphModel`, `MirroredDatabase`, `SQLDatabase`
        #[arg(long, value_name = "TYPE")]
        artifact_type: Option<String>,

        /// Data source instructions (how the agent should use this source)
        #[arg(long)]
        instructions: Option<String>,
    },
    /// Remove a data source from the agent
    #[command(display_order = 16)]
    RemoveDatasource {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Data source name or ID to remove
        #[arg(long)]
        datasource: String,
    },

    // ── Few-shot Management ──────────────────────────────────────────────
    /// List few-shot examples for a data source
    #[command(display_order = 17)]
    ListFewshots {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Data source name or ID
        #[arg(long)]
        datasource: String,
    },
    /// Add a few-shot example (question/query pair) to a data source
    #[command(display_order = 18)]
    AddFewshot {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Data source name or ID
        #[arg(long)]
        datasource: String,

        /// Natural language question
        #[arg(long)]
        question: String,

        /// SQL/KQL/DAX query that answers the question
        #[arg(long, visible_alias = "sql")]
        answer: String,
    },
    /// Remove a few-shot example by ID
    #[command(display_order = 19)]
    RemoveFewshot {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Data source name or ID
        #[arg(long)]
        datasource: String,

        /// Few-shot example ID to remove
        #[arg(long)]
        fewshot_id: String,
    },
    /// Bulk upload few-shot examples from a JSON or CSV file
    #[command(display_order = 20)]
    UploadFewshots {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Data source name or ID
        #[arg(long)]
        datasource: String,

        /// File with few-shots (JSON: [{"question":"...", "query":"..."}] or CSV with question,query columns)
        #[arg(long)]
        file: String,
    },
    /// Select or unselect tables in a data source
    #[command(display_order = 21)]
    SelectTables {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Data source name or ID
        #[arg(long)]
        datasource: String,

        /// Comma-separated table names to select (e.g. "orders,products,customers")
        #[arg(long)]
        tables: Option<String>,

        /// Select all tables
        #[arg(long, conflicts_with = "tables")]
        all_tables: bool,

        /// Unselect (instead of select)
        #[arg(long)]
        unselect: bool,
    },
    /// List elements (tables, columns) in a data source with selection state and descriptions
    #[command(display_order = 22)]
    ListElements {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Data source name or ID
        #[arg(long)]
        datasource: String,
    },
    /// Set or clear a description on a table or column in a data source
    #[command(display_order = 23)]
    DescribeElement {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Data source name or ID
        #[arg(long)]
        datasource: String,

        /// Dot-separated path to the element (e.g. `dbo.orders` for a table, `dbo.orders.total_amount` for a column)
        #[arg(long)]
        path: String,

        /// Description text (omit or pass empty string to clear)
        #[arg(long)]
        description: Option<String>,
    },

    // ── Definitions ──────────────────────────────────────────────────────
    /// Get the definition of a data agent (configuration, data sources, etc.)
    #[command(display_order = 10)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition of a data agent (configure data sources, instructions, etc.)
    #[command(display_order = 11)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Path to definition file (JSON with parts array)
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON definition (alternative to --file)
        #[arg(long)]
        content: Option<String>,

        /// Also update item metadata from .platform file if present
        #[arg(long)]
        update_metadata: bool,
    },
    /// Publish a data agent (promotes draft configuration to published state)
    #[command(display_order = 12)]
    Publish {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Optional publish description
        #[arg(long)]
        description: Option<String>,

        /// Also publish to Microsoft 365 Copilot Agent Store
        #[arg(long)]
        to_m365: bool,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &DataAgentCommand) -> Result<()> {
    match command {
        DataAgentCommand::List { workspace } => crud::list(cli, client, workspace).await,
        DataAgentCommand::Show { workspace, id } => crud::show(cli, client, workspace, id).await,
        DataAgentCommand::Create {
            workspace,
            name,
            description,
        } => crud::create(cli, client, workspace, name, description.as_deref())
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent create", "Member")),
        DataAgentCommand::Update {
            workspace,
            id,
            name,
            description,
        } => crud::update(
            cli,
            client,
            workspace,
            id,
            name.as_deref(),
            description.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-agent update", "Contributor")),
        DataAgentCommand::Delete {
            workspace,
            id,
            hard_delete,
        } => crud::delete(cli, client, workspace, id, *hard_delete)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent delete", "Member")),
        DataAgentCommand::Query {
            workspace,
            id,
            prompt,
            published_url,
            show_steps,
            stage,
            timeout,
        } => query::query(
            cli,
            client,
            workspace,
            id,
            prompt.as_deref(),
            published_url.as_deref(),
            *show_steps,
            stage,
            *timeout,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-agent query", "Viewer")),
        DataAgentCommand::GetConfig { workspace, id } => {
            config::get_config(cli, client, workspace, id)
                .await
                .map_err(|e| enrich_forbidden(e, "data-agent get-config", "Contributor"))
        }
        DataAgentCommand::UpdateConfig {
            workspace,
            id,
            instructions,
            instructions_file,
            enable_preview_runtime,
            disable_preview_runtime,
        } => config::update_config(
            cli,
            client,
            workspace,
            id,
            instructions.as_deref(),
            instructions_file.as_deref(),
            *enable_preview_runtime,
            *disable_preview_runtime,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-agent update-config", "Contributor")),
        DataAgentCommand::ListDatasources { workspace, id } => {
            datasources::list_datasources(cli, client, workspace, id)
                .await
                .map_err(|e| enrich_forbidden(e, "data-agent list-datasources", "Contributor"))
        }
        DataAgentCommand::ShowDatasource {
            workspace,
            id,
            datasource,
        } => datasources::show_datasource(cli, client, workspace, id, datasource)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent show-datasource", "Contributor")),
        DataAgentCommand::AddDatasource {
            workspace,
            id,
            artifact,
            artifact_workspace,
            artifact_type,
            instructions,
        } => datasources::add_datasource(
            cli,
            client,
            workspace,
            id,
            artifact,
            artifact_workspace.as_deref(),
            artifact_type.as_deref(),
            instructions.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-agent add-datasource", "Contributor")),
        DataAgentCommand::RemoveDatasource {
            workspace,
            id,
            datasource,
        } => datasources::remove_datasource(cli, client, workspace, id, datasource)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent remove-datasource", "Contributor")),
        DataAgentCommand::ListFewshots {
            workspace,
            id,
            datasource,
        } => fewshots::list_fewshots(cli, client, workspace, id, datasource)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent list-fewshots", "Contributor")),
        DataAgentCommand::AddFewshot {
            workspace,
            id,
            datasource,
            question,
            answer,
        } => fewshots::add_fewshot(cli, client, workspace, id, datasource, question, answer)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent add-fewshot", "Contributor")),
        DataAgentCommand::RemoveFewshot {
            workspace,
            id,
            datasource,
            fewshot_id,
        } => fewshots::remove_fewshot(cli, client, workspace, id, datasource, fewshot_id)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent remove-fewshot", "Contributor")),
        DataAgentCommand::UploadFewshots {
            workspace,
            id,
            datasource,
            file,
        } => fewshots::upload_fewshots(cli, client, workspace, id, datasource, file)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent upload-fewshots", "Contributor")),
        DataAgentCommand::SelectTables {
            workspace,
            id,
            datasource,
            tables,
            all_tables,
            unselect,
        } => datasources::select_tables(
            cli,
            client,
            workspace,
            id,
            datasource,
            tables.as_deref(),
            *all_tables,
            *unselect,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-agent select-tables", "Contributor")),
        DataAgentCommand::ListElements {
            workspace,
            id,
            datasource,
        } => elements::list_elements(cli, client, workspace, id, datasource)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent list-elements", "Contributor")),
        DataAgentCommand::DescribeElement {
            workspace,
            id,
            datasource,
            path,
            description,
        } => elements::describe_element(
            cli,
            client,
            workspace,
            id,
            datasource,
            path,
            description.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-agent describe-element", "Contributor")),
        DataAgentCommand::GetDefinition { workspace, id } => {
            definition::get_definition(cli, client, workspace, id)
                .await
                .map_err(|e| enrich_forbidden(e, "data-agent get-definition", "Contributor"))
        }
        DataAgentCommand::UpdateDefinition {
            workspace,
            id,
            file,
            content,
            update_metadata,
        } => definition::update_definition(
            cli,
            client,
            workspace,
            id,
            file.as_deref(),
            content.as_deref(),
            *update_metadata,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-agent update-definition", "Contributor")),
        DataAgentCommand::Publish {
            workspace,
            id,
            description,
            to_m365,
        } => definition::publish(cli, client, workspace, id, description.as_deref(), *to_m365)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent publish", "Contributor")),
    }
}

// ─── Shared Helpers ──────────────────────────────────────────────────────────

/// Get definition parts from the API.
pub(super) async fn get_definition_parts(
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<Vec<Value>> {
    let resp = client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await?;

    Ok(resp
        .get("definition")
        .and_then(|d| d.get("parts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// Decode a base64-encoded definition part payload to a UTF-8 string.
pub(super) fn decode_part_payload(payload: &str) -> Option<String> {
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

/// Find the directory path for a named datasource in definition parts.
pub(super) fn find_datasource_dir(parts: &[Value], datasource: &str) -> Result<String> {
    for part in parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if path.starts_with("Files/Config/draft/") && path.ends_with("/datasource.json") {
            let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");
            if let Some(decoded) = decode_part_payload(payload) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&decoded) {
                    let name = parsed
                        .get("displayName")
                        .or_else(|| parsed.get("display_name"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let art_id = parsed
                        .get("artifactId")
                        .or_else(|| parsed.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    if name.eq_ignore_ascii_case(datasource) || art_id == datasource {
                        // Return directory (path without /datasource.json)
                        return Ok(path.trim_end_matches("/datasource.json").to_string());
                    }
                }
            }
        }
    }
    Err(FabioError::with_hint(
        ErrorCode::NotFound,
        format!("Data source '{datasource}' not found in agent definition"),
        "List available data sources: fabio data-agent list-datasources -w <workspace> --id <id>",
    )
    .into())
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::*;

    #[test]
    fn decode_part_payload_valid_base64() {
        let payload = base64::engine::general_purpose::STANDARD.encode(br#"{"hello":"world"}"#);
        let decoded = decode_part_payload(&payload).unwrap();
        assert_eq!(decoded, r#"{"hello":"world"}"#);
    }

    #[test]
    fn decode_part_payload_invalid_base64() {
        assert!(decode_part_payload("not-valid-base64!!!").is_none());
    }

    #[test]
    fn find_datasource_dir_by_name() {
        let ds_json = serde_json::json!({"displayName": "MyWarehouse", "type": "data_warehouse", "artifactId": "bbb"});
        let payload =
            base64::engine::general_purpose::STANDARD.encode(ds_json.to_string().as_bytes());
        let parts = vec![serde_json::json!({
            "path": "Files/Config/draft/data_warehouse-MyWarehouse/datasource.json",
            "payload": payload,
            "payloadType": "InlineBase64"
        })];

        let dir = find_datasource_dir(&parts, "MyWarehouse").unwrap();
        assert_eq!(dir, "Files/Config/draft/data_warehouse-MyWarehouse");
    }

    #[test]
    fn find_datasource_dir_by_id() {
        let ds_json = serde_json::json!({"displayName": "TestLH", "type": "lakehouse_tables", "artifactId": "abc-123"});
        let payload =
            base64::engine::general_purpose::STANDARD.encode(ds_json.to_string().as_bytes());
        let parts = vec![serde_json::json!({
            "path": "Files/Config/draft/lakehouse_tables-TestLH/datasource.json",
            "payload": payload,
            "payloadType": "InlineBase64"
        })];

        let dir = find_datasource_dir(&parts, "abc-123").unwrap();
        assert_eq!(dir, "Files/Config/draft/lakehouse_tables-TestLH");
    }

    #[test]
    fn find_datasource_dir_not_found() {
        let parts = vec![serde_json::json!({
            "path": "Files/Config/data_agent.json",
            "payload": "e30=",
            "payloadType": "InlineBase64"
        })];
        assert!(find_datasource_dir(&parts, "nonexistent").is_err());
    }
}
