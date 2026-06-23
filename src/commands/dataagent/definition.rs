use anyhow::Result;
use base64::Engine;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

use super::get_definition_parts;

/// Get the definition of a data agent (data sources, instructions, etc.).
pub(super) async fn get_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await?;
    output::render_object(cli, &data, "definition");
    Ok(())
}

/// Update the definition of a data agent (configure data sources, instructions, etc.).
pub(super) async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
    update_metadata: bool,
) -> Result<()> {
    let definition_json = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(inline)) => inline.to_string(),
        (None, None) => {
            return Err(
                FabioError::invalid_input("Either --file or --content must be provided").into(),
            );
        }
    };

    let body: Value = serde_json::from_str(&definition_json).map_err(|e| {
        FabioError::new(
            ErrorCode::InvalidInput,
            format!("Invalid JSON definition: {e}"),
        )
    })?;

    // If the body already has a "definition" wrapper, use as-is; otherwise wrap it
    let request_body = if body.get("definition").is_some() {
        body
    } else {
        serde_json::json!({ "definition": body })
    };

    // Validate datasource element IDs before sending to the API
    validate_datasource_elements(&request_body)?;

    if output::dry_run_guard(
        cli,
        "data-agent update-definition",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "updateMetadata": update_metadata,
        }),
    ) {
        return Ok(());
    }

    let path = if update_metadata {
        format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition?updateMetadata=True")
    } else {
        format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition")
    };

    let data = client.post(&path, &request_body, true).await?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

/// Publish a data agent by promoting draft configuration to published state.
///
/// This fetches the current definition, copies draft-stage configuration
/// (including datasources and fewshots) to published, adds `publish_info.json`,
/// and updates the definition. This is the officially supported programmatic
/// publish path (no portal interaction required).
#[allow(clippy::too_many_lines)]
pub(super) async fn publish(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    description: Option<&str>,
    to_m365: bool,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent publish",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "description": description,
            "toM365": to_m365,
        }),
    ) {
        return Ok(());
    }

    // Step 1: Get current definition
    let parts = get_definition_parts(client, workspace, id).await?;

    if parts.is_empty() {
        return Err(FabioError::new(
            ErrorCode::ApiError,
            "Data agent has no definition parts. Configure data sources first with \
             'fabio data-agent update-definition'.",
        )
        .into());
    }

    // Step 2: Build new definition with published parts
    let mut new_parts: Vec<Value> = Vec::new();

    // Keep existing parts
    for part in &parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        // Skip existing published parts and publish_info (we'll regenerate them)
        if !path.starts_with("Files/Config/published/") && path != "Files/Config/publish_info.json"
        {
            new_parts.push(part.clone());
        }
    }

    // Copy draft parts to published
    for part in &parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if path.starts_with("Files/Config/draft/") {
            let published_path = path.replace("Files/Config/draft/", "Files/Config/published/");
            let mut published_part = part.clone();
            if let Some(obj) = published_part.as_object_mut() {
                obj.insert("path".to_string(), Value::String(published_path));
            }
            new_parts.push(published_part);
        }
    }

    // Add publish_info.json
    let publish_info = serde_json::json!({
        "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/publishInfo/1.0.0/schema.json",
        "description": description.unwrap_or("")
    });
    let publish_info_encoded =
        base64::engine::general_purpose::STANDARD.encode(publish_info.to_string().as_bytes());
    new_parts.push(serde_json::json!({
        "path": "Files/Config/publish_info.json",
        "payload": publish_info_encoded,
        "payloadType": "InlineBase64"
    }));

    // Step 3: Validate and update the definition
    let update_body = serde_json::json!({
        "definition": {
            "parts": new_parts
        }
    });

    validate_datasource_elements(&update_body)?;

    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    // Step 4: Try the V3 settings endpoint to check if chat endpoint is active
    let settings_path = format!("/workspaces/{workspace}/dataAgents/{id}/settings");
    let published_url = client.get(&settings_path).await.ok().and_then(|s| {
        s.get("publishedUrl")
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())
            .map(String::from)
    });

    let mut obj = serde_json::json!({
        "id": id,
        "status": "published",
        "description": description.unwrap_or(""),
    });

    if let Some(url) = published_url {
        obj["publishedUrl"] = Value::String(url);
    }

    // Step 5 (optional): Publish to M365 Copilot Agent Store
    if to_m365 {
        // Resolve capacity ID for the workspace
        let ws_info = client.get(&format!("/workspaces/{workspace}")).await?;
        let capacity_id = ws_info
            .get("capacityId")
            .and_then(Value::as_str)
            .unwrap_or("");

        if capacity_id.is_empty() {
            return Err(FabioError::with_hint(
                ErrorCode::ApiError,
                "Cannot resolve capacity ID for M365 publishing",
                "The workspace must have a capacity assigned. Check: fabio workspace show -w <workspace>",
            ).into());
        }

        // The M365 endpoint uses the internal workload API
        let m365_url = format!("/workspaces/{workspace}/dataAgents/{id}/publishToM365");
        // Try the public API first; if it doesn't exist, note it in output
        let m365_result = client
            .post(&m365_url, &serde_json::json!({"scope": "Shared"}), false)
            .await;

        match m365_result {
            Ok(_) => {
                obj["m365Status"] = Value::String("published_to_m365".to_string());
            }
            Err(e) => {
                // M365 publishing is best-effort; report in output but don't fail
                obj["m365Status"] = Value::from("failed");
                obj["m365Error"] = Value::String(e.to_string());
            }
        }
    }

    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Validation ──────────────────────────────────────────────────────────────

/// Validate datasource element IDs in a definition before sending to the API.
///
/// Elements with `id: null` or empty strings will cause the data agent to show
/// "This table has been deleted or you don't have permission to view it" in the
/// portal UI. IDs must follow the dot-path convention:
/// - Schema: `"dbo"`
/// - Table: `"dbo.table_name"`
/// - Column: `"dbo.table_name.column_name"`
fn validate_datasource_elements(body: &Value) -> Result<()> {
    let parts = body
        .get("definition")
        .or(Some(body))
        .and_then(|d| d.get("definition").or(Some(d)))
        .and_then(|d| d.get("parts"))
        .and_then(Value::as_array);

    let Some(parts) = parts else {
        return Ok(());
    };

    for part in parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if !path.contains("datasource.json") {
            continue;
        }

        let Some(payload_str) = part.get("payload").and_then(Value::as_str) else {
            continue;
        };

        // Decode base64 payload
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(payload_str)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok());

        let Some(json_str) = decoded else {
            continue;
        };

        let Ok(datasource) = serde_json::from_str::<Value>(&json_str) else {
            continue;
        };

        let Some(elements) = datasource.get("elements").and_then(Value::as_array) else {
            continue;
        };

        validate_elements_recursive(elements, path)?;
    }

    Ok(())
}

/// Recursively validate that all elements have non-null, non-empty IDs.
fn validate_elements_recursive(elements: &[Value], datasource_path: &str) -> Result<()> {
    for element in elements {
        let id = element.get("id");
        let display_name = element
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        let element_type = element
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");

        // Check for null or empty ID
        let id_is_missing = match id {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty(),
            _ => false,
        };

        if id_is_missing {
            let hint = if element_type.contains(".schema") {
                "schema elements should use the schema name as id, e.g. \"dbo\"".to_string()
            } else if element_type.contains(".table") {
                format!(
                    "table elements should use \"schema.table\" format, e.g. \"dbo.{display_name}\""
                )
            } else if element_type.contains(".column") {
                format!(
                    "column elements should use \"schema.table.column\" format, e.g. \"dbo.table_name.{display_name}\""
                )
            } else {
                "elements require a non-null id following dot-path convention".to_string()
            };

            return Err(FabioError::new(
                ErrorCode::InvalidInput,
                format!(
                    "Element '{display_name}' (type: {element_type}) in '{datasource_path}' has a \
                     null or empty 'id'. {hint}. Without valid IDs the portal will show \
                     'This table has been deleted or you don't have permission to view it'."
                ),
            )
            .into());
        }

        // Recurse into children
        if let Some(children) = element.get("children").and_then(Value::as_array) {
            validate_elements_recursive(children, datasource_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use serde_json::json;

    use super::*;

    fn make_definition(datasource_json: &serde_json::Value) -> serde_json::Value {
        let payload = base64::engine::general_purpose::STANDARD.encode(datasource_json.to_string());
        json!({
            "definition": {
                "parts": [
                    {
                        "path": "Files/Config/draft/lakehouse-tables-TestLH/datasource.json",
                        "payload": payload,
                        "payloadType": "InlineBase64"
                    }
                ]
            }
        })
    }

    #[test]
    fn valid_elements_pass_validation() {
        let datasource = json!({
            "artifactId": "00000000-0000-0000-0000-000000000001",
            "workspaceId": "00000000-0000-0000-0000-000000000002",
            "displayName": "TestLH",
            "type": "lakehouse_tables",
            "elements": [
                {
                    "id": "dbo",
                    "is_selected": true,
                    "display_name": "dbo",
                    "type": "lakehouse_tables.schema",
                    "children": [
                        {
                            "id": "dbo.my_table",
                            "is_selected": true,
                            "display_name": "my_table",
                            "type": "lakehouse_tables.table",
                            "children": [
                                {
                                    "id": "dbo.my_table.col1",
                                    "is_selected": true,
                                    "display_name": "col1",
                                    "type": "lakehouse_tables.column",
                                    "data_type": "string",
                                    "children": []
                                }
                            ]
                        }
                    ]
                }
            ]
        });

        let body = make_definition(&datasource);
        assert!(validate_datasource_elements(&body).is_ok());
    }

    #[test]
    fn null_table_id_fails_validation() {
        let datasource = json!({
            "artifactId": "00000000-0000-0000-0000-000000000001",
            "workspaceId": "00000000-0000-0000-0000-000000000002",
            "displayName": "TestLH",
            "type": "lakehouse_tables",
            "elements": [
                {
                    "id": null,
                    "is_selected": true,
                    "display_name": "my_table",
                    "type": "lakehouse_tables.table",
                    "children": []
                }
            ]
        });

        let body = make_definition(&datasource);
        let err = validate_datasource_elements(&body).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("my_table"),
            "Error should mention element name: {msg}"
        );
        assert!(
            msg.contains("null or empty"),
            "Error should explain the problem: {msg}"
        );
        assert!(
            msg.contains("dbo.my_table"),
            "Error should suggest correct format: {msg}"
        );
    }

    #[test]
    fn empty_string_id_fails_validation() {
        let datasource = json!({
            "artifactId": "00000000-0000-0000-0000-000000000001",
            "workspaceId": "00000000-0000-0000-0000-000000000002",
            "displayName": "TestLH",
            "type": "lakehouse_tables",
            "elements": [
                {
                    "id": "",
                    "is_selected": true,
                    "display_name": "bad_schema",
                    "type": "lakehouse_tables.schema",
                    "children": []
                }
            ]
        });

        let body = make_definition(&datasource);
        let err = validate_datasource_elements(&body).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("bad_schema"),
            "Error should mention element name: {msg}"
        );
    }

    #[test]
    fn nested_null_column_id_fails_validation() {
        let datasource = json!({
            "artifactId": "00000000-0000-0000-0000-000000000001",
            "workspaceId": "00000000-0000-0000-0000-000000000002",
            "displayName": "TestLH",
            "type": "lakehouse_tables",
            "elements": [
                {
                    "id": "dbo",
                    "is_selected": true,
                    "display_name": "dbo",
                    "type": "lakehouse_tables.schema",
                    "children": [
                        {
                            "id": "dbo.my_table",
                            "is_selected": true,
                            "display_name": "my_table",
                            "type": "lakehouse_tables.table",
                            "children": [
                                {
                                    "id": null,
                                    "is_selected": true,
                                    "display_name": "bad_col",
                                    "type": "lakehouse_tables.column",
                                    "data_type": "int",
                                    "children": []
                                }
                            ]
                        }
                    ]
                }
            ]
        });

        let body = make_definition(&datasource);
        let err = validate_datasource_elements(&body).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("bad_col"),
            "Error should mention nested element: {msg}"
        );
        assert!(
            msg.contains("dbo.table_name.bad_col"),
            "Error should suggest dot-path: {msg}"
        );
    }

    #[test]
    fn empty_elements_array_passes() {
        let datasource = json!({
            "artifactId": "00000000-0000-0000-0000-000000000001",
            "workspaceId": "00000000-0000-0000-0000-000000000002",
            "displayName": "TestLH",
            "type": "lakehouse_tables",
            "elements": []
        });

        let body = make_definition(&datasource);
        assert!(validate_datasource_elements(&body).is_ok());
    }

    #[test]
    fn no_datasource_parts_passes() {
        let body = json!({
            "definition": {
                "parts": [
                    {
                        "path": "Files/Config/data_agent.json",
                        "payload": "e30=",
                        "payloadType": "InlineBase64"
                    }
                ]
            }
        });
        assert!(validate_datasource_elements(&body).is_ok());
    }
}
