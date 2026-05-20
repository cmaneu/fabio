use anyhow::Result;
use clap::Subcommand;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum CapacityCommand {
    /// List capacities available to the caller
    #[command(display_order = 1)]
    List,
    /// Show details of a specific capacity
    #[command(display_order = 2)]
    Show {
        /// Capacity ID
        #[arg(long)]
        id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &CapacityCommand) -> Result<()> {
    match command {
        CapacityCommand::List => list(cli, client).await,
        CapacityCommand::Show { id } => show(cli, client, id).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/capacities",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id", "sku", "region", "state"],
        &["NAME", "ID", "SKU", "REGION", "STATE"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn show(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    let data = client.get(&format!("/capacities/{id}")).await?;
    output::render_object(cli, &data, "id");
    Ok(())
}
