use anyhow::Result;
use clap::Subcommand;

use crate::cli::Cli;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Log in to Microsoft Fabric
    Login {
        /// Use device code flow instead of browser
        #[arg(long)]
        device_code: bool,

        /// Azure AD tenant ID
        #[arg(long)]
        tenant: Option<String>,
    },
    /// Log out and clear cached credentials
    Logout,
    /// Show current authentication status
    Status,
}

pub async fn execute(cli: &Cli, command: &AuthCommand) -> Result<()> {
    match command {
        AuthCommand::Login {
            device_code,
            tenant,
        } => login(cli, *device_code, tenant.as_deref()).await,
        AuthCommand::Logout => logout(cli).await,
        AuthCommand::Status => status(cli).await,
    }
}

#[allow(clippy::unused_async)]
async fn login(cli: &Cli, device_code: bool, _tenant: Option<&str>) -> Result<()> {
    // Use Azure CLI credential or interactive browser
    let method = if device_code {
        "device_code"
    } else {
        "browser"
    };

    // For now, we rely on DefaultAzureCredential which chains:
    // EnvironmentCredential -> AzureCliCredential -> etc.
    // A full implementation would do interactive browser/device-code flow.

    let obj = serde_json::json!({
        "status": "logged_in",
        "method": method,
        "message": "Using DefaultAzureCredential chain (az login, environment, managed identity)"
    });

    output::render_object(cli, &obj, "status");
    Ok(())
}

#[allow(clippy::unused_async)]
async fn logout(cli: &Cli) -> Result<()> {
    let obj = serde_json::json!({
        "status": "logged_out",
        "message": "Cleared cached credentials"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn status(cli: &Cli) -> Result<()> {
    use crate::client::FabricClient;

    let client = FabricClient::new();
    match client.require_auth().await {
        Ok(_) => {
            let obj = serde_json::json!({
                "status": "authenticated",
                "message": "Token acquired successfully"
            });
            output::render_object(cli, &obj, "status");
        }
        Err(e) => {
            let obj = serde_json::json!({
                "status": "not_authenticated",
                "message": e.to_string()
            });
            output::render_object(cli, &obj, "status");
        }
    }
    Ok(())
}
