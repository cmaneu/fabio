use anyhow::Result;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::enrich_forbidden;
use crate::output;

// ─── Copy ────────────────────────────────────────────────────────────────────

pub(super) async fn copy(
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

pub(super) async fn move_item(
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

// ─── Move to Folder ──────────────────────────────────────────────────────────

pub(super) async fn move_to_folder(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    folder_id: Option<&str>,
) -> Result<()> {
    let body = serde_json::json!({ "targetFolderId": folder_id });

    if output::dry_run_guard(cli, "item move-to-folder", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/move"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "item move-to-folder", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "targetFolderId": folder_id,
        "status": "moved"
    });
    output::render_object(cli, &obj, "status");
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
