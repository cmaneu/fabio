use anyhow::Result;
use base64::Engine;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum EnvironmentCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List environments in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of an environment
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Create a new environment
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update environment properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete an environment
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,

        /// Permanently delete (cannot be recovered)
        #[arg(long)]
        hard_delete: bool,
    },

    // ── Publish ──────────────────────────────────────────────────────────
    /// Publish staged changes to an environment
    #[command(display_order = 10)]
    Publish {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Cancel a pending publish operation
    #[command(display_order = 11)]
    CancelPublish {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Get the published Spark settings (compute/pool/driver/executor)
    #[command(display_order = 12)]
    GetSparkSettings {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Get the staging (draft) Spark settings
    #[command(display_order = 13)]
    GetStagingSparkSettings {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },

    // ── Definitions ──────────────────────────────────────────────────────
    /// Get the definition of an environment
    #[command(display_order = 20)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition of an environment
    #[command(display_order = 21)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,

        /// Path to definition file
        #[arg(long)]
        file: Option<String>,

        /// Inline definition content
        #[arg(long)]
        content: Option<String>,
    },

    // ── Published Libraries ──────────────────────────────────────────────
    /// List published libraries of an environment
    #[command(display_order = 30)]
    ListLibraries {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Export external libraries configuration (published)
    #[command(display_order = 31)]
    ExportLibraries {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },

    // ── Staging Libraries ────────────────────────────────────────────────
    /// List staging libraries of an environment
    #[command(display_order = 40)]
    ListStagingLibraries {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Delete a staging library by name
    #[command(display_order = 41)]
    DeleteStagingLibrary {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,

        /// Library filename to delete
        #[arg(long)]
        library_name: String,
    },
    /// Export external libraries configuration (staging)
    #[command(display_order = 42)]
    ExportStagingLibraries {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,
    },
    /// Import external libraries configuration into staging
    #[command(display_order = 43)]
    ImportStagingLibraries {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,

        /// Path to JSON file with external libraries config
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content with external libraries config
        #[arg(long)]
        content: Option<String>,
    },
    /// Remove an external library from staging
    #[command(display_order = 44)]
    RemoveStagingLibrary {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,

        /// Library name to remove
        #[arg(long)]
        library_name: String,
    },

    // ── Staging Spark Compute ────────────────────────────────────────────
    /// Update staging Spark compute configuration
    #[command(display_order = 50)]
    UpdateStagingSparkCompute {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Environment ID
        #[arg(long)]
        id: String,

        /// Path to JSON file with spark compute config
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON content with spark compute config
        #[arg(long)]
        content: Option<String>,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &EnvironmentCommand) -> Result<()> {
    match command {
        EnvironmentCommand::List { workspace } => list(cli, client, workspace).await,
        EnvironmentCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        EnvironmentCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref()).await,
        EnvironmentCommand::Update {
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
        EnvironmentCommand::Delete {
            workspace,
            id,
            hard_delete,
        } => delete(cli, client, workspace, id, *hard_delete).await,
        EnvironmentCommand::Publish { workspace, id } => publish(cli, client, workspace, id).await,
        EnvironmentCommand::CancelPublish { workspace, id } => {
            cancel_publish(cli, client, workspace, id).await
        }
        EnvironmentCommand::GetSparkSettings { workspace, id } => {
            get_spark_settings(cli, client, workspace, id).await
        }
        EnvironmentCommand::GetStagingSparkSettings { workspace, id } => {
            get_staging_spark_settings(cli, client, workspace, id).await
        }
        EnvironmentCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        EnvironmentCommand::UpdateDefinition {
            workspace,
            id,
            file,
            content,
        } => {
            update_definition(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        EnvironmentCommand::ListLibraries { workspace, id } => {
            list_libraries(cli, client, workspace, id).await
        }
        EnvironmentCommand::ExportLibraries { workspace, id } => {
            export_libraries(cli, client, workspace, id).await
        }
        EnvironmentCommand::ListStagingLibraries { workspace, id } => {
            list_staging_libraries(cli, client, workspace, id).await
        }
        EnvironmentCommand::DeleteStagingLibrary {
            workspace,
            id,
            library_name,
        } => delete_staging_library(cli, client, workspace, id, library_name).await,
        EnvironmentCommand::ExportStagingLibraries { workspace, id } => {
            export_staging_libraries(cli, client, workspace, id).await
        }
        EnvironmentCommand::ImportStagingLibraries {
            workspace,
            id,
            file,
            content,
        } => {
            import_staging_libraries(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        EnvironmentCommand::RemoveStagingLibrary {
            workspace,
            id,
            library_name,
        } => remove_staging_library(cli, client, workspace, id, library_name).await,
        EnvironmentCommand::UpdateStagingSparkCompute {
            workspace,
            id,
            file,
            content,
        } => {
            update_staging_spark_compute(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/environments"),
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
        .get(&format!("/workspaces/{workspace}/environments/{id}"))
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
        "environment create",
        &serde_json::json!({
            "workspace": workspace,
            "displayName": name,
            "description": description
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/environments"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "environment create", "Member"))?;
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
            "Example: fabio environment update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "environment update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/environments/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "environment update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
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
        "environment delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id, "hardDelete": hard_delete
        }),
    ) {
        return Ok(());
    }

    let url = if hard_delete {
        format!("/workspaces/{workspace}/environments/{id}?hardDelete=true")
    } else {
        format!("/workspaces/{workspace}/environments/{id}")
    };

    client
        .delete(&url)
        .await
        .map_err(|e| enrich_forbidden(e, "environment delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Publish ─────────────────────────────────────────────────────────────────

async fn publish(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "environment publish",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/environments/{id}/staging/publish"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "environment publish", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "status": "publish_started"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn cancel_publish(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    client
        .post(
            &format!("/workspaces/{workspace}/environments/{id}/staging/cancelPublish"),
            &serde_json::json!({}),
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "environment cancel-publish", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "status": "publish_cancelled"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Spark Settings ──────────────────────────────────────────────────────────

async fn get_spark_settings(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/environments/{id}/sparkcompute"
        ))
        .await?;
    output::render_object(cli, &data, "instancePool");
    Ok(())
}

async fn get_staging_spark_settings(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/environments/{id}/staging/sparkcompute"
        ))
        .await?;
    output::render_object(cli, &data, "instancePool");
    Ok(())
}

// ─── Definitions ─────────────────────────────────────────────────────────────

async fn get_definition(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/environments/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "environment get-definition", "Contributor"))?;
    output::render_object(cli, &data, "definition");
    Ok(())
}

async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let script = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(c)) => c.to_string(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio environment update-definition --workspace <WS> --id <ID> --file definition.json".to_string(),
            ).into());
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(script.as_bytes());

    let body = serde_json::json!({
        "definition": {
            "parts": [
                {
                    "path": "environment.metadata.json",
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    if output::dry_run_guard(
        cli,
        "environment update-definition",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "contentLength": script.len()
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/environments/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "environment update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Published Libraries ─────────────────────────────────────────────────────

async fn list_libraries(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/environments/{id}/libraries"
        ))
        .await?;
    output::render_object(cli, &data, "customLibraries");
    Ok(())
}

async fn export_libraries(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/environments/{id}/libraries/exportExternalLibraries"
        ))
        .await?;
    output::render_object(cli, &data, "externalLibraries");
    Ok(())
}

// ─── Staging Libraries ───────────────────────────────────────────────────────

async fn list_staging_libraries(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/environments/{id}/staging/libraries"
        ))
        .await?;
    output::render_object(cli, &data, "customLibraries");
    Ok(())
}

async fn delete_staging_library(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    library_name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "environment delete-staging-library",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "libraryName": library_name
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!(
            "/workspaces/{workspace}/environments/{id}/staging/libraries?libraryToDelete={library_name}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "environment delete-staging-library", "Contributor"))?;

    let obj = serde_json::json!({ "id": id, "library": library_name, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn export_staging_libraries(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/environments/{id}/staging/libraries/exportExternalLibraries"
        ))
        .await?;
    output::render_object(cli, &data, "externalLibraries");
    Ok(())
}

async fn import_staging_libraries(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body_str = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(c)) => c.to_string(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio environment import-staging-libraries --workspace <WS> --id <ID> --file libs.json".to_string(),
            ).into());
        }
    };

    let body: Value =
        serde_json::from_str(&body_str).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}"))?;

    if output::dry_run_guard(
        cli,
        "environment import-staging-libraries",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "contentLength": body_str.len()
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/environments/{id}/staging/libraries/importExternalLibraries"
            ),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "environment import-staging-libraries", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "libraries_imported" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

async fn remove_staging_library(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    library_name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "environment remove-staging-library",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "libraryName": library_name
        }),
    ) {
        return Ok(());
    }

    let body = serde_json::json!({ "libraryToRemove": library_name });

    let data = client
        .post(
            &format!(
                "/workspaces/{workspace}/environments/{id}/staging/libraries/removeExternalLibrary"
            ),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "environment remove-staging-library", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "library": library_name, "status": "removed" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Staging Spark Compute ───────────────────────────────────────────────────

async fn update_staging_spark_compute(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body_str = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(c)) => c.to_string(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio environment update-staging-spark-compute --workspace <WS> --id <ID> --file compute.json".to_string(),
            ).into());
        }
    };

    let body: Value =
        serde_json::from_str(&body_str).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}"))?;

    if output::dry_run_guard(
        cli,
        "environment update-staging-spark-compute",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "contentLength": body_str.len()
        }),
    ) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/environments/{id}/staging/sparkcompute"),
            &body,
        )
        .await
        .map_err(|e| {
            enrich_forbidden(e, "environment update-staging-spark-compute", "Contributor")
        })?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "spark_compute_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}
