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

        /// SQL endpoint or lakehouse ID for live connection (generates definition.pbism)
        #[arg(long)]
        connection: Option<String>,
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
    /// Bind a semantic model to a connection
    #[command(name = "bind-connection", display_order = 10)]
    BindConnection {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,

        /// Connection ID to bind
        #[arg(long)]
        connection_id: String,
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
            connection,
        } => {
            create(
                cli,
                client,
                workspace,
                name,
                description.as_deref(),
                file,
                connection.as_deref(),
            )
            .await
        }
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
        SemanticModelCommand::BindConnection {
            workspace,
            id,
            connection_id,
        } => bind_connection(cli, client, workspace, id, connection_id).await,
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
    connection: Option<&str>,
) -> Result<()> {
    let content = std::fs::read_to_string(file).map_err(|e| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("Failed to read file '{file}': {e}"),
            "Provide a valid model.bim or .tmdl file path.".to_string(),
        )
    })?;
    let encoded = base64::engine::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        content.as_bytes(),
    );

    // Detect format from file extension
    let is_tmdl = std::path::Path::new(file)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("tmdl"));

    let mut parts = vec![serde_json::json!({
        "path": if is_tmdl { "definition/model.tmdl" } else { "model.bim" },
        "payload": encoded,
        "payloadType": "InlineBase64"
    })];

    // Always include definition.pbism (required by Fabric API)
    // Version "4.0" for TMDL, "3.0" for model.bim (v3 JSON)
    let pbism_version = if is_tmdl { "4.0" } else { "3.0" };
    let pbism = serde_json::json!({ "version": pbism_version });
    let pbism_encoded = base64::engine::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        pbism.to_string().as_bytes(),
    );
    parts.push(serde_json::json!({
        "path": "definition.pbism",
        "payload": pbism_encoded,
        "payloadType": "InlineBase64"
    }));

    // For TMDL models with --connection, generate the expressions.tmdl
    if let Some(conn_id) = connection {
        if is_tmdl {
            let expr = format!(
                "expression DatabaseQuery =\n\
                 \t\tlet\n\
                 \t\t\tdatabase = Sql.Database(\"placeholder\", \"{conn_id}\")\n\
                 \t\tin\n\
                 \t\t\tdatabase\n\
                 \tlineageTag: 00000000-0000-0000-0000-000000000001"
            );
            let expr_encoded = base64::engine::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                expr.as_bytes(),
            );
            parts.push(serde_json::json!({
                "path": "definition/expressions.tmdl",
                "payload": expr_encoded,
                "payloadType": "InlineBase64"
            }));
        }
    }

    let mut body = serde_json::json!({
        "displayName": name,
        "definition": {
            "parts": parts
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
            "file": file,
            "connection": connection
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
        .map_err(|e| enrich_create_error(enrich_forbidden(e, "semantic-model create", "Member")))?;
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

async fn bind_connection(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    connection_id: &str,
) -> Result<()> {
    let body = serde_json::json!({ "connectionId": connection_id });

    if output::dry_run_guard(cli, "semantic-model bind-connection", &body) {
        return Ok(());
    }

    client
        .post(
            &format!("/workspaces/{workspace}/semanticModels/{id}/bindConnection"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model bind-connection", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "connectionId": connection_id,
        "status": "connection_bound"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Error Enrichment ────────────────────────────────────────────────────────

/// Enrich semantic model API errors with actionable hints for common failures.
///
/// Intercepts known error patterns and provides corrective guidance so that
/// agents (and users) can self-correct without searching documentation.
fn enrich_create_error(err: anyhow::Error) -> anyhow::Error {
    let Some(fabio_err) = err.downcast_ref::<FabioError>() else {
        return err;
    };

    let msg = &fabio_err.message;
    let msg_lower = msg.to_lowercase();

    // Pattern: "Import from JSON supported for V3 models only"
    if msg_lower.contains("v3 models only") || msg_lower.contains("import from json") {
        return FabioError::with_hint(
            fabio_err.code,
            msg.clone(),
            "model.bim must use compatibilityLevel 1604 (not 1550) and include \
             \"defaultPowerBIDataSourceVersion\": \"powerBI_V3\" in the model object. \
             Example: {\"compatibilityLevel\": 1604, \"model\": {\"defaultPowerBIDataSourceVersion\": \"powerBI_V3\", ...}}"
        ).into();
    }

    // Pattern: TMDL "InvalidValueFormat" for PowerBIDataSourceVersion
    if msg_lower.contains("invalidvalueformat") && msg_lower.contains("powerbidatasourceversion") {
        return FabioError::with_hint(
            fabio_err.code,
            msg.clone(),
            "In TMDL, use 'defaultPowerBIDataSourceVersion: powerBI_V3' (with underscore). \
             The value 'powerBIDataSourceVersion3' is not valid. \
             Valid values: powerBI_V3.",
        )
        .into();
    }

    // Pattern: TMDL general parsing errors
    if msg_lower.contains("tmdl format error") {
        let hint = if msg_lower.contains("line number") {
            "Check TMDL syntax at the reported line. Common issues: \
             (1) Use tabs for indentation (not spaces). \
             (2) Enum values are case-sensitive (e.g., powerBI_V3, not powerbi_v3). \
             (3) Each table/column/partition needs a lineageTag GUID. \
             Reference: https://learn.microsoft.com/en-us/power-bi/developer/projects/projects-dataset#tmdl-format"
        } else {
            "TMDL parsing failed. Verify file uses tab indentation and valid enum values. \
             Reference: https://learn.microsoft.com/en-us/power-bi/developer/projects/projects-dataset#tmdl-format"
        };
        return FabioError::with_hint(fabio_err.code, msg.clone(), hint).into();
    }

    // Pattern: Definition parts missing or invalid
    if msg_lower.contains("definition") && msg_lower.contains("invalid") {
        return FabioError::with_hint(
            fabio_err.code,
            msg.clone(),
            "Semantic model creation requires: (1) a model definition file (model.bim or .tmdl), \
             (2) a definition.pbism entry. The CLI auto-generates definition.pbism. \
             For .bim files use compat 1604 + powerBI_V3. \
             For .tmdl files ensure 'defaultPowerBIDataSourceVersion: powerBI_V3'.",
        )
        .into();
    }

    // Pattern: DirectLake requires TMDL
    if msg_lower.contains("directlake") || msg_lower.contains("direct lake") {
        return FabioError::with_hint(
            fabio_err.code,
            msg.clone(),
            "Direct Lake semantic models require TMDL format (not model.bim). \
             Use a .tmdl file with partition mode: directLake and provide \
             --connection <sql-endpoint-id> to bind the lakehouse connection.",
        )
        .into();
    }

    // No known pattern matched — return original error
    err
}
