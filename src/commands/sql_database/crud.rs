use anyhow::Result;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

pub(super) async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/sqlDatabases"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    let has_labels = resp
        .items
        .iter()
        .any(|item| item.get("sensitivityLabel").is_some_and(|v| !v.is_null()));

    if has_labels {
        output::render_list_with_token(
            cli,
            &resp.items,
            &["displayName", "id", "description", "sensitivityLabel.id"],
            &["NAME", "ID", "DESCRIPTION", "SENSITIVITY LABEL"],
            "id",
            resp.continuation_token.as_deref(),
        );
    } else {
        output::render_list_with_token(
            cli,
            &resp.items,
            &["displayName", "id", "description"],
            &["NAME", "ID", "DESCRIPTION"],
            "id",
            resp.continuation_token.as_deref(),
        );
    }
    Ok(())
}

pub(super) async fn show(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/sqlDatabases/{id}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    description: Option<&str>,
    creation_mode: Option<&str>,
    backup_retention_days: Option<i32>,
    collation: Option<&str>,
    source_workspace: Option<&str>,
    source_database: Option<&str>,
    restore_point: Option<&str>,
    restorable_deleted_database_name: Option<&str>,
    sensitivity_label: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(desc) = description {
        body["description"] = Value::from(desc);
    }

    // Build creationPayload based on mode
    let mode = creation_mode.unwrap_or("New");
    match mode {
        "Restore" => {
            let src_ws = source_workspace.ok_or_else(|| {
                FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "--source-workspace is required for Restore mode".to_string(),
                    "Example: fabio sql-database create --workspace <WS> --name <NAME> --creation-mode Restore --source-workspace <SRC_WS> --source-database <SRC_ID> --restore-point 2024-01-01T00:00:00Z".to_string(),
                )
            })?;
            let src_db = source_database.ok_or_else(|| {
                FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "--source-database is required for Restore mode".to_string(),
                    "Provide the item ID of the source database to restore from".to_string(),
                )
            })?;
            let rp = restore_point.ok_or_else(|| {
                FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "--restore-point is required for Restore mode".to_string(),
                    "Provide an ISO 8601 timestamp (e.g., 2024-01-01T00:00:00Z)".to_string(),
                )
            })?;
            body["creationPayload"] = serde_json::json!({
                "creationMode": "Restore",
                "restorePointInTime": rp,
                "sourceDatabaseReference": {
                    "workspaceId": src_ws,
                    "id": src_db
                }
            });
        }
        "RestoreDeletedDatabase" => {
            let deleted_name = restorable_deleted_database_name.ok_or_else(|| {
                FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "--restorable-deleted-database-name is required for RestoreDeletedDatabase mode".to_string(),
                    "Use 'fabio sql-database list-deleted' to find available names".to_string(),
                )
            })?;
            let rp = restore_point.ok_or_else(|| {
                FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    "--restore-point is required for RestoreDeletedDatabase mode".to_string(),
                    "Provide an ISO 8601 timestamp (e.g., 2024-01-01T00:00:00Z)".to_string(),
                )
            })?;
            body["creationPayload"] = serde_json::json!({
                "creationMode": "RestoreDeletedDatabase",
                "restorePointInTime": rp,
                "restorableDeletedDatabaseName": deleted_name
            });
        }
        _ => {
            // "New" mode or default
            let mut payload = serde_json::json!({ "creationMode": "New" });
            if let Some(days) = backup_retention_days {
                payload["backupRetentionDays"] = Value::Number(serde_json::Number::from(days));
            }
            if let Some(c) = collation {
                payload["collation"] = Value::from(c);
            }
            // Only include creationPayload if there are extra settings
            if backup_retention_days.is_some() || collation.is_some() {
                body["creationPayload"] = payload;
            }
        }
    }
    if let Some(label_id) = sensitivity_label {
        body["sensitivityLabelSettings"] = serde_json::json!({
            "sensitivityLabelId": label_id
        });
    }

    if output::dry_run_guard(cli, "sql-database create", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/sqlDatabases"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database create", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

pub(super) async fn update(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    description: Option<&str>,
) -> Result<()> {
    if description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least --description must be provided".to_string(),
            "Example: fabio sql-database update --workspace <WS> --id <ID> --description \"New desc\""
                .to_string(),
        )
        .into());
    }

    let mut body = serde_json::json!({});
    if let Some(d) = description {
        body["description"] = Value::from(d);
    }

    if output::dry_run_guard(cli, "sql-database update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/sqlDatabases/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

pub(super) async fn delete(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    hard_delete: bool,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "sql-database delete",
        &serde_json::json!({ "workspace": workspace, "id": id, "hardDelete": hard_delete }),
    ) {
        return Ok(());
    }

    let url = if hard_delete {
        format!("/workspaces/{workspace}/sqlDatabases/{id}?hardDelete=true")
    } else {
        format!("/workspaces/{workspace}/sqlDatabases/{id}")
    };

    client
        .delete(&url)
        .await
        .map_err(|e| enrich_forbidden(e, "sql-database delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}
