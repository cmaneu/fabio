use std::io::{self, Read};
use std::time::Duration;

use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;
use tokio::time::sleep;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError};
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
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a data agent
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,
    },
    /// Create a new data agent
    Create {
        /// Workspace ID
        #[arg(short, long)]
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
        #[arg(short, long)]
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
        #[arg(short, long)]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,
    },
    /// Query (chat with) a published data agent using natural language
    Query {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Data agent ID
        #[arg(long)]
        id: String,

        /// Natural language question (omit to read from stdin)
        #[arg(short, long)]
        prompt: Option<String>,
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
        } => create(cli, client, workspace, name, description.as_deref()).await,
        DataAgentCommand::Update {
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
        DataAgentCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        DataAgentCommand::Query {
            workspace,
            id,
            prompt,
        } => query(cli, client, workspace, id, prompt.as_deref()).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/dataAgents"),
            "value",
            cli.all,
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

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    client
        .delete(&format!("/workspaces/{workspace}/dataAgents/{id}"))
        .await?;

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

    // Get the published URL from the data agent's definition/settings.
    // The Fabric API returns dataAgent properties; we need the published URL.
    // Try to get it from the data agent metadata.
    let published_url = get_published_url(client, workspace, id).await?;

    // Use the OpenAI Assistants protocol against the published URL
    let token = client.require_auth().await?;
    let response_text = run_assistant_query(&published_url, &token, &prompt_text).await?;

    let result = serde_json::json!({
        "question": prompt_text.trim(),
        "answer": response_text,
    });
    output::render_object(cli, &result, "answer");
    Ok(())
}

/// Get the published URL of a data agent from its properties.
/// Falls back to constructing the URL from workspace and agent IDs.
async fn get_published_url(client: &FabricClient, workspace: &str, id: &str) -> Result<String> {
    // Try to get the data agent properties which may include the published URL
    let data = client
        .get(&format!("/workspaces/{workspace}/dataAgents/{id}"))
        .await?;

    // Check if properties contain published URL
    if let Some(url) = data
        .get("properties")
        .and_then(|p| p.get("publishedUrl"))
        .and_then(Value::as_str)
    {
        if !url.is_empty() {
            return Ok(url.to_string());
        }
    }

    // Construct the standard published URL pattern for Fabric data agents
    // Format: https://api.fabric.microsoft.com/v1/workspaces/{ws}/dataAgents/{id}/chat/openai
    Ok(format!(
        "https://api.fabric.microsoft.com/v1/workspaces/{workspace}/dataAgents/{id}/chat/openai"
    ))
}

/// Run a query against the data agent using the `OpenAI` Assistants API protocol.
async fn run_assistant_query(base_url: &str, token: &str, question: &str) -> Result<String> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(360))
        .build()
        .map_err(|e| FabioError::new(ErrorCode::NetworkError, e.to_string()))?;

    let auth_header = format!("Bearer {token}");

    // Step 1: Create assistant + thread
    let assistant_id = create_assistant(&http, base_url, &auth_header).await?;
    let thread_id = create_thread(&http, base_url, &auth_header).await?;

    // Step 2: Post message and run
    post_message(&http, base_url, &auth_header, &thread_id, question).await?;
    let run_id = create_run(&http, base_url, &auth_header, &thread_id, &assistant_id).await?;

    // Step 3: Poll for completion
    poll_run_completion(&http, base_url, &auth_header, &thread_id, &run_id).await?;

    // Step 4: Retrieve response
    let response_text = retrieve_response(&http, base_url, &auth_header, &thread_id).await?;

    // Step 5: Clean up thread (best effort)
    let _ = http
        .delete(format!(
            "{base_url}/threads/{thread_id}?api-version=2024-05-01-preview"
        ))
        .header("Authorization", &auth_header)
        .send()
        .await;

    Ok(response_text)
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
        let text = resp.text().await.unwrap_or_default();
        return Err(
            FabioError::from_status(status, format!("Failed to create assistant: {text}")).into(),
        );
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
        let text = resp.text().await.unwrap_or_default();
        return Err(
            FabioError::from_status(status, format!("Failed to create thread: {text}")).into(),
        );
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
        let text = resp.text().await.unwrap_or_default();
        return Err(
            FabioError::from_status(status, format!("Failed to post message: {text}")).into(),
        );
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
        let text = resp.text().await.unwrap_or_default();
        return Err(
            FabioError::from_status(status, format!("Failed to create run: {text}")).into(),
        );
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
            return Err(FabioError::new(
                ErrorCode::Timeout,
                "Data agent query timed out waiting for response",
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
            let text = poll_resp.text().await.unwrap_or_default();
            return Err(FabioError::from_status(
                status,
                format!("Failed to poll run status: {text}"),
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
                return Err(
                    FabioError::api_error(format!("Run status '{status}': {err_msg}")).into(),
                );
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
        let text = resp.text().await.unwrap_or_default();
        return Err(FabioError::from_status(
            status,
            format!("Failed to retrieve messages: {text}"),
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
