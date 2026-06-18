use anyhow::Result;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::enrich_admin;
use crate::output;

use super::read_body;

pub(super) async fn list_items(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/items",
            "itemEntities",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["name", "id", "type", "workspaceId", "state"],
        &["NAME", "ID", "TYPE", "WORKSPACE", "STATE"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn show_item(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!("/admin/workspaces/{workspace}/items/{item_id}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

pub(super) async fn list_item_users(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/admin/workspaces/{workspace}/items/{item_id}/users"),
            "accessDetails",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["principal", "itemAccessDetails"],
        &["PRINCIPAL", "ACCESS"],
        "principal",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn bulk_set_labels(
    cli: &Cli,
    client: &FabricClient,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "bulk-set-labels")?;

    if output::dry_run_guard(cli, "admin bulk-set-labels", &body) {
        return Ok(());
    }

    let data = client
        .post("/admin/items/bulkSetLabels", &body, false)
        .await
        .map_err(|e| enrich_admin(e, "admin bulk-set-labels"))?;
    output::render_object(cli, &data, "status");
    Ok(())
}

pub(super) async fn bulk_remove_labels(
    cli: &Cli,
    client: &FabricClient,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "bulk-remove-labels")?;

    if output::dry_run_guard(cli, "admin bulk-remove-labels", &body) {
        return Ok(());
    }

    let data = client
        .post("/admin/items/bulkRemoveLabels", &body, false)
        .await
        .map_err(|e| enrich_admin(e, "admin bulk-remove-labels"))?;
    output::render_object(cli, &data, "status");
    Ok(())
}

pub(super) async fn list_external_data_shares(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/items/externalDataShares",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "itemId", "workspaceId"],
        &["SHARE_ID", "ITEM", "WORKSPACE"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn revoke_external_data_share(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    share_id: &str,
) -> Result<()> {
    let body = serde_json::json!({});

    if output::dry_run_guard(
        cli,
        "admin revoke-external-data-share",
        &serde_json::json!({ "workspace": workspace, "itemId": item_id, "shareId": share_id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!(
                "/admin/workspaces/{workspace}/items/{item_id}/externalDataShares/{share_id}/revoke"
            ),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_admin(e, "admin revoke-external-data-share"))?;

    let obj = serde_json::json!({ "shareId": share_id, "status": "revoked" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

pub(super) async fn remove_all_sharing_links(
    cli: &Cli,
    client: &FabricClient,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "remove-all-sharing-links")?;

    if output::dry_run_guard(cli, "admin remove-all-sharing-links", &body) {
        return Ok(());
    }

    let data = client
        .post("/admin/items/removeAllSharingLinks", &body, true)
        .await
        .map_err(|e| enrich_admin(e, "admin remove-all-sharing-links"))?;
    output::render_object(cli, &data, "status");
    Ok(())
}

pub(super) async fn bulk_remove_sharing_links(
    cli: &Cli,
    client: &FabricClient,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "bulk-remove-sharing-links")?;

    if output::dry_run_guard(cli, "admin bulk-remove-sharing-links", &body) {
        return Ok(());
    }

    let data = client
        .post("/admin/items/bulkRemoveSharingLinks", &body, true)
        .await
        .map_err(|e| enrich_admin(e, "admin bulk-remove-sharing-links"))?;
    output::render_object(cli, &data, "status");
    Ok(())
}
