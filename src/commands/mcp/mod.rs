//! MCP (Model Context Protocol) server for agent-native tool integration.
//!
//! Implements JSON-RPC 2.0 over stdio with `tools/list` and `tools/call` handlers.

mod serve;

use anyhow::Result;
use clap::Subcommand;

use crate::cli::Cli;

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Start the MCP server (JSON-RPC 2.0 over stdin/stdout)
    Serve,
}

pub async fn execute(cli: &Cli, command: &McpCommand) -> Result<()> {
    match command {
        McpCommand::Serve => serve::run(cli).await,
    }
}
