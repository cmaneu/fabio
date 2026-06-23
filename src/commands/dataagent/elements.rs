use anyhow::Result;
use base64::Engine;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

use super::{decode_part_payload, find_datasource_dir, get_definition_parts};

/// List elements (tables/columns) in a datasource with selection state and descriptions.
pub(super) async fn list_elements(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
) -> Result<()> {
    let parts = get_definition_parts(client, workspace, id).await?;
    let ds_dir = find_datasource_dir(&parts, datasource)?;
    let ds_path = format!("{ds_dir}/datasource.json");

    let ds_payload = parts
        .iter()
        .find_map(|part| {
            let path = part.get("path").and_then(Value::as_str)?;
            if path == ds_path {
                part.get("payload")
                    .and_then(Value::as_str)
                    .map(String::from)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::NotFound,
                format!("Datasource file not found at '{ds_path}'"),
            )
        })?;

    let ds_json: Value = decode_part_payload(&ds_payload)
        .and_then(|s| serde_json::from_str(&s).ok())
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "Failed to decode datasource definition",
            )
        })?;

    let elements = ds_json
        .get("elements")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    // Flatten the tree into a list of rows for output
    let mut flat: Vec<Value> = Vec::new();
    flatten_elements(&elements, &mut flat, 0);

    output::render_list_with_token(
        cli,
        &flat,
        &["path", "type", "selected", "description"],
        &["PATH", "TYPE", "SELECTED", "DESCRIPTION"],
        "path",
        None,
    );
    Ok(())
}

/// Set or clear a description on a specific element identified by dot-path.
#[allow(clippy::too_many_arguments)]
pub(super) async fn describe_element(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
    path: &str,
    description: Option<&str>,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent describe-element",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
            "path": path,
            "description": description,
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;
    let ds_dir = find_datasource_dir(&parts, datasource)?;
    let ds_path = format!("{ds_dir}/datasource.json");

    let ds_payload = parts
        .iter()
        .find_map(|part| {
            let p = part.get("path").and_then(Value::as_str)?;
            if p == ds_path {
                part.get("payload")
                    .and_then(Value::as_str)
                    .map(String::from)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::NotFound,
                format!("Datasource file not found at '{ds_path}'"),
            )
        })?;

    let mut ds_json: Value = decode_part_payload(&ds_payload)
        .and_then(|s| serde_json::from_str(&s).ok())
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "Failed to decode datasource definition",
            )
        })?;

    // Navigate to the element by dot-path
    let path_parts: Vec<&str> = path.split('.').collect();
    let element = find_element_by_path(
        ds_json.get_mut("elements").and_then(Value::as_array_mut),
        &path_parts,
    )
    .ok_or_else(|| {
        FabioError::with_hint(
            ErrorCode::NotFound,
            format!("Element not found at path '{path}'"),
            "Use 'fabio data-agent list-elements' to see available paths. Path format: schema.table or schema.table.column",
        )
    })?;

    // Set or clear the description
    match description {
        Some(desc) if !desc.is_empty() => {
            element["description"] = Value::from(desc);
        }
        _ => {
            // Clear description
            if let Some(obj) = element.as_object_mut() {
                obj.remove("description");
            }
        }
    }

    // Re-encode and push updated definition
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_string(&ds_json)?.as_bytes());

    let new_parts: Vec<Value> = parts
        .iter()
        .map(|p| {
            if p.get("path").and_then(Value::as_str) == Some(&ds_path) {
                serde_json::json!({
                    "path": ds_path,
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                })
            } else {
                p.clone()
            }
        })
        .collect();

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "status": if description.is_some_and(|d| !d.is_empty()) { "description_set" } else { "description_cleared" },
        "path": path,
        "description": description.unwrap_or(""),
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

// ─── Private Helpers ─────────────────────────────────────────────────────────

/// Flatten the element tree into a list of rows with indented paths.
fn flatten_elements(elements: &[Value], output: &mut Vec<Value>, depth: usize) {
    for elem in elements {
        let display_name = elem
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or("");
        let elem_type = elem.get("type").and_then(Value::as_str).unwrap_or("");
        let is_selected = elem.get("is_selected").and_then(Value::as_bool);
        let description = elem
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let id_path = elem
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or(display_name);

        // Build indented display name for tree visualization
        let indent = "  ".repeat(depth);
        let display_path = format!("{indent}{display_name}");

        let mut row = serde_json::json!({
            "path": id_path,
            "displayPath": display_path,
            "name": display_name,
            "type": elem_type,
        });

        if let Some(selected) = is_selected {
            row["selected"] = Value::Bool(selected);
        }
        if !description.is_empty() {
            row["description"] = Value::from(description);
        }
        if let Some(dt) = elem.get("data_type").and_then(Value::as_str) {
            row["dataType"] = Value::from(dt);
        }

        output.push(row);

        // Recurse into children
        if let Some(children) = elem.get("children").and_then(Value::as_array) {
            flatten_elements(children, output, depth + 1);
        }
    }
}

/// Navigate the element tree by dot-path (e.g. `dbo.orders.total_amount`).
///
/// Matches elements by `display_name` at each level of the path hierarchy.
fn find_element_by_path<'a>(
    elements: Option<&'a mut Vec<Value>>,
    path_parts: &[&str],
) -> Option<&'a mut Value> {
    let elements = elements?;
    let (first, rest) = path_parts.split_first()?;

    for elem in elements.iter_mut() {
        let name = elem
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or("");
        if name.eq_ignore_ascii_case(first) {
            if rest.is_empty() {
                return Some(elem);
            }
            // Recurse into children
            return find_element_by_path(
                elem.get_mut("children").and_then(Value::as_array_mut),
                rest,
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn flatten_elements_basic_tree() {
        let elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "id": "dbo",
            "is_selected": true,
            "children": [
                {
                    "display_name": "orders",
                    "type": "lakehouse_tables.table",
                    "id": "dbo.orders",
                    "is_selected": true,
                    "description": "Customer orders",
                    "children": [
                        {"display_name": "order_id", "type": "lakehouse_tables.column", "id": "dbo.orders.order_id", "data_type": "int", "children": []}
                    ]
                }
            ]
        })];

        let mut flat = Vec::new();
        flatten_elements(&elements, &mut flat, 0);
        assert_eq!(flat.len(), 3);
        assert_eq!(flat[0]["path"], "dbo");
        assert_eq!(flat[0]["selected"], true);
        assert_eq!(flat[1]["path"], "dbo.orders");
        assert_eq!(flat[1]["description"], "Customer orders");
        assert_eq!(flat[2]["path"], "dbo.orders.order_id");
        assert_eq!(flat[2]["dataType"], "int");
    }

    #[test]
    fn flatten_elements_empty() {
        let mut flat = Vec::new();
        flatten_elements(&[], &mut flat, 0);
        assert!(flat.is_empty());
    }

    #[test]
    fn find_element_by_path_schema_level() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "children": []
        })];
        let found = find_element_by_path(Some(&mut elements), &["dbo"]);
        assert!(found.is_some());
        assert_eq!(found.unwrap()["display_name"], "dbo");
    }

    #[test]
    fn find_element_by_path_table_level() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "children": [
                {"display_name": "orders", "type": "lakehouse_tables.table", "children": []}
            ]
        })];
        let found = find_element_by_path(Some(&mut elements), &["dbo", "orders"]);
        assert!(found.is_some());
        assert_eq!(found.unwrap()["display_name"], "orders");
    }

    #[test]
    fn find_element_by_path_column_level() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "children": [{
                "display_name": "orders",
                "type": "lakehouse_tables.table",
                "children": [
                    {"display_name": "total_amount", "type": "lakehouse_tables.column", "children": []}
                ]
            }]
        })];
        let found = find_element_by_path(Some(&mut elements), &["dbo", "orders", "total_amount"]);
        assert!(found.is_some());
        assert_eq!(found.unwrap()["display_name"], "total_amount");
    }

    #[test]
    fn find_element_by_path_not_found() {
        let mut elements = vec![json!({
            "display_name": "dbo",
            "type": "lakehouse_tables.schema",
            "children": []
        })];
        let found = find_element_by_path(Some(&mut elements), &["dbo", "nonexistent"]);
        assert!(found.is_none());
    }

    #[test]
    fn find_element_by_path_case_insensitive() {
        let mut elements = vec![json!({
            "display_name": "DBO",
            "type": "lakehouse_tables.schema",
            "children": [
                {"display_name": "Orders", "type": "lakehouse_tables.table", "children": []}
            ]
        })];
        let found = find_element_by_path(Some(&mut elements), &["dbo", "orders"]);
        assert!(found.is_some());
    }
}
