pub mod auth;
pub mod dataagent;
pub mod feedback;
pub mod git;
pub mod item;
pub mod jobs;
pub mod lakehouse;
pub mod notebook;
pub mod ontology;
pub mod profile;
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
        Command::Auth { command } => auth::execute(&cli, command).await,
        Command::Workspace { command } => workspace::execute(&cli, &client, command).await,
        Command::Item { command } => item::execute(&cli, &client, command).await,
        Command::Lakehouse { command } => lakehouse::execute(&cli, &client, command).await,
        Command::Notebook { command } => notebook::execute(&cli, &client, command).await,
        Command::Warehouse { command } => warehouse::execute(&cli, &client, command).await,
        Command::DataAgent { command } => dataagent::execute(&cli, &client, command).await,
        Command::Git { command } => git::execute(&cli, &client, command).await,
        Command::Ontology { command } => ontology::execute(&cli, &client, command).await,
        Command::Profile { command } => profile::execute(&cli, command),
        Command::Jobs { command } => jobs::execute(&cli, command),
        Command::Feedback { command } => feedback::execute(&cli, command),
        Command::AgentContext => agent_context::execute(&cli),
    }
}
