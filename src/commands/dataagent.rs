use std::io::{self, Read};
use std::time::Duration;

use anyhow::Result;
use base64::Engine;
use clap::Subcommand;
use serde_json::Value;
use tokio::time::sleep;

use crate::cli::Cli;
use crate::client::{self, FabricClient};
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

/// Polling interval for data agent query runs.
const QUERY_POLL_INTERVAL: Duration = Duration::from_secs(2);

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
        DataAgentCommand::List { workspace } => list(cli, client, workspace).await,
        DataAgentCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        DataAgentCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref())
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent create", "Member")),
        DataAgentCommand::Update {
            workspace,
            id,
            name,
            description,
        } => update(
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
        } => delete(cli, client, workspace, id, *hard_delete)
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
        } => query(
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
        DataAgentCommand::GetConfig { workspace, id } => get_config(cli, client, workspace, id)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent get-config", "Contributor")),
        DataAgentCommand::UpdateConfig {
            workspace,
            id,
            instructions,
            instructions_file,
            enable_preview_runtime,
            disable_preview_runtime,
        } => update_config(
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
            list_datasources(cli, client, workspace, id)
                .await
                .map_err(|e| enrich_forbidden(e, "data-agent list-datasources", "Contributor"))
        }
        DataAgentCommand::ShowDatasource {
            workspace,
            id,
            datasource,
        } => show_datasource(cli, client, workspace, id, datasource)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent show-datasource", "Contributor")),
        DataAgentCommand::AddDatasource {
            workspace,
            id,
            artifact,
            artifact_workspace,
            artifact_type,
            instructions,
        } => add_datasource(
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
        } => remove_datasource(cli, client, workspace, id, datasource)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent remove-datasource", "Contributor")),
        DataAgentCommand::ListFewshots {
            workspace,
            id,
            datasource,
        } => list_fewshots(cli, client, workspace, id, datasource)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent list-fewshots", "Contributor")),
        DataAgentCommand::AddFewshot {
            workspace,
            id,
            datasource,
            question,
            answer,
        } => add_fewshot(cli, client, workspace, id, datasource, question, answer)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent add-fewshot", "Contributor")),
        DataAgentCommand::RemoveFewshot {
            workspace,
            id,
            datasource,
            fewshot_id,
        } => remove_fewshot(cli, client, workspace, id, datasource, fewshot_id)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent remove-fewshot", "Contributor")),
        DataAgentCommand::UploadFewshots {
            workspace,
            id,
            datasource,
            file,
        } => upload_fewshots(cli, client, workspace, id, datasource, file)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent upload-fewshots", "Contributor")),
        DataAgentCommand::SelectTables {
            workspace,
            id,
            datasource,
            tables,
            all_tables,
            unselect,
        } => select_tables(
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
        DataAgentCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id)
                .await
                .map_err(|e| enrich_forbidden(e, "data-agent get-definition", "Contributor"))
        }
        DataAgentCommand::UpdateDefinition {
            workspace,
            id,
            file,
            content,
            update_metadata,
        } => update_definition(
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
        } => publish(cli, client, workspace, id, description.as_deref(), *to_m365)
            .await
            .map_err(|e| enrich_forbidden(e, "data-agent publish", "Contributor")),
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/dataAgents"),
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
        .get(&format!("/workspaces/{workspace}/dataAgents/{id}"))
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

    let data = client
        .post(
            &format!("/workspaces/{workspace}/dataAgents"),
            &body,
            true, // LRO-aware
        )
        .await?;

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
        return Err(FabioError::invalid_input(
            "At least one of --name or --description must be provided",
        )
        .into());
    }

    let mut body = serde_json::Map::new();
    if let Some(n) = name {
        body.insert("displayName".to_string(), Value::String(n.to_string()));
    }
    if let Some(d) = description {
        body.insert("description".to_string(), Value::String(d.to_string()));
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/dataAgents/{id}"),
            &Value::Object(body),
        )
        .await?;

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
    let url = if hard_delete {
        format!("/workspaces/{workspace}/dataAgents/{id}?hardDelete=true")
    } else {
        format!("/workspaces/{workspace}/dataAgents/{id}")
    };

    client.delete(&url).await?;

    let result = serde_json::json!({
        "id": id,
        "status": "deleted"
    });
    output::render_object(cli, &result, "id");
    Ok(())
}

/// Query a published data agent using the `OpenAI` Assistants protocol.
///
/// The data agent exposes an `OpenAI`-compatible endpoint at its published URL.
/// Flow: create assistant -> create thread -> post message -> create run -> poll -> read response.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn query(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    prompt: Option<&str>,
    published_url: Option<&str>,
    verbose: bool,
    stage: &str,
    timeout: u64,
) -> Result<()> {
    // Resolve prompt text: --prompt flag or stdin
    let prompt_text = if let Some(p) = prompt {
        p.to_string()
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Failed to read prompt from stdin: {e}"),
                "Use --prompt to provide the question directly, e.g.: fabio data-agent query --workspace <WS> --id <ID> --prompt \"What are the top 10 products?\"",
            )
        })?;
        if buf.trim().is_empty() {
            return Err(FabioError::invalid_input(
                "No prompt provided. Use --prompt or pipe text via stdin.",
            )
            .into());
        }
        buf
    };

    // Get the published URL: explicit flag, settings API, or constructed fallback.
    let resolved_url = if let Some(url) = published_url {
        client::validate_trusted_url(url, "--published-url")?;
        url.to_string()
    } else {
        let url = get_published_url(client, workspace, id).await?;
        // Validate API-returned URL to prevent token exfiltration via crafted settings
        client::validate_trusted_url(&url, "publishedUrl (from agent settings)")?;
        url
    };

    // Use the OpenAI Assistants protocol against the published URL
    let token = client.require_auth().await?;
    let max_wait = Duration::from_secs(timeout);
    let query_result = run_assistant_query(
        &resolved_url,
        &token,
        &prompt_text,
        verbose,
        max_wait,
        stage,
    )
    .await?;

    let mut result = serde_json::json!({
        "question": prompt_text.trim(),
        "answer": query_result.answer,
    });
    if let Some(steps) = query_result.steps {
        result["steps"] = steps;
    }
    output::render_object(cli, &result, "answer");
    Ok(())
}

/// Get the published URL of a data agent.
///
/// Strategy:
/// 1. Try `/dataAgents/{id}/settings` (V3 management plane, if enabled).
/// 2. Check the item properties for a `publishedUrl` field.
/// 3. Return an error explaining that the user must provide `--published-url`.
///
/// If the V3 settings endpoint is not available and no URL is found,
/// returns an error explaining that the user must provide `--published-url`.
async fn get_published_url(client: &FabricClient, workspace: &str, id: &str) -> Result<String> {
    // Attempt 1: Try the V3 settings endpoint (may not be enabled)
    let settings_path = format!("/workspaces/{workspace}/dataAgents/{id}/settings");
    if let Ok(settings) = client.get(&settings_path).await {
        if let Some(url) = settings
            .get("publishedUrl")
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())
        {
            return Ok(url.to_string());
        }
    }

    // Attempt 2: Check item properties
    let data = client
        .get(&format!("/workspaces/{workspace}/dataAgents/{id}"))
        .await?;

    if let Some(url) = data
        .get("properties")
        .and_then(|p| p.get("publishedUrl"))
        .and_then(Value::as_str)
        .filter(|u| !u.is_empty())
    {
        return Ok(url.to_string());
    }

    // No published URL found — the agent may not be published or V3 isn't enabled.
    Err(FabioError::with_hint(
        ErrorCode::ApiError,
        "Published URL not found. The data agent may not be published yet, or the V3 settings API is not enabled on this tenant.",
        format!(
            "Publish the agent with 'fabio data-agent publish', then provide the URL with --published-url. \
             The URL pattern is: https://api.fabric.microsoft.com/v1/workspaces/{workspace}/dataagents/{id}/aiassistant/openai"
        ),
    )
    .into())
}

/// Result of a data agent query, including the answer and optional execution steps.
struct QueryResult {
    answer: String,
    steps: Option<Value>,
}

/// Run a query against the data agent using the `OpenAI` Assistants API protocol.
async fn run_assistant_query(
    base_url: &str,
    token: &str,
    question: &str,
    verbose: bool,
    max_wait: Duration,
    stage: &str,
) -> Result<QueryResult> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(360))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| FabioError::with_hint(ErrorCode::NetworkError, e.to_string(), "Verify the data agent is published. Check status: fabio data-agent show --workspace <WS> --id <ID>. Publish if needed: fabio data-agent publish --workspace <WS> --id <ID>"))?;

    let auth_header = token;

    // Build stage-specific headers for internal workload API routing
    let _stage_header = match stage {
        "sandbox" | "draft" => "sandbox",
        _ => "production",
    };

    // Step 1: Create assistant + thread
    let assistant_id = create_assistant(&http, base_url, auth_header).await?;
    let thread_id = create_thread(&http, base_url, auth_header).await?;

    // Step 2: Post message and run
    post_message(&http, base_url, auth_header, &thread_id, question).await?;
    let run_id = create_run(&http, base_url, auth_header, &thread_id, &assistant_id).await?;

    // Step 3: Poll until complete
    poll_run_completion(&http, base_url, auth_header, &thread_id, &run_id, max_wait).await?;

    // Step 4 (optional): Retrieve run steps for verbose mode
    let steps = if verbose {
        Some(retrieve_run_steps(&http, base_url, auth_header, &thread_id, &run_id).await?)
    } else {
        None
    };

    // Step 5: Get response
    let response_text = retrieve_response(&http, base_url, auth_header, &thread_id).await?;

    // Step 6: Clean up thread (best effort)
    let _ = http
        .delete(format!(
            "{base_url}/threads/{thread_id}?api-version=2024-05-01-preview"
        ))
        .header("Authorization", auth_header)
        .send()
        .await;

    Ok(QueryResult {
        answer: response_text,
        steps,
    })
}

/// Create an assistant on the data agent endpoint.
async fn create_assistant(
    http: &reqwest::Client,
    base_url: &str,
    auth_header: &str,
) -> Result<String> {
    let resp = http
        .post(format!(
            "{base_url}/assistants?api-version=2024-05-01-preview"
        ))
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"model": "not used"}))
        .send()
        .await
        .map_err(|e| FabioError::with_hint(ErrorCode::NetworkError, format!("Create assistant: {e}"), "Verify the data agent is published. Check status: fabio data-agent show --workspace <WS> --id <ID>"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let retry_after = extract_retry_after(&resp);
        let text = resp.text().await.unwrap_or_default();
        return Err(enrich_query_error(
            status,
            &format!("Failed to create assistant: {text}"),
            base_url,
            retry_after.as_deref(),
        )
        .into());
    }
    let body: Value = resp.json().await.map_err(|e| {
        FabioError::with_hint(
            ErrorCode::ApiError,
            format!("Parse assistant response: {e}"),
            "Unexpected response format. This may indicate an API version mismatch.",
        )
    })?;
    Ok(body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string())
}

/// Create a thread on the data agent endpoint.
async fn create_thread(
    http: &reqwest::Client,
    base_url: &str,
    auth_header: &str,
) -> Result<String> {
    let resp = http
        .post(format!("{base_url}/threads?api-version=2024-05-01-preview"))
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| FabioError::with_hint(ErrorCode::NetworkError, format!("Create thread: {e}"), "Verify the data agent is published. Check status: fabio data-agent show --workspace <WS> --id <ID>"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let retry_after = extract_retry_after(&resp);
        let text = resp.text().await.unwrap_or_default();
        return Err(enrich_query_error(
            status,
            &format!("Failed to create thread: {text}"),
            base_url,
            retry_after.as_deref(),
        )
        .into());
    }
    let body: Value = resp.json().await.map_err(|e| {
        FabioError::with_hint(
            ErrorCode::ApiError,
            format!("Parse thread response: {e}"),
            "Unexpected response format. This may indicate an API version mismatch.",
        )
    })?;
    Ok(body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string())
}

/// Post a user message to an existing thread.
async fn post_message(
    http: &reqwest::Client,
    base_url: &str,
    auth_header: &str,
    thread_id: &str,
    question: &str,
) -> Result<()> {
    let resp = http
        .post(format!(
            "{base_url}/threads/{thread_id}/messages?api-version=2024-05-01-preview"
        ))
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "role": "user",
            "content": question
        }))
        .send()
        .await
        .map_err(|e| FabioError::with_hint(ErrorCode::NetworkError, format!("Post message: {e}"), "Verify the data agent is published. Check status: fabio data-agent show --workspace <WS> --id <ID>"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let retry_after = extract_retry_after(&resp);
        let text = resp.text().await.unwrap_or_default();
        return Err(enrich_query_error(
            status,
            &format!("Failed to post message: {text}"),
            base_url,
            retry_after.as_deref(),
        )
        .into());
    }
    Ok(())
}

/// Create a run on the thread and return the run ID.
async fn create_run(
    http: &reqwest::Client,
    base_url: &str,
    auth_header: &str,
    thread_id: &str,
    assistant_id: &str,
) -> Result<String> {
    let resp = http
        .post(format!(
            "{base_url}/threads/{thread_id}/runs?api-version=2024-05-01-preview"
        ))
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "assistant_id": assistant_id
        }))
        .send()
        .await
        .map_err(|e| FabioError::with_hint(ErrorCode::NetworkError, format!("Create run: {e}"), "Verify the data agent is published. Check status: fabio data-agent show --workspace <WS> --id <ID>"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let retry_after = extract_retry_after(&resp);
        let text = resp.text().await.unwrap_or_default();
        return Err(enrich_query_error(
            status,
            &format!("Failed to create run: {text}"),
            base_url,
            retry_after.as_deref(),
        )
        .into());
    }
    let body: Value = resp.json().await.map_err(|e| {
        FabioError::with_hint(
            ErrorCode::ApiError,
            format!("Parse run response: {e}"),
            "Unexpected response format. This may indicate an API version mismatch.",
        )
    })?;
    Ok(body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string())
}

/// Poll until the run reaches a terminal state.
async fn poll_run_completion(
    http: &reqwest::Client,
    base_url: &str,
    auth_header: &str,
    thread_id: &str,
    run_id: &str,
    max_wait: Duration,
) -> Result<()> {
    let start = std::time::Instant::now();
    let terminal_states = ["completed", "failed", "cancelled", "requires_action"];

    loop {
        if start.elapsed() > max_wait {
            return Err(FabioError::with_hint(
                ErrorCode::Timeout,
                "Data agent query timed out waiting for response",
                "The query exceeded the maximum wait time. Possible causes: \
                 (1) Spark cold start on small capacities can take 2-5 minutes. \
                 (2) Complex queries over large datasets take longer. \
                 (3) The Fabric capacity may be overloaded. \
                 Retry the query, or check capacity status in the Azure portal.",
            )
            .into());
        }

        sleep(QUERY_POLL_INTERVAL).await;

        let poll_resp = http
            .get(format!(
                "{base_url}/threads/{thread_id}/runs/{run_id}?api-version=2024-05-01-preview"
            ))
            .header("Authorization", auth_header)
            .send()
            .await
            .map_err(|e| FabioError::with_hint(ErrorCode::NetworkError, format!("Poll run: {e}"), "Verify the data agent is published. Check status: fabio data-agent show --workspace <WS> --id <ID>"))?;

        if !poll_resp.status().is_success() {
            let status = poll_resp.status().as_u16();
            let retry_after = extract_retry_after(&poll_resp);
            let text = poll_resp.text().await.unwrap_or_default();
            return Err(enrich_query_error(
                status,
                &format!("Failed to poll run status: {text}"),
                base_url,
                retry_after.as_deref(),
            )
            .into());
        }

        let run_state: Value = poll_resp.json().await.map_err(|e| {
            FabioError::with_hint(
                ErrorCode::ApiError,
                format!("Parse run poll response: {e}"),
                "Unexpected response format. This may indicate an API version mismatch.",
            )
        })?;
        let status = run_state
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("");

        if terminal_states.contains(&status) {
            if status != "completed" {
                let err_msg = run_state
                    .get("last_error")
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Data agent run did not complete successfully");
                let hint = match status {
                    "failed" => {
                        "The data agent run failed. Check: \
                        (1) Is the Fabric capacity active? \
                        (2) Does the agent have access to its configured data sources? \
                        (3) Are the lakehouse tables loaded and accessible? \
                        Inspect the agent definition: fabio data-agent get-definition -w <workspace> --id <id>"
                    }
                    "cancelled" => {
                        "The run was cancelled. This may happen if the capacity \
                        is under pressure or the query was interrupted. Retry the query."
                    }
                    "requires_action" => {
                        "The run requires additional action (tool approval). \
                        This is unexpected for data agent queries — check the agent configuration."
                    }
                    _ => "The run ended in an unexpected state. Retry the query.",
                };
                return Err(FabioError::with_hint(
                    ErrorCode::ApiError,
                    format!("Run status '{status}': {err_msg}"),
                    hint,
                )
                .into());
            }
            return Ok(());
        }
    }
}

/// Retrieve the assistant's response from the thread messages.
async fn retrieve_response(
    http: &reqwest::Client,
    base_url: &str,
    auth_header: &str,
    thread_id: &str,
) -> Result<String> {
    let resp = http
        .get(format!(
            "{base_url}/threads/{thread_id}/messages?api-version=2024-05-01-preview&order=asc"
        ))
        .header("Authorization", auth_header)
        .send()
        .await
        .map_err(|e| FabioError::with_hint(ErrorCode::NetworkError, format!("Retrieve messages: {e}"), "Verify the data agent is published. Check status: fabio data-agent show --workspace <WS> --id <ID>"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let retry_after = extract_retry_after(&resp);
        let text = resp.text().await.unwrap_or_default();
        return Err(enrich_query_error(
            status,
            &format!("Failed to retrieve messages: {text}"),
            base_url,
            retry_after.as_deref(),
        )
        .into());
    }

    let messages: Value = resp.json().await.map_err(|e| {
        FabioError::with_hint(
            ErrorCode::ApiError,
            format!("Parse messages response: {e}"),
            "Unexpected response format. This may indicate an API version mismatch.",
        )
    })?;

    // Extract the assistant's response (last message with role=assistant)
    let text = messages
        .get("data")
        .and_then(Value::as_array)
        .and_then(|arr| {
            arr.iter()
                .rev()
                .find(|m| m.get("role").and_then(Value::as_str) == Some("assistant"))
        })
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("(No response from data agent)");

    Ok(text.to_string())
}

/// Retrieve the run steps to show execution details (SQL queries, tool calls, etc.).
///
/// The `OpenAI` Assistants API exposes run steps at:
/// `GET /threads/{thread_id}/runs/{run_id}/steps`
///
/// Each step has a `step_details` field that may contain:
/// - `type: "tool_calls"` with tool call details (SQL queries, function calls)
/// - `type: "message_creation"` for the final response generation
async fn retrieve_run_steps(
    http: &reqwest::Client,
    base_url: &str,
    auth_header: &str,
    thread_id: &str,
    run_id: &str,
) -> Result<Value> {
    let resp = http
        .get(format!(
            "{base_url}/threads/{thread_id}/runs/{run_id}/steps?api-version=2024-05-01-preview"
        ))
        .header("Authorization", auth_header)
        .send()
        .await
        .map_err(|e| {
            FabioError::with_hint(ErrorCode::NetworkError, format!("Retrieve run steps: {e}"), "Verify the data agent is published. Check status: fabio data-agent show --workspace <WS> --id <ID>")
        })?;

    if !resp.status().is_success() {
        // Non-fatal: if steps endpoint is not available, return empty array
        return Ok(serde_json::json!([]));
    }

    let body: Value = resp.json().await.map_err(|e| {
        FabioError::with_hint(
            ErrorCode::ApiError,
            format!("Parse run steps response: {e}"),
            "Unexpected response format. This may indicate an API version mismatch.",
        )
    })?;

    // Extract meaningful step details
    let steps = body
        .get("data")
        .and_then(Value::as_array)
        .map(|steps_arr| {
            steps_arr
                .iter()
                .filter_map(|step| {
                    let step_type = step.get("type").and_then(Value::as_str)?;
                    let step_details = step.get("step_details")?;
                    let status = step
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");

                    match step_type {
                        "tool_calls" => {
                            let tool_calls = extract_tool_calls(step_details);
                            if tool_calls.is_empty() {
                                None
                            } else {
                                Some(serde_json::json!({
                                    "type": "tool_calls",
                                    "status": status,
                                    "tool_calls": tool_calls
                                }))
                            }
                        }
                        "message_creation" => Some(serde_json::json!({
                            "type": "message_creation",
                            "status": status,
                        })),
                        _ => Some(serde_json::json!({
                            "type": step_type,
                            "status": status,
                            "details": step_details
                        })),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(Value::Array(steps))
}

/// Extract tool call details from a step's `step_details` field.
/// Returns a vec of structured objects with type, name, input, and output.
fn extract_tool_calls(step_details: &Value) -> Vec<Value> {
    let Some(tool_calls) = step_details.get("tool_calls").and_then(Value::as_array) else {
        return vec![];
    };

    tool_calls
        .iter()
        .map(|tc| {
            let tc_type = tc.get("type").and_then(Value::as_str).unwrap_or("unknown");
            match tc_type {
                "code_interpreter" => {
                    let input = tc
                        .get("code_interpreter")
                        .and_then(|ci| ci.get("input"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let outputs = tc
                        .get("code_interpreter")
                        .and_then(|ci| ci.get("outputs"))
                        .cloned()
                        .unwrap_or(Value::Array(vec![]));
                    serde_json::json!({
                        "type": "code_interpreter",
                        "input": input,
                        "outputs": outputs
                    })
                }
                "function" => {
                    let name = tc
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let arguments = tc
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let output = tc
                        .get("function")
                        .and_then(|f| f.get("output"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    serde_json::json!({
                        "type": "function",
                        "name": name,
                        "arguments": arguments,
                        "output": output
                    })
                }
                // Fabric data agents may use custom tool types (e.g., SQL execution)
                _ => {
                    serde_json::json!({
                        "type": tc_type,
                        "details": tc
                    })
                }
            }
        })
        .collect()
}

// ─── Error Enrichment ────────────────────────────────────────────────────────

/// Extract the `Retry-After` header value from an HTTP response (seconds or date).
fn extract_retry_after(resp: &reqwest::Response) -> Option<String> {
    resp.headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

/// Enrich data agent query errors with actionable hints for common failures.
///
/// Intercepts HTTP status codes and known error patterns from the `OpenAI`
/// Assistants-compatible endpoint to guide agents toward self-correction.
fn enrich_query_error(
    status: u16,
    message: &str,
    base_url: &str,
    retry_after: Option<&str>,
) -> FabioError {
    let msg_lower = message.to_lowercase();

    // 404: The published URL is wrong or agent isn't published
    if status == 404 {
        return FabioError::with_hint(
            ErrorCode::NotFound,
            message.to_string(),
            format!(
                "The data agent endpoint returned 404. Possible causes: \
                 (1) The agent has not been published from the Fabric portal. \
                 (2) The --published-url is incorrect. \
                 Expected URL pattern: https://api.fabric.microsoft.com/v1/workspaces/{{workspace}}/dataagents/{{agentId}}/aiassistant/openai \
                 Current URL: {base_url}"
            ),
        );
    }

    // 401/403: Token or permission issue
    if status == 401 || status == 403 {
        return FabioError::with_hint(
            if status == 401 {
                ErrorCode::AuthRequired
            } else {
                ErrorCode::Forbidden
            },
            message.to_string(),
            "Authentication failed for the data agent endpoint. Ensure: \
             (1) You have at least Viewer role on the workspace. \
             (2) Your token is valid (re-run 'fabio auth login'). \
             (3) The data agent has been published and you have access to it."
                .to_string(),
        );
    }

    // 429: Rate limited — include Retry-After value
    if status == 429 {
        let hint = retry_after.map_or_else(
            || {
                "Rate-limited by the data agent endpoint. Wait at least 10 seconds \
                 before retrying. If this persists, the Fabric capacity may be under \
                 heavy load."
                    .to_string()
            },
            |seconds| {
                format!(
                    "Rate-limited by the data agent endpoint. Retry after {seconds} seconds. \
                     Do NOT retry before this time. If this persists, the Fabric capacity may \
                     be under heavy load."
                )
            },
        );
        return FabioError::with_hint(ErrorCode::RateLimited, message.to_string(), hint);
    }

    // Run failed or cancelled
    if msg_lower.contains("failed") || msg_lower.contains("cancelled") {
        return FabioError::with_hint(
            ErrorCode::ApiError,
            message.to_string(),
            "The data agent run failed. Possible causes: \
             (1) The data source (lakehouse/warehouse) is unavailable or the capacity is paused. \
             (2) The query references tables/columns not configured in the agent's data sources. \
             (3) The agent's AI instructions are misconfigured. \
             Check the agent definition with: fabio data-agent get-definition -w <workspace> --id <id>"
                .to_string(),
        );
    }

    // Timeout
    if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
        return FabioError::with_hint(
            ErrorCode::Timeout,
            message.to_string(),
            "The data agent query timed out. This may happen on first use due to Spark cold start \
             (2-5 minutes on small capacities). Retry the query, or check if the Fabric capacity \
             is active and not overloaded."
                .to_string(),
        );
    }

    // Default: return error without hint
    FabioError::from_status(status, message)
}

// ─── Validation ──────────────────────────────────────────────────────────────

/// Validate datasource element IDs in a definition before sending to the API.
///
/// Elements with `id: null` or empty strings will cause the data agent to show
/// "This table has been deleted or you don't have permission to view it" in the
/// portal UI. IDs must follow the dot-path convention:
/// - Schema: `"dbo"`
/// - Table: `"dbo.table_name"`
/// - Column: `"dbo.table_name.column_name"`
fn validate_datasource_elements(body: &Value) -> Result<()> {
    let parts = body
        .get("definition")
        .or(Some(body))
        .and_then(|d| d.get("definition").or(Some(d)))
        .and_then(|d| d.get("parts"))
        .and_then(Value::as_array);

    let Some(parts) = parts else {
        return Ok(());
    };

    for part in parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if !path.contains("datasource.json") {
            continue;
        }

        let Some(payload_str) = part.get("payload").and_then(Value::as_str) else {
            continue;
        };

        // Decode base64 payload
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(payload_str)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok());

        let Some(json_str) = decoded else {
            continue;
        };

        let Ok(datasource) = serde_json::from_str::<Value>(&json_str) else {
            continue;
        };

        let Some(elements) = datasource.get("elements").and_then(Value::as_array) else {
            continue;
        };

        validate_elements_recursive(elements, path)?;
    }

    Ok(())
}

/// Recursively validate that all elements have non-null, non-empty IDs.
fn validate_elements_recursive(elements: &[Value], datasource_path: &str) -> Result<()> {
    for element in elements {
        let id = element.get("id");
        let display_name = element
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        let element_type = element
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");

        // Check for null or empty ID
        let id_is_missing = match id {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty(),
            _ => false,
        };

        if id_is_missing {
            let hint = if element_type.contains(".schema") {
                "schema elements should use the schema name as id, e.g. \"dbo\"".to_string()
            } else if element_type.contains(".table") {
                format!(
                    "table elements should use \"schema.table\" format, e.g. \"dbo.{display_name}\""
                )
            } else if element_type.contains(".column") {
                format!(
                    "column elements should use \"schema.table.column\" format, e.g. \"dbo.table_name.{display_name}\""
                )
            } else {
                "elements require a non-null id following dot-path convention".to_string()
            };

            return Err(FabioError::new(
                ErrorCode::InvalidInput,
                format!(
                    "Element '{display_name}' (type: {element_type}) in '{datasource_path}' has a \
                     null or empty 'id'. {hint}. Without valid IDs the portal will show \
                     'This table has been deleted or you don't have permission to view it'."
                ),
            )
            .into());
        }

        // Recurse into children
        if let Some(children) = element.get("children").and_then(Value::as_array) {
            validate_elements_recursive(children, datasource_path)?;
        }
    }

    Ok(())
}

// ─── Configuration ───────────────────────────────────────────────────────────

/// Get agent configuration by parsing the definition's `stage_config.json`.
async fn get_config(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let definition_resp = client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await?;

    let parts = definition_resp
        .get("definition")
        .and_then(|d| d.get("parts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut config = serde_json::json!({
        "instructions": null,
        "enablePreviewRuntime": false,
        "dataSources": [],
    });

    for part in &parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");

        if path == "Files/Config/draft/stage_config.json" {
            if let Some(decoded) = decode_part_payload(payload) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&decoded) {
                    config["instructions"] = parsed
                        .get("aiInstructions")
                        .or_else(|| parsed.get("additionalInstructions"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    if let Some(experimental) = parsed.get("experimental") {
                        let enabled = experimental
                            .get("enableExperimentalFeatures")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        config["enablePreviewRuntime"] = Value::Bool(enabled);
                    }
                }
            }
        }

        // Collect datasource names
        if path.starts_with("Files/Config/draft/") && path.ends_with("/datasource.json") {
            if let Some(decoded) = decode_part_payload(payload) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&decoded) {
                    let ds_info = serde_json::json!({
                        "displayName": parsed.get("displayName").or_else(|| parsed.get("display_name")),
                        "type": parsed.get("type"),
                        "artifactId": parsed.get("artifactId").or_else(|| parsed.get("id")),
                    });
                    if let Some(arr) = config["dataSources"].as_array_mut() {
                        arr.push(ds_info);
                    }
                }
            }
        }
    }

    output::render_object(cli, &config, "instructions");
    Ok(())
}

/// Update agent configuration by modifying `stage_config.json` in the definition.
#[allow(
    clippy::fn_params_excessive_bools,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]
async fn update_config(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    instructions: Option<&str>,
    instructions_file: Option<&str>,
    enable_preview_runtime: bool,
    disable_preview_runtime: bool,
) -> Result<()> {
    // Resolve instructions from --instructions or --instructions-file
    let resolved_instructions = match (instructions, instructions_file) {
        (Some(instr), _) => Some(instr.to_string()),
        (_, Some(path)) => {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read instructions file '{path}': {e}"))?;
            Some(content)
        }
        _ => None,
    };

    if resolved_instructions.is_none() && !enable_preview_runtime && !disable_preview_runtime {
        return Err(FabioError::invalid_input(
            "At least one of --instructions, --instructions-file, --enable-preview-runtime, or --disable-preview-runtime must be provided",
        )
        .into());
    }

    if output::dry_run_guard(
        cli,
        "data-agent update-config",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "instructions": resolved_instructions.as_deref().map(|s| if s.len() > 100 { format!("{}...", &s[..100]) } else { s.to_string() }),
            "instructionsFile": instructions_file,
            "enablePreviewRuntime": enable_preview_runtime,
            "disablePreviewRuntime": disable_preview_runtime,
        }),
    ) {
        return Ok(());
    }

    // Fetch current definition
    let definition_resp = client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await?;

    let parts = definition_resp
        .get("definition")
        .and_then(|d| d.get("parts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    // Find and modify stage_config.json
    let mut new_parts: Vec<Value> = Vec::new();
    let mut found_config = false;

    for part in &parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if path == "Files/Config/draft/stage_config.json" {
            found_config = true;
            let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");
            let mut config = decode_part_payload(payload)
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| serde_json::json!({}));

            if let Some(instr) = &resolved_instructions {
                config["aiInstructions"] = Value::String(instr.clone());
            }
            if enable_preview_runtime || disable_preview_runtime {
                let experimental = config.as_object_mut().map(|o| {
                    o.entry("experimental")
                        .or_insert_with(|| serde_json::json!({}))
                });
                if let Some(exp) = experimental {
                    if let Some(obj) = exp.as_object_mut() {
                        obj.insert(
                            "enableExperimentalFeatures".to_string(),
                            Value::Bool(enable_preview_runtime),
                        );
                    }
                }
            }

            let encoded =
                base64::engine::general_purpose::STANDARD.encode(config.to_string().as_bytes());
            new_parts.push(serde_json::json!({
                "path": path,
                "payload": encoded,
                "payloadType": "InlineBase64"
            }));
        } else {
            new_parts.push(part.clone());
        }
    }

    // If no stage_config exists yet, create one
    if !found_config {
        let mut config = serde_json::json!({
            "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/stageConfiguration/1.0.0/schema.json"
        });
        if let Some(instr) = &resolved_instructions {
            config["aiInstructions"] = Value::String(instr.clone());
        }
        if enable_preview_runtime {
            config["experimental"] = serde_json::json!({"enableExperimentalFeatures": true});
        }
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(config.to_string().as_bytes());
        new_parts.push(serde_json::json!({
            "path": "Files/Config/draft/stage_config.json",
            "payload": encoded,
            "payloadType": "InlineBase64"
        }));
    }

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "id": id,
        "status": "config_updated",
        "instructions": resolved_instructions.as_deref(),
        "enablePreviewRuntime": enable_preview_runtime,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

// ─── Datasource Management ───────────────────────────────────────────────────

/// List configured data sources by parsing the agent's definition.
async fn list_datasources(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let parts = get_definition_parts(client, workspace, id).await?;
    let datasources = extract_datasources_from_parts(&parts);

    output::render_list_with_token(
        cli,
        &datasources,
        &["displayName", "type", "artifactId", "workspaceId"],
        &["NAME", "TYPE", "ARTIFACT ID", "WORKSPACE ID"],
        "displayName",
        None,
    );
    Ok(())
}

/// Show details of a specific data source from the definition.
async fn show_datasource(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
) -> Result<()> {
    let parts = get_definition_parts(client, workspace, id).await?;
    let datasources = extract_datasources_from_parts(&parts);

    let ds = datasources
        .iter()
        .find(|d| {
            let name = d.get("displayName").and_then(Value::as_str).unwrap_or("");
            let ds_id = d.get("artifactId").and_then(Value::as_str).unwrap_or("");
            name.eq_ignore_ascii_case(datasource) || ds_id == datasource
        })
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::NotFound,
                format!("Data source '{datasource}' not found"),
                "List available data sources: fabio data-agent list-datasources -w <workspace> --id <id>",
            )
        })?;

    output::render_object(cli, ds, "displayName");
    Ok(())
}

/// Add a data source to the agent's definition.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn add_datasource(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    artifact: &str,
    artifact_workspace: Option<&str>,
    artifact_type: Option<&str>,
    instructions: Option<&str>,
) -> Result<()> {
    let ds_workspace = artifact_workspace.unwrap_or(workspace);

    // Auto-detect artifact type if not provided
    let resolved_type = if let Some(t) = artifact_type {
        t.to_string()
    } else {
        // Try to find the artifact in the workspace items list
        let items = client
            .get_list(
                &format!("/workspaces/{ds_workspace}/items"),
                "value",
                true,
                None,
            )
            .await?;

        let found = items.items.iter().find(|item| {
            let item_name = item
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or("");
            let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");
            item_name.eq_ignore_ascii_case(artifact) || item_id == artifact
        });

        match found {
            Some(item) => item
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            None => {
                return Err(FabioError::with_hint(
                    ErrorCode::NotFound,
                    format!("Artifact '{artifact}' not found in workspace '{ds_workspace}'"),
                    "Specify the artifact type with --artifact-type, or check the workspace items: fabio item list -w <workspace>",
                ).into());
            }
        }
    };

    // Map Fabric item type to data agent datasource type
    let ds_type = map_item_type_to_datasource_type(&resolved_type)?;

    // Resolve artifact ID
    let items = client
        .get_list(
            &format!("/workspaces/{ds_workspace}/items?type={resolved_type}"),
            "value",
            true,
            None,
        )
        .await?;

    let artifact_item = items
        .items
        .iter()
        .find(|item| {
            let item_name = item.get("displayName").and_then(Value::as_str).unwrap_or("");
            let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");
            item_name.eq_ignore_ascii_case(artifact) || item_id == artifact
        })
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::NotFound,
                format!("Artifact '{artifact}' of type '{resolved_type}' not found"),
                format!("List items of this type: fabio item list -w {ds_workspace} --type {resolved_type}"),
            )
        })?;

    let artifact_id = artifact_item
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let artifact_name = artifact_item
        .get("displayName")
        .and_then(Value::as_str)
        .unwrap_or(artifact);

    if output::dry_run_guard(
        cli,
        "data-agent add-datasource",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "artifactId": artifact_id,
            "artifactName": artifact_name,
            "artifactWorkspace": ds_workspace,
            "datasourceType": ds_type,
        }),
    ) {
        return Ok(());
    }

    // Build datasource definition
    let mut datasource_json = serde_json::json!({
        "artifactId": artifact_id,
        "workspaceId": ds_workspace,
        "displayName": artifact_name,
        "type": ds_type,
    });
    if let Some(instr) = instructions {
        datasource_json["dataSourceInstructions"] = Value::String(instr.to_string());
    }

    // Fetch current definition and append the new datasource part
    let parts = get_definition_parts(client, workspace, id).await?;
    let mut new_parts = parts;

    // Determine path prefix based on type
    let path_prefix = format!("Files/Config/draft/{ds_type}-{artifact_name}");
    let ds_encoded = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_string(&datasource_json)?.as_bytes());

    new_parts.push(serde_json::json!({
        "path": format!("{path_prefix}/datasource.json"),
        "payload": ds_encoded,
        "payloadType": "InlineBase64"
    }));

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "status": "datasource_added",
        "artifactId": artifact_id,
        "displayName": artifact_name,
        "type": ds_type,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Remove a data source from the agent's definition.
async fn remove_datasource(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent remove-datasource",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;

    // Find datasource parts matching the name or ID
    let new_parts: Vec<Value> = parts
        .iter()
        .filter(|part| {
            let path = part.get("path").and_then(Value::as_str).unwrap_or("");
            if !path.starts_with("Files/Config/draft/") || !path.contains('/') {
                return true; // keep non-datasource parts
            }
            // Check if this part belongs to the datasource being removed
            if let Some(payload) = part.get("payload").and_then(Value::as_str) {
                if path.ends_with("/datasource.json") {
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
                                return false; // remove this datasource part
                            }
                        }
                    }
                }
            }
            // Also remove associated fewshots file in the same directory
            if path.contains(datasource) {
                return false;
            }
            true
        })
        .cloned()
        .collect();

    if new_parts.len() == parts.len() {
        return Err(FabioError::with_hint(
            ErrorCode::NotFound,
            format!("Data source '{datasource}' not found in agent definition"),
            "List available data sources: fabio data-agent list-datasources -w <workspace> --id <id>",
        )
        .into());
    }

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "id": id,
        "status": "datasource_removed",
        "datasource": datasource,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

// ─── Few-shot Management ─────────────────────────────────────────────────────

/// List few-shot examples for a specific data source.
async fn list_fewshots(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
) -> Result<()> {
    let parts = get_definition_parts(client, workspace, id).await?;
    let fewshots = extract_fewshots_for_datasource(&parts, datasource)?;

    output::render_list_with_token(
        cli,
        &fewshots,
        &["id", "question", "query"],
        &["ID", "QUESTION", "QUERY"],
        "id",
        None,
    );
    Ok(())
}

/// Add a few-shot example to a data source.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn add_fewshot(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
    question: &str,
    query_text: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent add-fewshot",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
            "question": question,
            "query": query_text,
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;

    // Find the fewshots part for this datasource and the datasource directory prefix
    let ds_dir = find_datasource_dir(&parts, datasource)?;
    let fewshots_path = format!("{ds_dir}/fewshots.json");

    // Find existing fewshots content or create empty
    let existing_payload = parts.iter().find_map(|part| {
        let path = part.get("path").and_then(Value::as_str)?;
        if path == fewshots_path {
            part.get("payload")
                .and_then(Value::as_str)
                .map(String::from)
        } else {
            None
        }
    });

    let mut fewshots_data = existing_payload.as_ref().map_or_else(
        || serde_json::json!({
            "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/fewShots/1.0.0/schema.json",
            "fewShots": []
        }),
        |payload| {
            decode_part_payload(payload)
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| serde_json::json!({"fewShots": []}))
        },
    );

    // Add the new fewshot (with duplicate detection)
    let fewshots_arr = fewshots_data
        .get_mut("fewShots")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "Invalid fewshots structure in definition",
            )
        })?;

    // Check for duplicates (case-insensitive)
    let question_lower = question.to_lowercase();
    let has_duplicate = fewshots_arr.iter().any(|f| {
        f.get("question")
            .or_else(|| f.get("Question"))
            .and_then(Value::as_str)
            .is_some_and(|q| q.to_lowercase() == question_lower)
    });

    let saved_question = if has_duplicate {
        // Find next available suffix
        let mut suffix = 1;
        loop {
            let candidate = format!("{question} [{suffix}]").to_lowercase();
            let exists = fewshots_arr.iter().any(|f| {
                f.get("question")
                    .or_else(|| f.get("Question"))
                    .and_then(Value::as_str)
                    .is_some_and(|q| q.to_lowercase() == candidate)
            });
            if !exists {
                break;
            }
            suffix += 1;
        }
        format!("{question} [{suffix}]")
    } else {
        question.to_string()
    };

    let new_id = uuid::Uuid::new_v4().to_string();
    fewshots_arr.push(serde_json::json!({
        "id": new_id,
        "question": saved_question,
        "query": query_text,
    }));

    // Rebuild definition parts
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_string(&fewshots_data)?.as_bytes());

    let mut new_parts: Vec<Value> = parts
        .iter()
        .filter(|p| p.get("path").and_then(Value::as_str) != Some(&fewshots_path))
        .cloned()
        .collect();
    new_parts.push(serde_json::json!({
        "path": fewshots_path,
        "payload": encoded,
        "payloadType": "InlineBase64"
    }));

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "status": "fewshot_added",
        "id": new_id,
        "question": saved_question,
        "query": query_text,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Remove a few-shot example by ID.
async fn remove_fewshot(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
    fewshot_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent remove-fewshot",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
            "fewshotId": fewshot_id,
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;
    let ds_dir = find_datasource_dir(&parts, datasource)?;
    let fewshots_path = format!("{ds_dir}/fewshots.json");

    let payload = parts
        .iter()
        .find_map(|part| {
            let path = part.get("path").and_then(Value::as_str)?;
            if path == fewshots_path {
                part.get("payload").and_then(Value::as_str).map(String::from)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::NotFound,
                format!("No fewshots found for data source '{datasource}'"),
                "Add fewshots first: fabio data-agent add-fewshot -w <workspace> --id <id> --datasource <ds> --question '...' --answer '...'",
            )
        })?;

    let mut fewshots_data = decode_part_payload(&payload)
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| serde_json::json!({"fewShots": []}));

    let arr = fewshots_data
        .get_mut("fewShots")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| FabioError::new(ErrorCode::ApiError, "Invalid fewshots structure"))?;

    let original_len = arr.len();
    arr.retain(|f| {
        f.get("id")
            .and_then(Value::as_str)
            .is_none_or(|fid| fid != fewshot_id)
    });

    if arr.len() == original_len {
        return Err(FabioError::with_hint(
            ErrorCode::NotFound,
            format!("Few-shot '{fewshot_id}' not found"),
            "List fewshots: fabio data-agent list-fewshots -w <workspace> --id <id> --datasource <ds>",
        )
        .into());
    }

    let encoded = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_string(&fewshots_data)?.as_bytes());

    let new_parts: Vec<Value> = parts
        .iter()
        .map(|p| {
            if p.get("path").and_then(Value::as_str) == Some(&fewshots_path) {
                serde_json::json!({
                    "path": fewshots_path,
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                })
            } else {
                p.clone()
            }
        })
        .collect();

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "id": id,
        "status": "fewshot_removed",
        "fewshotId": fewshot_id,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Bulk upload few-shot examples from a JSON file.
#[allow(clippy::too_many_lines)]
async fn upload_fewshots(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
    file: &str,
) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{file}': {e}"))?;

    // Detect format by file extension
    let ext = std::path::Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let items: Vec<Value> = if ext == "csv" || ext == "tsv" {
        parse_fewshots_csv(&content, file)?
    } else {
        serde_json::from_str(&content).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in '{file}': {e}"),
                r#"Expected JSON format: [{"question":"...","query":"..."}] or use .csv file with question,query columns"#,
            )
        })?
    };

    if items.is_empty() {
        return Err(
            FabioError::invalid_input("File contains no few-shot examples (empty array)").into(),
        );
    }

    // Validate all entries have question + query
    for (i, item) in items.iter().enumerate() {
        if item.get("question").and_then(Value::as_str).is_none() {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Item {i} is missing 'question' field"),
                r#"Each item must have: {{"question":"...", "query":"..."}}"#,
            )
            .into());
        }
        if item.get("query").and_then(Value::as_str).is_none() {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Item {i} is missing 'query' field"),
                r#"Each item must have: {{"question":"...", "query":"..."}}"#,
            )
            .into());
        }
    }

    if output::dry_run_guard(
        cli,
        "data-agent upload-fewshots",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
            "file": file,
            "count": items.len(),
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;
    let ds_dir = find_datasource_dir(&parts, datasource)?;
    let fewshots_path = format!("{ds_dir}/fewshots.json");

    // Load existing fewshots
    let existing_payload = parts.iter().find_map(|part| {
        let path = part.get("path").and_then(Value::as_str)?;
        if path == fewshots_path {
            part.get("payload")
                .and_then(Value::as_str)
                .map(String::from)
        } else {
            None
        }
    });

    let mut fewshots_data = existing_payload.as_ref().map_or_else(
        || {
            serde_json::json!({
                "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/fewShots/1.0.0/schema.json",
                "fewShots": []
            })
        },
        |payload| {
            decode_part_payload(payload)
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| serde_json::json!({"fewShots": []}))
        },
    );

    let fewshots_arr = fewshots_data
        .get_mut("fewShots")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "Invalid fewshots structure in definition",
            )
        })?;

    // Build set of existing questions for duplicate detection
    let mut existing_questions: std::collections::HashSet<String> = fewshots_arr
        .iter()
        .filter_map(|f| {
            f.get("question")
                .or_else(|| f.get("Question"))
                .and_then(Value::as_str)
                .map(str::to_lowercase)
        })
        .collect();

    let mut added = 0;
    let mut renamed = 0;

    for item in &items {
        let question = item
            .get("question")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let query_text = item
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or_default();

        let mut saved_question = question.to_string();
        if existing_questions.contains(&saved_question.to_lowercase()) {
            let mut suffix = 1;
            loop {
                let candidate = format!("{question} [{suffix}]").to_lowercase();
                if !existing_questions.contains(&candidate) {
                    break;
                }
                suffix += 1;
            }
            saved_question = format!("{question} [{suffix}]");
            renamed += 1;
        }

        existing_questions.insert(saved_question.to_lowercase());
        fewshots_arr.push(serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "question": saved_question,
            "query": query_text,
        }));
        added += 1;
    }

    let total = fewshots_arr.len();

    // Update definition (fewshots_arr borrow ends here)
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_string(&fewshots_data)?.as_bytes());

    let new_parts: Vec<Value> = parts
        .iter()
        .filter(|p| p.get("path").and_then(Value::as_str) != Some(&fewshots_path))
        .cloned()
        .chain(std::iter::once(serde_json::json!({
            "path": fewshots_path,
            "payload": encoded,
            "payloadType": "InlineBase64"
        })))
        .collect();

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "status": "fewshots_uploaded",
        "added": added,
        "renamed": renamed,
        "total": total,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Select or unselect tables in a data source.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn select_tables(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
    tables: Option<&str>,
    all_tables: bool,
    unselect: bool,
) -> Result<()> {
    if tables.is_none() && !all_tables {
        return Err(
            FabioError::invalid_input("Either --tables or --all-tables must be provided").into(),
        );
    }

    if output::dry_run_guard(
        cli,
        "data-agent select-tables",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
            "tables": tables,
            "allTables": all_tables,
            "unselect": unselect,
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;
    let ds_dir = find_datasource_dir(&parts, datasource)?;
    let ds_path = format!("{ds_dir}/datasource.json");

    // Find and parse the datasource
    let ds_payload = parts
        .iter()
        .find_map(|part| {
            let path = part.get("path").and_then(Value::as_str)?;
            if path == ds_path {
                part.get("payload")
                    .and_then(Value::as_str)
                    .map(String::from)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::NotFound,
                format!("Datasource file not found at '{ds_path}'"),
            )
        })?;

    let mut ds_json: Value = decode_part_payload(&ds_payload)
        .and_then(|s| serde_json::from_str(&s).ok())
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "Failed to decode datasource definition",
            )
        })?;

    let table_names: Vec<&str> = tables
        .map(|t| t.split(',').map(str::trim).collect())
        .unwrap_or_default();
    let target_selected = !unselect;

    // Recursively set is_selected on matching table elements
    let modified = ds_json
        .get_mut("elements")
        .and_then(Value::as_array_mut)
        .map_or(0, |elements| {
            set_table_selection(elements, &table_names, all_tables, target_selected)
        });

    if modified == 0 && !all_tables {
        return Err(FabioError::with_hint(
            ErrorCode::NotFound,
            format!("No matching tables found: {}", table_names.join(", ")),
            "List available tables: fabio data-agent show-datasource -w <workspace> --id <id> --datasource <ds>",
        )
        .into());
    }

    // Re-encode and update definition
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_string(&ds_json)?.as_bytes());

    let new_parts: Vec<Value> = parts
        .iter()
        .map(|p| {
            if p.get("path").and_then(Value::as_str) == Some(&ds_path) {
                serde_json::json!({
                    "path": ds_path,
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                })
            } else {
                p.clone()
            }
        })
        .collect();

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "status": if unselect { "tables_unselected" } else { "tables_selected" },
        "modified": modified,
        "allTables": all_tables,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Recursively set `is_selected` on table elements matching the given names.
/// Returns the number of elements modified.
fn set_table_selection(
    elements: &mut [Value],
    table_names: &[&str],
    all_tables: bool,
    selected: bool,
) -> usize {
    let selectable_types = [
        "semantic_model.table",
        "lakehouse_tables.table",
        "warehouse_tables.table",
        "kusto.table",
        "mirrored_database.table",
        "sql_database.table",
    ];

    let mut count = 0;
    for elem in elements.iter_mut() {
        let elem_type = elem.get("type").and_then(Value::as_str).unwrap_or_default();
        let display_name = elem
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or_default();

        // Check if this is a selectable table element
        if selectable_types.contains(&elem_type) {
            let should_modify = all_tables
                || table_names
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(display_name));
            if should_modify {
                elem["is_selected"] = Value::Bool(selected);
                count += 1;
            }
        }

        // Recurse into children
        if let Some(children) = elem.get_mut("children").and_then(Value::as_array_mut) {
            count += set_table_selection(children, table_names, all_tables, selected);
        }
    }
    count
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Get definition parts from the API.
async fn get_definition_parts(
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
fn decode_part_payload(payload: &str) -> Option<String> {
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

/// Extract datasource information from definition parts.
fn extract_datasources_from_parts(parts: &[Value]) -> Vec<Value> {
    let mut datasources = Vec::new();
    for part in parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if path.starts_with("Files/Config/draft/") && path.ends_with("/datasource.json") {
            let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");
            if let Some(decoded) = decode_part_payload(payload) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&decoded) {
                    datasources.push(parsed);
                }
            }
        }
    }
    datasources
}

/// Find the directory path for a named datasource in definition parts.
fn find_datasource_dir(parts: &[Value], datasource: &str) -> Result<String> {
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

/// Extract few-shot examples for a specific data source.
fn extract_fewshots_for_datasource(parts: &[Value], datasource: &str) -> Result<Vec<Value>> {
    let ds_dir = find_datasource_dir(parts, datasource)?;
    let fewshots_path = format!("{ds_dir}/fewshots.json");

    for part in parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if path == fewshots_path {
            let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");
            if let Some(decoded) = decode_part_payload(payload) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&decoded) {
                    return Ok(parsed
                        .get("fewShots")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default());
                }
            }
        }
    }
    Ok(Vec::new())
}

/// Parse few-shot examples from a CSV/TSV file.
///
/// Expects columns named `question` and `query` (case-insensitive headers).
/// TSV is auto-detected from `.tsv` extension.
fn parse_fewshots_csv(content: &str, file: &str) -> Result<Vec<Value>> {
    let ext = std::path::Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let delimiter = if ext == "tsv" { b'\t' } else { b',' };

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    // Find column indices for "question" and "query" (case-insensitive)
    let headers = reader.headers().map_err(|e| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("Failed to parse CSV headers in '{file}': {e}"),
            "CSV must have a header row with 'question' and 'query' columns",
        )
    })?;

    let question_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("question"));
    let query_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("query") || h.eq_ignore_ascii_case("answer"));

    let question_idx = question_idx.ok_or_else(|| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("CSV file '{file}' is missing a 'question' column header"),
            format!(
                "Found columns: {}. Expected: question,query",
                headers.iter().collect::<Vec<_>>().join(", ")
            ),
        )
    })?;
    let query_idx = query_idx.ok_or_else(|| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("CSV file '{file}' is missing a 'query' (or 'answer') column header"),
            format!(
                "Found columns: {}. Expected: question,query",
                headers.iter().collect::<Vec<_>>().join(", ")
            ),
        )
    })?;

    let mut items = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record.map_err(|e| {
            FabioError::new(
                ErrorCode::InvalidInput,
                format!("Failed to parse CSV row {i} in '{file}': {e}"),
            )
        })?;

        let question = record.get(question_idx).unwrap_or("").trim();
        let query = record.get(query_idx).unwrap_or("").trim();

        if question.is_empty() || query.is_empty() {
            continue; // Skip empty rows
        }

        items.push(serde_json::json!({
            "question": question,
            "query": query,
        }));
    }

    Ok(items)
}

/// Map a Fabric item type to the data agent datasource type string.
fn map_item_type_to_datasource_type(item_type: &str) -> Result<String> {
    let ds_type = match item_type.to_lowercase().as_str() {
        "lakehouse" => "lakehouse_tables",
        "warehouse" => "data_warehouse",
        "kqldatabase" => "kusto",
        "semanticmodel" => "semantic_model",
        "ontology" => "ontology",
        "graphmodel" => "graph",
        "mirroreddatabase" => "mirrored_database",
        "sqldatabase" => "sql_database",
        _ => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Unsupported artifact type '{item_type}' for data agent datasource"),
                "Supported types: Lakehouse, Warehouse, KQLDatabase, SemanticModel, Ontology, GraphModel, MirroredDatabase, SQLDatabase",
            )
            .into());
        }
    };
    Ok(ds_type.to_string())
}

// ─── Definitions ─────────────────────────────────────────────────────────────

/// Get the definition of a data agent (data sources, instructions, etc.).
async fn get_definition(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await?;
    output::render_object(cli, &data, "definition");
    Ok(())
}

/// Update the definition of a data agent (configure data sources, instructions, etc.).
async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
    update_metadata: bool,
) -> Result<()> {
    let definition_json = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(inline)) => inline.to_string(),
        (None, None) => {
            return Err(
                FabioError::invalid_input("Either --file or --content must be provided").into(),
            );
        }
    };

    let body: Value = serde_json::from_str(&definition_json).map_err(|e| {
        FabioError::new(
            ErrorCode::InvalidInput,
            format!("Invalid JSON definition: {e}"),
        )
    })?;

    // If the body already has a "definition" wrapper, use as-is; otherwise wrap it
    let request_body = if body.get("definition").is_some() {
        body
    } else {
        serde_json::json!({ "definition": body })
    };

    // Validate datasource element IDs before sending to the API
    validate_datasource_elements(&request_body)?;

    if output::dry_run_guard(
        cli,
        "data-agent update-definition",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "updateMetadata": update_metadata,
        }),
    ) {
        return Ok(());
    }

    let path = if update_metadata {
        format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition?updateMetadata=True")
    } else {
        format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition")
    };

    let data = client.post(&path, &request_body, true).await?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

/// Publish a data agent by promoting draft configuration to published state.
///
/// This fetches the current definition, copies draft-stage configuration
/// (including datasources and fewshots) to published, adds `publish_info.json`,
/// and updates the definition. This is the officially supported programmatic
/// publish path (no portal interaction required).
#[allow(clippy::too_many_lines)]
async fn publish(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    description: Option<&str>,
    to_m365: bool,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent publish",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "description": description,
            "toM365": to_m365,
        }),
    ) {
        return Ok(());
    }

    // Step 1: Get current definition
    let definition_resp = client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await?;

    let parts = definition_resp
        .get("definition")
        .and_then(|d| d.get("parts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if parts.is_empty() {
        return Err(FabioError::new(
            ErrorCode::ApiError,
            "Data agent has no definition parts. Configure data sources first with \
             'fabio data-agent update-definition'.",
        )
        .into());
    }

    // Step 2: Build new definition with published parts
    let mut new_parts: Vec<Value> = Vec::new();

    // Keep existing parts
    for part in &parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        // Skip existing published parts and publish_info (we'll regenerate them)
        if !path.starts_with("Files/Config/published/") && path != "Files/Config/publish_info.json"
        {
            new_parts.push(part.clone());
        }
    }

    // Copy draft parts to published
    for part in &parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if path.starts_with("Files/Config/draft/") {
            let published_path = path.replace("Files/Config/draft/", "Files/Config/published/");
            let mut published_part = part.clone();
            if let Some(obj) = published_part.as_object_mut() {
                obj.insert("path".to_string(), Value::String(published_path));
            }
            new_parts.push(published_part);
        }
    }

    // Add publish_info.json
    let publish_info = serde_json::json!({
        "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/publishInfo/1.0.0/schema.json",
        "description": description.unwrap_or("")
    });
    let publish_info_encoded =
        base64::engine::general_purpose::STANDARD.encode(publish_info.to_string().as_bytes());
    new_parts.push(serde_json::json!({
        "path": "Files/Config/publish_info.json",
        "payload": publish_info_encoded,
        "payloadType": "InlineBase64"
    }));

    // Step 3: Validate and update the definition
    let update_body = serde_json::json!({
        "definition": {
            "parts": new_parts
        }
    });

    validate_datasource_elements(&update_body)?;

    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    // Step 4: Try the V3 settings endpoint to check if chat endpoint is active
    let settings_path = format!("/workspaces/{workspace}/dataAgents/{id}/settings");
    let published_url = client.get(&settings_path).await.ok().and_then(|s| {
        s.get("publishedUrl")
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())
            .map(String::from)
    });

    let mut obj = serde_json::json!({
        "id": id,
        "status": "published",
        "description": description.unwrap_or(""),
    });

    if let Some(url) = published_url {
        obj["publishedUrl"] = Value::String(url);
    }

    // Step 5 (optional): Publish to M365 Copilot Agent Store
    if to_m365 {
        // Resolve capacity ID for the workspace
        let ws_info = client.get(&format!("/workspaces/{workspace}")).await?;
        let capacity_id = ws_info
            .get("capacityId")
            .and_then(Value::as_str)
            .unwrap_or("");

        if capacity_id.is_empty() {
            return Err(FabioError::with_hint(
                ErrorCode::ApiError,
                "Cannot resolve capacity ID for M365 publishing",
                "The workspace must have a capacity assigned. Check: fabio workspace show -w <workspace>",
            ).into());
        }

        // The M365 endpoint uses the internal workload API
        let m365_url = format!("/workspaces/{workspace}/dataAgents/{id}/publishToM365");
        // Try the public API first; if it doesn't exist, note it in output
        let m365_result = client
            .post(&m365_url, &serde_json::json!({"scope": "Shared"}), false)
            .await;

        match m365_result {
            Ok(_) => {
                obj["m365Status"] = Value::String("published_to_m365".to_string());
            }
            Err(e) => {
                // M365 publishing is best-effort; report in output but don't fail
                obj["m365Status"] = Value::String("failed".to_string());
                obj["m365Error"] = Value::String(e.to_string());
            }
        }
    }

    output::render_object(cli, &obj, "status");
    Ok(())
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use serde_json::json;

    use super::validate_datasource_elements;

    fn make_definition(datasource_json: &serde_json::Value) -> serde_json::Value {
        let payload = base64::engine::general_purpose::STANDARD.encode(datasource_json.to_string());
        json!({
            "definition": {
                "parts": [
                    {
                        "path": "Files/Config/draft/lakehouse-tables-TestLH/datasource.json",
                        "payload": payload,
                        "payloadType": "InlineBase64"
                    }
                ]
            }
        })
    }

    #[test]
    fn valid_elements_pass_validation() {
        let datasource = json!({
            "artifactId": "00000000-0000-0000-0000-000000000001",
            "workspaceId": "00000000-0000-0000-0000-000000000002",
            "displayName": "TestLH",
            "type": "lakehouse_tables",
            "elements": [
                {
                    "id": "dbo",
                    "is_selected": true,
                    "display_name": "dbo",
                    "type": "lakehouse_tables.schema",
                    "children": [
                        {
                            "id": "dbo.my_table",
                            "is_selected": true,
                            "display_name": "my_table",
                            "type": "lakehouse_tables.table",
                            "children": [
                                {
                                    "id": "dbo.my_table.col1",
                                    "is_selected": true,
                                    "display_name": "col1",
                                    "type": "lakehouse_tables.column",
                                    "data_type": "string",
                                    "children": []
                                }
                            ]
                        }
                    ]
                }
            ]
        });

        let body = make_definition(&datasource);
        assert!(validate_datasource_elements(&body).is_ok());
    }

    #[test]
    fn null_table_id_fails_validation() {
        let datasource = json!({
            "artifactId": "00000000-0000-0000-0000-000000000001",
            "workspaceId": "00000000-0000-0000-0000-000000000002",
            "displayName": "TestLH",
            "type": "lakehouse_tables",
            "elements": [
                {
                    "id": null,
                    "is_selected": true,
                    "display_name": "my_table",
                    "type": "lakehouse_tables.table",
                    "children": []
                }
            ]
        });

        let body = make_definition(&datasource);
        let err = validate_datasource_elements(&body).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("my_table"),
            "Error should mention element name: {msg}"
        );
        assert!(
            msg.contains("null or empty"),
            "Error should explain the problem: {msg}"
        );
        assert!(
            msg.contains("dbo.my_table"),
            "Error should suggest correct format: {msg}"
        );
    }

    #[test]
    fn empty_string_id_fails_validation() {
        let datasource = json!({
            "artifactId": "00000000-0000-0000-0000-000000000001",
            "workspaceId": "00000000-0000-0000-0000-000000000002",
            "displayName": "TestLH",
            "type": "lakehouse_tables",
            "elements": [
                {
                    "id": "",
                    "is_selected": true,
                    "display_name": "bad_schema",
                    "type": "lakehouse_tables.schema",
                    "children": []
                }
            ]
        });

        let body = make_definition(&datasource);
        let err = validate_datasource_elements(&body).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("bad_schema"),
            "Error should mention element name: {msg}"
        );
    }

    #[test]
    fn nested_null_column_id_fails_validation() {
        let datasource = json!({
            "artifactId": "00000000-0000-0000-0000-000000000001",
            "workspaceId": "00000000-0000-0000-0000-000000000002",
            "displayName": "TestLH",
            "type": "lakehouse_tables",
            "elements": [
                {
                    "id": "dbo",
                    "is_selected": true,
                    "display_name": "dbo",
                    "type": "lakehouse_tables.schema",
                    "children": [
                        {
                            "id": "dbo.my_table",
                            "is_selected": true,
                            "display_name": "my_table",
                            "type": "lakehouse_tables.table",
                            "children": [
                                {
                                    "id": null,
                                    "is_selected": true,
                                    "display_name": "bad_col",
                                    "type": "lakehouse_tables.column",
                                    "data_type": "int",
                                    "children": []
                                }
                            ]
                        }
                    ]
                }
            ]
        });

        let body = make_definition(&datasource);
        let err = validate_datasource_elements(&body).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("bad_col"),
            "Error should mention nested element: {msg}"
        );
        assert!(
            msg.contains("dbo.table_name.bad_col"),
            "Error should suggest dot-path: {msg}"
        );
    }

    #[test]
    fn empty_elements_array_passes() {
        let datasource = json!({
            "artifactId": "00000000-0000-0000-0000-000000000001",
            "workspaceId": "00000000-0000-0000-0000-000000000002",
            "displayName": "TestLH",
            "type": "lakehouse_tables",
            "elements": []
        });

        let body = make_definition(&datasource);
        assert!(validate_datasource_elements(&body).is_ok());
    }

    #[test]
    fn no_datasource_parts_passes() {
        let body = json!({
            "definition": {
                "parts": [
                    {
                        "path": "Files/Config/data_agent.json",
                        "payload": "e30=",
                        "payloadType": "InlineBase64"
                    }
                ]
            }
        });
        assert!(validate_datasource_elements(&body).is_ok());
    }

    // ─── Tests for new helper functions ──────────────────────────────────────

    use super::{
        decode_part_payload, extract_datasources_from_parts, extract_fewshots_for_datasource,
        find_datasource_dir, map_item_type_to_datasource_type, set_table_selection,
    };

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
    fn map_item_type_lakehouse() {
        assert_eq!(
            map_item_type_to_datasource_type("Lakehouse").unwrap(),
            "lakehouse_tables"
        );
    }

    #[test]
    fn map_item_type_warehouse() {
        assert_eq!(
            map_item_type_to_datasource_type("Warehouse").unwrap(),
            "data_warehouse"
        );
    }

    #[test]
    fn map_item_type_kql_database() {
        assert_eq!(
            map_item_type_to_datasource_type("KQLDatabase").unwrap(),
            "kusto"
        );
    }

    #[test]
    fn map_item_type_semantic_model() {
        assert_eq!(
            map_item_type_to_datasource_type("SemanticModel").unwrap(),
            "semantic_model"
        );
    }

    #[test]
    fn map_item_type_mirrored_database() {
        assert_eq!(
            map_item_type_to_datasource_type("MirroredDatabase").unwrap(),
            "mirrored_database"
        );
    }

    #[test]
    fn map_item_type_sql_database() {
        assert_eq!(
            map_item_type_to_datasource_type("SQLDatabase").unwrap(),
            "sql_database"
        );
    }

    #[test]
    fn map_item_type_unsupported() {
        let err = map_item_type_to_datasource_type("Notebook").unwrap_err();
        assert!(err.to_string().contains("Unsupported"));
    }

    #[test]
    fn map_item_type_case_insensitive() {
        assert_eq!(
            map_item_type_to_datasource_type("lakehouse").unwrap(),
            "lakehouse_tables"
        );
        assert_eq!(
            map_item_type_to_datasource_type("WAREHOUSE").unwrap(),
            "data_warehouse"
        );
    }

    #[test]
    fn extract_datasources_from_parts_finds_datasource() {
        let ds_json = json!({
            "artifactId": "aaa",
            "displayName": "TestLH",
            "type": "lakehouse_tables"
        });
        let payload =
            base64::engine::general_purpose::STANDARD.encode(ds_json.to_string().as_bytes());
        let parts = vec![
            json!({
                "path": "Files/Config/data_agent.json",
                "payload": "e30=",
                "payloadType": "InlineBase64"
            }),
            json!({
                "path": "Files/Config/draft/lakehouse_tables-TestLH/datasource.json",
                "payload": payload,
                "payloadType": "InlineBase64"
            }),
        ];

        let datasources = extract_datasources_from_parts(&parts);
        assert_eq!(datasources.len(), 1);
        assert_eq!(datasources[0]["displayName"], "TestLH");
        assert_eq!(datasources[0]["type"], "lakehouse_tables");
    }

    #[test]
    fn extract_datasources_from_parts_empty() {
        let parts = vec![json!({
            "path": "Files/Config/data_agent.json",
            "payload": "e30=",
            "payloadType": "InlineBase64"
        })];
        let datasources = extract_datasources_from_parts(&parts);
        assert!(datasources.is_empty());
    }

    #[test]
    fn find_datasource_dir_by_name() {
        let ds_json =
            json!({"displayName": "MyWarehouse", "type": "data_warehouse", "artifactId": "bbb"});
        let payload =
            base64::engine::general_purpose::STANDARD.encode(ds_json.to_string().as_bytes());
        let parts = vec![json!({
            "path": "Files/Config/draft/data_warehouse-MyWarehouse/datasource.json",
            "payload": payload,
            "payloadType": "InlineBase64"
        })];

        let dir = find_datasource_dir(&parts, "MyWarehouse").unwrap();
        assert_eq!(dir, "Files/Config/draft/data_warehouse-MyWarehouse");
    }

    #[test]
    fn find_datasource_dir_by_id() {
        let ds_json =
            json!({"displayName": "TestLH", "type": "lakehouse_tables", "artifactId": "abc-123"});
        let payload =
            base64::engine::general_purpose::STANDARD.encode(ds_json.to_string().as_bytes());
        let parts = vec![json!({
            "path": "Files/Config/draft/lakehouse_tables-TestLH/datasource.json",
            "payload": payload,
            "payloadType": "InlineBase64"
        })];

        let dir = find_datasource_dir(&parts, "abc-123").unwrap();
        assert_eq!(dir, "Files/Config/draft/lakehouse_tables-TestLH");
    }

    #[test]
    fn find_datasource_dir_not_found() {
        let parts = vec![json!({
            "path": "Files/Config/data_agent.json",
            "payload": "e30=",
            "payloadType": "InlineBase64"
        })];
        assert!(find_datasource_dir(&parts, "nonexistent").is_err());
    }

    #[test]
    fn extract_fewshots_for_datasource_found() {
        let ds_json =
            json!({"displayName": "TestLH", "type": "lakehouse_tables", "artifactId": "x"});
        let ds_payload =
            base64::engine::general_purpose::STANDARD.encode(ds_json.to_string().as_bytes());
        let fs_json = json!({
            "fewShots": [
                {"id": "fs1", "question": "How many?", "query": "SELECT COUNT(*) FROM t"}
            ]
        });
        let fs_payload =
            base64::engine::general_purpose::STANDARD.encode(fs_json.to_string().as_bytes());

        let parts = vec![
            json!({
                "path": "Files/Config/draft/lakehouse_tables-TestLH/datasource.json",
                "payload": ds_payload,
                "payloadType": "InlineBase64"
            }),
            json!({
                "path": "Files/Config/draft/lakehouse_tables-TestLH/fewshots.json",
                "payload": fs_payload,
                "payloadType": "InlineBase64"
            }),
        ];

        let fewshots = extract_fewshots_for_datasource(&parts, "TestLH").unwrap();
        assert_eq!(fewshots.len(), 1);
        assert_eq!(fewshots[0]["id"], "fs1");
        assert_eq!(fewshots[0]["question"], "How many?");
    }

    #[test]
    fn extract_fewshots_empty_when_no_file() {
        let ds_json =
            json!({"displayName": "TestLH", "type": "lakehouse_tables", "artifactId": "x"});
        let ds_payload =
            base64::engine::general_purpose::STANDARD.encode(ds_json.to_string().as_bytes());
        let parts = vec![json!({
            "path": "Files/Config/draft/lakehouse_tables-TestLH/datasource.json",
            "payload": ds_payload,
            "payloadType": "InlineBase64"
        })];

        let fewshots = extract_fewshots_for_datasource(&parts, "TestLH").unwrap();
        assert!(fewshots.is_empty());
    }

    #[test]
    fn set_table_selection_selects_by_name() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "is_selected": false,
            "children": [
                {"display_name": "orders", "type": "lakehouse_tables.table", "is_selected": false, "children": []},
                {"display_name": "products", "type": "lakehouse_tables.table", "is_selected": false, "children": []}
            ]
        })];

        let count = set_table_selection(&mut elements, &["orders"], false, true);
        assert_eq!(count, 1);
        let children = elements[0]["children"].as_array().unwrap();
        assert_eq!(children[0]["is_selected"], true);
        assert_eq!(children[1]["is_selected"], false);
    }

    #[test]
    fn set_table_selection_selects_all() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "is_selected": false,
            "children": [
                {"display_name": "orders", "type": "lakehouse_tables.table", "is_selected": false, "children": []},
                {"display_name": "products", "type": "lakehouse_tables.table", "is_selected": false, "children": []}
            ]
        })];

        let count = set_table_selection(&mut elements, &[], true, true);
        assert_eq!(count, 2);
        let children = elements[0]["children"].as_array().unwrap();
        assert_eq!(children[0]["is_selected"], true);
        assert_eq!(children[1]["is_selected"], true);
    }

    #[test]
    fn set_table_selection_unselects() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "is_selected": true,
            "children": [
                {"display_name": "orders", "type": "lakehouse_tables.table", "is_selected": true, "children": []}
            ]
        })];

        let count = set_table_selection(&mut elements, &["orders"], false, false);
        assert_eq!(count, 1);
        let children = elements[0]["children"].as_array().unwrap();
        assert_eq!(children[0]["is_selected"], false);
    }

    #[test]
    fn set_table_selection_case_insensitive() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "children": [
                {"display_name": "Orders", "type": "lakehouse_tables.table", "is_selected": false, "children": []}
            ]
        })];

        let count = set_table_selection(&mut elements, &["orders"], false, true);
        assert_eq!(count, 1);
    }

    #[test]
    fn parse_csv_fewshots_basic() {
        let csv = "question,query\nHow many?,SELECT COUNT(*) FROM t\nMax price?,SELECT MAX(price) FROM p\n";
        let items = super::parse_fewshots_csv(csv, "test.csv").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["question"], "How many?");
        assert_eq!(items[0]["query"], "SELECT COUNT(*) FROM t");
        assert_eq!(items[1]["question"], "Max price?");
    }

    #[test]
    fn parse_csv_fewshots_case_insensitive_headers() {
        let csv = "Question,Query\nTest?,SELECT 1\n";
        let items = super::parse_fewshots_csv(csv, "test.csv").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["question"], "Test?");
    }

    #[test]
    fn parse_csv_fewshots_answer_column() {
        // 'answer' is an alias for 'query' column
        let csv = "question,answer\nHow many?,SELECT COUNT(*) FROM t\n";
        let items = super::parse_fewshots_csv(csv, "test.csv").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["query"], "SELECT COUNT(*) FROM t");
    }

    #[test]
    fn parse_csv_fewshots_tsv() {
        let tsv = "question\tquery\nHow many?\tSELECT COUNT(*) FROM t\n";
        let items = super::parse_fewshots_csv(tsv, "data.tsv").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["question"], "How many?");
    }

    #[test]
    fn parse_csv_fewshots_missing_question_column() {
        let csv = "prompt,query\nHow?,SELECT 1\n";
        let err = super::parse_fewshots_csv(csv, "bad.csv").unwrap_err();
        assert!(err.to_string().contains("question"));
    }

    #[test]
    fn parse_csv_fewshots_skips_empty_rows() {
        let csv =
            "question,query\nHow many?,SELECT COUNT(*) FROM t\n,\nMax?,SELECT MAX(x) FROM y\n";
        let items = super::parse_fewshots_csv(csv, "test.csv").unwrap();
        assert_eq!(items.len(), 2);
    }
}
