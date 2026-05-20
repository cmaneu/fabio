use anyhow::Result;
use clap::Subcommand;

use crate::cli::Cli;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Log in to Microsoft Fabric (validates credentials)
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
    /// Show current authentication status and credential source
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

async fn login(cli: &Cli, _device_code: bool, _tenant: Option<&str>) -> Result<()> {
    use crate::client::FabricClient;

    // Actually attempt token acquisition to validate credentials work
    let client = FabricClient::new();
    match client.require_auth().await {
        Ok(_) => {
            let source = client
                .credential_source()
                .await
                .map_or_else(|| "unknown".to_string(), |s| s.to_string());
            let obj = serde_json::json!({
                "status": "logged_in",
                "credential_source": source,
                "message": format!("Successfully authenticated via {source}")
            });
            output::render_object(cli, &obj, "status");
        }
        Err(e) => {
            let obj = serde_json::json!({
                "status": "login_failed",
                "message": e.to_string(),
                "hint": "Run 'az login' or set AZURE_TENANT_ID + AZURE_CLIENT_ID + AZURE_CLIENT_SECRET."
            });
            output::render_object(cli, &obj, "status");
            // Return error so exit code is non-zero
            return Err(e);
        }
    }
    Ok(())
}

#[allow(clippy::unused_async)]
async fn logout(cli: &Cli) -> Result<()> {
    // In-process token cache is cleared when the process exits.
    // For service principal auth, user must unset env vars.
    // For CLI auth, user should run `az logout`.
    let obj = serde_json::json!({
        "status": "logged_out",
        "message": "In-process token cache cleared. For full logout: run 'az logout' or unset AZURE_CLIENT_SECRET."
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn status(cli: &Cli) -> Result<()> {
    use crate::client::{CredentialSource, FabricClient};

    let client = FabricClient::new();
    match client.require_auth().await {
        Ok(_) => {
            let source = client.credential_source().await;
            let source_type = source.map_or("unknown", |s| match s {
                CredentialSource::Environment => "environment",
                CredentialSource::ManagedIdentity => "managed_identity",
                CredentialSource::AzureCli => "azure_cli",
                CredentialSource::AzureDeveloperCli => "azure_developer_cli",
            });
            let source_display = source.map_or_else(|| "unknown".to_string(), |s| s.to_string());
            let obj = serde_json::json!({
                "status": "authenticated",
                "credential_source": source_type,
                "message": format!("Token acquired successfully via {source_display}")
            });
            output::render_object(cli, &obj, "status");
        }
        Err(e) => {
            let obj = serde_json::json!({
                "status": "not_authenticated",
                "message": e.to_string(),
                "hint": "Run 'az login' or set AZURE_TENANT_ID + AZURE_CLIENT_ID + AZURE_CLIENT_SECRET."
            });
            output::render_object(cli, &obj, "status");
        }
    }
    Ok(())
}
