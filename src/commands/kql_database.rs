use anyhow::Result;
use base64::Engine;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum KqlDatabaseCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List KQL databases in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a KQL database
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,
    },
    /// Create a new KQL database
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Database display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Parent eventhouse item ID
        #[arg(long)]
        eventhouse_id: String,

        /// Database type: `ReadWrite` or `ReadOnlyFollowing`
        #[arg(long, default_value = "ReadWrite", value_parser = ["ReadWrite", "ReadOnlyFollowing"])]
        database_type: String,
    },
    /// Update KQL database properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a KQL database
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,
    },

    // ── Definitions ──────────────────────────────────────────────────────
    /// Get the definition of a KQL database (KQL script)
    #[command(name = "get-definition", display_order = 6)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition of a KQL database
    #[command(name = "update-definition", display_order = 11)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// KQL script file path (reads file content)
        #[arg(long)]
        file: Option<String>,

        /// KQL script content (inline)
        #[arg(long)]
        content: Option<String>,
    },

    // ── Shortcuts ────────────────────────────────────────────────────────
    /// List shortcuts in a KQL database
    #[command(name = "list-shortcuts", display_order = 10)]
    ListShortcuts {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,
    },
    /// Create a shortcut in a KQL database
    #[command(name = "create-shortcut", display_order = 11)]
    CreateShortcut {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// Shortcut name
        #[arg(long)]
        name: String,

        /// JSON file with shortcut configuration
        #[arg(long)]
        file: Option<String>,

        /// Inline JSON shortcut configuration
        #[arg(long)]
        content: Option<String>,
    },
    /// Get a shortcut in a KQL database
    #[command(name = "get-shortcut", display_order = 12)]
    GetShortcut {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// Shortcut name
        #[arg(long)]
        shortcut_name: String,
    },
    /// Delete a shortcut in a KQL database
    #[command(name = "delete-shortcut", display_order = 13)]
    DeleteShortcut {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// KQL database ID
        #[arg(long)]
        id: String,

        /// Shortcut name
        #[arg(long)]
        shortcut_name: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &KqlDatabaseCommand) -> Result<()> {
    match command {
        KqlDatabaseCommand::List { workspace } => list(cli, client, workspace).await,
        KqlDatabaseCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        KqlDatabaseCommand::Create {
            workspace,
            name,
            description,
            eventhouse_id,
            database_type,
        } => {
            create(
                cli,
                client,
                workspace,
                name,
                description.as_deref(),
                eventhouse_id,
                database_type,
            )
            .await
        }
        KqlDatabaseCommand::Update {
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
        KqlDatabaseCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        KqlDatabaseCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        KqlDatabaseCommand::UpdateDefinition {
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
        KqlDatabaseCommand::ListShortcuts { workspace, id } => {
            list_shortcuts(cli, client, workspace, id).await
        }
        KqlDatabaseCommand::CreateShortcut {
            workspace,
            id,
            name,
            file,
            content,
        } => {
            create_shortcut(
                cli,
                client,
                workspace,
                id,
                name,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        KqlDatabaseCommand::GetShortcut {
            workspace,
            id,
            shortcut_name,
        } => get_shortcut(cli, client, workspace, id, shortcut_name).await,
        KqlDatabaseCommand::DeleteShortcut {
            workspace,
            id,
            shortcut_name,
        } => delete_shortcut(cli, client, workspace, id, shortcut_name).await,
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/kqlDatabases"),
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
        .get(&format!("/workspaces/{workspace}/kqlDatabases/{id}"))
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
    eventhouse_id: &str,
    database_type: &str,
) -> Result<()> {
    let mut body = serde_json::json!({
        "displayName": name,
        "creationPayload": {
            "databaseType": database_type,
            "parentEventhouseItemId": eventhouse_id
        }
    });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if output::dry_run_guard(cli, "kql-database create", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/kqlDatabases"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database create", "Member"))?;
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
            "Example: fabio kql-database update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "kql-database update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/kqlDatabases/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "kql-database delete",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/kqlDatabases/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Definitions ─────────────────────────────────────────────────────────────

async fn get_definition(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/kqlDatabases/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database get-definition", "Contributor"))?;
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
                "Example: fabio kql-database update-definition --workspace <WS> --id <ID> --file schema.kql".to_string(),
            ).into());
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(script.as_bytes());

    let body = serde_json::json!({
        "definition": {
            "parts": [
                {
                    "path": "DatabaseProperties.kql",
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    if output::dry_run_guard(
        cli,
        "kql-database update-definition",
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
            &format!("/workspaces/{workspace}/kqlDatabases/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Shortcuts ───────────────────────────────────────────────────────────────

async fn list_shortcuts(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/kqlDatabases/{id}/shortcuts"
        ))
        .await?;

    if let Some(arr) = data.as_array() {
        output::render_list_with_token(
            cli,
            arr,
            &["name", "target"],
            &["NAME", "TARGET"],
            "name",
            None,
        );
    } else {
        output::render_object(cli, &data, "shortcuts");
    }
    Ok(())
}

async fn create_shortcut(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let config: Value = match (file, content) {
        (Some(path), _) => {
            let raw = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?;
            serde_json::from_str(&raw)?
        }
        (_, Some(c)) => serde_json::from_str(c)?,
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio kql-database create-shortcut --workspace <WS> --id <ID> --name my-shortcut --content '{...}'"
                    .to_string(),
            )
            .into());
        }
    };

    let mut body = config;
    if let Some(obj) = body.as_object_mut() {
        obj.insert("name".to_string(), Value::String(name.to_string()));
    }

    if output::dry_run_guard(cli, "kql-database create-shortcut", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/kqlDatabases/{id}/shortcuts"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database create-shortcut", "Contributor"))?;
    output::render_object(cli, &data, "name");
    Ok(())
}

async fn get_shortcut(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    shortcut_name: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/kqlDatabases/{id}/shortcuts/{shortcut_name}"
        ))
        .await?;
    output::render_object(cli, &data, "name");
    Ok(())
}

async fn delete_shortcut(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    shortcut_name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "kql-database delete-shortcut",
        &serde_json::json!({ "workspace": workspace, "id": id, "shortcutName": shortcut_name }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!(
            "/workspaces/{workspace}/kqlDatabases/{id}/shortcuts/{shortcut_name}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "kql-database delete-shortcut", "Contributor"))?;

    let obj = serde_json::json!({ "shortcutName": shortcut_name, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}
