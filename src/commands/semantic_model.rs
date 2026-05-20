use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum SemanticModelCommand {
    /// List semantic models in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a semantic model
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,
    },
    /// Create a new semantic model from a definition file (model.bim)
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Path to model definition file (model.bim TMSL/TMDL format)
        #[arg(long)]
        file: String,
    },
    /// Update semantic model properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a semantic model
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,
    },
    /// Get the definition of a semantic model
    #[command(name = "get-definition", display_order = 6)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition of a semantic model from a file
    #[command(name = "update-definition", display_order = 7)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,

        /// Path to model definition file (model.bim TMSL/TMDL format)
        #[arg(long)]
        file: String,
    },
}

pub async fn execute(
    cli: &Cli,
    client: &FabricClient,
    command: &SemanticModelCommand,
) -> Result<()> {
    match command {
        SemanticModelCommand::List { workspace } => list(cli, client, workspace).await,
        SemanticModelCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        SemanticModelCommand::Create {
            workspace,
            name,
            description,
            file,
        } => create(cli, client, workspace, name, description.as_deref(), file).await,
        SemanticModelCommand::Update {
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
        SemanticModelCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        SemanticModelCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        SemanticModelCommand::UpdateDefinition {
            workspace,
            id,
            file,
        } => update_definition(cli, client, workspace, id, file).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/semanticModels"),
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
        .get(&format!("/workspaces/{workspace}/semanticModels/{id}"))
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
    file: &str,
) -> Result<()> {
    let content = std::fs::read_to_string(file).map_err(|e| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("Failed to read file '{file}': {e}"),
            "Provide a valid model.bim file path.".to_string(),
        )
    })?;
    let encoded = base64::engine::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        content.as_bytes(),
    );

    let mut body = serde_json::json!({
        "displayName": name,
        "definition": {
            "parts": [{
                "path": "model.bim",
                "payload": encoded,
                "payloadType": "InlineBase64"
            }]
        }
    });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if output::dry_run_guard(
        cli,
        "semantic-model create",
        &serde_json::json!({
            "workspace": workspace,
            "displayName": name,
            "description": description,
            "file": file
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/semanticModels"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model create", "Member"))?;
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
            "Example: fabio semantic-model update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "semantic-model update", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/semanticModels/{id}"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "semantic-model delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/semanticModels/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn get_definition(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/semanticModels/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model get-definition", "Contributor"))?;
    output::render_object(cli, &data, "definition");
    Ok(())
}

async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: &str,
) -> Result<()> {
    let content = std::fs::read_to_string(file).map_err(|e| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("Failed to read file '{file}': {e}"),
            "Provide a valid model.bim file path.".to_string(),
        )
    })?;
    let encoded = base64::engine::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        content.as_bytes(),
    );

    let body = serde_json::json!({
        "definition": {
            "parts": [{
                "path": "model.bim",
                "payload": encoded,
                "payloadType": "InlineBase64"
            }]
        }
    });

    if output::dry_run_guard(cli, "semantic-model update-definition", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/semanticModels/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model update-definition", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "workspace": workspace,
        "status": "definition_updated"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}
