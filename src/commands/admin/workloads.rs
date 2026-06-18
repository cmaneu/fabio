use anyhow::Result;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::enrich_admin;
use crate::output;

use super::read_body;

pub(super) async fn list_workloads(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/workloads",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "displayName", "state"],
        &["ID", "NAME", "STATE"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn list_workload_assignments(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/workloads/assignments",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "workloadId", "capacityId"],
        &["ID", "WORKLOAD", "CAPACITY"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn create_workload_assignment(
    cli: &Cli,
    client: &FabricClient,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "create-workload-assignment")?;

    if output::dry_run_guard(cli, "admin create-workload-assignment", &body) {
        return Ok(());
    }

    let data = client
        .post("/admin/workloads/assignments", &body, false)
        .await
        .map_err(|e| enrich_admin(e, "admin create-workload-assignment"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

pub(super) async fn delete_workload_assignment(
    cli: &Cli,
    client: &FabricClient,
    assignment_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "admin delete-workload-assignment",
        &serde_json::json!({ "assignmentId": assignment_id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/admin/workloads/assignments/{assignment_id}"))
        .await
        .map_err(|e| enrich_admin(e, "admin delete-workload-assignment"))?;

    let obj = serde_json::json!({ "assignmentId": assignment_id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}
