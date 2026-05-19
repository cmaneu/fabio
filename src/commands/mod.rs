pub mod auth;
pub mod dataagent;
pub mod item;
pub mod lakehouse;
pub mod notebook;
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
        Command::AgentContext => agent_context::execute(&cli),
    }
}
