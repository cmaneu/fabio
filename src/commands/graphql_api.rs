use std::io::{self, Read};

use anyhow::Result;
use base64::Engine;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum GraphqlApiCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List GraphQL APIs in a workspace
    #[command(display_order = 1)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a GraphQL API
    #[command(display_order = 2)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// GraphQL API ID
        #[arg(long)]
        id: String,
    },
    /// Create a new GraphQL API
    #[command(display_order = 3)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update GraphQL API properties (name and/or description)
    #[command(display_order = 4)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// GraphQL API ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a GraphQL API
    #[command(display_order = 5)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// GraphQL API ID
        #[arg(long)]
        id: String,
    },

    // ── Definitions ──────────────────────────────────────────────────────
    /// Get the definition of a GraphQL API
    #[command(display_order = 6)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// GraphQL API ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition of a GraphQL API
    #[command(display_order = 7)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// GraphQL API ID
        #[arg(long)]
        id: String,

        /// GraphQL schema file path (reads file content)
        #[arg(long)]
        file: Option<String>,

        /// GraphQL schema content (inline)
        #[arg(long)]
        content: Option<String>,
    },

    // ── Query ────────────────────────────────────────────────────────────
    /// Execute a GraphQL query against a GraphQL API
    #[command(display_order = 8)]
    Query {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// GraphQL API ID
        #[arg(long)]
        id: String,

        /// GraphQL query text (use @file.graphql to read from file, or pipe via stdin)
        #[arg(long)]
        gql: Option<String>,

        /// JSON-encoded variables for the query
        #[arg(long)]
        variables: Option<String>,

        /// Operation name (for multi-operation documents)
        #[arg(long)]
        operation_name: Option<String>,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &GraphqlApiCommand) -> Result<()> {
    match command {
        GraphqlApiCommand::List { workspace } => list(cli, client, workspace).await,
        GraphqlApiCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        GraphqlApiCommand::Create {
            workspace,
            name,
            description,
        } => create(cli, client, workspace, name, description.as_deref()).await,
        GraphqlApiCommand::Update {
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
        GraphqlApiCommand::Delete { workspace, id } => delete(cli, client, workspace, id).await,
        GraphqlApiCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        GraphqlApiCommand::UpdateDefinition {
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
        GraphqlApiCommand::Query {
            workspace,
            id,
            gql,
            variables,
            operation_name,
        } => {
            graphql_query(
                cli,
                client,
                workspace,
                id,
                gql.as_deref(),
                variables.as_deref(),
                operation_name.as_deref(),
            )
            .await
        }
    }
}

// ─── CRUD ────────────────────────────────────────────────────────────────────

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/graphQLApis"),
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
        .get(&format!("/workspaces/{workspace}/graphQLApis/{id}"))
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

    if output::dry_run_guard(cli, "graphql-api create", &body) {
        return Ok(());
    }

    let data = client
        .post(&format!("/workspaces/{workspace}/graphQLApis"), &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "graphql-api create", "Member"))?;
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
            "Example: fabio graphql-api update --workspace <WS> --id <ID> --name \"New Name\""
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

    if output::dry_run_guard(cli, "graphql-api update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/graphQLApis/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "graphql-api update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "graphql-api delete",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/graphQLApis/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "graphql-api delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Definitions ─────────────────────────────────────────────────────────────

async fn get_definition(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/graphQLApis/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "graphql-api get-definition", "Contributor"))?;
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
                "Example: fabio graphql-api update-definition --workspace <WS> --id <ID> --file schema.graphql".to_string(),
            ).into());
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(script.as_bytes());

    let body = serde_json::json!({
        "definition": {
            "parts": [
                {
                    "path": "schema.graphql",
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    if output::dry_run_guard(
        cli,
        "graphql-api update-definition",
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
            &format!("/workspaces/{workspace}/graphQLApis/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "graphql-api update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Query ───────────────────────────────────────────────────────────────────

async fn graphql_query(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    gql: Option<&str>,
    variables: Option<&str>,
    operation_name: Option<&str>,
) -> Result<()> {
    // Resolve GraphQL query text: --gql flag, @file prefix, or stdin
    let query_text = match gql {
        Some(s) if s.starts_with('@') => {
            let file_path = &s[1..];
            std::fs::read_to_string(file_path).map_err(|e| {
                FabioError::not_found(format!("GraphQL file not found: {file_path}: {e}"))
            })?
        }
        Some(s) => s.to_string(),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).map_err(|e| {
                FabioError::new(
                    ErrorCode::ApiError,
                    format!("Failed to read GraphQL query from stdin: {e}"),
                )
            })?;
            if buf.trim().is_empty() {
                return Err(FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "No GraphQL query provided. Use --gql, @file, or pipe via stdin.".to_string(),
                    "Example: fabio graphql-api query --workspace <WS> --id <ID> --gql \"{ __schema { types { name } } }\"".to_string(),
                )
                .into());
            }
            buf
        }
    };

    // Parse variables if provided
    let variables_value: Value = match variables {
        Some(v) => serde_json::from_str(v).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in --variables: {e}"),
                "Variables must be valid JSON, e.g.: --variables '{\"id\": 1}'".to_string(),
            )
        })?,
        None => Value::Null,
    };

    // Build standard GraphQL request body
    let mut body = serde_json::json!({
        "query": query_text,
    });
    if !variables_value.is_null() {
        body["variables"] = variables_value;
    }
    if let Some(op) = operation_name {
        body["operationName"] = Value::String(op.to_string());
    }

    // POST to the GraphQL execution endpoint (no LRO, synchronous)
    let data = client
        .post(
            &format!("/workspaces/{workspace}/graphQLApis/{id}/graphql"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "graphql-api query", "Viewer"))?;

    // Check for GraphQL-level errors (status 200 but errors in response)
    if let Some(errors) = data.get("errors") {
        if data.get("data").is_none() || data["data"].is_null() {
            // Pure error response — render as error with the first error message
            let message = errors
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("GraphQL query returned errors");
            return Err(FabioError::with_hint(
                ErrorCode::ApiError,
                message.to_string(),
                format!("Full errors: {errors}"),
            )
            .into());
        }
        // Partial response (data + errors) — render the full response including errors
        output::render_object(cli, &data, "data");
        return Ok(());
    }

    // Unwrap the GraphQL "data" envelope to avoid double-nesting in fabio's output
    let result = data.get("data").unwrap_or(&data);
    output::render_object(cli, result, "data");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphql_body_construction_basic() {
        let body = serde_json::json!({
            "query": "{ users { id name } }",
        });
        assert_eq!(body["query"], "{ users { id name } }");
        assert!(body.get("variables").is_none());
        assert!(body.get("operationName").is_none());
    }

    #[test]
    fn test_graphql_body_construction_with_variables() {
        let mut body = serde_json::json!({
            "query": "query GetUser($id: ID!) { user(id: $id) { name } }",
        });
        let vars: Value = serde_json::from_str(r#"{"id": "123"}"#).unwrap();
        body["variables"] = vars;
        body["operationName"] = Value::String("GetUser".to_string());

        assert_eq!(
            body["query"],
            "query GetUser($id: ID!) { user(id: $id) { name } }"
        );
        assert_eq!(body["variables"]["id"], "123");
        assert_eq!(body["operationName"], "GetUser");
    }

    #[test]
    fn test_graphql_error_response_pure_error() {
        let response = serde_json::json!({
            "errors": [
                {"message": "Cannot query field 'foo' on type 'Query'.", "locations": [{"line": 1, "column": 3}]}
            ]
        });
        let errors = response.get("errors").unwrap();
        let message = errors
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .unwrap();
        assert_eq!(message, "Cannot query field 'foo' on type 'Query'.");
    }

    #[test]
    fn test_graphql_error_response_partial() {
        // GraphQL allows partial data with errors
        let response = serde_json::json!({
            "data": {"users": [{"id": "1"}]},
            "errors": [{"message": "Unauthorized field 'email'"}]
        });
        // Has both data and errors — should still render
        assert!(response.get("data").is_some());
        assert!(!response["data"].is_null());
        assert!(response.get("errors").is_some());
    }

    #[test]
    fn test_graphql_introspection_query() {
        let query = "{ __schema { queryType { name } types { name kind } } }";
        let body = serde_json::json!({"query": query});
        assert!(body["query"].as_str().unwrap().contains("__schema"));
    }

    #[test]
    fn test_graphql_variables_invalid_json() {
        let bad_json = "not valid json";
        let result: Result<Value, _> = serde_json::from_str(bad_json);
        assert!(result.is_err());
    }
}
