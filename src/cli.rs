use clap::{Parser, Subcommand, ValueEnum};

use crate::commands::{auth, dataagent, item, lakehouse, notebook, warehouse, workspace};

/// Agent-first CLI for managing Microsoft Fabric artifacts and data.
///
/// Structured JSON output by default. Designed for composability via stdin/stdout.
#[derive(Parser, Debug)]
#[command(name = "fabio", version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Output format
    #[arg(short, long, global = true, default_value = "json")]
    pub output: OutputFormat,

    /// Query projection (dot-notation field extraction)
    #[arg(short, long, global = true)]
    pub query: Option<String>,

    /// Suppress all output
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Plain,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        command: auth::AuthCommand,
    },
    /// Manage workspaces
    Workspace {
        #[command(subcommand)]
        command: workspace::WorkspaceCommand,
    },
    /// Manage items (datasets, reports, notebooks, etc.)
    Item {
        #[command(subcommand)]
        command: item::ItemCommand,
    },
    /// Manage lakehouses (tables, files, shortcuts)
    Lakehouse {
        #[command(subcommand)]
        command: lakehouse::LakehouseCommand,
    },
    /// Manage notebooks
    Notebook {
        #[command(subcommand)]
        command: notebook::NotebookCommand,
    },
    /// Manage warehouses and run SQL queries
    Warehouse {
        #[command(subcommand)]
        command: warehouse::WarehouseCommand,
    },
    /// Manage data agents (create, query, and interact with AI agents)
    #[command(visible_alias = "da")]
    DataAgent {
        #[command(subcommand)]
        command: dataagent::DataAgentCommand,
    },
}
