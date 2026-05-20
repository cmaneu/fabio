use clap::{Parser, Subcommand, ValueEnum};

use crate::commands::{
    auth, connection, dataagent, feedback, git, item, jobs, lakehouse, notebook, ontology, profile,
    warehouse, workspace,
};

/// Agent-first CLI for managing Microsoft Fabric artifacts and data.
///
/// Structured JSON output by default. Designed for composability via stdin/stdout.
#[derive(Parser, Debug)]
#[command(name = "fabio", version, about, long_about = None)]
#[command(propagate_version = true)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    /// Output format
    #[arg(short, long, global = true, default_value = "json")]
    pub output: OutputFormat,

    /// Shorthand for --output json (agent-native convention)
    #[arg(long, global = true)]
    pub json: bool,

    /// Query projection (dot-notation field extraction)
    #[arg(short, long, global = true)]
    pub query: Option<String>,

    /// Suppress all output
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Skip confirmation prompts (for destructive operations)
    #[arg(long, global = true)]
    pub force: bool,

    /// Preview what would happen without making changes
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Maximum number of items to return in list commands
    #[arg(long, global = true)]
    pub limit: Option<usize>,

    /// Fetch all pages (auto-paginate). Without this, only the first page is returned.
    #[arg(long, global = true)]
    pub all: bool,

    /// Use a named profile for default settings
    #[arg(long, global = true)]
    pub profile: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    /// Returns the effective output format, considering --json shorthand.
    pub const fn effective_output(&self) -> &OutputFormat {
        if self.json {
            &OutputFormat::Json
        } else {
            &self.output
        }
    }
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
    /// Manage Git integration (connect, commit, pull, status)
    Git {
        #[command(subcommand)]
        command: git::GitCommand,
    },
    /// Manage ontologies (entity types, data bindings)
    Ontology {
        #[command(subcommand)]
        command: ontology::OntologyCommand,
    },
    /// Manage connections (cloud, on-premises, virtual network)
    Connection {
        #[command(subcommand)]
        command: connection::ConnectionCommand,
    },
    /// Manage saved configuration profiles
    Profile {
        #[command(subcommand)]
        command: profile::ProfileCommand,
    },
    /// Inspect and manage async job history
    Jobs {
        #[command(subcommand)]
        command: jobs::JobsCommand,
    },
    /// Report CLI friction or issues for improvement
    Feedback {
        #[command(subcommand)]
        command: feedback::FeedbackCommand,
    },
    /// Machine-readable CLI schema for agent introspection
    #[command(name = "agent-context")]
    AgentContext,
}
