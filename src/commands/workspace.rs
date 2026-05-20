use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

/// Known workspace roles for error hints.
const KNOWN_ROLES: &[&str] = &["Admin", "Member", "Contributor", "Viewer"];

/// Known principal types for error hints.
const KNOWN_PRINCIPAL_TYPES: &[&str] = &[
    "User",
    "Group",
    "ServicePrincipal",
    "ServicePrincipalProfile",
];

#[derive(Debug, Subcommand)]
pub enum WorkspaceCommand {
    // ── Read ─────────────────────────────────────────────────────────────
    /// List all workspaces
    #[command(display_order = 1)]
    List,
    /// Show details of a workspace
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(long)]
        id: String,
    },

    // ── Create/Update/Delete ─────────────────────────────────────────────
    /// Create a new workspace
    #[command(display_order = 10)]
    Create {
        /// Display name for the workspace
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update workspace properties (name and/or description)
    #[command(display_order = 11)]
    Update {
        /// Workspace ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a workspace
    #[command(display_order = 12)]
    Delete {
        /// Workspace ID
        #[arg(long)]
        id: String,
    },

    // ── Capacity ─────────────────────────────────────────────────────────
    /// Assign a workspace to a capacity
    #[command(display_order = 20)]
    AssignCapacity {
        /// Workspace ID
        #[arg(long)]
        id: String,

        /// Target capacity ID
        #[arg(short, long)]
        capacity: String,
    },
    /// Unassign a workspace from its capacity
    #[command(display_order = 21)]
    UnassignCapacity {
        /// Workspace ID
        #[arg(long)]
        id: String,
    },

    // ── Identity ─────────────────────────────────────────────────────────
    /// Provision a workspace identity (managed identity)
    #[command(display_order = 30)]
    ProvisionIdentity {
        /// Workspace ID
        #[arg(long)]
        id: String,
    },
    /// Deprovision a workspace identity
    #[command(display_order = 31)]
    DeprovisionIdentity {
        /// Workspace ID
        #[arg(long)]
        id: String,
    },

    // ── Role Assignments ─────────────────────────────────────────────────
    /// List workspace role assignments
    #[command(display_order = 40)]
    ListRoleAssignments {
        /// Workspace ID
        #[arg(long)]
        id: String,
    },
    /// Add a role assignment to a workspace
    #[command(display_order = 41)]
    AddRoleAssignment {
        /// Workspace ID
        #[arg(long)]
        id: String,

        /// Principal ID (user, group, or service principal object ID)
        #[arg(long)]
        principal_id: String,

        /// Principal type: User, Group, `ServicePrincipal`, or `ServicePrincipalProfile`
        #[arg(long)]
        principal_type: String,

        /// Role to assign (Admin, Member, Contributor, Viewer)
        #[arg(long)]
        role: String,
    },
    /// Update a workspace role assignment
    #[command(display_order = 42)]
    UpdateRoleAssignment {
        /// Workspace ID
        #[arg(long)]
        id: String,

        /// Role assignment ID (same as the principal ID)
        #[arg(long)]
        assignment_id: String,

        /// New role (Admin, Member, Contributor, Viewer)
        #[arg(long)]
        role: String,
    },
    /// Delete a workspace role assignment
    #[command(display_order = 43)]
    DeleteRoleAssignment {
        /// Workspace ID
        #[arg(long)]
        id: String,

        /// Role assignment ID (same as the principal ID)
        #[arg(long)]
        assignment_id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &WorkspaceCommand) -> Result<()> {
    match command {
        WorkspaceCommand::List => list(cli, client).await,
        WorkspaceCommand::Show { id } => show(cli, client, id).await,
        WorkspaceCommand::Create { name, description } => {
            create(cli, client, name, description.as_deref()).await
        }
        WorkspaceCommand::Update {
            id,
            name,
            description,
        } => update(cli, client, id, name.as_deref(), description.as_deref()).await,
        WorkspaceCommand::Delete { id } => delete(cli, client, id).await,
        WorkspaceCommand::AssignCapacity { id, capacity } => {
            assign_capacity(cli, client, id, capacity).await
        }
        WorkspaceCommand::UnassignCapacity { id } => unassign_capacity(cli, client, id).await,
        WorkspaceCommand::ProvisionIdentity { id } => provision_identity(cli, client, id).await,
        WorkspaceCommand::DeprovisionIdentity { id } => deprovision_identity(cli, client, id).await,
        WorkspaceCommand::ListRoleAssignments { id } => {
            list_role_assignments(cli, client, id).await
        }
        WorkspaceCommand::AddRoleAssignment {
            id,
            principal_id,
            principal_type,
            role,
        } => add_role_assignment(cli, client, id, principal_id, principal_type, role).await,
        WorkspaceCommand::UpdateRoleAssignment {
            id,
            assignment_id,
            role,
        } => update_role_assignment(cli, client, id, assignment_id, role).await,
        WorkspaceCommand::DeleteRoleAssignment { id, assignment_id } => {
            delete_role_assignment(cli, client, id, assignment_id).await
        }
    }
}

// ─── List ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client.get_list("/workspaces", "value", cli.all).await?;

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

async fn show(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    let data = client.get(&format!("/workspaces/{id}")).await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

// ─── Create ──────────────────────────────────────────────────────────────────

async fn create(
    cli: &Cli,
    client: &FabricClient,
    name: &str,
    description: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if output::dry_run_guard(cli, "workspace create", &body) {
        return Ok(());
    }

    let data = client
        .post("/workspaces", &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "workspace create", "Fabric user (tenant-level)"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

// ─── Update ──────────────────────────────────────────────────────────────────

async fn update(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --name or --description must be provided".to_string(),
            "Example: fabio workspace update --id <ID> --name \"New Name\" --description \"New description\"".to_string(),
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

    if output::dry_run_guard(cli, "workspace update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "workspace update", "Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

// ─── Delete ──────────────────────────────────────────────────────────────────

async fn delete(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    if output::dry_run_guard(cli, "workspace delete", &serde_json::json!({ "id": id })) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace delete", "Admin"))?;
    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Assign Capacity ─────────────────────────────────────────────────────────

async fn assign_capacity(cli: &Cli, client: &FabricClient, id: &str, capacity: &str) -> Result<()> {
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

    let obj = serde_json::json!({
        "workspaceId": id,
        "capacityId": capacity,
        "status": "assigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Unassign Capacity ───────────────────────────────────────────────────────

async fn unassign_capacity(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
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

    let obj = serde_json::json!({
        "workspaceId": id,
        "status": "unassigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Provision Identity ──────────────────────────────────────────────────────

async fn provision_identity(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
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
            true, // LRO-aware: may return 202
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace provision-identity", "Admin"))?;
    output::render_object(cli, &data, "servicePrincipalId");
    Ok(())
}

// ─── Deprovision Identity ────────────────────────────────────────────────────

async fn deprovision_identity(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
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

    let obj = serde_json::json!({
        "workspaceId": id,
        "status": "deprovisioned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── List Role Assignments ───────────────────────────────────────────────────

async fn list_role_assignments(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{id}/roleAssignments"),
            "value",
            cli.all,
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

// ─── Add Role Assignment ─────────────────────────────────────────────────────

async fn add_role_assignment(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    principal_id: &str,
    principal_type: &str,
    role: &str,
) -> Result<()> {
    // Validate role
    if !KNOWN_ROLES.iter().any(|r| r.eq_ignore_ascii_case(role)) {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("Invalid role: '{role}'"),
            format!(
                "Valid roles: {}. Example: fabio workspace add-role-assignment --id <WS> --principal-id <ID> --principal-type User --role Member",
                KNOWN_ROLES.join(", ")
            ),
        )
        .into());
    }

    // Validate principal type
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

    let body = serde_json::json!({
        "principal": {
            "id": principal_id,
            "type": principal_type,
        },
        "role": role,
    });

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

// ─── Update Role Assignment ──────────────────────────────────────────────────

async fn update_role_assignment(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    assignment_id: &str,
    role: &str,
) -> Result<()> {
    // Validate role
    if !KNOWN_ROLES.iter().any(|r| r.eq_ignore_ascii_case(role)) {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("Invalid role: '{role}'"),
            format!(
                "Valid roles: {}. Example: fabio workspace update-role-assignment --id <WS> --assignment-id <ID> --role Contributor",
                KNOWN_ROLES.join(", ")
            ),
        )
        .into());
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

// ─── Delete Role Assignment ──────────────────────────────────────────────────

async fn delete_role_assignment(
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

    let obj = serde_json::json!({
        "workspaceId": id,
        "assignmentId": assignment_id,
        "status": "deleted"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Error Enrichment ────────────────────────────────────────────────────────

/// Enrich capacity assignment errors with actionable hints.
fn enrich_assign_capacity_error(err: anyhow::Error, capacity: &str) -> anyhow::Error {
    let Some(fabio_err) = err.downcast_ref::<FabioError>() else {
        return err;
    };

    let hint = format!(
        "Capacity '{capacity}' was not found or is not accessible. \
         List available capacities with: az fabric capacity list --query '[].{{name:name, id:id, state:properties.state}}' -o table. \
         Create one with: az fabric capacity create --name <name> --resource-group <rg> --location <region> --sku F2 --administration-members <email>"
    );

    FabioError::with_hint(fabio_err.code, fabio_err.message.clone(), hint).into()
}

// ─── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_roles_are_capitalized() {
        for role in KNOWN_ROLES {
            let first = role.chars().next().unwrap();
            assert!(first.is_uppercase(), "Role '{role}' should be capitalized");
        }
    }

    #[test]
    fn known_principal_types_are_capitalized() {
        for pt in KNOWN_PRINCIPAL_TYPES {
            let first = pt.chars().next().unwrap();
            assert!(
                first.is_uppercase(),
                "Principal type '{pt}' should be capitalized"
            );
        }
    }

    #[test]
    fn role_validation_is_case_insensitive() {
        assert!(KNOWN_ROLES.iter().any(|r| r.eq_ignore_ascii_case("admin")));
        assert!(KNOWN_ROLES.iter().any(|r| r.eq_ignore_ascii_case("MEMBER")));
        assert!(!KNOWN_ROLES.iter().any(|r| r.eq_ignore_ascii_case("Owner")));
    }

    #[test]
    fn principal_type_validation_is_case_insensitive() {
        assert!(
            KNOWN_PRINCIPAL_TYPES
                .iter()
                .any(|t| t.eq_ignore_ascii_case("user"))
        );
        assert!(
            KNOWN_PRINCIPAL_TYPES
                .iter()
                .any(|t| t.eq_ignore_ascii_case("GROUP"))
        );
        assert!(
            !KNOWN_PRINCIPAL_TYPES
                .iter()
                .any(|t| t.eq_ignore_ascii_case("Application"))
        );
    }

    #[test]
    fn enrich_capacity_error_preserves_non_fabio_errors() {
        let err = anyhow::anyhow!("generic error");
        let enriched = enrich_assign_capacity_error(err, "some-cap");
        assert!(enriched.to_string().contains("generic error"));
    }

    #[test]
    fn enrich_capacity_error_adds_hint() {
        let err: anyhow::Error =
            FabioError::new(ErrorCode::NotFound, "capacity not found".to_string()).into();
        let enriched = enrich_assign_capacity_error(err, "bad-cap-id");
        let fabio_err = enriched.downcast_ref::<FabioError>().unwrap();
        assert!(
            fabio_err
                .hint
                .as_ref()
                .unwrap()
                .contains("az fabric capacity")
        );
        assert!(fabio_err.hint.as_ref().unwrap().contains("bad-cap-id"));
    }
}
