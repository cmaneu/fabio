use anyhow::Result;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

use super::resolve_datasource_id;

/// List configured data sources via the staging datasources API.
///
/// Uses: `GET /workspaces/{ws}/dataAgents/{id}/staging/datasources`
pub(super) async fn list_datasources(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/dataAgents/{id}/staging/datasources"),
            "value",
            true,
            None,
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "displayName", "type"],
        &["ID", "NAME", "TYPE"],
        "displayName",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

/// Show details of a specific data source.
///
/// Uses: `GET /workspaces/{ws}/dataAgents/{id}/staging/datasources/{dsId}`
pub(super) async fn show_datasource(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
) -> Result<()> {
    let ds_id = resolve_datasource_id(client, workspace, id, datasource).await?;

    let data = client
        .get(&format!(
            "/workspaces/{workspace}/dataAgents/{id}/staging/datasources/{ds_id}"
        ))
        .await?;

    output::render_object(cli, &data, "displayName");
    Ok(())
}

/// Add a data source to the agent via the staging datasources API.
///
/// Uses: `POST /workspaces/{ws}/dataAgents/{id}/staging/datasources` (LRO)
#[allow(clippy::too_many_arguments)]
pub(super) async fn add_datasource(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    artifact: &str,
    artifact_workspace: Option<&str>,
    artifact_type: Option<&str>,
    instructions: Option<&str>,
) -> Result<()> {
    let ds_workspace = artifact_workspace.unwrap_or(workspace);

    // Resolve artifact type and ID
    let (resolved_type, artifact_id, artifact_name) =
        resolve_artifact(client, ds_workspace, artifact, artifact_type).await?;

    if output::dry_run_guard(
        cli,
        "data-agent add-datasource",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "artifactId": artifact_id,
            "artifactName": artifact_name,
            "artifactWorkspace": ds_workspace,
            "fabricItemType": resolved_type,
        }),
    ) {
        return Ok(());
    }

    // Build request body per the new API schema
    // The API uses DatasourceType discriminator: "FabricItem" or "LakehouseTables"
    let mut body = serde_json::json!({
        "type": "FabricItem",
        "itemReference": {
            "itemId": artifact_id,
            "workspaceId": ds_workspace,
        },
        "fabricItemType": resolved_type,
    });
    if let Some(instr) = instructions {
        body["instructions"] = Value::from(instr);
    }

    // Datasource creation triggers async schema discovery (LRO).
    // The server may take 1-3 minutes to complete schema indexing.
    let resp = client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/staging/datasources"),
            &body,
            true,
        )
        .await?;

    // API returns the created datasource object (or empty on LRO)
    let result = if resp.is_null() || resp.as_object().is_some_and(serde_json::Map::is_empty) {
        serde_json::json!({
            "status": "datasource_added",
            "artifactId": artifact_id,
            "displayName": artifact_name,
            "fabricItemType": resolved_type,
        })
    } else {
        let mut r = resp;
        if let Some(obj) = r.as_object_mut() {
            obj.insert("status".to_string(), Value::from("datasource_added"));
        }
        r
    };
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Remove a data source from the agent.
///
/// Uses: `DELETE /workspaces/{ws}/dataAgents/{id}/staging/datasources/{dsId}`
pub(super) async fn remove_datasource(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent remove-datasource",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
        }),
    ) {
        return Ok(());
    }

    let ds_id = resolve_datasource_id(client, workspace, id, datasource).await?;

    client
        .delete(&format!(
            "/workspaces/{workspace}/dataAgents/{id}/staging/datasources/{ds_id}"
        ))
        .await?;

    let result = serde_json::json!({
        "id": id,
        "status": "datasource_removed",
        "datasource": datasource,
        "datasourceId": ds_id,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Select or unselect tables in a data source via the elements API.
///
/// Uses: `GET .../staging/datasources/{dsId}/elements` to list tables,
/// then `PATCH .../staging/datasources/{dsId}/elements?id={elementId}` per element.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) async fn select_tables(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
    tables: Option<&str>,
    all_tables: bool,
    unselect: bool,
) -> Result<()> {
    if tables.is_none() && !all_tables {
        return Err(
            FabioError::invalid_input("Either --tables or --all-tables must be provided").into(),
        );
    }

    if output::dry_run_guard(
        cli,
        "data-agent select-tables",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
            "tables": tables,
            "allTables": all_tables,
            "unselect": unselect,
        }),
    ) {
        return Ok(());
    }

    let ds_id = resolve_datasource_id(client, workspace, id, datasource).await?;
    let base_path =
        format!("/workspaces/{workspace}/dataAgents/{id}/staging/datasources/{ds_id}/elements");

    // Fetch all elements (paginated) to find table-level elements
    let elements_resp = client.get_list(&base_path, "value", true, None).await?;

    let table_names: Vec<&str> = tables
        .map(|t| t.split(',').map(str::trim).collect())
        .unwrap_or_default();
    let target_selected = !unselect;

    // Collect elements that need updating — filter to table-type elements
    let table_types = ["Table", "ExternalTable", "MaterializedView", "View"];
    let mut modified = 0;

    for elem in &elements_resp.items {
        let elem_type = elem.get("type").and_then(Value::as_str).unwrap_or("");
        if !table_types
            .iter()
            .any(|t| t.eq_ignore_ascii_case(elem_type))
        {
            // Might need to check sub-elements; first try flat list at root
            // The API uses ?rootId for drill-down; tables may be nested under schemas
            if elem_type.eq_ignore_ascii_case("Schema") || elem_type.eq_ignore_ascii_case("Schemas")
            {
                // Drill into this schema to find tables
                let elem_id = elem.get("id").and_then(Value::as_str).unwrap_or("");
                if !elem_id.is_empty() {
                    let sub_resp = client
                        .get_list(
                            &format!("{base_path}?rootId={elem_id}"),
                            "value",
                            true,
                            None,
                        )
                        .await?;
                    for sub_elem in &sub_resp.items {
                        let sub_type = sub_elem.get("type").and_then(Value::as_str).unwrap_or("");
                        if !table_types.iter().any(|t| t.eq_ignore_ascii_case(sub_type)) {
                            continue;
                        }
                        let display_name = sub_elem
                            .get("displayName")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        let sub_id = sub_elem.get("id").and_then(Value::as_str).unwrap_or("");
                        let should_modify = all_tables
                            || table_names
                                .iter()
                                .any(|t| t.eq_ignore_ascii_case(display_name));
                        if should_modify && !sub_id.is_empty() {
                            let patch_body = serde_json::json!({
                                "isSelected": target_selected,
                            });
                            client
                                .patch(&format!("{base_path}?id={sub_id}"), &patch_body)
                                .await?;
                            modified += 1;
                        }
                    }
                }
            }
            continue;
        }

        let display_name = elem
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or("");
        let elem_id = elem.get("id").and_then(Value::as_str).unwrap_or("");
        let should_modify = all_tables
            || table_names
                .iter()
                .any(|t| t.eq_ignore_ascii_case(display_name));

        if should_modify && !elem_id.is_empty() {
            let patch_body = serde_json::json!({
                "isSelected": target_selected,
            });
            client
                .patch(&format!("{base_path}?id={elem_id}"), &patch_body)
                .await?;
            modified += 1;
        }
    }

    if modified == 0 && !all_tables {
        return Err(FabioError::with_hint(
            ErrorCode::NotFound,
            format!("No matching tables found: {}", table_names.join(", ")),
            "List available tables: fabio data-agent list-elements -w <workspace> --id <id> --datasource <ds>",
        )
        .into());
    }

    let result = serde_json::json!({
        "status": if unselect { "tables_unselected" } else { "tables_selected" },
        "modified": modified,
        "allTables": all_tables,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

// ─── Private Helpers ─────────────────────────────────────────────────────────

/// Resolve an artifact (name or ID) to its type, ID, and display name.
async fn resolve_artifact(
    client: &FabricClient,
    ds_workspace: &str,
    artifact: &str,
    artifact_type: Option<&str>,
) -> Result<(String, String, String)> {
    // Auto-detect artifact type if not provided
    let resolved_type = if let Some(t) = artifact_type {
        t.to_string()
    } else {
        let items = client
            .get_list(
                &format!("/workspaces/{ds_workspace}/items"),
                "value",
                true,
                None,
            )
            .await?;

        let found = items.items.iter().find(|item| {
            let item_name = item
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or("");
            let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");
            item_name.eq_ignore_ascii_case(artifact) || item_id == artifact
        });

        match found {
            Some(item) => item
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            None => {
                return Err(FabioError::with_hint(
                    ErrorCode::NotFound,
                    format!("Artifact '{artifact}' not found in workspace '{ds_workspace}'"),
                    "Specify the artifact type with --artifact-type, or check the workspace items: fabio item list -w <workspace>",
                ).into());
            }
        }
    };

    // Resolve artifact ID
    let items = client
        .get_list(
            &format!("/workspaces/{ds_workspace}/items?type={resolved_type}"),
            "value",
            true,
            None,
        )
        .await?;

    let artifact_item = items
        .items
        .iter()
        .find(|item| {
            let item_name = item.get("displayName").and_then(Value::as_str).unwrap_or("");
            let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");
            item_name.eq_ignore_ascii_case(artifact) || item_id == artifact
        })
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::NotFound,
                format!("Artifact '{artifact}' of type '{resolved_type}' not found"),
                format!("List items of this type: fabio item list -w {ds_workspace} --type {resolved_type}"),
            )
        })?;

    let artifact_id = artifact_item
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let artifact_name = artifact_item
        .get("displayName")
        .and_then(Value::as_str)
        .unwrap_or(artifact)
        .to_string();

    Ok((resolved_type, artifact_id, artifact_name))
}

#[cfg(test)]
mod tests {
    // Integration tests in tests/e2e_dataagent.rs cover the full flow.
    // Unit tests for `resolve_artifact` and other async helpers require mocking.
}
