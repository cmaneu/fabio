use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

/// HTTP method for raw REST calls.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Subcommand)]
pub enum RestCommand {
    /// Send a raw REST request to the Fabric API
    ///
    /// Similar to `az rest` or `gh api`. Uses the authenticated client
    /// (same token as all other commands). Paths are relative to the
    /// Fabric base URL (`https://api.fabric.microsoft.com/v1`).
    ///
    /// Examples:
    ///   fabio rest call --method get --path /workspaces
    ///   fabio rest call --method post --path /workspaces --body '{"displayName":"Test"}'
    ///   echo '{"displayName":"X"}' | fabio rest call --method post --path /workspaces --body @-
    ///   fabio rest call --method get --path /capacities --query-params "beta=true"
    #[command(display_order = 0)]
    Call {
        /// HTTP method (get, post, put, patch, delete)
        #[arg(short, long)]
        method: HttpMethod,

        /// API path relative to base URL (e.g., /workspaces or /workspaces/{id}/items)
        #[arg(short, long)]
        path: String,

        /// Request body (JSON string, @file path, or @- for stdin)
        #[arg(short, long)]
        body: Option<String>,

        /// Additional query parameters (appended to URL, e.g., "beta=true&format=json")
        #[arg(long, visible_alias = "params")]
        query_params: Option<String>,

        /// Use LRO polling for the response (for async operations)
        #[arg(long)]
        poll: bool,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &RestCommand) -> Result<()> {
    match command {
        RestCommand::Call {
            method,
            path,
            body,
            query_params,
            poll,
        } => {
            call(
                cli,
                client,
                method,
                path,
                body.as_deref(),
                query_params.as_deref(),
                *poll,
            )
            .await
        }
    }
}

async fn call(
    cli: &Cli,
    client: &FabricClient,
    method: &HttpMethod,
    path: &str,
    body: Option<&str>,
    query_params: Option<&str>,
    poll: bool,
) -> Result<()> {
    // Build the full path with optional query params
    let full_path = query_params.map_or_else(
        || path.to_string(),
        |params| {
            if path.contains('?') {
                format!("{path}&{params}")
            } else {
                format!("{path}?{params}")
            }
        },
    );

    // Parse body if provided
    let parsed_body = match body {
        Some(b) => Some(resolve_body(b)?),
        None => None,
    };

    // Dry-run guard
    if matches!(
        method,
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Delete
    ) {
        let dry_run_details = serde_json::json!({
            "method": format!("{method:?}").to_uppercase(),
            "path": full_path,
            "body": parsed_body,
        });
        if output::dry_run_guard(cli, "rest call", &dry_run_details) {
            return Ok(());
        }
    }

    // Execute the request
    let data = match method {
        HttpMethod::Get => client.get(&full_path).await?,
        HttpMethod::Post => {
            let b = parsed_body.unwrap_or(serde_json::json!({}));
            client.post(&full_path, &b, poll).await?
        }
        HttpMethod::Put => {
            let b = parsed_body.unwrap_or(serde_json::json!({}));
            client.put(&full_path, &b).await?
        }
        HttpMethod::Patch => {
            let b = parsed_body.unwrap_or(serde_json::json!({}));
            client.patch(&full_path, &b).await?
        }
        HttpMethod::Delete => client.delete(&full_path).await?,
    };

    // Render response — for raw REST we pass through as-is
    output::render_object(cli, &data, "data");
    Ok(())
}

/// Resolve body from inline JSON, @file, or @- (stdin).
fn resolve_body(input: &str) -> Result<Value> {
    let content = if input == "@-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else if let Some(file_path) = input.strip_prefix('@') {
        std::fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{file_path}': {e}"))?
    } else {
        input.to_string()
    };

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(serde_json::json!({}));
    }

    serde_json::from_str(trimmed).map_err(|e| {
        anyhow::anyhow!("Invalid JSON body: {e}. Provide valid JSON, @<file>, or @- for stdin.")
    })
}
