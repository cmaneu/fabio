use anyhow::Result;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_admin};
use crate::output;

use super::read_body;

pub(super) async fn list_tags(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/tags",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "displayName", "description"],
        &["ID", "NAME", "DESCRIPTION"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn create_tags(
    cli: &Cli,
    client: &FabricClient,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "create-tags")?;

    if output::dry_run_guard(cli, "admin create-tags", &body) {
        return Ok(());
    }

    let data = client
        .post("/admin/tags/bulkCreateTags", &body, false)
        .await
        .map_err(|e| enrich_admin(e, "admin create-tags"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

pub(super) async fn update_tag(
    cli: &Cli,
    client: &FabricClient,
    tag_id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --name or --description must be provided".to_string(),
            "Example: fabio admin update-tag --tag-id <ID> --name \"New Name\"".to_string(),
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

    if output::dry_run_guard(cli, "admin update-tag", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/admin/tags/{tag_id}"), &body)
        .await
        .map_err(|e| enrich_admin(e, "admin update-tag"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

pub(super) async fn delete_tag(cli: &Cli, client: &FabricClient, tag_id: &str) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "admin delete-tag",
        &serde_json::json!({ "tagId": tag_id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/admin/tags/{tag_id}"))
        .await
        .map_err(|e| enrich_admin(e, "admin delete-tag"))?;

    let obj = serde_json::json!({ "tagId": tag_id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}
