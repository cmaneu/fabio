use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum AdminCommand {
    // ── Tenant Settings ──────────────────────────────────────────────────
    /// List all tenant settings
    #[command(display_order = 1)]
    ListTenantSettings,
    /// Update a tenant setting
    #[command(display_order = 2)]
    UpdateTenantSetting {
        /// Tenant setting name
        #[arg(long)]
        setting_name: String,

        /// Path to JSON file with setting body
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content for setting body
        #[arg(long)]
        content: Option<String>,
    },
    /// List all capacities' delegated tenant setting overrides
    #[command(display_order = 3)]
    ListCapacitiesTenantOverrides,
    /// List delegated tenant setting overrides for a capacity
    #[command(display_order = 4)]
    ListCapacityTenantOverrides {
        /// Capacity ID
        #[arg(long)]
        capacity_id: String,
    },
    /// Delete a capacity delegated tenant setting override
    #[command(display_order = 5)]
    DeleteCapacityTenantOverride {
        /// Capacity ID
        #[arg(long)]
        capacity_id: String,

        /// Tenant setting name
        #[arg(long)]
        setting_name: String,
    },
    /// Update a capacity delegated tenant setting override
    #[command(display_order = 6)]
    UpdateCapacityTenantOverride {
        /// Capacity ID
        #[arg(long)]
        capacity_id: String,

        /// Tenant setting name
        #[arg(long)]
        setting_name: String,

        /// Path to JSON file with override body
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content for override body
        #[arg(long)]
        content: Option<String>,
    },
    /// List all domains' delegated tenant setting overrides
    #[command(display_order = 7)]
    ListDomainsTenantOverrides,
    /// List all workspaces' delegated tenant setting overrides
    #[command(display_order = 8)]
    ListWorkspacesTenantOverrides,

    // ── Tags ─────────────────────────────────────────────────────────────
    /// List tags
    #[command(display_order = 10)]
    ListTags,
    /// Bulk-create tags
    #[command(display_order = 11)]
    CreateTags {
        /// Path to JSON file with tag definitions
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content with tag definitions
        #[arg(long)]
        content: Option<String>,
    },
    /// Update a tag
    #[command(display_order = 12)]
    UpdateTag {
        /// Tag ID
        #[arg(long)]
        tag_id: String,

        /// New tag name
        #[arg(long)]
        name: Option<String>,

        /// New tag description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a tag
    #[command(display_order = 13)]
    DeleteTag {
        /// Tag ID
        #[arg(long)]
        tag_id: String,
    },

    // ── Workloads ────────────────────────────────────────────────────────
    /// List workloads
    #[command(display_order = 20)]
    ListWorkloads,
    /// List workload assignments
    #[command(display_order = 21)]
    ListWorkloadAssignments,
    /// Create a workload assignment
    #[command(display_order = 22)]
    CreateWorkloadAssignment {
        /// Path to JSON file with assignment body
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content for assignment body
        #[arg(long)]
        content: Option<String>,
    },
    /// Delete a workload assignment
    #[command(display_order = 23)]
    DeleteWorkloadAssignment {
        /// Assignment ID
        #[arg(long)]
        assignment_id: String,
    },

    // ── Workspaces ───────────────────────────────────────────────────────
    /// List workspaces (admin view)
    #[command(display_order = 30)]
    ListWorkspaces,
    /// Show workspace details (admin view)
    #[command(display_order = 31)]
    ShowWorkspace {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// List users in a workspace (admin view)
    #[command(display_order = 32)]
    ListWorkspaceUsers {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// List git connections across workspaces
    #[command(display_order = 33)]
    ListGitConnections,
    /// Grant temporary admin access to a workspace
    #[command(display_order = 34)]
    GrantAdminAccess {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// Remove temporary admin access from a workspace
    #[command(display_order = 35)]
    RemoveAdminAccess {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// Restore a deleted workspace
    #[command(display_order = 36)]
    RestoreWorkspace {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Restored workspace name
        #[arg(long)]
        name: String,

        /// Capacity ID to assign the restored workspace
        #[arg(long)]
        capacity_id: String,
    },
    /// List network communication policies
    #[command(display_order = 37)]
    ListNetworkPolicies,

    // ── Items ────────────────────────────────────────────────────────────
    /// List items (admin view)
    #[command(display_order = 40)]
    ListItems,
    /// Show item details (admin view)
    #[command(display_order = 41)]
    ShowItem {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        item_id: String,
    },
    /// List users with access to an item (admin view)
    #[command(display_order = 42)]
    ListItemUsers {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        item_id: String,
    },
    /// Bulk-set sensitivity labels on items
    #[command(display_order = 43)]
    BulkSetLabels {
        /// Path to JSON file with label assignments
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content with label assignments
        #[arg(long)]
        content: Option<String>,
    },
    /// Bulk-remove sensitivity labels from items
    #[command(display_order = 44)]
    BulkRemoveLabels {
        /// Path to JSON file with item IDs
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content with item IDs
        #[arg(long)]
        content: Option<String>,
    },
    /// List external data shares
    #[command(display_order = 45)]
    ListExternalDataShares,
    /// Revoke an external data share
    #[command(display_order = 46)]
    RevokeExternalDataShare {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Item ID
        #[arg(long)]
        item_id: String,

        /// External data share ID
        #[arg(long)]
        share_id: String,
    },
    /// Remove all sharing links for specified items
    #[command(display_order = 47)]
    RemoveAllSharingLinks {
        /// Path to JSON file with items
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content with items
        #[arg(long)]
        content: Option<String>,
    },
    /// Bulk-remove sharing links
    #[command(display_order = 48)]
    BulkRemoveSharingLinks {
        /// Path to JSON file with sharing link removals
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content with sharing link removals
        #[arg(long)]
        content: Option<String>,
    },

    // ── Domains ──────────────────────────────────────────────────────────
    /// List domains (admin view)
    #[command(display_order = 50)]
    ListDomains,
    /// Create a domain
    #[command(display_order = 51)]
    CreateDomain {
        /// Domain name
        #[arg(long)]
        name: String,

        /// Domain description
        #[arg(long)]
        description: Option<String>,

        /// Parent domain ID (for subdomains)
        #[arg(long)]
        parent_id: Option<String>,
    },
    /// Show domain details
    #[command(display_order = 52)]
    ShowDomain {
        /// Domain ID
        #[arg(long)]
        domain_id: String,
    },
    /// Update a domain
    #[command(display_order = 53)]
    UpdateDomain {
        /// Domain ID
        #[arg(long)]
        domain_id: String,

        /// New name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a domain
    #[command(display_order = 54)]
    DeleteDomain {
        /// Domain ID
        #[arg(long)]
        domain_id: String,
    },
    /// List workspaces in a domain
    #[command(display_order = 55)]
    ListDomainWorkspaces {
        /// Domain ID
        #[arg(long)]
        domain_id: String,
    },
    /// Assign workspaces to a domain
    #[command(display_order = 56)]
    AssignDomainWorkspaces {
        /// Domain ID
        #[arg(long)]
        domain_id: String,

        /// Workspace IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        workspace_ids: Vec<String>,
    },
    /// Unassign workspaces from a domain
    #[command(display_order = 57)]
    UnassignDomainWorkspaces {
        /// Domain ID
        #[arg(long)]
        domain_id: String,

        /// Workspace IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        workspace_ids: Vec<String>,
    },
    /// Unassign all workspaces from a domain
    #[command(display_order = 58)]
    UnassignAllDomainWorkspaces {
        /// Domain ID
        #[arg(long)]
        domain_id: String,
    },
    /// List role assignments for a domain
    #[command(display_order = 59)]
    ListDomainRoleAssignments {
        /// Domain ID
        #[arg(long)]
        domain_id: String,
    },
    /// Bulk-assign roles to a domain
    #[command(display_order = 60)]
    BulkAssignDomainRoles {
        /// Domain ID
        #[arg(long)]
        domain_id: String,

        /// Path to JSON file with role assignments
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content with role assignments
        #[arg(long)]
        content: Option<String>,
    },
    /// Bulk-unassign roles from a domain
    #[command(display_order = 61)]
    BulkUnassignDomainRoles {
        /// Domain ID
        #[arg(long)]
        domain_id: String,

        /// Path to JSON file with role unassignments
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content with role unassignments
        #[arg(long)]
        content: Option<String>,
    },
    /// Sync domain role assignments to subdomains
    #[command(display_order = 62)]
    SyncDomainRolesToSubdomains {
        /// Domain ID
        #[arg(long)]
        domain_id: String,

        /// Role to sync (Contributor or Admin). Note: syncing Admins is not supported by the API.
        #[arg(long, default_value = "Contributor")]
        role: String,
    },
    /// Assign workspaces to a domain by capacities
    #[command(display_order = 63)]
    AssignDomainWorkspacesByCapacities {
        /// Domain ID
        #[arg(long)]
        domain_id: String,

        /// Capacity IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        capacity_ids: Vec<String>,
    },
    /// Assign workspaces to a domain by principals
    #[command(display_order = 64)]
    AssignDomainWorkspacesByPrincipals {
        /// Domain ID
        #[arg(long)]
        domain_id: String,

        /// Principal IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        principal_ids: Vec<String>,

        /// Principal type (User, Group, `ServicePrincipal`, `ServicePrincipalProfile`)
        #[arg(long, default_value = "User")]
        principal_type: String,
    },

    // ── Users ────────────────────────────────────────────────────────────
    /// List access details for a user
    #[command(display_order = 70)]
    ListUserAccess {
        /// User ID
        #[arg(long)]
        user_id: String,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &AdminCommand) -> Result<()> {
    match command {
        // Tenant Settings
        AdminCommand::ListTenantSettings => list_tenant_settings(cli, client).await,
        AdminCommand::UpdateTenantSetting {
            setting_name,
            file,
            content,
        } => {
            update_tenant_setting(
                cli,
                client,
                setting_name,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        AdminCommand::ListCapacitiesTenantOverrides => {
            list_capacities_tenant_overrides(cli, client).await
        }
        AdminCommand::ListCapacityTenantOverrides { capacity_id } => {
            list_capacity_tenant_overrides(cli, client, capacity_id).await
        }
        AdminCommand::DeleteCapacityTenantOverride {
            capacity_id,
            setting_name,
        } => delete_capacity_tenant_override(cli, client, capacity_id, setting_name).await,
        AdminCommand::UpdateCapacityTenantOverride {
            capacity_id,
            setting_name,
            file,
            content,
        } => {
            update_capacity_tenant_override(
                cli,
                client,
                capacity_id,
                setting_name,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        AdminCommand::ListDomainsTenantOverrides => {
            list_domains_tenant_overrides(cli, client).await
        }
        AdminCommand::ListWorkspacesTenantOverrides => {
            list_workspaces_tenant_overrides(cli, client).await
        }
        // Tags
        AdminCommand::ListTags => list_tags(cli, client).await,
        AdminCommand::CreateTags { file, content } => {
            create_tags(cli, client, file.as_deref(), content.as_deref()).await
        }
        AdminCommand::UpdateTag {
            tag_id,
            name,
            description,
        } => update_tag(cli, client, tag_id, name.as_deref(), description.as_deref()).await,
        AdminCommand::DeleteTag { tag_id } => delete_tag(cli, client, tag_id).await,
        // Workloads
        AdminCommand::ListWorkloads => list_workloads(cli, client).await,
        AdminCommand::ListWorkloadAssignments => list_workload_assignments(cli, client).await,
        AdminCommand::CreateWorkloadAssignment { file, content } => {
            create_workload_assignment(cli, client, file.as_deref(), content.as_deref()).await
        }
        AdminCommand::DeleteWorkloadAssignment { assignment_id } => {
            delete_workload_assignment(cli, client, assignment_id).await
        }
        // Workspaces
        AdminCommand::ListWorkspaces => list_workspaces(cli, client).await,
        AdminCommand::ShowWorkspace { workspace } => show_workspace(cli, client, workspace).await,
        AdminCommand::ListWorkspaceUsers { workspace } => {
            list_workspace_users(cli, client, workspace).await
        }
        AdminCommand::ListGitConnections => list_git_connections(cli, client).await,
        AdminCommand::GrantAdminAccess { workspace } => {
            grant_admin_access(cli, client, workspace).await
        }
        AdminCommand::RemoveAdminAccess { workspace } => {
            remove_admin_access(cli, client, workspace).await
        }
        AdminCommand::RestoreWorkspace {
            workspace,
            name,
            capacity_id,
        } => restore_workspace(cli, client, workspace, name, capacity_id).await,
        AdminCommand::ListNetworkPolicies => list_network_policies(cli, client).await,
        // Items
        AdminCommand::ListItems => list_items(cli, client).await,
        AdminCommand::ShowItem { workspace, item_id } => {
            show_item(cli, client, workspace, item_id).await
        }
        AdminCommand::ListItemUsers { workspace, item_id } => {
            list_item_users(cli, client, workspace, item_id).await
        }
        AdminCommand::BulkSetLabels { file, content } => {
            bulk_set_labels(cli, client, file.as_deref(), content.as_deref()).await
        }
        AdminCommand::BulkRemoveLabels { file, content } => {
            bulk_remove_labels(cli, client, file.as_deref(), content.as_deref()).await
        }
        AdminCommand::ListExternalDataShares => list_external_data_shares(cli, client).await,
        AdminCommand::RevokeExternalDataShare {
            workspace,
            item_id,
            share_id,
        } => revoke_external_data_share(cli, client, workspace, item_id, share_id).await,
        AdminCommand::RemoveAllSharingLinks { file, content } => {
            remove_all_sharing_links(cli, client, file.as_deref(), content.as_deref()).await
        }
        AdminCommand::BulkRemoveSharingLinks { file, content } => {
            bulk_remove_sharing_links(cli, client, file.as_deref(), content.as_deref()).await
        }
        // Domains
        AdminCommand::ListDomains => list_domains(cli, client).await,
        AdminCommand::CreateDomain {
            name,
            description,
            parent_id,
        } => {
            create_domain(
                cli,
                client,
                name,
                description.as_deref(),
                parent_id.as_deref(),
            )
            .await
        }
        AdminCommand::ShowDomain { domain_id } => show_domain(cli, client, domain_id).await,
        AdminCommand::UpdateDomain {
            domain_id,
            name,
            description,
        } => {
            update_domain(
                cli,
                client,
                domain_id,
                name.as_deref(),
                description.as_deref(),
            )
            .await
        }
        AdminCommand::DeleteDomain { domain_id } => delete_domain(cli, client, domain_id).await,
        AdminCommand::ListDomainWorkspaces { domain_id } => {
            list_domain_workspaces(cli, client, domain_id).await
        }
        AdminCommand::AssignDomainWorkspaces {
            domain_id,
            workspace_ids,
        } => assign_domain_workspaces(cli, client, domain_id, workspace_ids).await,
        AdminCommand::UnassignDomainWorkspaces {
            domain_id,
            workspace_ids,
        } => unassign_domain_workspaces(cli, client, domain_id, workspace_ids).await,
        AdminCommand::UnassignAllDomainWorkspaces { domain_id } => {
            unassign_all_domain_workspaces(cli, client, domain_id).await
        }
        AdminCommand::ListDomainRoleAssignments { domain_id } => {
            list_domain_role_assignments(cli, client, domain_id).await
        }
        AdminCommand::BulkAssignDomainRoles {
            domain_id,
            file,
            content,
        } => {
            bulk_assign_domain_roles(cli, client, domain_id, file.as_deref(), content.as_deref())
                .await
        }
        AdminCommand::BulkUnassignDomainRoles {
            domain_id,
            file,
            content,
        } => {
            bulk_unassign_domain_roles(cli, client, domain_id, file.as_deref(), content.as_deref())
                .await
        }
        AdminCommand::SyncDomainRolesToSubdomains { domain_id, role } => {
            sync_domain_roles_to_subdomains(cli, client, domain_id, role).await
        }
        AdminCommand::AssignDomainWorkspacesByCapacities {
            domain_id,
            capacity_ids,
        } => assign_domain_workspaces_by_capacities(cli, client, domain_id, capacity_ids).await,
        AdminCommand::AssignDomainWorkspacesByPrincipals {
            domain_id,
            principal_ids,
            principal_type,
        } => {
            assign_domain_workspaces_by_principals(
                cli,
                client,
                domain_id,
                principal_ids,
                principal_type,
            )
            .await
        }
        // Users
        AdminCommand::ListUserAccess { user_id } => list_user_access(cli, client, user_id).await,
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn read_body(file: Option<&str>, content: Option<&str>, command: &str) -> Result<Value> {
    let raw = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(c)) => c.to_string(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                format!("Example: fabio admin {command} --file body.json"),
            )
            .into());
        }
    };
    let body: Value =
        serde_json::from_str(&raw).map_err(|e| anyhow::anyhow!("Invalid JSON in input: {e}"))?;
    Ok(body)
}

// ─── Tenant Settings ─────────────────────────────────────────────────────────

async fn list_tenant_settings(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/tenantsettings",
            "tenantSettings",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["settingName", "title", "enabled"],
        &["SETTING", "TITLE", "ENABLED"],
        "settingName",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn update_tenant_setting(
    cli: &Cli,
    client: &FabricClient,
    setting_name: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "update-tenant-setting")?;

    if output::dry_run_guard(cli, "admin update-tenant-setting", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/admin/tenantsettings/{setting_name}/update"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "admin update-tenant-setting", "Fabric Admin"))?;
    output::render_object(cli, &data, "settingName");
    Ok(())
}

async fn list_capacities_tenant_overrides(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/capacities/delegatedTenantSettingOverrides",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["capacityId", "tenantSettingName"],
        &["CAPACITY", "SETTING"],
        "capacityId",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn list_capacity_tenant_overrides(
    cli: &Cli,
    client: &FabricClient,
    capacity_id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/admin/capacities/{capacity_id}/delegatedTenantSettingOverrides"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["tenantSettingName", "enabled"],
        &["SETTING", "ENABLED"],
        "tenantSettingName",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn delete_capacity_tenant_override(
    cli: &Cli,
    client: &FabricClient,
    capacity_id: &str,
    setting_name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "admin delete-capacity-tenant-override",
        &serde_json::json!({ "capacityId": capacity_id, "settingName": setting_name }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!(
            "/admin/capacities/{capacity_id}/delegatedTenantSettingOverrides/{setting_name}"
        ))
        .await
        .map_err(|e| {
            enrich_forbidden(e, "admin delete-capacity-tenant-override", "Fabric Admin")
        })?;

    let obj = serde_json::json!({ "capacityId": capacity_id, "settingName": setting_name, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn update_capacity_tenant_override(
    cli: &Cli,
    client: &FabricClient,
    capacity_id: &str,
    setting_name: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "update-capacity-tenant-override")?;

    if output::dry_run_guard(cli, "admin update-capacity-tenant-override", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!(
                "/admin/capacities/{capacity_id}/delegatedTenantSettingOverrides/{setting_name}/update"
            ),
            &body,
            false,
        )
        .await
        .map_err(|e| {
            enrich_forbidden(e, "admin update-capacity-tenant-override", "Fabric Admin")
        })?;
    output::render_object(cli, &data, "tenantSettingName");
    Ok(())
}

async fn list_domains_tenant_overrides(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/domains/delegatedTenantSettingOverrides",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["domainId", "tenantSettingName"],
        &["DOMAIN", "SETTING"],
        "domainId",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn list_workspaces_tenant_overrides(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/workspaces/delegatedTenantSettingOverrides",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["workspaceId", "tenantSettingName"],
        &["WORKSPACE", "SETTING"],
        "workspaceId",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

// ─── Tags ────────────────────────────────────────────────────────────────────

async fn list_tags(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/tags",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "displayName", "description"],
        &["ID", "NAME", "DESCRIPTION"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn create_tags(
    cli: &Cli,
    client: &FabricClient,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "create-tags")?;

    if output::dry_run_guard(cli, "admin create-tags", &body) {
        return Ok(());
    }

    let data = client
        .post("/admin/tags/bulkCreateTags", &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "admin create-tags", "Fabric Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update_tag(
    cli: &Cli,
    client: &FabricClient,
    tag_id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --name or --description must be provided".to_string(),
            "Example: fabio admin update-tag --tag-id <ID> --name \"New Name\"".to_string(),
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

    if output::dry_run_guard(cli, "admin update-tag", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/admin/tags/{tag_id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "admin update-tag", "Fabric Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete_tag(cli: &Cli, client: &FabricClient, tag_id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "admin delete-tag",
        &serde_json::json!({ "tagId": tag_id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/admin/tags/{tag_id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "admin delete-tag", "Fabric Admin"))?;

    let obj = serde_json::json!({ "tagId": tag_id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Workloads ───────────────────────────────────────────────────────────────

async fn list_workloads(cli: &Cli, client: &FabricClient) -> Result<()> {
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

async fn list_workload_assignments(cli: &Cli, client: &FabricClient) -> Result<()> {
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

async fn create_workload_assignment(
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
        .map_err(|e| enrich_forbidden(e, "admin create-workload-assignment", "Fabric Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete_workload_assignment(
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
        .map_err(|e| enrich_forbidden(e, "admin delete-workload-assignment", "Fabric Admin"))?;

    let obj = serde_json::json!({ "assignmentId": assignment_id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Workspaces ──────────────────────────────────────────────────────────────

async fn list_workspaces(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/workspaces",
            "workspaces",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["name", "id", "state", "type", "capacityId"],
        &["NAME", "ID", "STATE", "TYPE", "CAPACITY"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn show_workspace(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!("/admin/workspaces/{workspace}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn list_workspace_users(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/admin/workspaces/{workspace}/users"),
            "accessDetails",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["principal", "workspaceAccessDetails"],
        &["PRINCIPAL", "ACCESS"],
        "principal",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn list_git_connections(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/workspaces/discoverGitConnections",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["workspaceId", "gitProviderType"],
        &["WORKSPACE", "PROVIDER"],
        "workspaceId",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn grant_admin_access(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let body = serde_json::json!({});

    if output::dry_run_guard(cli, "admin grant-admin-access", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/workspaces/{workspace}/grantAdminTemporaryAccess"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "admin grant-admin-access", "Fabric Admin"))?;

    let obj = serde_json::json!({ "workspaceId": workspace, "status": "granted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn remove_admin_access(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let body = serde_json::json!({});

    if output::dry_run_guard(cli, "admin remove-admin-access", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/workspaces/{workspace}/removeAdminTemporaryAccess"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "admin remove-admin-access", "Fabric Admin"))?;

    let obj = serde_json::json!({ "workspaceId": workspace, "status": "removed" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn restore_workspace(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    capacity_id: &str,
) -> Result<()> {
    let body = serde_json::json!({
        "restoredWorkspaceName": name,
        "capacityId": capacity_id
    });

    if output::dry_run_guard(cli, "admin restore-workspace", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/admin/workspaces/{workspace}/restore"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "admin restore-workspace", "Fabric Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn list_network_policies(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/workspaces/networking/communicationpolicies",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "workspaceId", "policyType"],
        &["ID", "WORKSPACE", "TYPE"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

// ─── Items ───────────────────────────────────────────────────────────────────

async fn list_items(cli: &Cli, client: &FabricClient) -> Result<()> {
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

async fn show_item(cli: &Cli, client: &FabricClient, workspace: &str, item_id: &str) -> Result<()> {
    let data = client
        .get(&format!("/admin/workspaces/{workspace}/items/{item_id}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn list_item_users(
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

async fn bulk_set_labels(
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
        .map_err(|e| enrich_forbidden(e, "admin bulk-set-labels", "Fabric Admin"))?;
    output::render_object(cli, &data, "status");
    Ok(())
}

async fn bulk_remove_labels(
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
        .map_err(|e| enrich_forbidden(e, "admin bulk-remove-labels", "Fabric Admin"))?;
    output::render_object(cli, &data, "status");
    Ok(())
}

async fn list_external_data_shares(cli: &Cli, client: &FabricClient) -> Result<()> {
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

async fn revoke_external_data_share(
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
        .map_err(|e| enrich_forbidden(e, "admin revoke-external-data-share", "Fabric Admin"))?;

    let obj = serde_json::json!({ "shareId": share_id, "status": "revoked" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn remove_all_sharing_links(
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
        .post("/admin/items/removeAllSharingLinks", &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "admin remove-all-sharing-links", "Fabric Admin"))?;
    output::render_object(cli, &data, "status");
    Ok(())
}

async fn bulk_remove_sharing_links(
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
        .post("/admin/items/bulkRemoveSharingLinks", &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "admin bulk-remove-sharing-links", "Fabric Admin"))?;
    output::render_object(cli, &data, "status");
    Ok(())
}

// ─── Domains ─────────────────────────────────────────────────────────────────

async fn list_domains(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/domains",
            "domains",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id", "description"],
        &["NAME", "ID", "DESCRIPTION"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn create_domain(
    cli: &Cli,
    client: &FabricClient,
    name: &str,
    description: Option<&str>,
    parent_id: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }
    if let Some(parent) = parent_id {
        body["parentDomainId"] = Value::String(parent.to_string());
    }

    if output::dry_run_guard(cli, "admin create-domain", &body) {
        return Ok(());
    }

    let data = client
        .post("/admin/domains", &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "admin create-domain", "Fabric Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn show_domain(cli: &Cli, client: &FabricClient, domain_id: &str) -> Result<()> {
    let data = client.get(&format!("/admin/domains/{domain_id}")).await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update_domain(
    cli: &Cli,
    client: &FabricClient,
    domain_id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --name or --description must be provided".to_string(),
            "Example: fabio admin update-domain --domain-id <ID> --name \"New Name\"".to_string(),
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

    if output::dry_run_guard(cli, "admin update-domain", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/admin/domains/{domain_id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "admin update-domain", "Fabric Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete_domain(cli: &Cli, client: &FabricClient, domain_id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "admin delete-domain",
        &serde_json::json!({ "domainId": domain_id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/admin/domains/{domain_id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "admin delete-domain", "Fabric Admin"))?;

    let obj = serde_json::json!({ "domainId": domain_id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn list_domain_workspaces(cli: &Cli, client: &FabricClient, domain_id: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/admin/domains/{domain_id}/workspaces"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

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

async fn assign_domain_workspaces(
    cli: &Cli,
    client: &FabricClient,
    domain_id: &str,
    workspace_ids: &[String],
) -> Result<()> {
    let body = serde_json::json!({ "workspacesIds": workspace_ids });

    if output::dry_run_guard(cli, "admin assign-domain-workspaces", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/domains/{domain_id}/assignWorkspaces"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "admin assign-domain-workspaces", "Fabric Admin"))?;

    let obj = serde_json::json!({
        "domainId": domain_id,
        "workspacesAssigned": workspace_ids.len(),
        "status": "assigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn unassign_domain_workspaces(
    cli: &Cli,
    client: &FabricClient,
    domain_id: &str,
    workspace_ids: &[String],
) -> Result<()> {
    let body = serde_json::json!({ "workspacesIds": workspace_ids });

    if output::dry_run_guard(cli, "admin unassign-domain-workspaces", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/domains/{domain_id}/unassignWorkspaces"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "admin unassign-domain-workspaces", "Fabric Admin"))?;

    let obj = serde_json::json!({
        "domainId": domain_id,
        "workspacesUnassigned": workspace_ids.len(),
        "status": "unassigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn unassign_all_domain_workspaces(
    cli: &Cli,
    client: &FabricClient,
    domain_id: &str,
) -> Result<()> {
    let body = serde_json::json!({});

    if output::dry_run_guard(cli, "admin unassign-all-domain-workspaces", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/domains/{domain_id}/unassignAllWorkspaces"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "admin unassign-all-domain-workspaces", "Fabric Admin"))?;

    let obj = serde_json::json!({ "domainId": domain_id, "status": "all_unassigned" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn list_domain_role_assignments(
    cli: &Cli,
    client: &FabricClient,
    domain_id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/admin/domains/{domain_id}/roleAssignments"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "principal", "role"],
        &["ID", "PRINCIPAL", "ROLE"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn bulk_assign_domain_roles(
    cli: &Cli,
    client: &FabricClient,
    domain_id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "bulk-assign-domain-roles")?;

    if output::dry_run_guard(cli, "admin bulk-assign-domain-roles", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/admin/domains/{domain_id}/roleAssignments/bulkAssign"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "admin bulk-assign-domain-roles", "Fabric Admin"))?;
    output::render_object(cli, &data, "status");
    Ok(())
}

async fn bulk_unassign_domain_roles(
    cli: &Cli,
    client: &FabricClient,
    domain_id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "bulk-unassign-domain-roles")?;

    if output::dry_run_guard(cli, "admin bulk-unassign-domain-roles", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/admin/domains/{domain_id}/roleAssignments/bulkUnassign"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "admin bulk-unassign-domain-roles", "Fabric Admin"))?;
    output::render_object(cli, &data, "status");
    Ok(())
}

async fn sync_domain_roles_to_subdomains(
    cli: &Cli,
    client: &FabricClient,
    domain_id: &str,
    role: &str,
) -> Result<()> {
    let body = serde_json::json!({"role": role});

    if output::dry_run_guard(cli, "admin sync-domain-roles-to-subdomains", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/domains/{domain_id}/roleAssignments/syncToSubdomains"),
            &body,
            false,
        )
        .await
        .map_err(|e| {
            enrich_forbidden(e, "admin sync-domain-roles-to-subdomains", "Fabric Admin")
        })?;

    let obj = serde_json::json!({ "domainId": domain_id, "status": "synced" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn assign_domain_workspaces_by_capacities(
    cli: &Cli,
    client: &FabricClient,
    domain_id: &str,
    capacity_ids: &[String],
) -> Result<()> {
    let body = serde_json::json!({ "capacitiesIds": capacity_ids });

    if output::dry_run_guard(cli, "admin assign-domain-workspaces-by-capacities", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/domains/{domain_id}/assignWorkspacesByCapacities"),
            &body,
            false,
        )
        .await
        .map_err(|e| {
            enrich_forbidden(
                e,
                "admin assign-domain-workspaces-by-capacities",
                "Fabric Admin",
            )
        })?;

    let obj = serde_json::json!({
        "domainId": domain_id,
        "capacitiesUsed": capacity_ids.len(),
        "status": "assigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn assign_domain_workspaces_by_principals(
    cli: &Cli,
    client: &FabricClient,
    domain_id: &str,
    principal_ids: &[String],
    principal_type: &str,
) -> Result<()> {
    let principals: Vec<Value> = principal_ids
        .iter()
        .map(|id| serde_json::json!({ "id": id, "type": principal_type }))
        .collect();
    let body = serde_json::json!({ "principals": principals });

    if output::dry_run_guard(cli, "admin assign-domain-workspaces-by-principals", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/domains/{domain_id}/assignWorkspacesByPrincipals"),
            &body,
            false,
        )
        .await
        .map_err(|e| {
            enrich_forbidden(
                e,
                "admin assign-domain-workspaces-by-principals",
                "Fabric Admin",
            )
        })?;

    let obj = serde_json::json!({
        "domainId": domain_id,
        "principalsUsed": principal_ids.len(),
        "status": "assigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Users ───────────────────────────────────────────────────────────────────

async fn list_user_access(cli: &Cli, client: &FabricClient, user_id: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/admin/users/{user_id}/access"),
            "accessEntities",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "displayName", "type", "accessDetails"],
        &["ID", "NAME", "TYPE", "ACCESS"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}
