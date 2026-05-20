use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum DomainCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List domains in the tenant
    #[command(display_order = 1)]
    List,
    /// Show details of a domain
    #[command(display_order = 2)]
    Show {
        /// Domain ID
        #[arg(long)]
        id: String,
    },
    /// Create a new domain
    #[command(display_order = 3)]
    Create {
        /// Domain display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Parent domain ID (for subdomains)
        #[arg(long)]
        parent_domain_id: Option<String>,
    },
    /// Update domain properties
    #[command(display_order = 4)]
    Update {
        /// Domain ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a domain
    #[command(display_order = 5)]
    Delete {
        /// Domain ID
        #[arg(long)]
        id: String,
    },

    // ── Workspace assignments ────────────────────────────────────────────
    /// List workspaces assigned to a domain
    #[command(display_order = 10)]
    ListWorkspaces {
        /// Domain ID
        #[arg(long)]
        id: String,
    },
    /// Assign workspaces to a domain
    #[command(display_order = 11)]
    AssignWorkspaces {
        /// Domain ID
        #[arg(long)]
        id: String,

        /// Workspace IDs to assign (comma-separated or repeated)
        #[arg(long, value_delimiter = ',')]
        workspaces: Vec<String>,
    },
    /// Unassign workspaces from a domain
    #[command(display_order = 12)]
    UnassignWorkspaces {
        /// Domain ID
        #[arg(long)]
        id: String,

        /// Workspace IDs to unassign (comma-separated or repeated)
        #[arg(long, value_delimiter = ',')]
        workspaces: Vec<String>,
    },
    /// Bulk-assign all workspaces by capacity to a domain
    #[command(display_order = 13)]
    AssignByCapacity {
        /// Domain ID
        #[arg(long)]
        id: String,

        /// Capacity IDs whose workspaces should be assigned (comma-separated)
        #[arg(long, value_delimiter = ',')]
        capacities: Vec<String>,
    },
    /// Bulk-assign all workspaces by principal to a domain
    #[command(display_order = 14)]
    AssignByPrincipal {
        /// Domain ID
        #[arg(long)]
        id: String,

        /// Principal IDs whose workspaces should be assigned (comma-separated)
        #[arg(long, value_delimiter = ',')]
        principals: Vec<String>,

        /// Principal type
        #[arg(long, value_parser = ["User", "Group", "ServicePrincipal"])]
        principal_type: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &DomainCommand) -> Result<()> {
    match command {
        DomainCommand::List => list(cli, client).await,
        DomainCommand::Show { id } => show(cli, client, id).await,
        DomainCommand::Create {
            name,
            description,
            parent_domain_id,
        } => {
            create(
                cli,
                client,
                name,
                description.as_deref(),
                parent_domain_id.as_deref(),
            )
            .await
        }
        DomainCommand::Update {
            id,
            name,
            description,
        } => update(cli, client, id, name.as_deref(), description.as_deref()).await,
        DomainCommand::Delete { id } => delete(cli, client, id).await,
        DomainCommand::ListWorkspaces { id } => list_workspaces(cli, client, id).await,
        DomainCommand::AssignWorkspaces { id, workspaces } => {
            assign_workspaces(cli, client, id, workspaces).await
        }
        DomainCommand::UnassignWorkspaces { id, workspaces } => {
            unassign_workspaces(cli, client, id, workspaces).await
        }
        DomainCommand::AssignByCapacity { id, capacities } => {
            assign_by_capacity(cli, client, id, capacities).await
        }
        DomainCommand::AssignByPrincipal {
            id,
            principals,
            principal_type,
        } => assign_by_principal(cli, client, id, principals, principal_type).await,
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list("/admin/domains", "domains", cli.all)
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id", "description", "parentDomainId"],
        &["NAME", "ID", "DESCRIPTION", "PARENT"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn show(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    let data = client.get(&format!("/admin/domains/{id}")).await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn create(
    cli: &Cli,
    client: &FabricClient,
    name: &str,
    description: Option<&str>,
    parent_domain_id: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }
    if let Some(parent) = parent_domain_id {
        body["parentDomainId"] = Value::String(parent.to_string());
    }

    if output::dry_run_guard(cli, "domain create", &body) {
        return Ok(());
    }

    let data = client
        .post("/admin/domains", &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "domain create", "Fabric Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

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
            "Example: fabio domain update --id <ID> --name \"New Domain Name\"".to_string(),
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

    if output::dry_run_guard(cli, "domain update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/admin/domains/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "domain update", "Fabric Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    if output::dry_run_guard(cli, "domain delete", &serde_json::json!({ "id": id })) {
        return Ok(());
    }

    client
        .delete(&format!("/admin/domains/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "domain delete", "Fabric Admin"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Workspace assignments ───────────────────────────────────────────────────

async fn list_workspaces(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    let resp = client
        .get_list(&format!("/admin/domains/{id}/workspaces"), "value", cli.all)
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

async fn assign_workspaces(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    workspaces: &[String],
) -> Result<()> {
    let body = serde_json::json!({ "workspacesIds": workspaces });

    if output::dry_run_guard(cli, "domain assign-workspaces", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/domains/{id}/assignWorkspaces"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "domain assign-workspaces", "Fabric Admin"))?;

    let obj = serde_json::json!({
        "domainId": id,
        "workspacesAssigned": workspaces.len(),
        "status": "assigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn unassign_workspaces(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    workspaces: &[String],
) -> Result<()> {
    let body = serde_json::json!({ "workspacesIds": workspaces });

    if output::dry_run_guard(cli, "domain unassign-workspaces", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/domains/{id}/unassignWorkspaces"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "domain unassign-workspaces", "Fabric Admin"))?;

    let obj = serde_json::json!({
        "domainId": id,
        "workspacesUnassigned": workspaces.len(),
        "status": "unassigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn assign_by_capacity(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    capacities: &[String],
) -> Result<()> {
    let body = serde_json::json!({ "capacitiesIds": capacities });

    if output::dry_run_guard(cli, "domain assign-by-capacity", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/domains/{id}/assignWorkspacesByCapacities"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "domain assign-by-capacity", "Fabric Admin"))?;

    let obj = serde_json::json!({
        "domainId": id,
        "capacitiesUsed": capacities.len(),
        "status": "assigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn assign_by_principal(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    principals: &[String],
    principal_type: &str,
) -> Result<()> {
    let body = serde_json::json!({
        "principals": principals.iter().map(|p| {
            serde_json::json!({ "id": p, "type": principal_type })
        }).collect::<Vec<_>>()
    });

    if output::dry_run_guard(cli, "domain assign-by-principal", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/admin/domains/{id}/assignWorkspacesByPrincipals"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "domain assign-by-principal", "Fabric Admin"))?;

    let obj = serde_json::json!({
        "domainId": id,
        "principalsUsed": principals.len(),
        "status": "assigned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}
