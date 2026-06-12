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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Item ID (e.g., lakehouse ID)
        #[arg(long)]
        id: String,
    },
    /// Show details of a data access role
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Data access role name
        #[arg(long)]
        role_name: String,
    },
    /// Create or update a single data access role
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Role definition as JSON or path to JSON file (prefix with @)
        #[arg(long)]
        role: String,

        /// Conflict policy when role already exists (Abort or Overwrite)
        #[arg(long, default_value = "Overwrite")]
        conflict_policy: String,
    },
    /// Replace all data access roles for an item (atomic PUT)
    #[command(display_order = 4)]
    Upsert {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        #[arg(short, long, env = "FABIO_WORKSPACE")]
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
        OnelakeSecurityCommand::Create {
            workspace,
            id,
            role,
            conflict_policy,
        } => create(cli, client, workspace, id, role, conflict_policy).await,
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
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/items/{item_id}/dataAccessRoles/{role_name}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "onelake-security show", "Member"))?;

    output::render_object(cli, &data, "name");
    Ok(())
}

async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    role: &str,
    conflict_policy: &str,
) -> Result<()> {
    let role_value: Value = if let Some(path) = role.strip_prefix('@') {
        let content = std::fs::read_to_string(path).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Failed to read file '{path}': {e}"),
                "Verify the file path is correct and the file is readable.",
            )
        })?;
        serde_json::from_str(&content).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in file '{path}': {e}"),
                "The file must contain valid JSON role definition.",
            )
        })?
    } else {
        serde_json::from_str(role).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid --role JSON: {e}"),
                "Provide valid JSON or use @path/to/file.json prefix to read from a file.",
            )
        })?
    };

    if output::dry_run_guard(cli, "onelake-security create", &role_value) {
        return Ok(());
    }

    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/items/{item_id}/dataAccessRoles?dataAccessRoleConflictPolicy={conflict_policy}"
            ),
            &role_value,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "onelake-security create", "Admin"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({
            "itemId": item_id,
            "status": "role_created"
        });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "name");
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
        let content = std::fs::read_to_string(path).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Failed to read file '{path}': {e}"),
                "Verify the file path is correct and the file is readable.",
            )
        })?;
        serde_json::from_str(&content).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in file '{path}': {e}"),
                "The file must contain valid JSON roles array.",
            )
        })?
    } else {
        serde_json::from_str(roles).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid --roles JSON: {e}"),
                "Provide valid JSON or use @path/to/file.json prefix to read from a file.",
            )
        })?
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

    client
        .delete(&format!(
            "/workspaces/{workspace}/items/{item_id}/dataAccessRoles/{role_name}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "onelake-security delete", "Admin"))?;

    let obj = serde_json::json!({
        "roleName": role_name,
        "status": "deleted"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}
