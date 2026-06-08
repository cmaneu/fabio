use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum MlModelCommand {
    /// List ML models in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,
    },
    /// Show details of an ML model
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,
    },
    /// Create a new ML model
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update ML model properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete an ML model
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,

        /// Permanently delete (cannot be recovered)
        #[arg(long)]
        hard_delete: bool,
    },
    /// Get the ML model serving endpoint configuration
    #[command(display_order = 10)]
    GetEndpoint {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,
    },
    /// Update the ML model serving endpoint configuration
    #[command(display_order = 11)]
    UpdateEndpoint {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,

        /// Path to JSON file with endpoint config
        #[arg(long, conflicts_with = "content")]
        file: Option<String>,

        /// Inline JSON content with endpoint config
        #[arg(long, conflicts_with = "file")]
        content: Option<String>,
    },
    /// Score against the ML model endpoint
    #[command(display_order = 12)]
    Score {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,

        /// Path to JSON file with input data
        #[arg(long, conflicts_with = "content")]
        file: Option<String>,

        /// Inline JSON input data
        #[arg(long, conflicts_with = "file")]
        content: Option<String>,
    },
    /// List endpoint versions
    #[command(display_order = 20)]
    ListVersions {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,
    },
    /// Get a specific endpoint version
    #[command(display_order = 21)]
    GetVersion {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,

        /// Version name
        #[arg(long)]
        version_name: String,
    },
    /// Update a specific endpoint version
    #[command(display_order = 22)]
    UpdateVersion {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,

        /// Version name
        #[arg(long)]
        version_name: String,

        /// Path to JSON file with version config
        #[arg(long, conflicts_with = "content")]
        file: Option<String>,

        /// Inline JSON content with version config
        #[arg(long, conflicts_with = "file")]
        content: Option<String>,
    },
    /// Activate a specific endpoint version
    #[command(display_order = 23)]
    ActivateVersion {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,

        /// Version name
        #[arg(long)]
        version_name: String,
    },
    /// Deactivate a specific endpoint version
    #[command(display_order = 24)]
    DeactivateVersion {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,

        /// Version name
        #[arg(long)]
        version_name: String,
    },
    /// Score against a specific endpoint version
    #[command(display_order = 25)]
    ScoreVersion {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,

        /// Version name
        #[arg(long)]
        version_name: String,

        /// Path to JSON file with input data
        #[arg(long, conflicts_with = "content")]
        file: Option<String>,

        /// Inline JSON input data
        #[arg(long, conflicts_with = "file")]
        content: Option<String>,
    },
    /// Deactivate all endpoint versions
    #[command(display_order = 26)]
    DeactivateAllVersions {
        /// Workspace ID
        #[arg(short, long, env = "FABIO_WORKSPACE")]
        workspace: String,

        /// ML model ID
        #[arg(long)]
        id: String,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &MlModelCommand) -> Result<()> {
    match command {
        MlModelCommand::List { workspace } => list(cli, client, workspace).await,
        MlModelCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        MlModelCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref()).await,
        MlModelCommand::Update {
            workspace,
            id,
            name,
            description,
        } => {
            update(
                cli,
                client,
                workspace,
                id,
                name.as_deref(),
                description.as_deref(),
            )
            .await
        }
        MlModelCommand::Delete {
            workspace,
            id,
            hard_delete,
        } => delete(cli, client, workspace, id, *hard_delete).await,
        MlModelCommand::GetEndpoint { workspace, id } => {
            get_endpoint(cli, client, workspace, id).await
        }
        MlModelCommand::UpdateEndpoint {
            workspace,
            id,
            file,
            content,
        } => {
            update_endpoint(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        MlModelCommand::Score {
            workspace,
            id,
            file,
            content,
        } => {
            score(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        MlModelCommand::ListVersions { workspace, id } => {
            list_versions(cli, client, workspace, id).await
        }
        MlModelCommand::GetVersion {
            workspace,
            id,
            version_name,
        } => get_version(cli, client, workspace, id, version_name).await,
        MlModelCommand::UpdateVersion {
            workspace,
            id,
            version_name,
            file,
            content,
        } => {
            update_version(
                cli,
                client,
                workspace,
                id,
                version_name,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        MlModelCommand::ActivateVersion {
            workspace,
            id,
            version_name,
        } => activate_version(cli, client, workspace, id, version_name).await,
        MlModelCommand::DeactivateVersion {
            workspace,
            id,
            version_name,
        } => deactivate_version(cli, client, workspace, id, version_name).await,
        MlModelCommand::ScoreVersion {
            workspace,
            id,
            version_name,
            file,
            content,
        } => {
            score_version(
                cli,
                client,
                workspace,
                id,
                version_name,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        MlModelCommand::DeactivateAllVersions { workspace, id } => {
            deactivate_all_versions(cli, client, workspace, id).await
        }
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/mlModels"),
            "value",
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

async fn show(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/mlModels/{id}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    description: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({
        "displayName": name,
    });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if output::dry_run_guard(
        cli,
        "ml-model create",
        &serde_json::json!({
            "workspace": workspace,
            "displayName": name,
            "description": description
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(&format!("/workspaces/{workspace}/mlModels"), &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "ml-model create", "Member"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --name or --description must be provided".to_string(),
            "Example: fabio ml-model update --workspace <WS> --id <ID> --name \"New Name\""
                .to_string(),
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

    if output::dry_run_guard(cli, "ml-model update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/mlModels/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "ml-model update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

fn read_json_body(file: Option<&str>, content: Option<&str>, command: &str) -> Result<Value> {
    match (file, content) {
        (Some(f), _) => {
            let text = std::fs::read_to_string(f)
                .map_err(|e| FabioError::not_found(format!("File not found: {f}: {e}")))?;
            Ok(serde_json::from_str(&text)?)
        }
        (_, Some(c)) => Ok(serde_json::from_str(c)?),
        _ => Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Either --file or --content must be provided".to_string(),
            format!(
                "Example: fabio ml-model {command} --workspace <WS> --id <ID> --content '{{...}}'"
            ),
        )
        .into()),
    }
}

async fn delete(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    hard_delete: bool,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "ml-model delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id, "hardDelete": hard_delete
        }),
    ) {
        return Ok(());
    }

    let url = if hard_delete {
        format!("/workspaces/{workspace}/mlModels/{id}?hardDelete=true")
    } else {
        format!("/workspaces/{workspace}/mlModels/{id}")
    };

    client
        .delete(&url)
        .await
        .map_err(|e| enrich_forbidden(e, "ml-model delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn get_endpoint(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/mlModels/{id}/endpoint"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update_endpoint(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "update-endpoint")?;

    if output::dry_run_guard(cli, "ml-model update-endpoint", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/mlModels/{id}/endpoint"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ml-model update-endpoint", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn score(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "score")?;

    let data = client
        .post(
            &format!("/workspaces/{workspace}/mlModels/{id}/endpoint/score"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ml-model score", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn list_versions(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/mlModels/{id}/endpoint/versions"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn get_version(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    version_name: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/mlModels/{id}/endpoint/versions/{version_name}"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update_version(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    version_name: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "update-version")?;

    if output::dry_run_guard(cli, "ml-model update-version", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/mlModels/{id}/endpoint/versions/{version_name}"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ml-model update-version", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn activate_version(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    version_name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "ml-model activate-version",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "versionName": version_name
        }),
    ) {
        return Ok(());
    }

    let body = serde_json::json!({});
    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/mlModels/{id}/endpoint/versions/{version_name}/activate"
            ),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ml-model activate-version", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn deactivate_version(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    version_name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "ml-model deactivate-version",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "versionName": version_name
        }),
    ) {
        return Ok(());
    }

    let body = serde_json::json!({});
    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/mlModels/{id}/endpoint/versions/{version_name}/deactivate"
            ),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ml-model deactivate-version", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn score_version(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    version_name: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "score-version")?;

    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/mlModels/{id}/endpoint/versions/{version_name}/score"
            ),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ml-model score-version", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn deactivate_all_versions(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "ml-model deactivate-all-versions",
        &serde_json::json!({
            "workspace": workspace,
            "id": id
        }),
    ) {
        return Ok(());
    }

    let body = serde_json::json!({});
    let data = client
        .post(
            &format!("/workspaces/{workspace}/mlModels/{id}/endpoint/versions/deactivateAll"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ml-model deactivate-all-versions", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
