use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::enrich_forbidden;
use crate::output;
use anyhow::Result;
pub(super) async fn provision_identity(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "workspace provision-identity",
        &serde_json::json!({ "workspaceId": id }),
    ) {
        return Ok(());
    }
    let data = client
        .post(
            &format!("/workspaces/{id}/provisionIdentity"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace provision-identity", "Admin"))?;
    output::render_object(cli, &data, "servicePrincipalId");
    Ok(())
}
pub(super) async fn deprovision_identity(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "workspace deprovision-identity",
        &serde_json::json!({ "workspaceId": id }),
    ) {
        return Ok(());
    }
    client
        .post(
            &format!("/workspaces/{id}/deprovisionIdentity"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace deprovision-identity", "Admin"))?;
    output::render_object(
        cli,
        &serde_json::json!({ "workspaceId": id, "status": "deprovisioned" }),
        "status",
    );
    Ok(())
}
