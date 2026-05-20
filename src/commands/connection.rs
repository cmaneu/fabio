use anyhow::{Result, bail};
use clap::Subcommand;
use serde_json::{Value, json};

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum ConnectionCommand {
    /// List all connections you have permission to access
    List,
    /// Show details of a specific connection
    Show {
        /// Connection ID
        #[arg(long)]
        id: String,
    },
    /// Create a new connection
    Create {
        /// Display name for the connection
        #[arg(long)]
        name: String,

        /// Connectivity type
        #[arg(long, value_name = "TYPE", value_parser = ["ShareableCloud", "OnPremises", "VirtualNetworkGateway", "PersonalCloud"])]
        connectivity_type: String,

        /// Connection type path (e.g., Web, SQL)
        #[arg(long, value_name = "TYPE")]
        connection_type: String,

        /// Connection parameters as JSON (e.g., '{"server":"host","database":"db"}')
        #[arg(long)]
        parameters: String,

        /// Credential type
        #[arg(long, value_parser = ["Basic", "OAuth2", "Key", "Anonymous", "ServicePrincipal", "SharedAccessSignature"])]
        credential_type: String,

        /// Credentials as JSON (format depends on credential type)
        #[arg(long)]
        credentials: Option<String>,

        /// Privacy level
        #[arg(long, default_value = "Organizational", value_parser = ["None", "Public", "Organizational", "Private"])]
        privacy_level: String,
    },
    /// Delete a connection
    Delete {
        /// Connection ID
        #[arg(long)]
        id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &ConnectionCommand) -> Result<()> {
    match command {
        ConnectionCommand::List => list(cli, client).await,
        ConnectionCommand::Show { id } => show(cli, client, id).await,
        ConnectionCommand::Create {
            name,
            connectivity_type,
            connection_type,
            parameters,
            credential_type,
            credentials,
            privacy_level,
        } => {
            create(
                cli,
                client,
                name,
                connectivity_type,
                connection_type,
                parameters,
                credential_type,
                credentials.as_deref(),
                privacy_level,
            )
            .await
        }
        ConnectionCommand::Delete { id } => delete(cli, client, id).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient) -> Result<()> {
    let data = client.get("/connections").await?;
    let items = data
        .get("value")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    output::render_list(
        cli,
        &items,
        &["displayName", "id", "connectivityType"],
        &["NAME", "ID", "CONNECTIVITY TYPE"],
        "id",
    );
    Ok(())
}

async fn show(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    let data = client.get(&format!("/connections/{id}")).await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create(
    cli: &Cli,
    client: &FabricClient,
    name: &str,
    connectivity_type: &str,
    connection_type: &str,
    parameters: &str,
    credential_type: &str,
    credentials: Option<&str>,
    privacy_level: &str,
) -> Result<()> {
    if cli.dry_run {
        let preview = json!({
            "status": "dry_run",
            "message": format!("Would create connection '{name}' ({connectivity_type})"),
            "displayName": name,
            "connectivityType": connectivity_type,
        });
        output::render_object(cli, &preview, "status");
        return Ok(());
    }

    let params: Value = serde_json::from_str(parameters).map_err(|e| {
        anyhow::anyhow!("Invalid --parameters JSON: {e}. Expected format: '{{\"key\":\"value\"}}'")
    })?;

    let cred_details = if let Some(creds) = credentials {
        let cred_value: Value = serde_json::from_str(creds).map_err(|e| {
            anyhow::anyhow!("Invalid --credentials JSON: {e}")
        })?;
        json!({
            "credentialType": credential_type,
            "credentials": cred_value,
        })
    } else {
        json!({
            "credentialType": credential_type,
        })
    };

    // Build connection parameters in the API format
    let connection_params: Vec<Value> = if let Some(obj) = params.as_object() {
        obj.iter()
            .map(|(k, v)| {
                json!({
                    "name": k,
                    "value": v.as_str().unwrap_or(&v.to_string()),
                })
            })
            .collect()
    } else {
        bail!("--parameters must be a JSON object (e.g., '{{\"server\":\"host\"}}')");
    };

    let body = json!({
        "displayName": name,
        "connectivityType": connectivity_type,
        "connectionDetails": {
            "type": connection_type,
            "parameters": connection_params,
        },
        "credentialDetails": cred_details,
        "privacyLevel": privacy_level,
    });

    let data = client.post("/connections", &body, false).await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    if cli.dry_run {
        let preview = json!({
            "status": "dry_run",
            "message": format!("Would delete connection '{id}'"),
        });
        output::render_object(cli, &preview, "status");
        return Ok(());
    }

    client.delete(&format!("/connections/{id}")).await?;

    let result = json!({
        "status": "deleted",
        "id": id,
    });
    output::render_object(cli, &result, "id");
    Ok(())
}
