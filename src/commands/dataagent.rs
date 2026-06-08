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
/// Maximum wait time for data agent query runs.
const QUERY_MAX_WAIT: Duration = Duration::from_secs(300);

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
    },
}

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
        } => query(
            cli,
            client,
            workspace,
            id,
            prompt.as_deref(),
            published_url.as_deref(),
            *show_steps,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "data-agent query", "Viewer")),
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
        } => publish(cli, client, workspace, id, description.as_deref())
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
async fn query(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    prompt: Option<&str>,
    published_url: Option<&str>,
    verbose: bool,
) -> Result<()> {
    // Resolve prompt text: --prompt flag or stdin
    let prompt_text = if let Some(p) = prompt {
        p.to_string()
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).map_err(|e| {
            FabioError::new(
                ErrorCode::ApiError,
                format!("Failed to read prompt from stdin: {e}"),
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
    let query_result = run_assistant_query(&resolved_url, &token, &prompt_text, verbose).await?;

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
        "Published URL not found. The data agent must be published from the Fabric portal first.",
        format!(
            "After publishing in the portal, provide the URL with --published-url. \
             The URL pattern is: https://api.fabric.microsoft.com/v1/workspaces/{workspace}/dataagents/{id}/aiassistant/openai \
             (found in the agent's Settings page in the portal)."
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
) -> Result<QueryResult> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(360))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

    let auth_header = token;

    // Step 1: Create assistant + thread
    let assistant_id = create_assistant(&http, base_url, auth_header).await?;
    let thread_id = create_thread(&http, base_url, auth_header).await?;

    // Step 2: Post message and run
    post_message(&http, base_url, auth_header, &thread_id, question).await?;
    let run_id = create_run(&http, base_url, auth_header, &thread_id, &assistant_id).await?;

    // Step 3: Poll until complete
    poll_run_completion(&http, base_url, auth_header, &thread_id, &run_id).await?;

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
        .map_err(|e| FabioError::new(ErrorCode::NetworkError, format!("Create assistant: {e}")))?;

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
        FabioError::new(
            ErrorCode::ApiError,
            format!("Parse assistant response: {e}"),
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
        .map_err(|e| FabioError::new(ErrorCode::NetworkError, format!("Create thread: {e}")))?;

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
    let body: Value = resp
        .json()
        .await
        .map_err(|e| FabioError::new(ErrorCode::ApiError, format!("Parse thread response: {e}")))?;
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
        .map_err(|e| FabioError::new(ErrorCode::NetworkError, format!("Post message: {e}")))?;

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
        .map_err(|e| FabioError::new(ErrorCode::NetworkError, format!("Create run: {e}")))?;

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
    let body: Value = resp
        .json()
        .await
        .map_err(|e| FabioError::new(ErrorCode::ApiError, format!("Parse run response: {e}")))?;
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
) -> Result<()> {
    let start = std::time::Instant::now();
    let terminal_states = ["completed", "failed", "cancelled", "requires_action"];

    loop {
        if start.elapsed() > QUERY_MAX_WAIT {
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
            .map_err(|e| FabioError::new(ErrorCode::NetworkError, format!("Poll run: {e}")))?;

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
            FabioError::new(ErrorCode::ApiError, format!("Parse run poll response: {e}"))
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
        .map_err(|e| FabioError::new(ErrorCode::NetworkError, format!("Retrieve messages: {e}")))?;

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
        FabioError::new(ErrorCode::ApiError, format!("Parse messages response: {e}"))
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
            FabioError::new(ErrorCode::NetworkError, format!("Retrieve run steps: {e}"))
        })?;

    if !resp.status().is_success() {
        // Non-fatal: if steps endpoint is not available, return empty array
        return Ok(serde_json::json!([]));
    }

    let body: Value = resp.json().await.map_err(|e| {
        FabioError::new(
            ErrorCode::ApiError,
            format!("Parse run steps response: {e}"),
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
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent publish",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "description": description,
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
}
