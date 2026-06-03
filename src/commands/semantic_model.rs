use std::io::{self, Read};

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
    /// Execute a DAX query against a semantic model
    #[command(display_order = 8)]
    Query {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,

        /// DAX query (e.g., "EVALUATE Sales"). If omitted, reads from stdin.
        #[arg(long)]
        dax: Option<String>,

        /// Read DAX query from a file
        #[arg(long, conflicts_with = "dax")]
        file: Option<String>,
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
    /// Refresh a semantic model (required to frame Direct Lake models after creation)
    #[command(display_order = 11)]
    Refresh {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,

        /// Refresh type
        #[arg(long, default_value = "Full")]
        r#type: String,
    },
    /// Take over a semantic model (converts definition-managed to service-managed for portal editing)
    #[command(display_order = 12)]
    Takeover {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,
    },
    /// List parameters of a semantic model
    #[command(name = "list-parameters", display_order = 13)]
    ListParameters {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,
    },
    /// Update parameters of a semantic model
    #[command(name = "update-parameters", display_order = 14)]
    UpdateParameters {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,

        /// JSON content with parameter updates (inline or @file or @- for stdin)
        #[arg(long)]
        content: String,
    },
    /// List datasources of a semantic model
    #[command(name = "list-datasources", display_order = 15)]
    ListDatasources {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,
    },
    /// Update datasources of a semantic model
    #[command(name = "update-datasources", display_order = 16)]
    UpdateDatasources {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,

        /// JSON content with datasource updates (inline or @file or @- for stdin)
        #[arg(long)]
        content: String,
    },
    /// List users (permissions) of a semantic model
    #[command(name = "list-users", display_order = 17)]
    ListUsers {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,
    },
    /// Add a user to a semantic model
    #[command(name = "add-user", display_order = 18)]
    AddUser {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,

        /// Principal identifier (email, OID, or group ID)
        #[arg(long)]
        principal: String,

        /// Principal type
        #[arg(long, value_parser = ["User", "Group", "App"])]
        principal_type: String,

        /// Access right for the dataset
        #[arg(long, value_parser = ["Read", "ReadExplore", "ReadReshare", "ReadReshareExplore"])]
        access_right: String,
    },
    /// Remove a user from a semantic model
    #[command(name = "delete-user", display_order = 19)]
    DeleteUser {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,

        /// User email or principal ID to remove
        #[arg(long)]
        user: String,
    },
    /// Get refresh history and status for a semantic model
    #[command(name = "refresh-status", display_order = 20)]
    RefreshStatus {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,

        /// Maximum number of refresh entries to return (default: 10)
        #[arg(long, default_value = "10")]
        top: u32,
    },
    /// List upstream (lineage) datasets that this semantic model depends on
    #[command(name = "list-upstream", display_order = 21)]
    ListUpstream {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Semantic model ID
        #[arg(long)]
        id: String,
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
            update(cli, client, workspace, id, name.as_deref(), description.as_deref()).await
        }
        SemanticModelCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        SemanticModelCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        SemanticModelCommand::UpdateDefinition { workspace, id, file } => {
            update_definition(cli, client, workspace, id, file).await
        }
        SemanticModelCommand::Query { workspace, id, dax, file } => {
            query(cli, client, workspace, id, dax.as_deref(), file.as_deref()).await
        }
        SemanticModelCommand::BindConnection { workspace, id, connection_id } => {
            bind_connection(cli, client, workspace, id, connection_id).await
        }
        SemanticModelCommand::Refresh { workspace, id, r#type } => {
            refresh(cli, client, workspace, id, r#type).await
        }
        SemanticModelCommand::Takeover { workspace, id } => {
            takeover(cli, client, workspace, id).await
        }
        SemanticModelCommand::ListParameters { workspace, id } => {
            list_parameters(cli, client, workspace, id).await
        }
        SemanticModelCommand::UpdateParameters { workspace, id, content } => {
            update_parameters(cli, client, workspace, id, content).await
        }
        SemanticModelCommand::ListDatasources { workspace, id } => {
            list_datasources(cli, client, workspace, id).await
        }
        SemanticModelCommand::UpdateDatasources { workspace, id, content } => {
            update_datasources(cli, client, workspace, id, content).await
        }
        SemanticModelCommand::ListUsers { workspace, id } => {
            list_users(cli, client, workspace, id).await
        }
        SemanticModelCommand::AddUser { workspace, id, principal, principal_type, access_right } => {
            add_user(cli, client, workspace, id, principal, principal_type, access_right).await
        }
        SemanticModelCommand::DeleteUser { workspace, id, user } => {
            delete_user(cli, client, workspace, id, user).await
        }
        SemanticModelCommand::RefreshStatus { workspace, id, top } => {
            refresh_status(cli, client, workspace, id, *top).await
        }
        SemanticModelCommand::ListUpstream { workspace, id } => {
            list_upstream(cli, client, workspace, id).await
        }
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

async fn query(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    dax: Option<&str>,
    file: Option<&str>,
) -> Result<()> {
    // Resolve DAX query from --dax flag, --file flag, or stdin
    let dax_query = if let Some(d) = dax {
        d.to_string()
    } else if let Some(f) = file {
        std::fs::read_to_string(f).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Failed to read DAX file '{f}': {e}"),
                "Provide a valid file path containing a DAX query.".to_string(),
            )
        })?
    } else {
        // Read from stdin
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Failed to read DAX from stdin: {e}"),
                "Provide DAX via --dax flag, --file flag, or pipe to stdin.".to_string(),
            )
        })?;
        if buf.trim().is_empty() {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "No DAX query provided".to_string(),
                "Usage: fabio semantic-model query --workspace <WS> --id <ID> --dax \"EVALUATE MyTable\"\n\
                 Or pipe: echo 'EVALUATE MyTable' | fabio semantic-model query --workspace <WS> --id <ID>"
                    .to_string(),
            )
            .into());
        }
        buf
    };

    let body = serde_json::json!({
        "queries": [{"query": dax_query.trim()}],
        "serializerSettings": {"includeNulls": true}
    });

    let data = client
        .post_powerbi(
            &format!("/groups/{workspace}/datasets/{id}/executeQueries"),
            &body,
        )
        .await
        .map_err(|e| enrich_dax_error(enrich_forbidden(e, "semantic-model query", "Viewer")))?;

    // Extract rows from the response: results[0].tables[0].rows
    let rows = data
        .get("results")
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first())
        .and_then(|t| t.get("tables"))
        .and_then(|t| t.as_array())
        .and_then(|arr| arr.first())
        .and_then(|t| t.get("rows"))
        .and_then(Value::as_array);

    if let Some(rows) = rows {
        // Build column names from the first row's keys
        let columns: Vec<&str> = rows
            .first()
            .and_then(Value::as_object)
            .map_or_else(Vec::new, |first| first.keys().map(String::as_str).collect());

        let items: Vec<Value> = rows.clone();
        output::render_list_with_token(
            cli,
            &items,
            &columns,
            &columns,
            columns.first().copied().unwrap_or("value"),
            None,
        );
    } else {
        // No rows — might be an error or empty result
        output::render_object(cli, &data, "results");
    }

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

/// Enrich DAX query errors with actionable hints.
fn enrich_dax_error(err: anyhow::Error) -> anyhow::Error {
    let Some(fabio_err) = err.downcast_ref::<FabioError>() else {
        return err;
    };

    let msg = &fabio_err.message;
    let msg_lower = msg.to_lowercase();

    // Pattern: model not found
    if msg_lower.contains("dataset not found") || msg_lower.contains("datasetnotfound") {
        return FabioError::with_hint(
            ErrorCode::NotFound,
            msg.clone(),
            "The semantic model ID was not found in this workspace. \
             Use: fabio semantic-model list --workspace <WS> to find available models."
                .to_string(),
        )
        .into();
    }

    // Pattern: model not refreshed / framing required
    if msg_lower.contains("3242524690") || msg_lower.contains("not framed") {
        return FabioError::with_hint(
            fabio_err.code,
            msg.clone(),
            "Direct Lake model needs framing before queries work. \
             Run: fabio semantic-model refresh --workspace <WS> --id <ID> --type Full"
                .to_string(),
        )
        .into();
    }

    // Pattern: DAX syntax error
    if msg_lower.contains("dax") && msg_lower.contains("syntax") {
        return FabioError::with_hint(
            fabio_err.code,
            msg.clone(),
            "DAX query has a syntax error. Ensure EVALUATE is followed by a valid table expression. \
             Example: EVALUATE SUMMARIZE(sales_summary, sales_summary[country], \"Revenue\", SUM(sales_summary[total]))"
                .to_string(),
        )
        .into();
    }

    err
}

async fn refresh(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    refresh_type: &str,
) -> Result<()> {
    const VALID_TYPES: &[&str] = &[
        "Full",
        "Automatic",
        "ClearValues",
        "Calculate",
        "DataOnly",
        "Defragment",
    ];

    // Case-insensitive normalization
    let refresh_type = VALID_TYPES
        .iter()
        .find(|v| v.eq_ignore_ascii_case(refresh_type))
        .copied()
        .unwrap_or(refresh_type);

    if !VALID_TYPES.contains(&refresh_type) {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("Invalid refresh type: '{refresh_type}'"),
            format!(
                "--type must be one of: {} (got: '{refresh_type}')",
                VALID_TYPES.join(", ")
            ),
        )
        .into());
    }

    let body = serde_json::json!({ "type": refresh_type });

    if output::dry_run_guard(cli, "semantic-model refresh", &body) {
        return Ok(());
    }

    client
        .post_powerbi(
            &format!("/groups/{workspace}/datasets/{id}/refreshes"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model refresh", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "type": refresh_type,
        "status": "refresh_triggered"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn takeover(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let body = serde_json::json!({});

    if output::dry_run_guard(cli, "semantic-model takeover", &body) {
        return Ok(());
    }

    client
        .post_powerbi(
            &format!("/groups/{workspace}/datasets/{id}/Default.TakeOver"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model takeover", "Admin"))?;

    let obj = serde_json::json!({
        "id": id,
        "status": "takeover_complete",
        "note": "Model is now service-managed (editable in portal)"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Power BI API Commands ──────────────────────────────────────────────────

fn parse_json_content(content: &str, command: &str) -> Result<Value> {
    serde_json::from_str(content).map_err(|e| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("Invalid JSON in --content: {e}"),
            format!(
                "Example: fabio semantic-model {command} --content '{{\"updateDetails\":[...]}}'"
            ),
        )
        .into()
    })
}

async fn list_parameters(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get_powerbi(&format!("/groups/{workspace}/datasets/{id}/parameters"))
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model list-parameters", "Contributor"))?;

    if let Some(items) = data.get("value").and_then(Value::as_array) {
        output::render_list_with_token(
            cli,
            items,
            &["name", "type", "currentValue", "isRequired"],
            &["NAME", "TYPE", "CURRENT VALUE", "REQUIRED"],
            "name",
            None,
        );
    } else {
        output::render_object(cli, &data, "name");
    }
    Ok(())
}

async fn update_parameters(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    content: &str,
) -> Result<()> {
    let body = parse_json_content(content, "update-parameters")?;

    if output::dry_run_guard(cli, "semantic-model update-parameters", &body) {
        return Ok(());
    }

    client
        .post_powerbi(
            &format!("/groups/{workspace}/datasets/{id}/Default.UpdateParameters"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model update-parameters", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "status": "parameters_updated"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn list_datasources(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get_powerbi(&format!("/groups/{workspace}/datasets/{id}/datasources"))
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model list-datasources", "Contributor"))?;

    if let Some(items) = data.get("value").and_then(Value::as_array) {
        output::render_list_with_token(
            cli,
            items,
            &["datasourceId", "datasourceType", "gatewayId"],
            &["DATASOURCE ID", "TYPE", "GATEWAY ID"],
            "datasourceId",
            None,
        );
    } else {
        output::render_object(cli, &data, "datasourceId");
    }
    Ok(())
}

async fn update_datasources(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    content: &str,
) -> Result<()> {
    let body = parse_json_content(content, "update-datasources")?;

    if output::dry_run_guard(cli, "semantic-model update-datasources", &body) {
        return Ok(());
    }

    client
        .post_powerbi(
            &format!("/groups/{workspace}/datasets/{id}/Default.UpdateDatasources"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model update-datasources", "Contributor"))?;

    let obj = serde_json::json!({
        "id": id,
        "status": "datasources_updated"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn list_users(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get_powerbi(&format!("/groups/{workspace}/datasets/{id}/users"))
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model list-users", "Admin"))?;

    if let Some(items) = data.get("value").and_then(Value::as_array) {
        output::render_list_with_token(
            cli,
            items,
            &[
                "identifier",
                "principalType",
                "datasetUserAccessRight",
                "displayName",
            ],
            &["IDENTIFIER", "TYPE", "ACCESS RIGHT", "DISPLAY NAME"],
            "identifier",
            None,
        );
    } else {
        output::render_object(cli, &data, "identifier");
    }
    Ok(())
}

async fn add_user(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    principal: &str,
    principal_type: &str,
    access_right: &str,
) -> Result<()> {
    let body = serde_json::json!({
        "identifier": principal,
        "principalType": principal_type,
        "datasetUserAccessRight": access_right
    });

    if output::dry_run_guard(cli, "semantic-model add-user", &body) {
        return Ok(());
    }

    client
        .post_powerbi(
            &format!("/groups/{workspace}/datasets/{id}/users"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model add-user", "Admin"))?;

    let obj = serde_json::json!({
        "id": id,
        "principal": principal,
        "access_right": access_right,
        "status": "user_added"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn delete_user(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    user: &str,
) -> Result<()> {
    let body = serde_json::json!({
        "datasetId": id,
        "user": user
    });

    if output::dry_run_guard(cli, "semantic-model delete-user", &body) {
        return Ok(());
    }

    client
        .delete_powerbi(&format!(
            "/groups/{workspace}/datasets/{id}/users/{user}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model delete-user", "Admin"))?;

    let obj = serde_json::json!({
        "id": id,
        "user": user,
        "status": "user_removed"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

async fn refresh_status(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    top: u32,
) -> Result<()> {
    let data = client
        .get_powerbi(&format!(
            "/groups/{workspace}/datasets/{id}/refreshes?$top={top}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model refresh-status", "Contributor"))?;

    if let Some(items) = data.get("value").and_then(Value::as_array) {
        output::render_list_with_token(
            cli,
            items,
            &["requestId", "refreshType", "status", "startTime", "endTime"],
            &["REQUEST ID", "TYPE", "STATUS", "START", "END"],
            "requestId",
            None,
        );
    } else {
        output::render_object(cli, &data, "requestId");
    }
    Ok(())
}

async fn list_upstream(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get_powerbi(&format!(
            "/groups/{workspace}/datasets/{id}/upstreamDatasets"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "semantic-model list-upstream", "Contributor"))?;

    if let Some(items) = data.get("value").and_then(Value::as_array) {
        output::render_list_with_token(
            cli,
            items,
            &["targetDatasetId", "groupId"],
            &["DATASET ID", "WORKSPACE ID"],
            "targetDatasetId",
            None,
        );
    } else {
        output::render_object(cli, &data, "targetDatasetId");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enrich_dax_error_dataset_not_found() {
        let err: anyhow::Error = FabioError::new(
            ErrorCode::NotFound,
            "Dataset not found in workspace".to_string(),
        )
        .into();

        let enriched = enrich_dax_error(err);
        let fabio_err = enriched.downcast_ref::<FabioError>().unwrap();
        assert_eq!(fabio_err.code, ErrorCode::NotFound);
        assert!(
            fabio_err
                .hint
                .as_ref()
                .unwrap()
                .contains("semantic-model list")
        );
    }

    #[test]
    fn test_enrich_dax_error_not_framed() {
        let err: anyhow::Error = FabioError::new(
            ErrorCode::ApiError,
            "Query failed with error code 3242524690".to_string(),
        )
        .into();

        let enriched = enrich_dax_error(err);
        let fabio_err = enriched.downcast_ref::<FabioError>().unwrap();
        assert!(fabio_err.hint.as_ref().unwrap().contains("framing"));
    }

    #[test]
    fn test_enrich_dax_error_syntax() {
        let err: anyhow::Error = FabioError::new(
            ErrorCode::ApiError,
            "DAX syntax error near 'EVALUAT'".to_string(),
        )
        .into();

        let enriched = enrich_dax_error(err);
        let fabio_err = enriched.downcast_ref::<FabioError>().unwrap();
        assert!(fabio_err.hint.as_ref().unwrap().contains("EVALUATE"));
    }

    #[test]
    fn test_enrich_dax_error_passthrough() {
        let err: anyhow::Error =
            FabioError::new(ErrorCode::ApiError, "Some unknown error".to_string()).into();

        let enriched = enrich_dax_error(err);
        let fabio_err = enriched.downcast_ref::<FabioError>().unwrap();
        // No hint added — returned as-is
        assert!(fabio_err.hint.is_none());
    }

    #[test]
    fn test_enrich_create_error_v3_models() {
        let err: anyhow::Error = FabioError::new(
            ErrorCode::ApiError,
            "Import from JSON supported for V3 models only".to_string(),
        )
        .into();

        let enriched = enrich_create_error(err);
        let fabio_err = enriched.downcast_ref::<FabioError>().unwrap();
        assert!(fabio_err.hint.as_ref().unwrap().contains("1604"));
    }

    #[test]
    fn test_enrich_create_error_tmdl_format() {
        let err: anyhow::Error = FabioError::new(
            ErrorCode::ApiError,
            "TMDL Format Error: Parsing error at line number 5".to_string(),
        )
        .into();

        let enriched = enrich_create_error(err);
        let fabio_err = enriched.downcast_ref::<FabioError>().unwrap();
        assert!(fabio_err.hint.as_ref().unwrap().contains("tab"));
    }
}
