use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
#[command(
    after_help = "For complete flag reference, run: fabio context agent\nReturns machine-readable JSON schema of all commands, flags, and types."
)]
pub enum RtiCommand {
    /// Convert natural language to a KQL query (beta)
    #[command(display_order = 1)]
    NlToKql {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Item ID used for billing (`KQLQueryset`, `KQLDashboard`, or `Eventhouse`)
        #[arg(long)]
        item_id: String,

        /// Kusto cluster URL (e.g., https://<id>.<region>.kusto.fabric.microsoft.com)
        #[arg(long)]
        cluster_url: String,

        /// Database name
        #[arg(long)]
        database_name: String,

        /// Natural language question to convert to KQL
        #[arg(long)]
        question: String,

        /// User-provided example shots as JSON array: [{"naturalLanguage":"...","kqlQuery":"..."}]
        #[arg(long)]
        user_shots: Option<String>,

        /// Chat messages for additional context as JSON array: [{"role":"User","content":"..."}]
        #[arg(long)]
        chat_messages: Option<String>,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &RtiCommand) -> Result<()> {
    match command {
        RtiCommand::NlToKql {
            workspace,
            item_id,
            cluster_url,
            database_name,
            question,
            user_shots,
            chat_messages,
        } => {
            nl_to_kql(
                cli,
                client,
                workspace,
                item_id,
                cluster_url,
                database_name,
                question,
                user_shots.as_deref(),
                chat_messages.as_deref(),
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn nl_to_kql(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    cluster_url: &str,
    database_name: &str,
    question: &str,
    user_shots: Option<&str>,
    chat_messages: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({
        "itemIdForBilling": item_id,
        "clusterUrl": cluster_url,
        "databaseName": database_name,
        "naturalLanguage": question
    });

    if let Some(shots_json) = user_shots {
        let shots: Value = serde_json::from_str(shots_json).map_err(|e| {
            anyhow::anyhow!(
                "Invalid --user-shots JSON: {e}. Expected: [{{\"naturalLanguage\":\"...\",\"kqlQuery\":\"...\"}}]"
            )
        })?;
        body["userShots"] = shots;
    }

    if let Some(messages_json) = chat_messages {
        let messages: Value = serde_json::from_str(messages_json).map_err(|e| {
            anyhow::anyhow!(
                "Invalid --chat-messages JSON: {e}. Expected: [{{\"role\":\"User\",\"content\":\"...\"}}]"
            )
        })?;
        body["chatMessages"] = messages;
    }

    if output::dry_run_guard(cli, "rti nl-to-kql", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/realTimeIntelligence/nltokql?beta=true"),
            &body,
            false,
        )
        .await?;

    output::render_object(cli, &data, "kqlQuery");
    Ok(())
}
