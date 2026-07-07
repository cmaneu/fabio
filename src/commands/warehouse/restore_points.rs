use anyhow::Result;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::enrich_forbidden;
use crate::output;

pub(super) async fn list_restore_points(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/warehouses/{id}/restorePoints"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse list-restore-points", "Viewer"))?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["restorePointLabel", "id", "createdDateTime"],
        &["LABEL", "ID", "CREATED"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn create_restore_point(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: Option<&str>,
) -> Result<()> {
    let body = name.map_or_else(
        || serde_json::json!({}),
        |n| serde_json::json!({ "restorePointLabel": n }),
    );

    if output::dry_run_guard(cli, "warehouse create-restore-point", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/warehouses/{id}/restorePoints"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse create-restore-point", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

pub(super) async fn show_restore_point(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    restore_point_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/warehouses/{id}/restorePoints/{restore_point_id}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse show-restore-point", "Viewer"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

pub(super) async fn update_restore_point(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    restore_point_id: &str,
    name: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({});
    if let Some(n) = name {
        body["restorePointLabel"] = Value::from(n);
    }

    if output::dry_run_guard(cli, "warehouse update-restore-point", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/warehouses/{id}/restorePoints/{restore_point_id}"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse update-restore-point", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

pub(super) async fn delete_restore_point(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    restore_point_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "warehouse delete-restore-point",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "restorePointId": restore_point_id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!(
            "/workspaces/{workspace}/warehouses/{id}/restorePoints/{restore_point_id}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse delete-restore-point", "Contributor"))?;

    let obj = serde_json::json!({ "id": restore_point_id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

pub(super) async fn restore_to_point(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    restore_point_id: &str,
    name: &str,
) -> Result<()> {
    let body = serde_json::json!({
        "restoreToWarehouseName": name,
    });

    if output::dry_run_guard(cli, "warehouse restore-to-point", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/warehouses/{id}/restorePoints/{restore_point_id}/restore"
            ),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "warehouse restore-to-point", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
