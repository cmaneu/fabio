use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

/// Microsoft Graph base URL for sensitivity label queries.
/// Uses beta because the v1.0 /security/informationProtection segment is not yet GA.
const GRAPH_BASE_URL: &str = "https://graph.microsoft.com/beta";

#[derive(Debug, Subcommand)]
pub enum LabelCommand {
    /// List available sensitivity labels (from Microsoft Purview via Graph API)
    #[command(display_order = 1)]
    List,
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &LabelCommand) -> Result<()> {
    match command {
        LabelCommand::List => list(cli, client).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient) -> Result<()> {
    let token = client.require_graph_auth().await.map_err(|e| {
        // Wrap auth errors with a helpful hint about Graph permissions
        let fabio_err = e.downcast_ref::<FabioError>();
        if fabio_err.is_some() {
            FabioError::with_hint(
                ErrorCode::AuthRequired,
                "Failed to acquire Microsoft Graph token for sensitivity label resolution"
                    .to_string(),
                "This command requires InformationProtection.Read permission on Microsoft Graph. \
                 Ensure your identity (user or service principal) has this permission granted in \
                 Microsoft Entra ID. If using az login, try: \
                 az login --scope https://graph.microsoft.com/.default"
                    .to_string(),
            )
            .into()
        } else {
            e
        }
    })?;

    let url = format!("{GRAPH_BASE_URL}/security/informationProtection/sensitivityLabels");

    let resp = client
        .http()
        .get(&url)
        .header("Authorization", &token)
        .send()
        .await
        .map_err(|e| {
            FabioError::with_hint(
                ErrorCode::ApiError,
                format!("Failed to call Microsoft Graph: {e}"),
                "Check network connectivity to graph.microsoft.com".to_string(),
            )
        })?;

    let status = resp.status();
    let body: Value = resp.json().await.map_err(|e| {
        FabioError::new(
            ErrorCode::ApiError,
            format!("Failed to parse Graph response: {e}"),
        )
    })?;

    if !status.is_success() {
        let error_msg = body
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Unknown error from Microsoft Graph");

        let error_code = body
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        // Provide specific guidance for common auth failures
        let hint = if error_code == "Authorization_RequestDenied"
            || error_msg.contains("Insufficient privileges")
            || status.as_u16() == 403
        {
            "Your identity lacks the InformationProtection.Read permission on Microsoft Graph. \
             This requires: (1) M365 E5 licensing, (2) Microsoft Purview configured in the \
             tenant, and (3) InformationProtection.Read permission granted in Entra ID. \
             If using az login, try: az login --scope https://graph.microsoft.com/.default"
                .to_string()
        } else if status.as_u16() == 401 {
            "Authentication failed for Microsoft Graph. Try re-authenticating: \
             az login --scope https://graph.microsoft.com/.default"
                .to_string()
        } else {
            format!("Microsoft Graph returned HTTP {status}: {error_code}")
        };

        return Err(FabioError::with_hint(
            ErrorCode::ApiError,
            format!("Graph API error: {error_msg}"),
            hint,
        )
        .into());
    }

    // Extract the labels array from the response
    let labels = body
        .get("value")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    output::render_list_with_token(
        cli,
        &labels,
        &["name", "id", "description"],
        &["NAME", "ID", "DESCRIPTION"],
        "id",
        None,
    );
    Ok(())
}
