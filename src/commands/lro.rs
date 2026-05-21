use anyhow::Result;
use clap::Subcommand;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum LroCommand {
    /// Get the state of a long-running operation
    #[command(display_order = 1)]
    GetState {
        /// Operation ID
        #[arg(long)]
        operation_id: String,
    },
    /// Get the result of a completed long-running operation
    #[command(display_order = 2)]
    GetResult {
        /// Operation ID
        #[arg(long)]
        operation_id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &LroCommand) -> Result<()> {
    match command {
        LroCommand::GetState { operation_id } => get_state(cli, client, operation_id).await,
        LroCommand::GetResult { operation_id } => get_result(cli, client, operation_id).await,
    }
}

async fn get_state(cli: &Cli, client: &FabricClient, operation_id: &str) -> Result<()> {
    let data = client.get(&format!("/operations/{operation_id}")).await?;
    output::render_object(cli, &data, "status");
    Ok(())
}

async fn get_result(cli: &Cli, client: &FabricClient, operation_id: &str) -> Result<()> {
    let data = client
        .get(&format!("/operations/{operation_id}/result"))
        .await?;
    output::render_object(cli, &data, "status");
    Ok(())
}
