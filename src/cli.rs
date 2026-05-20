use clap::{Parser, Subcommand, ValueEnum};

use crate::commands::{
    auth, capacity, connection, data_pipeline, dataagent, deployment_pipeline, domain, environment,
    eventhouse, feedback, git, item, job_scheduler, jobs, kql_database, lakehouse,
    managed_private_endpoint, mirrored_database, notebook, onelake_security, ontology, profile,
    spark, warehouse, workspace,
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
    // ── Core ─────────────────────────────────────────────────────────────
    /// Manage workspaces
    #[command(display_order = 1)]
    Workspace {
        #[command(subcommand)]
        command: workspace::WorkspaceCommand,
    },
    /// Manage items (datasets, reports, notebooks, etc.)
    #[command(display_order = 2)]
    Item {
        #[command(subcommand)]
        command: item::ItemCommand,
    },
    /// Manage lakehouses (tables, files, shortcuts)
    #[command(display_order = 3)]
    Lakehouse {
        #[command(subcommand)]
        command: lakehouse::LakehouseCommand,
    },
    /// List and inspect Fabric capacities
    #[command(display_order = 4)]
    Capacity {
        #[command(subcommand)]
        command: capacity::CapacityCommand,
    },

    // ── Data & Compute ───────────────────────────────────────────────────
    /// Manage notebooks
    #[command(display_order = 10)]
    Notebook {
        #[command(subcommand)]
        command: notebook::NotebookCommand,
    },
    /// Manage warehouses and run SQL queries
    #[command(display_order = 11)]
    Warehouse {
        #[command(subcommand)]
        command: warehouse::WarehouseCommand,
    },
    /// Manage data agents (create, query, and interact with AI agents)
    #[command(visible_alias = "da", display_order = 12)]
    DataAgent {
        #[command(subcommand)]
        command: dataagent::DataAgentCommand,
    },
    /// Manage ontologies (entity types, data bindings)
    #[command(display_order = 13)]
    Ontology {
        #[command(subcommand)]
        command: ontology::OntologyCommand,
    },
    /// Manage environments (Spark compute, libraries, publish)
    #[command(display_order = 14)]
    Environment {
        #[command(subcommand)]
        command: environment::EnvironmentCommand,
    },
    /// Manage data pipelines (orchestration, scheduling)
    #[command(display_order = 15)]
    DataPipeline {
        #[command(subcommand)]
        command: data_pipeline::DataPipelineCommand,
    },
    /// Manage eventhouses (real-time analytics)
    #[command(display_order = 16)]
    Eventhouse {
        #[command(subcommand)]
        command: eventhouse::EventhouseCommand,
    },
    /// Manage KQL databases (within eventhouses)
    #[command(display_order = 17)]
    KqlDatabase {
        #[command(subcommand)]
        command: kql_database::KqlDatabaseCommand,
    },
    /// Manage mirrored databases (real-time replication)
    #[command(display_order = 18)]
    MirroredDatabase {
        #[command(subcommand)]
        command: mirrored_database::MirroredDatabaseCommand,
    },
    /// Manage Spark compute (settings, custom pools)
    #[command(display_order = 19)]
    Spark {
        #[command(subcommand)]
        command: spark::SparkCommand,
    },

    // ── Integration ──────────────────────────────────────────────────────
    /// Manage Git integration (connect, commit, pull, status)
    #[command(display_order = 20)]
    Git {
        #[command(subcommand)]
        command: git::GitCommand,
    },
    /// Manage connections (cloud, on-premises, virtual network)
    #[command(display_order = 21)]
    Connection {
        #[command(subcommand)]
        command: connection::ConnectionCommand,
    },
    /// Manage deployment pipelines (CI/CD stages, deploy items)
    #[command(display_order = 22)]
    DeploymentPipeline {
        #[command(subcommand)]
        command: deployment_pipeline::DeploymentPipelineCommand,
    },
    /// Manage domains (organize workspaces into business domains)
    #[command(display_order = 23)]
    Domain {
        #[command(subcommand)]
        command: domain::DomainCommand,
    },
    /// Manage item job scheduling (run, cancel, schedules)
    #[command(display_order = 24)]
    JobScheduler {
        #[command(subcommand)]
        command: job_scheduler::JobSchedulerCommand,
    },

    // ── Security & Governance ────────────────────────────────────────────
    /// Manage `OneLake` data access roles (row/column-level security)
    #[command(display_order = 25)]
    OnelakeSecurity {
        #[command(subcommand)]
        command: onelake_security::OnelakeSecurityCommand,
    },
    /// Manage workspace managed private endpoints
    #[command(display_order = 26)]
    ManagedPrivateEndpoint {
        #[command(subcommand)]
        command: managed_private_endpoint::ManagedPrivateEndpointCommand,
    },

    // ── Configuration ────────────────────────────────────────────────────
    /// Manage authentication
    #[command(display_order = 30)]
    Auth {
        #[command(subcommand)]
        command: auth::AuthCommand,
    },
    /// Manage saved configuration profiles
    #[command(display_order = 31)]
    Profile {
        #[command(subcommand)]
        command: profile::ProfileCommand,
    },
    /// Inspect and manage async job history
    #[command(display_order = 32)]
    Jobs {
        #[command(subcommand)]
        command: jobs::JobsCommand,
    },
    /// Report CLI friction or issues for improvement
    #[command(display_order = 33)]
    Feedback {
        #[command(subcommand)]
        command: feedback::FeedbackCommand,
    },
    /// Machine-readable CLI schema for agent introspection
    #[command(name = "agent-context", display_order = 34)]
    AgentContext,
}
