use super::{KNOWN_PRINCIPAL_TYPES, KNOWN_ROLES};
use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;
use anyhow::Result;
pub(super) async fn list_role_assignments(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{id}/roleAssignments"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace list-role-assignments", "Member"))?;
    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "role"],
        &["PRINCIPAL_ID", "ROLE"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}
pub(super) async fn add_role_assignment(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    principal_id: &str,
    principal_type: &str,
    role: &str,
) -> Result<()> {
    if !KNOWN_ROLES.iter().any(|r| r.eq_ignore_ascii_case(role)) {
        return Err(FabioError::with_hint(ErrorCode::InvalidInput, format!("Invalid role: '{role}'"), format!("Valid roles: {}. Example: fabio workspace add-role-assignment --id <WS> --principal-id <ID> --principal-type User --role Member", KNOWN_ROLES.join(", "))).into());
    }
    if !KNOWN_PRINCIPAL_TYPES
        .iter()
        .any(|t| t.eq_ignore_ascii_case(principal_type))
    {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("Invalid principal type: '{principal_type}'"),
            format!(
                "Valid principal types: {}",
                KNOWN_PRINCIPAL_TYPES.join(", ")
            ),
        )
        .into());
    }
    let body = serde_json::json!({ "principal": { "id": principal_id, "type": principal_type }, "role": role });
    if output::dry_run_guard(cli, "workspace add-role-assignment", &body) {
        return Ok(());
    }
    let data = client
        .post(&format!("/workspaces/{id}/roleAssignments"), &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "workspace add-role-assignment", "Member"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
pub(super) async fn update_role_assignment(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    assignment_id: &str,
    role: &str,
) -> Result<()> {
    if !KNOWN_ROLES.iter().any(|r| r.eq_ignore_ascii_case(role)) {
        return Err(FabioError::with_hint(ErrorCode::InvalidInput, format!("Invalid role: '{role}'"), format!("Valid roles: {}. Example: fabio workspace update-role-assignment --id <WS> --assignment-id <ID> --role Contributor", KNOWN_ROLES.join(", "))).into());
    }
    let body = serde_json::json!({ "role": role });
    if output::dry_run_guard(cli, "workspace update-role-assignment", &body) {
        return Ok(());
    }
    let data = client
        .patch(
            &format!("/workspaces/{id}/roleAssignments/{assignment_id}"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace update-role-assignment", "Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
pub(super) async fn delete_role_assignment(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    assignment_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "workspace delete-role-assignment",
        &serde_json::json!({ "workspaceId": id, "assignmentId": assignment_id }),
    ) {
        return Ok(());
    }
    client
        .delete(&format!("/workspaces/{id}/roleAssignments/{assignment_id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace delete-role-assignment", "Admin"))?;
    output::render_object(
        cli,
        &serde_json::json!({ "workspaceId": id, "assignmentId": assignment_id, "status": "deleted" }),
        "status",
    );
    Ok(())
}
pub(super) async fn show_role_assignment(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    assignment_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{id}/roleAssignments/{assignment_id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace show-role-assignment", "Member"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
