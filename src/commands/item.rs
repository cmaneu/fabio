use std::fmt::Write;
use std::fs;
use std::path::Path;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum ItemCommand {
    // ── Read ─────────────────────────────────────────────────────────────
    /// List items in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Filter by item type (e.g., Notebook, Lakehouse, Warehouse)
        #[arg(short = 't', long = "type", visible_alias = "item-type")]
        item_type: Option<String>,
    },
    /// Show details of an item
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,
    },
    /// Get the definition (source code/content) of an item
    #[command(display_order = 3)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Definition format (optional, item-type dependent)
        #[arg(long)]
        format: Option<String>,

        /// Decode base64 payloads inline (adds decodedPayload field)
        #[arg(long)]
        decode: bool,
    },
    /// List connections used by an item
    #[command(display_order = 4)]
    ListConnections {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,
    },

    // ── Create/Update/Delete ─────────────────────────────────────────────
    /// Create a new item
    #[command(display_order = 10)]
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

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update item properties (name and/or description)
    #[command(display_order = 11)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update (override) item definition from file(s)
    #[command(display_order = 12)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Path to definition file (will be base64-encoded as a single part)
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON definition (full definition payload with parts array)
        #[arg(long)]
        definition: Option<String>,

        /// When true, also update item metadata from .platform file
        #[arg(long)]
        update_metadata: bool,
    },
    /// Delete an item
    #[command(display_order = 13)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,
    },

    // ── Copy/Move ────────────────────────────────────────────────────────
    /// Copy an item to another workspace
    #[command(display_order = 14)]
    Copy {
        /// Source workspace ID
        #[arg(long)]
        source_workspace: String,

        /// Item ID to copy
        #[arg(long)]
        id: String,

        /// Destination workspace ID
        #[arg(long)]
        dest_workspace: String,

        /// New name for the copy (optional, defaults to source name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Move an item to another workspace (copy + delete source)
    #[command(display_order = 15)]
    Move {
        /// Source workspace ID
        #[arg(long)]
        source_workspace: String,

        /// Item ID to move
        #[arg(long)]
        id: String,

        /// Destination workspace ID
        #[arg(long)]
        dest_workspace: String,

        /// New name (optional, defaults to source name)
        #[arg(long)]
        name: Option<String>,
    },

    // ── Tags ─────────────────────────────────────────────────────────────
    /// Apply tags to an item
    #[command(display_order = 20)]
    ApplyTags {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Comma-separated tag IDs
        #[arg(long, value_delimiter = ',')]
        tag_ids: Vec<String>,
    },
    /// Remove tags from an item
    #[command(display_order = 21)]
    UnapplyTags {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Comma-separated tag IDs
        #[arg(long, value_delimiter = ',')]
        tag_ids: Vec<String>,
    },

    // ── Bulk Operations ──────────────────────────────────────────────────
    /// Bulk export item definitions (LRO)
    #[command(display_order = 30)]
    BulkExportDefinitions {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Path to JSON file with request body
        #[arg(long, group = "input")]
        file: Option<String>,

        /// Inline JSON request body
        #[arg(long, group = "input")]
        content: Option<String>,
    },
    /// Bulk import item definitions (LRO)
    #[command(display_order = 31)]
    BulkImportDefinitions {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Path to JSON file with request body
        #[arg(long, group = "input")]
        file: Option<String>,

        /// Inline JSON request body
        #[arg(long, group = "input")]
        content: Option<String>,
    },
    /// Bulk move items to another workspace (LRO)
    #[command(display_order = 32)]
    BulkMove {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Path to JSON file with request body
        #[arg(long, group = "input")]
        file: Option<String>,

        /// Inline JSON request body
        #[arg(long, group = "input")]
        content: Option<String>,
    },

    // ── External Data Shares ─────────────────────────────────────────────
    /// List external data shares for an item
    #[command(display_order = 40)]
    ListExternalDataShares {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,
    },
    /// Create an external data share for an item
    #[command(display_order = 41)]
    CreateExternalDataShare {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// Comma-separated paths to share
        #[arg(long, value_delimiter = ',')]
        paths: Vec<String>,

        /// Recipient tenant ID
        #[arg(long)]
        recipient_tenant_id: String,
    },
    /// Show details of an external data share
    #[command(display_order = 42)]
    ShowExternalDataShare {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// External data share ID
        #[arg(long)]
        share_id: String,
    },
    /// Revoke an external data share
    #[command(display_order = 43)]
    RevokeExternalDataShare {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// External data share ID
        #[arg(long)]
        share_id: String,
    },
    /// Delete an external data share
    #[command(display_order = 44)]
    DeleteExternalDataShare {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,

        /// External data share ID
        #[arg(long)]
        share_id: String,
    },

    // ── Identity ─────────────────────────────────────────────────────────
    /// Assign a managed identity to an item
    #[command(display_order = 50)]
    AssignIdentity {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        id: String,
    },

    // ── External Data Share Invitations ──────────────────────────────────
    /// Get an external data share invitation (platform-level)
    #[command(display_order = 55)]
    GetInvitation {
        /// Invitation ID
        #[arg(long)]
        invitation_id: String,
    },
    /// Accept an external data share invitation
    #[command(display_order = 56)]
    AcceptInvitation {
        /// Invitation ID
        #[arg(long)]
        invitation_id: String,

        /// Target workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Display name for the created item
        #[arg(long)]
        name: String,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &ItemCommand) -> Result<()> {
    match command {
        ItemCommand::List {
            workspace,
            item_type,
        } => list(cli, client, workspace, item_type.as_deref()).await,
        ItemCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        ItemCommand::GetDefinition {
            workspace,
            id,
            format,
            decode,
        } => get_definition(cli, client, workspace, id, format.as_deref(), *decode).await,
        ItemCommand::ListConnections { workspace, id } => {
            list_connections(cli, client, workspace, id).await
        }
        ItemCommand::Create {
            workspace,
            name,
            item_type,
            description,
        } => {
            create(
                cli,
                client,
                workspace,
                name,
                item_type,
                description.as_deref(),
            )
            .await
        }
        ItemCommand::Update {
            workspace,
            id,
            name,
            description,
        } => {
            update(
                cli,
                client,
                workspace,
                id,
                name.as_deref(),
                description.as_deref(),
            )
            .await
        }
        ItemCommand::UpdateDefinition {
            workspace,
            id,
            file,
            definition,
            update_metadata,
        } => {
            update_definition(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                definition.as_deref(),
                *update_metadata,
            )
            .await
        }
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
        ItemCommand::ApplyTags {
            workspace,
            id,
            tag_ids,
        } => apply_tags(cli, client, workspace, id, tag_ids).await,
        ItemCommand::UnapplyTags {
            workspace,
            id,
            tag_ids,
        } => unapply_tags(cli, client, workspace, id, tag_ids).await,
        ItemCommand::BulkExportDefinitions {
            workspace,
            file,
            content,
        } => {
            bulk_post(
                cli,
                client,
                workspace,
                "bulkExportDefinitions",
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        ItemCommand::BulkImportDefinitions {
            workspace,
            file,
            content,
        } => {
            bulk_post(
                cli,
                client,
                workspace,
                "bulkImportDefinitions",
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        ItemCommand::BulkMove {
            workspace,
            file,
            content,
        } => {
            bulk_post(
                cli,
                client,
                workspace,
                "bulkMove",
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        ItemCommand::ListExternalDataShares { workspace, id } => {
            list_external_data_shares(cli, client, workspace, id).await
        }
        ItemCommand::CreateExternalDataShare {
            workspace,
            id,
            paths,
            recipient_tenant_id,
        } => {
            create_external_data_share(cli, client, workspace, id, paths, recipient_tenant_id).await
        }
        ItemCommand::ShowExternalDataShare {
            workspace,
            id,
            share_id,
        } => show_external_data_share(cli, client, workspace, id, share_id).await,
        ItemCommand::RevokeExternalDataShare {
            workspace,
            id,
            share_id,
        } => revoke_external_data_share(cli, client, workspace, id, share_id).await,
        ItemCommand::DeleteExternalDataShare {
            workspace,
            id,
            share_id,
        } => delete_external_data_share(cli, client, workspace, id, share_id).await,
        ItemCommand::AssignIdentity { workspace, id } => {
            assign_identity(cli, client, workspace, id).await
        }
        ItemCommand::GetInvitation { invitation_id } => {
            get_invitation(cli, client, invitation_id).await
        }
        ItemCommand::AcceptInvitation {
            invitation_id,
            workspace,
            name,
        } => accept_invitation(cli, client, invitation_id, workspace, name).await,
    }
}

// ─── List ────────────────────────────────────────────────────────────────────

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

    let resp = client
        .get_list(&path, "value", cli.all, cli.continuation_token.as_deref())
        .await?;

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

// ─── Show ────────────────────────────────────────────────────────────────────

async fn show(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/items/{id}"))
        .await
        .map_err(|e| enrich_item_not_found_error(e, workspace, id))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

// ─── Get Definition ──────────────────────────────────────────────────────────

async fn get_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    format: Option<&str>,
    decode: bool,
) -> Result<()> {
    let mut path = format!("/workspaces/{workspace}/items/{id}/getDefinition");
    if let Some(f) = format {
        let _ = write!(path, "?format={f}");
    }

    let data = client
        .post(&path, &serde_json::json!({}), true)
        .await
        .map_err(|e| enrich_forbidden(e, "item get-definition", "ReadWrite"))?;
    if decode {
        let decoded = output::decode_definition_parts(&data);
        output::render_object(cli, &decoded, "definition");
    } else {
        output::render_object(cli, &data, "definition");
    }
    Ok(())
}

// ─── List Connections ────────────────────────────────────────────────────────

async fn list_connections(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/items/{id}/connections"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "item list-connections", "ReadWrite"))?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "connectivityType", "displayName"],
        &["ID", "TYPE", "NAME"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

// ─── Create ──────────────────────────────────────────────────────────────────

async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    item_type: &str,
    description: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({
        "displayName": name,
        "type": item_type,
    });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if output::dry_run_guard(
        cli,
        "item create",
        &serde_json::json!({
            "workspace": workspace,
            "displayName": name,
            "type": item_type,
            "description": description
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(&format!("/workspaces/{workspace}/items"), &body, true)
        .await
        .map_err(|e| enrich_item_create_error(e, item_type))
        .map_err(|e| enrich_forbidden(e, "item create", "Member"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

// ─── Update ──────────────────────────────────────────────────────────────────

async fn update(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --name or --description must be provided".to_string(),
            "Example: fabio item update --workspace <WS> --id <ID> --name \"New Name\"".to_string(),
        )
        .into());
    }

    let mut body = serde_json::json!({});
    if let Some(n) = name {
        body["displayName"] = Value::String(n.to_string());
    }
    if let Some(d) = description {
        body["description"] = Value::String(d.to_string());
    }

    if output::dry_run_guard(cli, "item update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/items/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "item update", "ReadWrite"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

// ─── Update Definition ───────────────────────────────────────────────────────

async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    definition: Option<&str>,
    update_metadata: bool,
) -> Result<()> {
    if file.is_none() && definition.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Either --file or --definition must be provided".to_string(),
            "Example: fabio item update-definition --workspace <WS> --id <ID> --file ./notebook.ipynb"
                .to_string(),
        )
        .into());
    }

    let body = if let Some(def_json) = definition {
        // Inline JSON definition payload
        serde_json::from_str::<Value>(def_json).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in --definition: {e}"),
                "Provide valid JSON: {\"definition\":{\"parts\":[{\"path\":\"...\",\"payload\":\"base64...\",\"payloadType\":\"InlineBase64\"}]}}"
                    .to_string(),
            )
        })?
    } else if let Some(file_path) = file {
        // Read file and encode as base64
        let path = Path::new(file_path);
        let content = fs::read(path).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Failed to read file '{file_path}': {e}"),
                "Provide a valid file path.".to_string(),
            )
        })?;

        let encoded = BASE64.encode(&content);
        let filename = path
            .file_name()
            .map_or("definition", |f| f.to_str().unwrap_or("definition"));

        serde_json::json!({
            "definition": {
                "parts": [{
                    "path": filename,
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }]
            }
        })
    } else {
        unreachable!()
    };

    if output::dry_run_guard(cli, "item update-definition", &body) {
        return Ok(());
    }

    let mut path = format!("/workspaces/{workspace}/items/{id}/updateDefinition");
    if update_metadata {
        path.push_str("?updateMetadata=true");
    }

    client
        .post(&path, &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "item update-definition", "ReadWrite"))?;

    let obj = serde_json::json!({
        "id": id,
        "workspace": workspace,
        "status": "definition_updated"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Delete ──────────────────────────────────────────────────────────────────

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
        .await
        .map_err(|e| enrich_forbidden(e, "item delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Copy ────────────────────────────────────────────────────────────────────

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

    let result = copy_item_impl(client, source_workspace, id, dest_workspace, name)
        .await
        .map_err(|e| enrich_forbidden(e, "item copy", "Member"))?;
    output::render_object(cli, &result, "id");
    Ok(())
}

// ─── Move ────────────────────────────────────────────────────────────────────

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

    let result = copy_item_impl(client, source_workspace, id, dest_workspace, name)
        .await
        .map_err(|e| enrich_forbidden(e, "item move", "Member"))?;

    // Delete source after successful copy
    client
        .delete(&format!("/workspaces/{source_workspace}/items/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "item move (delete source)", "Member"))?;

    let mut obj = result;
    obj["status"] = Value::String("moved".to_string());
    output::render_object(cli, &obj, "id");
    Ok(())
}

// ─── Shared Copy Implementation ──────────────────────────────────────────────

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

// ─── Apply Tags ──────────────────────────────────────────────────────────────

async fn apply_tags(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    tag_ids: &[String],
) -> Result<()> {
    let body = serde_json::json!({ "tagIds": tag_ids });

    if output::dry_run_guard(cli, "item apply-tags", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/applyTags"),
            &body,
            false,
        )
        .await?;

    let obj = serde_json::json!({ "id": id, "status": "tags_applied" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Unapply Tags ────────────────────────────────────────────────────────────

async fn unapply_tags(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    tag_ids: &[String],
) -> Result<()> {
    let body = serde_json::json!({ "tagIds": tag_ids });

    if output::dry_run_guard(cli, "item unapply-tags", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/unapplyTags"),
            &body,
            false,
        )
        .await?;

    let obj = serde_json::json!({ "id": id, "status": "tags_removed" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Bulk Operations ─────────────────────────────────────────────────────────

async fn bulk_post(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    operation: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_input(file, content, operation)?;

    if output::dry_run_guard(cli, &format!("item {operation}"), &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/items/{operation}"),
            &body,
            true,
        )
        .await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

// ─── External Data Shares ────────────────────────────────────────────────────

async fn list_external_data_shares(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/items/{id}/externalDataShares"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "status"],
        &["ID", "STATUS"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn create_external_data_share(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    paths: &[String],
    recipient_tenant_id: &str,
) -> Result<()> {
    let body = serde_json::json!({
        "paths": paths,
        "recipient": { "tenantId": recipient_tenant_id }
    });

    if output::dry_run_guard(cli, "item create-external-data-share", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/externalDataShares"),
            &body,
            false,
        )
        .await?;

    output::render_object(cli, &data, "id");
    Ok(())
}

async fn show_external_data_share(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    share_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/items/{id}/externalDataShares/{share_id}"
        ))
        .await?;

    output::render_object(cli, &data, "id");
    Ok(())
}

async fn revoke_external_data_share(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    share_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "item revoke-external-data-share",
        &serde_json::json!({ "workspace": workspace, "id": id, "share_id": share_id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/externalDataShares/{share_id}/revoke"),
            &serde_json::json!({}),
            false,
        )
        .await?;

    let obj = serde_json::json!({ "id": share_id, "status": "revoked" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn delete_external_data_share(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    share_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "item delete-external-data-share",
        &serde_json::json!({ "workspace": workspace, "id": id, "share_id": share_id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!(
            "/workspaces/{workspace}/items/{id}/externalDataShares/{share_id}"
        ))
        .await?;

    let obj = serde_json::json!({ "id": share_id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Assign Identity ─────────────────────────────────────────────────────────

async fn assign_identity(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "item assign-identity",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/identities/default/assign"),
            &serde_json::json!({}),
            false,
        )
        .await?;

    let obj = serde_json::json!({ "id": id, "status": "identity_assigned" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── External Data Share Invitations ─────────────────────────────────────────

async fn get_invitation(cli: &Cli, client: &FabricClient, invitation_id: &str) -> Result<()> {
    let data = client
        .get(&format!("/externalDataShares/invitations/{invitation_id}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn accept_invitation(
    cli: &Cli,
    client: &FabricClient,
    invitation_id: &str,
    workspace: &str,
    name: &str,
) -> Result<()> {
    let body = serde_json::json!({
        "workspaceId": workspace,
        "displayName": name
    });

    if output::dry_run_guard(cli, "item accept-invitation", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/externalDataShares/invitations/{invitation_id}/accept"),
            &body,
            false,
        )
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Read JSON body from --file or --content flag.
fn read_json_input(file: Option<&str>, content: Option<&str>, command: &str) -> Result<Value> {
    if let Some(c) = content {
        serde_json::from_str(c).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in --content: {e}"),
                format!("Provide valid JSON for {command}."),
            )
            .into()
        })
    } else if let Some(f) = file {
        let data = fs::read_to_string(f).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Failed to read file '{f}': {e}"),
                "Provide a valid file path.".to_string(),
            )
        })?;
        serde_json::from_str(&data).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in file '{f}': {e}"),
                format!("Provide valid JSON for {command}."),
            )
            .into()
        })
    } else {
        Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Either --file or --content must be provided".to_string(),
            format!("Example: fabio item {command} --workspace <WS> --file request.json"),
        )
        .into())
    }
}

// ─── Error Enrichment ────────────────────────────────────────────────────────

/// Known Fabric item types for error hints.
const KNOWN_ITEM_TYPES: &[&str] = &[
    "CopyJob",
    "Dashboard",
    "DataAgent",
    "DataPipeline",
    "Dataflow",
    "Datamart",
    "Environment",
    "Eventhouse",
    "Eventstream",
    "GraphQLApi",
    "KQLDashboard",
    "KQLDatabase",
    "KQLQueryset",
    "Lakehouse",
    "MLExperiment",
    "MLModel",
    "MirroredDatabase",
    "MirroredWarehouse",
    "MountedDataFactory",
    "Notebook",
    "Ontology",
    "PaginatedReport",
    "Reflex",
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

// ─── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_item_types_are_sorted() {
        let mut sorted = KNOWN_ITEM_TYPES.to_vec();
        sorted.sort_unstable();
        assert_eq!(KNOWN_ITEM_TYPES, sorted.as_slice());
    }

    #[test]
    fn known_item_types_are_pascal_case() {
        for t in KNOWN_ITEM_TYPES {
            let first = t.chars().next().unwrap();
            assert!(first.is_uppercase(), "Type '{t}' should be PascalCase");
        }
    }

    #[test]
    fn enrich_create_error_adds_hint_for_invalid_type() {
        let err: anyhow::Error = FabioError::new(
            ErrorCode::ApiError,
            "The request is invalid. Item type FakeType is not supported.".to_string(),
        )
        .into();
        let enriched = enrich_item_create_error(err, "FakeType");
        let fabio_err = enriched.downcast_ref::<FabioError>().unwrap();
        assert_eq!(fabio_err.code, ErrorCode::InvalidInput);
        assert!(fabio_err.hint.as_ref().unwrap().contains("Lakehouse"));
    }

    #[test]
    fn enrich_not_found_adds_hint() {
        let err: anyhow::Error =
            FabioError::new(ErrorCode::NotFound, "item not found".to_string()).into();
        let enriched = enrich_item_not_found_error(err, "ws-123", "item-456");
        let fabio_err = enriched.downcast_ref::<FabioError>().unwrap();
        assert!(fabio_err.hint.as_ref().unwrap().contains("item-456"));
        assert!(fabio_err.hint.as_ref().unwrap().contains("ws-123"));
    }

    #[test]
    fn enrich_preserves_non_matching_errors() {
        let err: anyhow::Error =
            FabioError::new(ErrorCode::ApiError, "something else".to_string()).into();
        let enriched = enrich_item_create_error(err, "Notebook");
        let fabio_err = enriched.downcast_ref::<FabioError>().unwrap();
        // Should NOT be INVALID_INPUT since message doesn't contain "invalid" + type
        assert_eq!(fabio_err.code, ErrorCode::ApiError);
    }
}
