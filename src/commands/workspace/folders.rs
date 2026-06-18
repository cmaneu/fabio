use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;
use anyhow::Result;
use serde_json::Value;
pub(super) async fn list_folders(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/folders"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace list-folders", "Member"))?;
    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id"],
        &["NAME", "ID"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}
pub(super) async fn create_folder(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    description: Option<&str>,
    parent_folder_id: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }
    if let Some(parent) = parent_folder_id {
        body["parentFolderId"] = Value::String(parent.to_string());
    }
    if output::dry_run_guard(cli, "workspace create-folder", &body) {
        return Ok(());
    }
    let data = client
        .post(&format!("/workspaces/{workspace}/folders"), &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "workspace create-folder", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
pub(super) async fn show_folder(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    folder_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/folders/{folder_id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace show-folder", "Member"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
pub(super) async fn update_folder(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    folder_id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(ErrorCode::InvalidInput, "At least one of --name or --description must be provided".to_string(), "Example: fabio workspace update-folder --workspace <WS> --folder-id <ID> --name \"New Name\"".to_string()).into());
    }
    let mut body = serde_json::json!({});
    if let Some(n) = name {
        body["displayName"] = Value::String(n.to_string());
    }
    if let Some(d) = description {
        body["description"] = Value::String(d.to_string());
    }
    if output::dry_run_guard(cli, "workspace update-folder", &body) {
        return Ok(());
    }
    let data = client
        .patch(
            &format!("/workspaces/{workspace}/folders/{folder_id}"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace update-folder", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
pub(super) async fn delete_folder(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    folder_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "workspace delete-folder",
        &serde_json::json!({ "workspaceId": workspace, "folderId": folder_id }),
    ) {
        return Ok(());
    }
    client
        .delete(&format!("/workspaces/{workspace}/folders/{folder_id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace delete-folder", "Contributor"))?;
    output::render_object(
        cli,
        &serde_json::json!({ "workspaceId": workspace, "folderId": folder_id, "status": "deleted" }),
        "status",
    );
    Ok(())
}
pub(super) async fn move_folder(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    folder_id: &str,
    target_folder_id: Option<&str>,
) -> Result<()> {
    let body = target_folder_id.map_or_else(
        || serde_json::json!({ "targetFolderId": null }),
        |target| serde_json::json!({ "targetFolderId": target }),
    );
    if output::dry_run_guard(cli, "workspace move-folder", &body) {
        return Ok(());
    }
    let data = client
        .post(
            &format!("/workspaces/{workspace}/folders/{folder_id}/move"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace move-folder", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
