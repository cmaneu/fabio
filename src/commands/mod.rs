pub mod auth;
pub mod capacity;
pub mod connection;
pub mod data_pipeline;
pub mod dataagent;
pub mod deployment_pipeline;
pub mod domain;
pub mod environment;
pub mod eventhouse;
pub mod feedback;
pub mod git;
pub mod item;
pub mod job_scheduler;
pub mod jobs;
pub mod kql_database;
pub mod lakehouse;
pub mod managed_private_endpoint;
pub mod mirrored_database;
pub mod notebook;
pub mod onelake_security;
pub mod ontology;
pub mod profile;
pub mod spark;
pub mod warehouse;
pub mod workspace;

mod agent_context;

use anyhow::Result;

use crate::cli::{Cli, Command};
use crate::client::FabricClient;

/// Execute the CLI command.
pub async fn execute(cli: Cli) -> Result<()> {
    let client = FabricClient::new();

    match &cli.command {
        // Core
        Command::Workspace { command } => workspace::execute(&cli, &client, command).await,
        Command::Item { command } => item::execute(&cli, &client, command).await,
        Command::Lakehouse { command } => lakehouse::execute(&cli, &client, command).await,
        Command::Capacity { command } => capacity::execute(&cli, &client, command).await,
        // Data & Compute
        Command::Notebook { command } => notebook::execute(&cli, &client, command).await,
        Command::Warehouse { command } => warehouse::execute(&cli, &client, command).await,
        Command::DataAgent { command } => dataagent::execute(&cli, &client, command).await,
        Command::Ontology { command } => ontology::execute(&cli, &client, command).await,
        Command::Environment { command } => environment::execute(&cli, &client, command).await,
        Command::DataPipeline { command } => data_pipeline::execute(&cli, &client, command).await,
        Command::Eventhouse { command } => eventhouse::execute(&cli, &client, command).await,
        Command::KqlDatabase { command } => kql_database::execute(&cli, &client, command).await,
        Command::MirroredDatabase { command } => {
            mirrored_database::execute(&cli, &client, command).await
        }
        Command::Spark { command } => spark::execute(&cli, &client, command).await,
        // Integration
        Command::Git { command } => git::execute(&cli, &client, command).await,
        Command::Connection { command } => connection::execute(&cli, &client, command).await,
        Command::DeploymentPipeline { command } => {
            deployment_pipeline::execute(&cli, &client, command).await
        }
        Command::Domain { command } => domain::execute(&cli, &client, command).await,
        Command::JobScheduler { command } => job_scheduler::execute(&cli, &client, command).await,
        // Security & Governance
        Command::OnelakeSecurity { command } => {
            onelake_security::execute(&cli, &client, command).await
        }
        Command::ManagedPrivateEndpoint { command } => {
            managed_private_endpoint::execute(&cli, &client, command).await
        }
        // Configuration
        Command::Auth { command } => auth::execute(&cli, command).await,
        Command::Profile { command } => profile::execute(&cli, command),
        Command::Jobs { command } => jobs::execute(&cli, command),
        Command::Feedback { command } => feedback::execute(&cli, command),
        Command::AgentContext => agent_context::execute(&cli),
    }
}
