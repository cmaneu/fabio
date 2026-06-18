use anyhow::Result;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::enrich_admin;
use crate::output;

use super::read_body;

pub(super) async fn list_tenant_settings(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/tenantsettings",
            "tenantSettings",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["settingName", "title", "enabled"],
        &["SETTING", "TITLE", "ENABLED"],
        "settingName",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn update_tenant_setting(
    cli: &Cli,
    client: &FabricClient,
    setting_name: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "update-tenant-setting")?;

    if output::dry_run_guard(cli, "admin update-tenant-setting", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/admin/tenantsettings/{setting_name}/update"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_admin(e, "admin update-tenant-setting"))?;
    output::render_object(cli, &data, "settingName");
    Ok(())
}

pub(super) async fn list_capacities_tenant_overrides(
    cli: &Cli,
    client: &FabricClient,
) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/capacities/delegatedTenantSettingOverrides",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["capacityId", "tenantSettingName"],
        &["CAPACITY", "SETTING"],
        "capacityId",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn list_capacity_tenant_overrides(
    cli: &Cli,
    client: &FabricClient,
    capacity_id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/admin/capacities/{capacity_id}/delegatedTenantSettingOverrides"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["tenantSettingName", "enabled"],
        &["SETTING", "ENABLED"],
        "tenantSettingName",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn delete_capacity_tenant_override(
    cli: &Cli,
    client: &FabricClient,
    capacity_id: &str,
    setting_name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "admin delete-capacity-tenant-override",
        &serde_json::json!({ "capacityId": capacity_id, "settingName": setting_name }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!(
            "/admin/capacities/{capacity_id}/delegatedTenantSettingOverrides/{setting_name}"
        ))
        .await
        .map_err(|e| enrich_admin(e, "admin delete-capacity-tenant-override"))?;

    let obj = serde_json::json!({ "capacityId": capacity_id, "settingName": setting_name, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

pub(super) async fn update_capacity_tenant_override(
    cli: &Cli,
    client: &FabricClient,
    capacity_id: &str,
    setting_name: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_body(file, content, "update-capacity-tenant-override")?;

    if output::dry_run_guard(cli, "admin update-capacity-tenant-override", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!(
                "/admin/capacities/{capacity_id}/delegatedTenantSettingOverrides/{setting_name}/update"
            ),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_admin(e, "admin update-capacity-tenant-override"))?;
    output::render_object(cli, &data, "tenantSettingName");
    Ok(())
}

pub(super) async fn list_domains_tenant_overrides(cli: &Cli, client: &FabricClient) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/domains/delegatedTenantSettingOverrides",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["domainId", "tenantSettingName"],
        &["DOMAIN", "SETTING"],
        "domainId",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

pub(super) async fn list_workspaces_tenant_overrides(
    cli: &Cli,
    client: &FabricClient,
) -> Result<()> {
    let resp = client
        .get_list(
            "/admin/workspaces/delegatedTenantSettingOverrides",
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["workspaceId", "tenantSettingName"],
        &["WORKSPACE", "SETTING"],
        "workspaceId",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}
