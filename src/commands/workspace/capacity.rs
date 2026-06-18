use super::enrich_assign_capacity_error;
use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::enrich_forbidden;
use crate::output;
use anyhow::Result;
pub(super) async fn assign_capacity(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    capacity: &str,
) -> Result<()> {
    let body = serde_json::json!({ "capacityId": capacity });
    if output::dry_run_guard(
        cli,
        "workspace assign-capacity",
        &serde_json::json!({ "workspaceId": id, "capacityId": capacity }),
    ) {
        return Ok(());
    }
    if let Err(e) = client
        .post(&format!("/workspaces/{id}/assignToCapacity"), &body, false)
        .await
    {
        return Err(enrich_assign_capacity_error(e, capacity));
    }
    output::render_object(
        cli,
        &serde_json::json!({ "workspaceId": id, "capacityId": capacity, "status": "assigned" }),
        "status",
    );
    Ok(())
}
pub(super) async fn unassign_capacity(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "workspace unassign-capacity",
        &serde_json::json!({ "workspaceId": id }),
    ) {
        return Ok(());
    }
    client
        .post(
            &format!("/workspaces/{id}/unassignFromCapacity"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace unassign-capacity", "Admin"))?;
    output::render_object(
        cli,
        &serde_json::json!({ "workspaceId": id, "status": "unassigned" }),
        "status",
    );
    Ok(())
}
