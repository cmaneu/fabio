//! MCP (Model Context Protocol) server for agent-native tool integration.
//!
//! Implements JSON-RPC 2.0 over stdio with `tools/list` and `tools/call` handlers.
//! Read-only by default — mutation tools are hidden unless `--allow-write` is set.

mod serve;

use anyhow::Result;
use clap::Subcommand;

use crate::cli::Cli;

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Start the MCP server (JSON-RPC 2.0 over stdin/stdout).
    /// Read-only by default: only non-mutating tools are exposed.
    Serve {
        /// Expose mutation tools (create, update, delete, run, etc.).
        /// Without this flag, only read-only tools are visible.
        #[arg(long)]
        allow_write: bool,

        /// Only expose tools matching these patterns (comma-separated).
        /// Patterns match tool name prefixes: "workspace" matches all workspace tools.
        /// Without this flag, all tools (subject to --allow-write) are exposed.
        #[arg(long, value_delimiter = ',')]
        allow_tool: Option<Vec<String>>,
    },
}

pub async fn execute(cli: &Cli, command: &McpCommand) -> Result<()> {
    match command {
        McpCommand::Serve {
            allow_write,
            allow_tool,
        } => serve::run(cli, *allow_write, allow_tool.as_deref()).await,
    }
}
