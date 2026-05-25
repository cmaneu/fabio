use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum OnelakeSecurityCommand {
    /// List data access roles for an item
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Item ID (e.g., lakehouse ID)
        #[arg(long)]
        id: String,
    },
    /// Show details of a data access role
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Data access role name
        #[arg(long)]
        role_name: String,
    },
    /// Create or replace all data access roles for an item
    #[command(display_order = 3)]
    Upsert {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Roles definition as JSON (array of role objects)
        /// or path to a JSON file (prefix with @)
        #[arg(long)]
        roles: String,
    },
    /// Delete a data access role
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Data access role name to delete
        #[arg(long)]
        role_name: String,
    },
}

pub async fn execute(
    cli: &Cli,
    client: &FabricClient,
    command: &OnelakeSecurityCommand,
) -> Result<()> {
    match command {
        OnelakeSecurityCommand::List { workspace, id } => list(cli, client, workspace, id).await,
        OnelakeSecurityCommand::Show {
            workspace,
            id,
            role_name,
        } => show(cli, client, workspace, id, role_name).await,
        OnelakeSecurityCommand::Upsert {
            workspace,
            id,
            roles,
        } => upsert(cli, client, workspace, id, roles).await,
        OnelakeSecurityCommand::Delete {
            workspace,
            id,
            role_name,
        } => delete(cli, client, workspace, id, role_name).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str, item_id: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/items/{item_id}/dataAccessRoles"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["name", "decisionRules", "members"],
        &["NAME", "DECISION RULES", "MEMBERS"],
        "name",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn show(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    role_name: &str,
) -> Result<()> {
    // The API doesn't have a single-role GET; list and filter
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/items/{item_id}/dataAccessRoles"),
            "value",
            true,
            None,
        )
        .await?;

    let role = resp
        .items
        .iter()
        .find(|r| r.get("name").and_then(Value::as_str) == Some(role_name));

    match role {
        Some(r) => output::render_object(cli, r, "name"),
        None => {
            return Err(FabioError::with_hint(
                ErrorCode::NotFound,
                format!("Data access role '{role_name}' not found"),
                "Use 'fabio onelake-security list' to see available roles".to_string(),
            )
            .into());
        }
    }
    Ok(())
}

async fn upsert(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    roles: &str,
) -> Result<()> {
    let roles_value: Value = if let Some(path) = roles.strip_prefix('@') {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?;
        serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid JSON in file '{path}': {e}"))?
    } else {
        serde_json::from_str(roles).map_err(|e| anyhow::anyhow!("Invalid --roles JSON: {e}"))?
    };

    let body = serde_json::json!({ "value": roles_value });

    if output::dry_run_guard(cli, "onelake-security upsert", &body) {
        return Ok(());
    }

    client
        .put(
            &format!("/workspaces/{workspace}/items/{item_id}/dataAccessRoles"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "onelake-security upsert", "Admin"))?;

    let obj = serde_json::json!({
        "itemId": item_id,
        "status": "roles_updated"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn delete(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    role_name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "onelake-security delete",
        &serde_json::json!({
            "workspace": workspace,
            "itemId": item_id,
            "roleName": role_name
        }),
    ) {
        return Ok(());
    }

    // The Fabric API uses PUT with the full list minus the role to delete.
    // We need to: 1) list, 2) remove the role, 3) PUT back
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/items/{item_id}/dataAccessRoles"),
            "value",
            true,
            None,
        )
        .await?;

    let remaining: Vec<&Value> = resp
        .items
        .iter()
        .filter(|r| r.get("name").and_then(Value::as_str) != Some(role_name))
        .collect();

    if remaining.len() == resp.items.len() {
        return Err(FabioError::with_hint(
            ErrorCode::NotFound,
            format!("Data access role '{role_name}' not found"),
            "Use 'fabio onelake-security list' to see available roles".to_string(),
        )
        .into());
    }

    let body = serde_json::json!({ "value": remaining });

    client
        .put(
            &format!("/workspaces/{workspace}/items/{item_id}/dataAccessRoles"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "onelake-security delete", "Admin"))?;

    let obj = serde_json::json!({
        "roleName": role_name,
        "status": "deleted"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}
