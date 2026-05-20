use std::fmt::Write;

use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum ItemCommand {
    /// List items in a workspace
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Filter by item type (e.g., Notebook, Lakehouse, Warehouse)
        #[arg(short = 't', long = "type")]
        item_type: Option<String>,
    },
    /// Show details of an item
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,
    },
    /// Create a new item
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item display name
        #[arg(long)]
        name: String,

        /// Item type (e.g., Lakehouse, Warehouse)
        #[arg(short = 't', long = "type")]
        item_type: String,
    },
    /// Delete an item
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,
    },
    /// Copy an item to another workspace
    Copy {
        /// Source workspace ID
        #[arg(short = 's', long)]
        source_workspace: String,

        /// Item ID to copy
        #[arg(long)]
        id: String,

        /// Destination workspace ID
        #[arg(short = 'd', long)]
        dest_workspace: String,

        /// New name for the copy (optional, defaults to source name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Move an item to another workspace (copy + delete source)
    Move {
        /// Source workspace ID
        #[arg(short = 's', long)]
        source_workspace: String,

        /// Item ID to move
        #[arg(long)]
        id: String,

        /// Destination workspace ID
        #[arg(short = 'd', long)]
        dest_workspace: String,

        /// New name (optional, defaults to source name)
        #[arg(long)]
        name: Option<String>,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &ItemCommand) -> Result<()> {
    match command {
        ItemCommand::List {
            workspace,
            item_type,
        } => list(cli, client, workspace, item_type.as_deref()).await,
        ItemCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        ItemCommand::Create {
            workspace,
            name,
            item_type,
        } => create(cli, client, workspace, name, item_type).await,
        ItemCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        ItemCommand::Copy {
            source_workspace,
            id,
            dest_workspace,
            name,
        } => {
            copy(
                cli,
                client,
                source_workspace,
                id,
                dest_workspace,
                name.as_deref(),
            )
            .await
        }
        ItemCommand::Move {
            source_workspace,
            id,
            dest_workspace,
            name,
        } => {
            move_item(
                cli,
                client,
                source_workspace,
                id,
                dest_workspace,
                name.as_deref(),
            )
            .await
        }
    }
}

async fn list(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_type: Option<&str>,
) -> Result<()> {
    let mut path = format!("/workspaces/{workspace}/items");
    if let Some(t) = item_type {
        let _ = write!(path, "?type={t}");
    }

    let resp = client.get_list(&path, "value", cli.all).await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id", "type"],
        &["NAME", "ID", "TYPE"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn show(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/items/{id}"))
        .await
        .map_err(|e| enrich_item_not_found_error(e, workspace, id))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    item_type: &str,
) -> Result<()> {
    let body = serde_json::json!({
        "displayName": name,
        "type": item_type,
    });

    if output::dry_run_guard(
        cli,
        "item create",
        &serde_json::json!({
            "workspace": workspace,
            "displayName": name,
            "type": item_type
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(&format!("/workspaces/{workspace}/items"), &body, true)
        .await
        .map_err(|e| enrich_item_create_error(e, item_type))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "item delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/items/{id}"))
        .await?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn copy(
    cli: &Cli,
    client: &FabricClient,
    source_workspace: &str,
    id: &str,
    dest_workspace: &str,
    name: Option<&str>,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "item copy",
        &serde_json::json!({
            "source_workspace": source_workspace,
            "id": id,
            "dest_workspace": dest_workspace,
            "name": name
        }),
    ) {
        return Ok(());
    }

    let result = copy_item_impl(client, source_workspace, id, dest_workspace, name).await?;
    output::render_object(cli, &result, "id");
    Ok(())
}

async fn move_item(
    cli: &Cli,
    client: &FabricClient,
    source_workspace: &str,
    id: &str,
    dest_workspace: &str,
    name: Option<&str>,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "item move",
        &serde_json::json!({
            "source_workspace": source_workspace,
            "id": id,
            "dest_workspace": dest_workspace,
            "name": name
        }),
    ) {
        return Ok(());
    }

    let result = copy_item_impl(client, source_workspace, id, dest_workspace, name).await?;

    // Delete source after successful copy
    client
        .delete(&format!("/workspaces/{source_workspace}/items/{id}"))
        .await?;

    let mut obj = result;
    obj["status"] = Value::String("moved".to_string());
    output::render_object(cli, &obj, "id");
    Ok(())
}

/// Shared implementation for item copy (used by both copy and move).
async fn copy_item_impl(
    client: &FabricClient,
    source_workspace: &str,
    id: &str,
    dest_workspace: &str,
    name: Option<&str>,
) -> Result<Value> {
    // Get item definition from source (LRO)
    let definition = client
        .post(
            &format!("/workspaces/{source_workspace}/items/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await?;

    // Get source item metadata for name/type
    let source_item = client
        .get(&format!("/workspaces/{source_workspace}/items/{id}"))
        .await?;

    let item_name = name.map_or_else(
        || {
            source_item
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or("unnamed")
                .to_string()
        },
        String::from,
    );

    let item_type = source_item
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("Unknown");

    // Create item in destination workspace with definition
    let body = serde_json::json!({
        "displayName": item_name,
        "type": item_type,
        "definition": definition.get("definition").unwrap_or(&Value::Null),
    });

    let result = client
        .post(&format!("/workspaces/{dest_workspace}/items"), &body, true)
        .await?;

    Ok(result)
}

/// Known Fabric item types for error hints.
const KNOWN_ITEM_TYPES: &[&str] = &[
    "Dashboard",
    "DataPipeline",
    "Datamart",
    "Environment",
    "Eventhouse",
    "Eventstream",
    "KQLDatabase",
    "KQLQueryset",
    "Lakehouse",
    "MLExperiment",
    "MLModel",
    "MirroredWarehouse",
    "Notebook",
    "PaginatedReport",
    "Report",
    "SQLEndpoint",
    "SemanticModel",
    "SparkJobDefinition",
    "Warehouse",
];

/// Enrich item create errors with valid type hints.
fn enrich_item_create_error(err: anyhow::Error, item_type: &str) -> anyhow::Error {
    let Some(fabio_err) = err.downcast_ref::<FabioError>() else {
        return err;
    };

    if fabio_err.message.contains("invalid") && fabio_err.message.contains(item_type) {
        let valid_types = KNOWN_ITEM_TYPES.join(", ");
        let hint = format!(
            "'{item_type}' is not a valid Fabric item type. Valid types: {valid_types}. \
             List items to see types in your workspace: fabio item list --workspace <ID>"
        );
        return FabioError::with_hint(ErrorCode::InvalidInput, &fabio_err.message, hint).into();
    }

    err
}

/// Enrich item not-found errors with guidance.
fn enrich_item_not_found_error(err: anyhow::Error, workspace: &str, id: &str) -> anyhow::Error {
    let Some(fabio_err) = err.downcast_ref::<FabioError>() else {
        return err;
    };

    if fabio_err.code == ErrorCode::NotFound {
        let hint = format!(
            "Item '{id}' not found in workspace '{workspace}'. \
             List available items: fabio item list --workspace {workspace}"
        );
        return FabioError::with_hint(ErrorCode::NotFound, &fabio_err.message, hint).into();
    }

    err
}
