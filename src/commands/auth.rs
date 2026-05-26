use anyhow::Result;
use clap::Subcommand;

use crate::cli::Cli;
use crate::output;
use crate::token_cache;

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Log in to Microsoft Fabric via device code flow (independent of Azure CLI)
    Login {
        /// Azure AD tenant ID (defaults to "common" for multi-tenant)
        #[arg(long)]
        tenant: Option<String>,

        /// `OAuth2` scope (defaults to Fabric API scope)
        #[arg(long)]
        scope: Option<String>,
    },
    /// Log out and clear cached credentials
    Logout,
    /// Show current authentication status and credential source
    Status,
}

pub async fn execute(cli: &Cli, command: &AuthCommand) -> Result<()> {
    match command {
        AuthCommand::Login { tenant, scope } => {
            login(cli, tenant.as_deref(), scope.as_deref()).await
        }
        AuthCommand::Logout => logout(cli),
        AuthCommand::Status => status(cli).await,
    }
}

async fn login(cli: &Cli, tenant: Option<&str>, scope: Option<&str>) -> Result<()> {
    let data = token_cache::device_code_login(tenant, scope).await?;

    let expires_in = data.expires_on.saturating_sub(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    );

    let obj = serde_json::json!({
        "status": "logged_in",
        "credential_source": "fabio_device_code",
        "tenant": data.tenant,
        "expires_in_seconds": expires_in,
        "message": "Successfully authenticated via device code flow. Token cached at ~/.fabio/token_cache.json"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

fn logout(cli: &Cli) -> Result<()> {
    token_cache::clear_cache()?;

    let obj = serde_json::json!({
        "status": "logged_out",
        "message": "Token cache cleared. Run 'fabio auth login' to authenticate again."
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
                CredentialSource::FabioCache => "fabio_cache",
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
                "hint": "Run 'fabio auth login' to authenticate via device code flow."
            });
            output::render_object(cli, &obj, "status");
        }
    }
    Ok(())
}
