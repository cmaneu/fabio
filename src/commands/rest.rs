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

/// Target API for raw REST calls.
#[derive(Debug, Clone, Default, clap::ValueEnum)]
pub enum ApiTarget {
    /// Fabric REST API (<https://api.fabric.microsoft.com/v1>)
    #[default]
    Fabric,
    /// Power BI REST API (<https://api.powerbi.com/v1.0/myorg>)
    Powerbi,
}

#[derive(Debug, Subcommand)]
pub enum RestCommand {
    /// Send a raw REST request to the Fabric or Power BI API
    ///
    /// Similar to `az rest` or `gh api`. Uses the authenticated client
    /// (same token as all other commands). Paths are relative to the
    /// selected API base URL.
    ///
    /// Fabric API (default): `https://api.fabric.microsoft.com/v1`
    /// Power BI API (--api powerbi): `https://api.powerbi.com/v1.0/myorg`
    ///
    /// Examples:
    ///   fabio rest call --method get --path /workspaces
    ///   fabio rest call --method post --path /workspaces --body '{"displayName":"Test"}'
    ///   fabio rest call --method get --path /groups/{ws}/datasets --api powerbi
    ///   fabio rest call --method post --path /groups/{ws}/datasets/{id}/refreshes --api powerbi
    ///   echo '{"displayName":"X"}' | fabio rest call --method post --path /workspaces --body @-
    ///   fabio rest call --method get --path /capacities --query-params "beta=true"
    #[command(display_order = 0)]
    Call {
        /// HTTP method (get, post, put, patch, delete)
        #[arg(short, long)]
        method: HttpMethod,

        /// API path relative to base URL (e.g., /workspaces or /groups/{ws}/datasets/{id}/refreshes)
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

        /// Target API (fabric or powerbi)
        #[arg(long, default_value = "fabric")]
        api: ApiTarget,
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
            api,
        } => {
            call(
                cli,
                client,
                method,
                path,
                body.as_deref(),
                query_params.as_deref(),
                *poll,
                api,
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn call(
    cli: &Cli,
    client: &FabricClient,
    method: &HttpMethod,
    path: &str,
    body: Option<&str>,
    query_params: Option<&str>,
    poll: bool,
    api: &ApiTarget,
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
        let api_label = match api {
            ApiTarget::Fabric => "fabric",
            ApiTarget::Powerbi => "powerbi",
        };
        // Redact sensitive fields in the body for the dry-run preview
        let redacted_body = parsed_body.as_ref().map(|b| {
            let serialized = serde_json::to_string(b).unwrap_or_default();
            let redacted = crate::verbose::redact_body_if_json(&serialized);
            serde_json::from_str::<serde_json::Value>(&redacted).unwrap_or_else(|_| b.clone())
        });
        let dry_run_details = serde_json::json!({
            "method": format!("{method:?}").to_uppercase(),
            "path": full_path,
            "api": api_label,
            "body": redacted_body,
        });
        if output::dry_run_guard(cli, "rest call", &dry_run_details) {
            return Ok(());
        }
    }

    // Execute the request
    let data = match api {
        ApiTarget::Fabric => execute_fabric(client, method, &full_path, parsed_body, poll).await?,
        ApiTarget::Powerbi => execute_powerbi(client, method, &full_path, parsed_body).await?,
    };

    // Render response — for raw REST we pass through as-is
    output::render_object(cli, &data, "data");
    Ok(())
}

async fn execute_fabric(
    client: &FabricClient,
    method: &HttpMethod,
    path: &str,
    body: Option<Value>,
    poll: bool,
) -> Result<Value> {
    match method {
        HttpMethod::Get => client.get(path).await,
        HttpMethod::Post => {
            let b = body.unwrap_or_else(|| serde_json::json!({}));
            client.post(path, &b, poll).await
        }
        HttpMethod::Put => {
            let b = body.unwrap_or_else(|| serde_json::json!({}));
            client.put(path, &b).await
        }
        HttpMethod::Patch => {
            let b = body.unwrap_or_else(|| serde_json::json!({}));
            client.patch(path, &b).await
        }
        HttpMethod::Delete => client.delete(path).await,
    }
}

async fn execute_powerbi(
    client: &FabricClient,
    method: &HttpMethod,
    path: &str,
    body: Option<Value>,
) -> Result<Value> {
    match method {
        HttpMethod::Get => client.get_powerbi(path).await,
        HttpMethod::Post => {
            let b = body.unwrap_or_else(|| serde_json::json!({}));
            client.post_powerbi(path, &b).await
        }
        HttpMethod::Put => {
            let b = body.unwrap_or_else(|| serde_json::json!({}));
            client.put_powerbi(path, &b).await
        }
        HttpMethod::Patch => {
            let b = body.unwrap_or_else(|| serde_json::json!({}));
            client.patch_powerbi(path, &b).await
        }
        HttpMethod::Delete => client.delete_powerbi(path).await,
    }
}

/// Resolve body from inline JSON, @file, or @- (stdin).
fn resolve_body(input: &str) -> Result<Value> {
    let content = if input == "@-" {
        std::io::read_to_string(std::io::stdin())?
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_body_inline_json_object() {
        let result = resolve_body(r#"{"key": "value"}"#).unwrap();
        assert_eq!(result, serde_json::json!({"key": "value"}));
    }

    #[test]
    fn resolve_body_inline_json_array() {
        let result = resolve_body("[1, 2, 3]").unwrap();
        assert_eq!(result, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn resolve_body_empty_string_returns_empty_object() {
        let result = resolve_body("").unwrap();
        assert_eq!(result, serde_json::json!({}));
    }

    #[test]
    fn resolve_body_whitespace_only_returns_empty_object() {
        let result = resolve_body("   \n  ").unwrap();
        assert_eq!(result, serde_json::json!({}));
    }

    #[test]
    fn resolve_body_invalid_json_errors() {
        let result = resolve_body("not json at all");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid JSON body"));
    }

    #[test]
    fn resolve_body_file_not_found_errors() {
        let result = resolve_body("@/nonexistent/path/body.json");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to read file"));
    }

    #[test]
    fn resolve_body_file_reads_json() {
        let dir = std::env::temp_dir().join("fabio_test_rest");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("body.json");
        std::fs::write(&file, r#"{"from_file": true}"#).unwrap();

        let result = resolve_body(&format!("@{}", file.display())).unwrap();
        assert_eq!(result, serde_json::json!({"from_file": true}));

        std::fs::remove_file(file).ok();
    }
}
