use std::io::Read;

use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum OntologyCommand {
    /// List ontologies in a workspace
    List {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// Show details of an ontology
    Show {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Ontology ID
        #[arg(long)]
        id: String,
    },
    /// Create an ontology
    Create {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Display name (must start with letter, alphanumeric/underscore, <100 chars)
        #[arg(long)]
        name: String,

        /// Description (max 256 characters)
        #[arg(long)]
        description: Option<String>,

        /// Path to definition JSON file (base64-encoded parts)
        #[arg(long)]
        definition: Option<String>,
    },
    /// Update ontology properties (name and/or description)
    Update {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Ontology ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete an ontology
    Delete {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Ontology ID
        #[arg(long)]
        id: String,

        /// Permanently delete (cannot be recovered)
        #[arg(long)]
        hard: bool,
    },
    /// Get the ontology definition (entity types, bindings)
    GetDefinition {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Ontology ID
        #[arg(long)]
        id: String,

        /// Definition format
        #[arg(long)]
        format: Option<String>,
    },
    /// Update the ontology definition (replaces current definition)
    UpdateDefinition {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Ontology ID
        #[arg(long)]
        id: String,

        /// Path to definition JSON file, or - for stdin
        #[arg(long)]
        definition: String,

        /// Also update item metadata from .platform file
        #[arg(long)]
        update_metadata: bool,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &OntologyCommand) -> Result<()> {
    match command {
        OntologyCommand::List { workspace } => list(cli, client, workspace).await,
        OntologyCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        OntologyCommand::Create {
            workspace,
            name,
            description,
            definition,
        } => {
            create(
                cli,
                client,
                workspace,
                name,
                description.as_deref(),
                definition.as_deref(),
            )
            .await
        }
        OntologyCommand::Update {
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
        OntologyCommand::Delete {
            workspace,
            id,
            hard,
        } => delete(cli, client, workspace, id, *hard).await,
        OntologyCommand::GetDefinition {
            workspace,
            id,
            format,
        } => get_definition(cli, client, workspace, id, format.as_deref()).await,
        OntologyCommand::UpdateDefinition {
            workspace,
            id,
            definition,
            update_metadata,
        } => update_definition(cli, client, workspace, id, definition, *update_metadata).await,
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/ontologies"))
        .await?;

    let items = data
        .get("value")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    output::render_list(
        cli,
        &items,
        &["displayName", "id", "description"],
        &["NAME", "ID", "DESCRIPTION"],
        "displayName",
    );
    Ok(())
}

async fn show(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/ontologies/{id}"))
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
    definition_path: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({
        "displayName": name,
    });

    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if let Some(path) = definition_path {
        let content = read_file_or_stdin(path)?;
        let def: Value = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid definition JSON: {e}"))?;
        body["definition"] = def;
    }

    let data = client
        .post(&format!("/workspaces/{workspace}/ontologies"), &body, true)
        .await?;

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
        anyhow::bail!("Specify at least one of --name or --description to update");
    }

    let mut body = serde_json::json!({});
    if let Some(n) = name {
        body["displayName"] = Value::String(n.to_string());
    }
    if let Some(d) = description {
        body["description"] = Value::String(d.to_string());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/ontologies/{id}"), &body)
        .await?;

    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    hard: bool,
) -> Result<()> {
    let path = if hard {
        format!("/workspaces/{workspace}/ontologies/{id}?hardDelete=True")
    } else {
        format!("/workspaces/{workspace}/ontologies/{id}")
    };

    client.delete(&path).await?;

    output::render_object(
        cli,
        &serde_json::json!({"id": id, "status": "deleted"}),
        "status",
    );
    Ok(())
}

async fn get_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    format: Option<&str>,
) -> Result<()> {
    let path = format.map_or_else(
        || format!("/workspaces/{workspace}/ontologies/{id}/getDefinition"),
        |f| format!("/workspaces/{workspace}/ontologies/{id}/getDefinition?format={f}"),
    );

    let data = client.post(&path, &serde_json::json!({}), true).await?;

    output::render_object(cli, &data, "definition");
    Ok(())
}

async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    definition_path: &str,
    update_metadata: bool,
) -> Result<()> {
    let content = read_file_or_stdin(definition_path)?;
    let def: Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Invalid definition JSON: {e}"))?;

    let body = serde_json::json!({"definition": def});

    let path = if update_metadata {
        format!("/workspaces/{workspace}/ontologies/{id}/updateDefinition?updateMetadata=True")
    } else {
        format!("/workspaces/{workspace}/ontologies/{id}/updateDefinition")
    };

    let data = client.post(&path, &body, true).await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

fn read_file_or_stdin(path: &str) -> Result<String> {
    if path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| anyhow::anyhow!("Failed to read from stdin: {e}"))?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))
    }
}
