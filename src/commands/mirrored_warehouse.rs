use anyhow::Result;
use clap::Subcommand;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
#[command(after_help = "CONTEXT: fabio context agent")]
pub enum MirroredWarehouseCommand {
    /// List mirrored warehouses in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
}

pub async fn execute(
    cli: &Cli,
    client: &FabricClient,
    command: &MirroredWarehouseCommand,
) -> Result<()> {
    match command {
        MirroredWarehouseCommand::List { workspace } => list(cli, client, workspace).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/mirroredWarehouses"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id", "description"],
        &["NAME", "ID", "DESCRIPTION"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}
