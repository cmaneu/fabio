use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum CatalogCommand {
    /// Search the Fabric catalog
    #[command(display_order = 1)]
    Search {
        /// Search query string
        #[arg(short = 's', long = "search")]
        search_query: Option<String>,

        /// Filter by item type (e.g., Notebook, Lakehouse). Comma-separated for multiple.
        #[arg(short = 't', long = "type")]
        item_type: Option<String>,

        /// Exclude item types from results. Comma-separated for multiple.
        #[arg(long)]
        exclude_type: Option<String>,

        /// Maximum number of results to return
        #[arg(long)]
        top: Option<u32>,

        /// Path to JSON file with full search request body
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON search request body
        #[arg(long)]
        content: Option<String>,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &CatalogCommand) -> Result<()> {
    match command {
        CatalogCommand::Search {
            search_query,
            item_type,
            exclude_type,
            top,
            file,
            content,
        } => {
            search(
                cli,
                client,
                search_query.as_deref(),
                item_type.as_deref(),
                exclude_type.as_deref(),
                *top,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn search(
    cli: &Cli,
    client: &FabricClient,
    query: Option<&str>,
    item_type: Option<&str>,
    exclude_type: Option<&str>,
    top: Option<u32>,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    // --file and --content take full control of the body (raw passthrough)
    let body = match (file, content) {
        (Some(path), _) => {
            let raw = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?;
            serde_json::from_str::<Value>(&raw).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}"))?
        }
        (_, Some(c)) => {
            serde_json::from_str::<Value>(c).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}"))?
        }
        _ => {
            // Build body from convenience flags
            if query.is_none() && item_type.is_none() && exclude_type.is_none() {
                return Err(FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "At least one of --query, --type, --file, or --content must be provided"
                        .to_string(),
                    "Example: fabio catalog search --query \"my lakehouse\" --type Notebook --top 10"
                        .to_string(),
                )
                .into());
            }
            build_search_body(query, item_type, exclude_type, top)
        }
    };

    if output::dry_run_guard(cli, "catalog search", &body) {
        return Ok(());
    }

    let data = client.post("/catalog/search", &body, false).await?;
    output::render_object(cli, &data, "value");
    Ok(())
}

/// Build a catalog search request body from convenience flags.
fn build_search_body(
    query: Option<&str>,
    item_type: Option<&str>,
    exclude_type: Option<&str>,
    top: Option<u32>,
) -> Value {
    let mut body = serde_json::Map::new();

    if let Some(q) = query {
        body.insert("searchString".to_string(), Value::from(q));
    }

    if let Some(t) = top {
        body.insert("top".to_string(), Value::Number(t.into()));
    }

    // itemTypes and excludeItemTypes are top-level arrays (NOT nested under "filter")
    if let Some(types) = item_type {
        let type_array: Vec<Value> = types
            .split(',')
            .map(|s| Value::String(s.trim().to_string()))
            .collect();
        body.insert("itemTypes".to_string(), Value::Array(type_array));
    }

    if let Some(types) = exclude_type {
        let type_array: Vec<Value> = types
            .split(',')
            .map(|s| Value::String(s.trim().to_string()))
            .collect();
        body.insert("excludeItemTypes".to_string(), Value::Array(type_array));
    }

    Value::Object(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_search_body_query_only() {
        let body = build_search_body(Some("lakehouse"), None, None, None);
        assert_eq!(body["searchString"], "lakehouse");
        assert!(body.get("itemTypes").is_none());
        assert!(body.get("top").is_none());
    }

    #[test]
    fn build_search_body_with_type_filter() {
        let body = build_search_body(Some("test"), Some("Notebook,Lakehouse"), None, Some(5));
        assert_eq!(body["searchString"], "test");
        assert_eq!(body["top"], 5);
        let types = body["itemTypes"].as_array().unwrap();
        assert_eq!(types.len(), 2);
        assert_eq!(types[0], "Notebook");
        assert_eq!(types[1], "Lakehouse");
    }

    #[test]
    fn build_search_body_with_exclude_type() {
        let body = build_search_body(None, None, Some("Dashboard"), None);
        let excluded = body["excludeItemTypes"].as_array().unwrap();
        assert_eq!(excluded.len(), 1);
        assert_eq!(excluded[0], "Dashboard");
    }

    #[test]
    fn build_search_body_both_filters() {
        let body = build_search_body(Some("sales"), Some("Notebook"), Some("Lakehouse"), Some(20));
        assert_eq!(body["searchString"], "sales");
        assert_eq!(body["top"], 20);
        assert_eq!(body["itemTypes"][0], "Notebook");
        assert_eq!(body["excludeItemTypes"][0], "Lakehouse");
    }
}
