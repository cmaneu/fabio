use clap::{Parser, Subcommand, ValueEnum};

use crate::commands::{
    auth, capacity, connection, copy_job, data_pipeline, dataagent, dataflow, deployment_pipeline,
    domain, environment, eventhouse, eventstream, feedback, gateway, git, graphql_api, item,
    job_scheduler, jobs, kql_dashboard, kql_database, kql_queryset, lakehouse,
    managed_private_endpoint, mirrored_database, ml_experiment, ml_model, notebook,
    onelake_security, ontology, profile, reflex, report, semantic_model, spark,
    spark_job_definition, sql_database, sql_endpoint, warehouse, workspace,
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

    /// Resume pagination from a specific continuation token (returned by a previous list call)
    #[arg(long, global = true)]
    pub continuation_token: Option<String>,

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
    /// Manage SQL databases (Fabric-native transactional databases)
    #[command(display_order = 12)]
    SqlDatabase {
        #[command(subcommand)]
        command: sql_database::SqlDatabaseCommand,
    },
    /// Manage SQL endpoints (analytics endpoints for lakehouses)
    #[command(display_order = 13)]
    SqlEndpoint {
        #[command(subcommand)]
        command: sql_endpoint::SqlEndpointCommand,
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
    /// Manage copy jobs (data movement)
    #[command(display_order = 16)]
    CopyJob {
        #[command(subcommand)]
        command: copy_job::CopyJobCommand,
    },
    /// Manage dataflows (Power BI data transformation)
    #[command(display_order = 17)]
    Dataflow {
        #[command(subcommand)]
        command: dataflow::DataflowCommand,
    },
    /// Manage reports (Power BI)
    #[command(display_order = 18)]
    Report {
        #[command(subcommand)]
        command: report::ReportCommand,
    },
    /// Manage semantic models (Power BI datasets)
    #[command(display_order = 19)]
    SemanticModel {
        #[command(subcommand)]
        command: semantic_model::SemanticModelCommand,
    },
    /// Manage eventhouses (real-time analytics)
    #[command(display_order = 20)]
    Eventhouse {
        #[command(subcommand)]
        command: eventhouse::EventhouseCommand,
    },
    /// Manage eventstreams (real-time data ingestion)
    #[command(display_order = 21)]
    Eventstream {
        #[command(subcommand)]
        command: eventstream::EventstreamCommand,
    },
    /// Manage KQL databases (within eventhouses)
    #[command(display_order = 22)]
    KqlDatabase {
        #[command(subcommand)]
        command: kql_database::KqlDatabaseCommand,
    },
    /// Manage KQL querysets (saved KQL queries)
    #[command(display_order = 23)]
    KqlQueryset {
        #[command(subcommand)]
        command: kql_queryset::KqlQuerysetCommand,
    },
    /// Manage KQL dashboards (real-time dashboards)
    #[command(display_order = 24)]
    KqlDashboard {
        #[command(subcommand)]
        command: kql_dashboard::KqlDashboardCommand,
    },
    /// Manage mirrored databases (real-time replication)
    #[command(display_order = 25)]
    MirroredDatabase {
        #[command(subcommand)]
        command: mirrored_database::MirroredDatabaseCommand,
    },
    /// Manage Reflex items (Data Activator triggers and alerts)
    #[command(display_order = 26)]
    Reflex {
        #[command(subcommand)]
        command: reflex::ReflexCommand,
    },
    /// Manage ML models (data science)
    #[command(display_order = 27)]
    MlModel {
        #[command(subcommand)]
        command: ml_model::MlModelCommand,
    },
    /// Manage ML experiments (data science)
    #[command(display_order = 28)]
    MlExperiment {
        #[command(subcommand)]
        command: ml_experiment::MlExperimentCommand,
    },
    /// Manage Spark compute (settings, custom pools)
    #[command(display_order = 29)]
    Spark {
        #[command(subcommand)]
        command: spark::SparkCommand,
    },
    /// Manage Spark job definitions (batch Spark jobs)
    #[command(display_order = 30)]
    SparkJobDefinition {
        #[command(subcommand)]
        command: spark_job_definition::SparkJobDefinitionCommand,
    },
    /// Manage GraphQL APIs
    #[command(display_order = 31)]
    GraphqlApi {
        #[command(subcommand)]
        command: graphql_api::GraphqlApiCommand,
    },

    // ── Integration ──────────────────────────────────────────────────────
    /// Manage gateways (on-premises, `VNet`, members, role assignments)
    #[command(display_order = 45)]
    Gateway {
        #[command(subcommand)]
        command: gateway::GatewayCommand,
    },
    /// Manage Git integration (connect, commit, pull, status)
    #[command(display_order = 40)]
    Git {
        #[command(subcommand)]
        command: git::GitCommand,
    },
    /// Manage connections (cloud, on-premises, virtual network)
    #[command(display_order = 41)]
    Connection {
        #[command(subcommand)]
        command: connection::ConnectionCommand,
    },
    /// Manage deployment pipelines (CI/CD stages, deploy items)
    #[command(display_order = 42)]
    DeploymentPipeline {
        #[command(subcommand)]
        command: deployment_pipeline::DeploymentPipelineCommand,
    },
    /// Manage domains (organize workspaces into business domains)
    #[command(display_order = 43)]
    Domain {
        #[command(subcommand)]
        command: domain::DomainCommand,
    },
    /// Manage item job scheduling (run, cancel, schedules)
    #[command(display_order = 44)]
    JobScheduler {
        #[command(subcommand)]
        command: job_scheduler::JobSchedulerCommand,
    },

    // ── Security & Governance ────────────────────────────────────────────
    /// Manage `OneLake` data access roles (row/column-level security)
    #[command(display_order = 50)]
    OnelakeSecurity {
        #[command(subcommand)]
        command: onelake_security::OnelakeSecurityCommand,
    },
    /// Manage workspace managed private endpoints
    #[command(display_order = 51)]
    ManagedPrivateEndpoint {
        #[command(subcommand)]
        command: managed_private_endpoint::ManagedPrivateEndpointCommand,
    },

    // ── Configuration ────────────────────────────────────────────────────
    /// Manage authentication
    #[command(display_order = 60)]
    Auth {
        #[command(subcommand)]
        command: auth::AuthCommand,
    },
    /// Manage saved configuration profiles
    #[command(display_order = 61)]
    Profile {
        #[command(subcommand)]
        command: profile::ProfileCommand,
    },
    /// Inspect and manage async job history
    #[command(display_order = 62)]
    Jobs {
        #[command(subcommand)]
        command: jobs::JobsCommand,
    },
    /// Report CLI friction or issues for improvement
    #[command(display_order = 63)]
    Feedback {
        #[command(subcommand)]
        command: feedback::FeedbackCommand,
    },
    /// Machine-readable CLI schema for agent introspection
    #[command(name = "agent-context", display_order = 64)]
    AgentContext,
}
