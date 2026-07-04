use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;
use anyhow::Result;
use serde_json::Value;
pub(super) async fn list(
    cli: &Cli,
    client: &FabricClient,
    roles: Option<&str>,
    capacity: Option<&str>,
) -> Result<()> {
    let path = roles.map_or_else(
        || "/workspaces".to_string(),
        |r| format!("/workspaces?roles={r}"),
    );
    let resp = client
        .get_list(&path, "value", cli.all, cli.continuation_token.as_deref())
        .await?;
    let items = if let Some(cap_id) = capacity {
        resp.items
            .into_iter()
            .filter(|item| {
                item.get("capacityId")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| id.eq_ignore_ascii_case(cap_id))
            })
            .collect()
    } else {
        resp.items
    };

    let has_labels = items
        .iter()
        .any(|item| item.get("sensitivityLabel").is_some_and(|v| !v.is_null()));
    let has_tags = output::has_tags(&items);

    let display_items;
    let items_ref: &[Value] = if has_tags {
        display_items = output::enrich_with_tags_display(&items);
        &display_items
    } else {
        &items
    };

    match (has_labels, has_tags) {
        (true, true) => output::render_list_with_token(
            cli,
            items_ref,
            &[
                "displayName",
                "id",
                "type",
                "sensitivityLabel.id",
                "_tagsDisplay",
            ],
            &["NAME", "ID", "TYPE", "SENSITIVITY LABEL", "TAGS"],
            "id",
            resp.continuation_token.as_deref(),
        ),
        (true, false) => output::render_list_with_token(
            cli,
            items_ref,
            &["displayName", "id", "type", "sensitivityLabel.id"],
            &["NAME", "ID", "TYPE", "SENSITIVITY LABEL"],
            "id",
            resp.continuation_token.as_deref(),
        ),
        (false, true) => output::render_list_with_token(
            cli,
            items_ref,
            &["displayName", "id", "type", "_tagsDisplay"],
            &["NAME", "ID", "TYPE", "TAGS"],
            "id",
            resp.continuation_token.as_deref(),
        ),
        (false, false) => output::render_list_with_token(
            cli,
            items_ref,
            &["displayName", "id", "type"],
            &["NAME", "ID", "TYPE"],
            "id",
            resp.continuation_token.as_deref(),
        ),
    }
    Ok(())
}
pub(super) async fn show(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    let data = client.get(&format!("/workspaces/{id}")).await?;
    output::render_object(cli, &data, "id");
    Ok(())
}
#[allow(clippy::unnecessary_wraps)]
pub(super) fn url(cli: &Cli, id: &str) -> Result<()> {
    let data = serde_json::json!({ "url": format!("https://app.fabric.microsoft.com/groups/{id}"), "workspaceId": id });
    output::render_object(cli, &data, "url");
    Ok(())
}
pub(super) async fn create(
    cli: &Cli,
    client: &FabricClient,
    name: &str,
    description: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(desc) = description {
        body["description"] = Value::from(desc);
    }
    if output::dry_run_guard(cli, "workspace create", &body) {
        return Ok(());
    }
    let data = client
        .post("/workspaces", &body, false)
        .await
        .map_err(|e| enrich_forbidden(e, "workspace create", "Fabric user (tenant-level)"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
pub(super) async fn update(
    cli: &Cli,
    client: &FabricClient,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(ErrorCode::InvalidInput, "At least one of --name or --description must be provided".to_string(), "Example: fabio workspace update --id <ID> --name \"New Name\" --description \"New description\"".to_string()).into());
    }
    let mut body = serde_json::json!({});
    if let Some(n) = name {
        body["displayName"] = Value::from(n);
    }
    if let Some(d) = description {
        body["description"] = Value::from(d);
    }
    if output::dry_run_guard(cli, "workspace update", &body) {
        return Ok(());
    }
    let data = client
        .patch(&format!("/workspaces/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "workspace update", "Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}
pub(super) async fn delete(cli: &Cli, client: &FabricClient, id: &str) -> Result<()> {
    if output::dry_run_guard(cli, "workspace delete", &serde_json::json!({ "id": id })) {
        return Ok(());
    }
    client
        .delete(&format!("/workspaces/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "workspace delete", "Admin"))?;
    output::render_object(
        cli,
        &serde_json::json!({ "id": id, "status": "deleted" }),
        "status",
    );
    Ok(())
}
