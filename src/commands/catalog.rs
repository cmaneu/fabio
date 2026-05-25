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
        #[arg(short, long)]
        query: Option<String>,

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
            query,
            file,
            content,
        } => {
            search(
                cli,
                client,
                query.as_deref(),
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
    }
}

async fn search(
    cli: &Cli,
    client: &FabricClient,
    query: Option<&str>,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = match (file, content, query) {
        (Some(path), _, _) => {
            let raw = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?;
            serde_json::from_str::<Value>(&raw).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}"))?
        }
        (_, Some(c), _) => {
            serde_json::from_str::<Value>(c).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}"))?
        }
        (_, _, Some(q)) => serde_json::json!({ "searchString": q }),
        (None, None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "At least one of --query, --file, or --content must be provided".to_string(),
                "Example: fabio catalog search --query \"my lakehouse\"".to_string(),
            )
            .into());
        }
    };

    if output::dry_run_guard(cli, "catalog search", &body) {
        return Ok(());
    }

    let data = client.post("/catalog/search", &body, false).await?;
    output::render_object(cli, &data, "value");
    Ok(())
}
