use anyhow::{Result, bail};
use clap::Subcommand;
use serde_json::{Value, json};

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum ConnectionCommand {
    /// List all connections you have permission to access
    #[command(display_order = 1)]
    List,
    /// Show details of a specific connection
    #[command(display_order = 2)]
    Show {
        /// Connection ID
        #[arg(long)]
        id: String,
    },
    /// Create a new connection
    #[command(display_order = 3)]
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

        /// Skip connection test during creation
        #[arg(long, default_value_t = false)]
        skip_test_connection: bool,
    },
    /// Update a connection's name, credentials, or privacy level
    #[command(display_order = 4)]
    Update {
        /// Connection ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New privacy level
        #[arg(long, value_parser = ["None", "Public", "Organizational", "Private"])]
        privacy_level: Option<String>,

        /// New credential type
        #[arg(long, value_parser = ["Basic", "OAuth2", "Key", "Anonymous", "ServicePrincipal", "SharedAccessSignature"])]
        credential_type: Option<String>,

        /// New credentials as JSON
        #[arg(long)]
        credentials: Option<String>,
    },
    /// Delete a connection
    #[command(display_order = 5)]
    Delete {
        /// Connection ID
        #[arg(long)]
        id: String,
    },
    /// List supported connection types (gateway types catalog)
    #[command(display_order = 10)]
    ListSupportedTypes,
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
            skip_test_connection,
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
                *skip_test_connection,
            )
            .await
        }
        ConnectionCommand::Update {
            id,
            name,
            privacy_level,
            credential_type,
            credentials,
        } => {
            update(
                cli,
                client,
                id,
                name.as_deref(),
                privacy_level.as_deref(),
                credential_type.as_deref(),
                credentials.as_deref(),
            )
            .await
        }
        ConnectionCommand::Delete { id } => delete(cli, client, id).await,
        ConnectionCommand::ListSupportedTypes => list_supported_types(cli, client).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client.get_list("/connections", "value", cli.all).await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id", "connectivityType"],
        &["NAME", "ID", "CONNECTIVITY TYPE"],
        "id",
        resp.continuation_token.as_deref(),
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
    skip_test_connection: bool,
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
        let cred_value: Value = serde_json::from_str(creds)
            .map_err(|e| anyhow::anyhow!("Invalid --credentials JSON: {e}"))?;
        let mut details = json!({
            "singleSignOnType": "None",
            "connectionEncryption": "NotEncrypted",
            "skipTestConnection": skip_test_connection,
            "credentials": cred_value,
        });
        // Ensure credentialType is set inside credentials
        if details["credentials"]["credentialType"].is_null() {
            details["credentials"]["credentialType"] = json!(credential_type);
        }
        details
    } else {
        json!({
            "singleSignOnType": "None",
            "connectionEncryption": "NotEncrypted",
            "skipTestConnection": skip_test_connection,
            "credentials": {
                "credentialType": credential_type,
            },
        })
    };

    // Build connection parameters in the API array format
    let connection_params: Vec<Value> = if let Some(obj) = params.as_object() {
        obj.iter()
            .map(|(k, v)| {
                json!({
                    "dataType": "Text",
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
            "creationMethod": connection_type,
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

#[allow(clippy::too_many_arguments)]
async fn update(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    name: Option<&str>,
    privacy_level: Option<&str>,
    credential_type: Option<&str>,
    credentials: Option<&str>,
) -> Result<()> {
    if name.is_none() && privacy_level.is_none() && credential_type.is_none() {
        bail!(
            "At least one of --name, --privacy-level, or --credential-type must be provided. Example: fabio connection update --id <ID> --name \"New Name\""
        );
    }

    let mut body = json!({});
    if let Some(n) = name {
        body["displayName"] = json!(n);
    }
    if let Some(pl) = privacy_level {
        body["privacyLevel"] = json!(pl);
    }
    if credential_type.is_some() || credentials.is_some() {
        let mut cred_details = json!({});
        if let Some(ct) = credential_type {
            cred_details["credentials"] = json!({ "credentialType": ct });
        }
        if let Some(creds) = credentials {
            let cred_value: Value = serde_json::from_str(creds)
                .map_err(|e| anyhow::anyhow!("Invalid --credentials JSON: {e}"))?;
            if cred_details["credentials"].is_null() {
                cred_details["credentials"] = cred_value;
            } else if let Some(obj) = cred_details["credentials"].as_object_mut() {
                if let Some(cred_obj) = cred_value.as_object() {
                    for (k, v) in cred_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        body["credentialDetails"] = cred_details;
    }

    if cli.dry_run {
        let preview = json!({
            "status": "dry_run",
            "message": format!("Would update connection '{id}'"),
            "updates": body,
        });
        output::render_object(cli, &preview, "status");
        return Ok(());
    }

    let data = client.patch(&format!("/connections/{id}"), &body).await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn list_supported_types(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list("/connections/supportedConnectionTypes", "value", cli.all)
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["name", "displayName"],
        &["TYPE", "DISPLAY NAME"],
        "name",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}
