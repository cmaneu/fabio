use anyhow::Result;
use clap::Subcommand;

use crate::cli::Cli;
use crate::output;
use crate::token_cache;

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Log in to Microsoft Fabric via device code flow or service principal
    Login {
        /// Azure AD tenant ID (defaults to "common" for multi-tenant; required for --service-principal)
        #[arg(long)]
        tenant: Option<String>,

        /// `OAuth2` scope (defaults to Fabric API scope)
        #[arg(long)]
        scope: Option<String>,

        /// Authenticate as a service principal (requires --tenant and --client-id)
        #[arg(long)]
        service_principal: bool,

        /// Application (client) ID of the service principal
        #[arg(long)]
        client_id: Option<String>,

        /// Client secret for service principal authentication
        #[arg(long)]
        client_secret: Option<String>,

        /// Path to a PEM or PFX certificate file for service principal authentication
        #[arg(long)]
        certificate: Option<String>,

        /// Password for the certificate file (PFX/PKCS12)
        #[arg(long)]
        certificate_password: Option<String>,

        /// Federated token (OIDC assertion) for workload identity authentication
        #[arg(long)]
        federated_token: Option<String>,

        /// Path to a file containing the federated token (OIDC assertion)
        #[arg(long)]
        federated_token_file: Option<String>,
    },
    /// Log out and clear cached credentials
    Logout,
    /// Show current authentication status and credential source
    Status,
}

pub async fn execute(cli: &Cli, command: &AuthCommand) -> Result<()> {
    match command {
        AuthCommand::Login {
            tenant,
            scope,
            service_principal,
            client_id,
            client_secret,
            certificate,
            certificate_password,
            federated_token,
            federated_token_file,
        } => {
            if *service_principal {
                login_service_principal(
                    cli,
                    tenant.as_deref(),
                    scope.as_deref(),
                    client_id.as_deref(),
                    client_secret.as_deref(),
                    certificate.as_deref(),
                    certificate_password.as_deref(),
                    federated_token.as_deref(),
                    federated_token_file.as_deref(),
                )
                .await
            } else {
                login(cli, tenant.as_deref(), scope.as_deref()).await
            }
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

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn login_service_principal(
    cli: &Cli,
    tenant: Option<&str>,
    scope: Option<&str>,
    client_id: Option<&str>,
    client_secret: Option<&str>,
    certificate: Option<&str>,
    certificate_password: Option<&str>,
    federated_token: Option<&str>,
    federated_token_file: Option<&str>,
) -> Result<()> {
    use crate::errors::{ErrorCode, FabioError};

    let tenant = tenant
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                "--tenant is required for service principal authentication.",
                "Example: fabio auth login --service-principal --tenant <TENANT_ID> --client-id <CLIENT_ID> --client-secret <SECRET>".to_string(),
            )
        })?;

    let client_id = client_id
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                "--client-id is required for service principal authentication.",
                "Example: fabio auth login --service-principal --tenant <TENANT_ID> --client-id <CLIENT_ID> --client-secret <SECRET>".to_string(),
            )
        })?;

    let scope = scope.unwrap_or("https://api.fabric.microsoft.com/.default");

    // Filter empty strings to treat them as "not provided"
    let client_secret = client_secret.filter(|s| !s.is_empty());
    let certificate = certificate.filter(|s| !s.is_empty());
    let federated_token = federated_token.filter(|s| !s.is_empty());
    let federated_token_file = federated_token_file.filter(|s| !s.is_empty());

    // Determine credential type: secret vs certificate vs federated token
    let has_secret = client_secret.is_some();
    let has_cert = certificate.is_some();
    let has_federated = federated_token.is_some() || federated_token_file.is_some();

    let credential_count = u8::from(has_secret) + u8::from(has_cert) + u8::from(has_federated);
    if credential_count == 0 {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Service principal login requires one of: --client-secret, --certificate, or --federated-token/--federated-token-file.",
            "Example: fabio auth login --service-principal --tenant <T> --client-id <C> --client-secret <S>".to_string(),
        ).into());
    }
    if credential_count > 1 {
        return Err(FabioError::new(
            ErrorCode::InvalidInput,
            "Only one credential type allowed: --client-secret, --certificate, or --federated-token/--federated-token-file.",
        ).into());
    }

    let data = if has_secret {
        token_cache::sp_login_secret(tenant, client_id, client_secret.unwrap(), scope).await?
    } else if has_cert {
        token_cache::sp_login_certificate(
            tenant,
            client_id,
            certificate.unwrap(),
            certificate_password,
            scope,
        )
        .await?
    } else {
        // Federated token: prefer inline token over file
        let token_value = if let Some(token) = federated_token {
            token.to_string()
        } else {
            let path = federated_token_file.unwrap();
            let content = std::fs::read_to_string(path).map_err(|e| {
                FabioError::new(
                    ErrorCode::InvalidInput,
                    format!("Failed to read federated token file '{path}': {e}"),
                )
            })?;
            let trimmed = content.trim().to_string();
            if trimmed.is_empty() {
                return Err(FabioError::new(
                    ErrorCode::InvalidInput,
                    format!("Federated token file '{path}' is empty."),
                )
                .into());
            }
            trimmed
        };
        token_cache::sp_login_federated(tenant, client_id, &token_value, scope).await?
    };

    let expires_in = data.expires_on.saturating_sub(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    );

    let method = if has_secret {
        "client_secret"
    } else if has_cert {
        "certificate"
    } else {
        "federated_token"
    };

    let obj = serde_json::json!({
        "status": "logged_in",
        "credential_source": "service_principal",
        "method": method,
        "tenant": data.tenant,
        "client_id": client_id,
        "expires_in_seconds": expires_in,
        "message": format!("Successfully authenticated service principal via {method}. Token cached at ~/.fabio/token_cache.json")
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
                "hint": "Run 'fabio auth login' to authenticate via device code flow, or use --service-principal for non-interactive auth."
            });
            output::render_object(cli, &obj, "status");
        }
    }
    Ok(())
}
