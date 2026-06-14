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
    List {
        /// Filter by role: Admin, Member, Contributor, Viewer (comma-separated)
        #[arg(long)]
        roles: Option<String>,

        /// Filter by capacity ID (client-side filter)
        #[arg(long)]
        capacity: Option<String>,
    },
    /// Show details of a workspace
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(long)]
        id: String,
    },
    /// Get the Fabric portal URL for a workspace
    #[command(display_order = 3)]
    Url {
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
        #[arg(long, visible_alias = "workspace")]
        id: String,

        /// Target capacity ID
        #[arg(short, long, visible_alias = "capacity-id", env = "FABIO_CAPACITY")]
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

    /// Show a specific workspace role assignment
    #[command(display_order = 16)]
    ShowRoleAssignment {
        /// Workspace ID
        #[arg(long)]
        id: String,

        /// Role assignment ID
        #[arg(long)]
        assignment_id: String,
    },

    // ── Folder Management ────────────────────────────────────────────────
    /// List workspace folders
    #[command(display_order = 30)]
    ListFolders {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Create a folder in a workspace
    #[command(display_order = 31)]
    CreateFolder {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Folder display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Optional parent folder ID (omit for root)
        #[arg(long)]
        parent_folder_id: Option<String>,
    },
    /// Show details of a workspace folder
    #[command(display_order = 32)]
    ShowFolder {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Folder ID
        #[arg(long)]
        folder_id: String,
    },
    /// Update a workspace folder
    #[command(display_order = 33)]
    UpdateFolder {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Folder ID
        #[arg(long)]
        folder_id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a workspace folder
    #[command(display_order = 34)]
    DeleteFolder {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Folder ID
        #[arg(long)]
        folder_id: String,
    },
    /// Move a folder to another parent (or root)
    #[command(display_order = 35)]
    MoveFolder {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Folder ID to move
        #[arg(long)]
        folder_id: String,

        /// Target parent folder ID (omit to move to root)
        #[arg(long)]
        target_folder_id: Option<String>,
    },

    // ── Tags ─────────────────────────────────────────────────────────────
    /// Apply tags to a workspace
    #[command(display_order = 40)]
    ApplyTags {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Comma-separated tag IDs
        #[arg(long, value_delimiter = ',')]
        tag_ids: Vec<String>,
    },
    /// Remove tags from a workspace
    #[command(display_order = 41)]
    UnapplyTags {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Comma-separated tag IDs
        #[arg(long, value_delimiter = ',')]
        tag_ids: Vec<String>,
    },

    // ── Domain ───────────────────────────────────────────────────────────
    /// Assign workspace to a domain
    #[command(display_order = 45)]
    AssignToDomain {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Domain ID
        #[arg(long)]
        domain_id: String,
    },
    /// Unassign workspace from its domain
    #[command(display_order = 46)]
    UnassignFromDomain {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },

    // ── OneLake Settings ─────────────────────────────────────────────────
    /// Get `OneLake` settings for a workspace
    #[command(display_order = 55)]
    GetOnelakeSettings {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Modify `OneLake` default tier (Hot, Cool, or Cold)
    #[command(display_order = 56)]
    ModifyDefaultTier {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Tier: "Hot", "Cool", or "Cold"
        #[arg(long)]
        tier: String,
    },
    /// Modify `OneLake` diagnostics configuration
    #[command(display_order = 57)]
    ModifyDiagnostics {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Path to JSON file with diagnostics config
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON diagnostics config
        #[arg(long)]
        content: Option<String>,
    },
    /// Modify `OneLake` immutability policy
    #[command(display_order = 58)]
    ModifyImmutabilityPolicy {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Path to JSON file with policy config
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON policy config
        #[arg(long)]
        content: Option<String>,
    },
    /// Export `OneLake` lifecycle policy
    #[command(display_order = 59)]
    ExportLifecyclePolicy {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Import `OneLake` lifecycle policy
    #[command(display_order = 60)]
    ImportLifecyclePolicy {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Path to JSON file with lifecycle policy
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON lifecycle policy
        #[arg(long)]
        content: Option<String>,
    },
    /// Reset `OneLake` shortcut cache for a workspace
    #[command(display_order = 61)]
    ResetShortcutCache {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },

    // ── Networking ───────────────────────────────────────────────────────
    /// Get workspace network communication policy
    #[command(display_order = 50)]
    GetNetworkPolicy {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Set workspace network communication policy
    #[command(display_order = 51)]
    SetNetworkPolicy {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Path to JSON file with policy configuration
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON policy configuration
        #[arg(long)]
        content: Option<String>,
    },
    /// Get workspace IP firewall rules
    #[command(display_order = 52)]
    GetFirewallRules {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Set workspace IP firewall rules (replaces all existing rules)
    #[command(display_order = 53)]
    SetFirewallRules {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Path to JSON file with firewall rules
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON firewall rules (e.g. '{"rules":[{"displayName":"Allow Office","value":"10.0.0.0/24"}]}')
        #[arg(long)]
        content: Option<String>,
    },
    /// Get workspace git outbound policy
    #[command(display_order = 54)]
    GetGitOutboundPolicy {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Set workspace git outbound policy (requires Outbound Access Protection enabled)
    #[command(display_order = 54)]
    SetGitOutboundPolicy {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Path to JSON file with git outbound policy
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON git outbound policy (e.g. '{"defaultAction":"Allow","rules":[]}')
        #[arg(long)]
        content: Option<String>,
    },
    /// Get workspace inbound Azure resource instance rules
    #[command(display_order = 54)]
    GetInboundAzureResourceRules {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Set workspace inbound Azure resource instance rules
    #[command(display_order = 54)]
    SetInboundAzureResourceRules {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Path to JSON file with resource rules
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON resource rules
        #[arg(long)]
        content: Option<String>,
    },
    /// Get workspace outbound cloud connection rules (requires OAP enabled)
    #[command(display_order = 54)]
    GetOutboundCloudConnectionRules {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Set workspace outbound cloud connection rules (requires OAP enabled)
    #[command(display_order = 54)]
    SetOutboundCloudConnectionRules {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Path to JSON file with cloud connection rules
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON cloud connection rules
        #[arg(long)]
        content: Option<String>,
    },
    /// Get workspace outbound gateway rules (requires OAP enabled)
    #[command(display_order = 54)]
    GetOutboundGatewayRules {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Set workspace outbound gateway rules (requires OAP enabled)
    #[command(display_order = 54)]
    SetOutboundGatewayRules {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Path to JSON file with gateway rules
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON gateway rules
        #[arg(long)]
        content: Option<String>,
    },

    // ── Workspace Settings ───────────────────────────────────────────────
    /// Get workspace settings (properties including `automaticMetadataSync`)
    #[command(display_order = 62)]
    GetSettings {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Update workspace settings (e.g. enable automatic metadata sync)
    ///
    /// Pass a JSON object with the properties to update. Example:
    ///   fabio workspace update-settings -w <WS> --content '{"automaticMetadataSync":"Enabled"}'
    #[command(display_order = 63)]
    UpdateSettings {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Path to JSON file with settings
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON settings (e.g. '{"automaticMetadataSync":"Enabled"}')
        #[arg(long)]
        content: Option<String>,
    },
    /// Set default dataset storage format (Small or Large) via Power BI API
    #[command(display_order = 64)]
    SetDatasetStorageFormat {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// Storage format: "Small" or "Large"
        #[arg(long)]
        format: String,
    },
    /// Get default dataset storage format via Power BI API
    #[command(display_order = 65)]
    GetDatasetStorageFormat {
        /// Workspace ID
        #[arg(short = 'w', long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &WorkspaceCommand) -> Result<()> {
    match command {
        WorkspaceCommand::List { roles, capacity } => {
            list(cli, client, roles.as_deref(), capacity.as_deref()).await
        }
        WorkspaceCommand::Show { id } => show(cli, client, id).await,
        WorkspaceCommand::Url { id } => url(cli, id),
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
        WorkspaceCommand::ShowRoleAssignment { id, assignment_id } => {
            show_role_assignment(cli, client, id, assignment_id).await
        }
        WorkspaceCommand::ListFolders { workspace } => list_folders(cli, client, workspace).await,
        WorkspaceCommand::CreateFolder {
            workspace,
            name,
            description,
            parent_folder_id,
        } => {
            create_folder(
                cli,
                client,
                workspace,
                name,
                description.as_deref(),
                parent_folder_id.as_deref(),
            )
            .await
        }
        WorkspaceCommand::ShowFolder {
            workspace,
            folder_id,
        } => show_folder(cli, client, workspace, folder_id).await,
        WorkspaceCommand::UpdateFolder {
            workspace,
            folder_id,
            name,
            description,
        } => {
            update_folder(
                cli,
                client,
                workspace,
                folder_id,
                name.as_deref(),
                description.as_deref(),
            )
            .await
        }
        WorkspaceCommand::DeleteFolder {
            workspace,
            folder_id,
        } => delete_folder(cli, client, workspace, folder_id).await,
        WorkspaceCommand::MoveFolder {
            workspace,
            folder_id,
            target_folder_id,
        } => {
            move_folder(
                cli,
                client,
                workspace,
                folder_id,
                target_folder_id.as_deref(),
            )
            .await
        }
        WorkspaceCommand::ApplyTags { workspace, tag_ids } => {
            apply_tags(cli, client, workspace, tag_ids).await
        }
        WorkspaceCommand::UnapplyTags { workspace, tag_ids } => {
            unapply_tags(cli, client, workspace, tag_ids).await
        }
        WorkspaceCommand::AssignToDomain {
            workspace,
            domain_id,
        } => assign_to_domain(cli, client, workspace, domain_id).await,
        WorkspaceCommand::UnassignFromDomain { workspace } => {
            unassign_from_domain(cli, client, workspace).await
        }
        WorkspaceCommand::GetOnelakeSettings { workspace } => {
            get_onelake_settings(cli, client, workspace).await
        }
        WorkspaceCommand::ModifyDefaultTier { workspace, tier } => {
            modify_default_tier(cli, client, workspace, tier).await
        }
        WorkspaceCommand::ModifyDiagnostics {
            workspace,
            file,
            content,
        } => modify_diagnostics(cli, client, workspace, file.as_deref(), content.as_deref()).await,
        WorkspaceCommand::ModifyImmutabilityPolicy {
            workspace,
            file,
            content,
        } => {
            modify_immutability_policy(cli, client, workspace, file.as_deref(), content.as_deref())
                .await
        }
        WorkspaceCommand::ExportLifecyclePolicy { workspace } => {
            export_lifecycle_policy(cli, client, workspace).await
        }
        WorkspaceCommand::ImportLifecyclePolicy {
            workspace,
            file,
            content,
        } => {
            import_lifecycle_policy(cli, client, workspace, file.as_deref(), content.as_deref())
                .await
        }
        WorkspaceCommand::ResetShortcutCache { workspace } => {
            reset_shortcut_cache(cli, client, workspace).await
        }
        WorkspaceCommand::GetNetworkPolicy { workspace } => {
            get_network_policy(cli, client, workspace).await
        }
        WorkspaceCommand::SetNetworkPolicy {
            workspace,
            file,
            content,
        } => set_network_policy(cli, client, workspace, file.as_deref(), content.as_deref()).await,
        WorkspaceCommand::GetFirewallRules { workspace } => {
            get_firewall_rules(cli, client, workspace).await
        }
        WorkspaceCommand::SetFirewallRules {
            workspace,
            file,
            content,
        } => set_firewall_rules(cli, client, workspace, file.as_deref(), content.as_deref()).await,
        WorkspaceCommand::GetGitOutboundPolicy { workspace } => {
            get_git_outbound_policy(cli, client, workspace).await
        }
        WorkspaceCommand::SetGitOutboundPolicy {
            workspace,
            file,
            content,
        } => {
            set_git_outbound_policy(cli, client, workspace, file.as_deref(), content.as_deref())
                .await
        }
        WorkspaceCommand::GetInboundAzureResourceRules { workspace } => {
            get_inbound_azure_resource_rules(cli, client, workspace).await
        }
        WorkspaceCommand::SetInboundAzureResourceRules {
            workspace,
            file,
            content,
        } => {
            set_inbound_azure_resource_rules(
                cli,
                client,
                workspace,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        WorkspaceCommand::GetOutboundCloudConnectionRules { workspace } => {
            get_outbound_cloud_connection_rules(cli, client, workspace).await
        }
        WorkspaceCommand::SetOutboundCloudConnectionRules {
            workspace,
            file,
            content,
        } => {
            set_outbound_cloud_connection_rules(
                cli,
                client,
                workspace,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        WorkspaceCommand::GetOutboundGatewayRules { workspace } => {
            get_outbound_gateway_rules(cli, client, workspace).await
        }
        WorkspaceCommand::SetOutboundGatewayRules {
            workspace,
            file,
            content,
        } => {
            set_outbound_gateway_rules(cli, client, workspace, file.as_deref(), content.as_deref())
                .await
        }
        WorkspaceCommand::GetSettings { workspace } => get_settings(cli, client, workspace).await,
        WorkspaceCommand::UpdateSettings {
            workspace,
            file,
            content,
        } => update_settings(cli, client, workspace, file.as_deref(), content.as_deref()).await,
        WorkspaceCommand::SetDatasetStorageFormat { workspace, format } => {
            set_dataset_storage_format(cli, client, workspace, format).await
        }
        WorkspaceCommand::GetDatasetStorageFormat { workspace } => {
            get_dataset_storage_format(cli, client, workspace).await
        }
    }
}

// ─── List ────────────────────────────────────────────────────────────────────

async fn list(
    cli: &Cli,
    client: &FabricClient,
    roles: Option<&str>,
    capacity: Option<&str>,
) -> Result<()> {
    let path = roles.map_or_else(
        || "/workspaces".to_string(),
        |r| format!("/workspaces?roles={r}"),
    );
    let resp = client
        .get_list(&path, "value", cli.all, cli.continuation_token.as_deref())
        .await?;

    // Client-side filter by capacity ID if specified
    let items = if let Some(cap_id) = capacity {
        resp.items
            .into_iter()
            .filter(|item| {
                item.get("capacityId")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| id.eq_ignore_ascii_case(cap_id))
            })
            .collect()
    } else {
        resp.items
    };

    output::render_list_with_token(
        cli,
        &items,
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

#[allow(clippy::unnecessary_wraps)]
fn url(cli: &Cli, id: &str) -> Result<()> {
    let portal_url = format!("https://app.fabric.microsoft.com/groups/{id}");
    let data = serde_json::json!({ "url": portal_url, "workspaceId": id });
    output::render_object(cli, &data, "url");
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

// ─── Show Role Assignment ────────────────────────────────────────────────────

async fn show_role_assignment(
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

// ─── List Folders ────────────────────────────────────────────────────────────

async fn list_folders(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
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

// ─── Create Folder ───────────────────────────────────────────────────────────

async fn create_folder(
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

// ─── Show Folder ─────────────────────────────────────────────────────────────

async fn show_folder(
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

// ─── Update Folder ───────────────────────────────────────────────────────────

async fn update_folder(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    folder_id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --name or --description must be provided".to_string(),
            "Example: fabio workspace update-folder --workspace <WS> --folder-id <ID> --name \"New Name\"".to_string(),
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

// ─── Delete Folder ───────────────────────────────────────────────────────────

async fn delete_folder(
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

    let obj = serde_json::json!({
        "workspaceId": workspace,
        "folderId": folder_id,
        "status": "deleted"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Move Folder ─────────────────────────────────────────────────────────────

async fn move_folder(
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

// ─── Apply Tags ──────────────────────────────────────────────────────────────

async fn apply_tags(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    tag_ids: &[String],
) -> Result<()> {
    let body = serde_json::json!({ "tagIds": tag_ids });

    if output::dry_run_guard(cli, "workspace apply-tags", &body) {
        return Ok(());
    }

    client
        .post(&format!("/workspaces/{workspace}/applyTags"), &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "workspace apply-tags", "Admin"))?;

    let obj = serde_json::json!({
        "workspaceId": workspace,
        "tagIds": tag_ids,
        "status": "applied"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Unapply Tags ────────────────────────────────────────────────────────────

async fn unapply_tags(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    tag_ids: &[String],
) -> Result<()> {
    let body = serde_json::json!({ "tagIds": tag_ids });

    if output::dry_run_guard(cli, "workspace unapply-tags", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/unapplyTags"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace unapply-tags", "Admin"))?;

    let obj = serde_json::json!({
        "workspaceId": workspace,
        "tagIds": tag_ids,
        "status": "unapplied"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Assign to Domain ────────────────────────────────────────────────────────

async fn assign_to_domain(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    domain_id: &str,
) -> Result<()> {
    let body = serde_json::json!({ "domainId": domain_id });

    if output::dry_run_guard(cli, "workspace assign-to-domain", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/assignToDomain"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace assign-to-domain", "Admin"))?;

    let obj = serde_json::json!({
        "workspaceId": workspace,
        "domainId": domain_id,
        "status": "assigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Unassign from Domain ────────────────────────────────────────────────────

async fn unassign_from_domain(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "workspace unassign-from-domain",
        &serde_json::json!({ "workspaceId": workspace }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/unassignFromDomain"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace unassign-from-domain", "Admin"))?;

    let obj = serde_json::json!({
        "workspaceId": workspace,
        "status": "unassigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── OneLake Settings ────────────────────────────────────────────────────────

async fn get_onelake_settings(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/onelake/settings"))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace get-onelake-settings", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn modify_default_tier(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    tier: &str,
) -> Result<()> {
    let body = serde_json::json!({ "defaultTier": tier });

    if output::dry_run_guard(cli, "workspace modify-default-tier", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/onelake/settings/modifyDefaultTier?defaultTier={tier}"
            ),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace modify-default-tier", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn modify_diagnostics(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "workspace modify-diagnostics")?;

    if output::dry_run_guard(cli, "workspace modify-diagnostics", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/onelake/settings/modifyDiagnostics"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace modify-diagnostics", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn modify_immutability_policy(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "workspace modify-immutability-policy")?;

    if output::dry_run_guard(cli, "workspace modify-immutability-policy", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/onelake/settings/modifyImmutabilityPolicy"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace modify-immutability-policy", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn export_lifecycle_policy(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/onelake/lifecycle/exportPolicy"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace export-lifecycle-policy", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn import_lifecycle_policy(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "workspace import-lifecycle-policy")?;

    if output::dry_run_guard(cli, "workspace import-lifecycle-policy", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/onelake/lifecycle/importPolicy"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace import-lifecycle-policy", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn reset_shortcut_cache(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "workspace reset-shortcut-cache",
        &serde_json::json!({ "workspaceId": workspace }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/onelake/resetShortcutCache"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace reset-shortcut-cache", "Admin"))?;

    let obj = serde_json::json!({
        "workspaceId": workspace,
        "status": "cache_reset"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Get Network Policy ──────────────────────────────────────────────────────

async fn get_network_policy(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/networking/communicationPolicy"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace get-network-policy", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

// ─── Set Network Policy ──────────────────────────────────────────────────────

async fn set_network_policy(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let raw = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(c)) => c.to_string(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio workspace set-network-policy --workspace <WS> --file policy.json"
                    .to_string(),
            )
            .into());
        }
    };

    let body: Value =
        serde_json::from_str(&raw).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}"))?;

    if output::dry_run_guard(cli, "workspace set-network-policy", &body) {
        return Ok(());
    }

    let data = client
        .put(
            &format!("/workspaces/{workspace}/networking/communicationPolicy"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace set-network-policy", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

// ─── Firewall Rules ──────────────────────────────────────────────────────────

async fn get_firewall_rules(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/networking/communicationPolicy/inbound/firewall"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace get-firewall-rules", "Viewer"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn set_firewall_rules(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "workspace set-firewall-rules")?;

    if output::dry_run_guard(cli, "workspace set-firewall-rules", &body) {
        return Ok(());
    }

    let data = client
        .put(
            &format!("/workspaces/{workspace}/networking/communicationPolicy/inbound/firewall"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace set-firewall-rules", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

// ─── Git Outbound Policy ─────────────────────────────────────────────────────

async fn get_git_outbound_policy(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/networking/communicationPolicy/outbound/git"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace get-git-outbound-policy", "Viewer"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn set_git_outbound_policy(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "workspace set-git-outbound-policy")?;

    if output::dry_run_guard(cli, "workspace set-git-outbound-policy", &body) {
        return Ok(());
    }

    let data = client
        .put(
            &format!("/workspaces/{workspace}/networking/communicationPolicy/outbound/git"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace set-git-outbound-policy", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

// ─── Inbound Azure Resource Rules ────────────────────────────────────────────

async fn get_inbound_azure_resource_rules(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/networking/communicationPolicy/inbound/azureResourceInstances"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace get-inbound-azure-resource-rules", "Viewer"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn set_inbound_azure_resource_rules(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "workspace set-inbound-azure-resource-rules")?;

    if output::dry_run_guard(cli, "workspace set-inbound-azure-resource-rules", &body) {
        return Ok(());
    }

    let data = client
        .put(
            &format!(
                "/workspaces/{workspace}/networking/communicationPolicy/inbound/azureResourceInstances"
            ),
            &body,
        )
        .await
        .map_err(|e| {
            enrich_forbidden(e, "workspace set-inbound-azure-resource-rules", "Admin")
        })?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

// ─── Outbound Cloud Connection Rules ─────────────────────────────────────────

async fn get_outbound_cloud_connection_rules(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/networking/communicationPolicy/outbound/cloudConnections"
        ))
        .await
        .map_err(|e| {
            enrich_forbidden(e, "workspace get-outbound-cloud-connection-rules", "Viewer")
        })?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn set_outbound_cloud_connection_rules(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(
        file,
        content,
        "workspace set-outbound-cloud-connection-rules",
    )?;

    if output::dry_run_guard(cli, "workspace set-outbound-cloud-connection-rules", &body) {
        return Ok(());
    }

    let data = client
        .put(
            &format!(
                "/workspaces/{workspace}/networking/communicationPolicy/outbound/cloudConnections"
            ),
            &body,
        )
        .await
        .map_err(|e| {
            enrich_forbidden(e, "workspace set-outbound-cloud-connection-rules", "Admin")
        })?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

// ─── Outbound Gateway Rules ──────────────────────────────────────────────────

async fn get_outbound_gateway_rules(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/networking/communicationPolicy/outbound/gateways"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace get-outbound-gateway-rules", "Viewer"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

async fn set_outbound_gateway_rules(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "workspace set-outbound-gateway-rules")?;

    if output::dry_run_guard(cli, "workspace set-outbound-gateway-rules", &body) {
        return Ok(());
    }

    let data = client
        .put(
            &format!("/workspaces/{workspace}/networking/communicationPolicy/outbound/gateways"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "workspace set-outbound-gateway-rules", "Admin"))?;
    output::render_object(cli, &data, "workspaceId");
    Ok(())
}

// ─── Dataset Storage Format ──────────────────────────────────────────────────

async fn set_dataset_storage_format(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    format: &str,
) -> Result<()> {
    let body = serde_json::json!({ "defaultDatasetStorageFormat": format });

    if output::dry_run_guard(cli, "workspace set-dataset-storage-format", &body) {
        return Ok(());
    }

    let data = client
        .patch_powerbi(&format!("/groups/{workspace}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "workspace set-dataset-storage-format", "Admin"))?;

    // PBI PATCH returns empty 200, so synthesize confirmation
    if data.is_null() {
        let obj = serde_json::json!({
            "workspaceId": workspace,
            "defaultDatasetStorageFormat": format,
            "status": "updated"
        });
        output::render_object(cli, &obj, "workspaceId");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Get Dataset Storage Format ──────────────────────────────────────────────

async fn get_dataset_storage_format(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
) -> Result<()> {
    let data = client
        .get_powerbi(&format!("/groups/{workspace}"))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace get-dataset-storage-format", "Viewer"))?;

    // Extract just the storage format field if present, otherwise return full response
    let format_value = data
        .get("defaultDatasetStorageFormat")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let obj = serde_json::json!({
        "workspaceId": workspace,
        "defaultDatasetStorageFormat": format_value,
    });
    output::render_object(cli, &obj, "workspaceId");
    Ok(())
}

// ─── Get Settings ────────────────────────────────────────────────────────────

async fn get_settings(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}"))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace get-settings", "Viewer"))?;

    // Extract "properties" sub-object if present; otherwise return full response
    if let Some(props) = data.get("properties") {
        output::render_object(cli, props, "automaticMetadataSync");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Update Settings ─────────────────────────────────────────────────────────

async fn update_settings(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "workspace update-settings")?;

    if output::dry_run_guard(cli, "workspace update-settings", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "workspace update-settings", "Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Read JSON body from --file or --content flag.
fn read_json_body(file: Option<&str>, content: Option<&str>, command: &str) -> Result<Value> {
    match (file, content) {
        (Some(path), _) => {
            let raw = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?;
            serde_json::from_str(&raw).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}"))
        }
        (_, Some(c)) => serde_json::from_str(c).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}")),
        (None, None) => Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Either --file or --content must be provided".to_string(),
            format!("Example: fabio {command} --workspace <WS> --file config.json"),
        )
        .into()),
    }
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

    #[test]
    fn read_json_body_from_content() {
        let result = read_json_body(None, Some(r#"{"automaticMetadataSync":"Enabled"}"#), "test");
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!(val["automaticMetadataSync"], "Enabled");
    }

    #[test]
    fn read_json_body_invalid_json_returns_error() {
        let result = read_json_body(None, Some("not json"), "test");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Invalid JSON"),
            "Expected 'Invalid JSON' in error, got: {err_msg}"
        );
    }

    #[test]
    fn read_json_body_missing_both_returns_error() {
        let result = read_json_body(None, None, "workspace update-settings");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("--file or --content"),
            "Expected hint about --file or --content, got: {err_msg}"
        );
    }

    #[test]
    fn update_settings_body_is_passed_directly() {
        // Verify the JSON body is used directly (user controls full body shape)
        let body: Value = serde_json::from_str(r#"{"automaticMetadataSync":"Enabled"}"#).unwrap();
        assert_eq!(body["automaticMetadataSync"], "Enabled");
        // No wrapping — user controls the full PATCH body structure
        assert!(body.get("properties").is_none());
    }

    #[test]
    fn update_settings_supports_complex_body() {
        let body: Value = serde_json::from_str(
            r#"{"displayName":"NewName","properties":{"automaticMetadataSync":"Enabled"}}"#,
        )
        .unwrap();
        assert_eq!(body["displayName"], "NewName");
        assert_eq!(body["properties"]["automaticMetadataSync"], "Enabled");
    }
}
