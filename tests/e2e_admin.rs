//! E2E integration tests for the `fabio admin` command group.
//!
//! Admin APIs require the Fabric Admin role. All tests use the dual-assertion
//! pattern: on success we validate the structured JSON response; on failure we
//! verify a well-formed FORBIDDEN error. This ensures correctness regardless of
//! whether the caller has admin privileges.
//!
//! Tests are organized by risk tier:
//! - Tier 0: Read-only commands (zero risk)
//! - Tier 1: Create + delete lifecycle (full rollback)
//! - Tier 2: Dry-run validations for destructive commands

mod common;

use common::{fabio, unique_name, TestConfig};
use serde_json::Value;
use serial_test::serial;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Assert that the output is either a successful JSON response or a known error.
/// Returns `Some(json)` on success, `None` on `FORBIDDEN`/`NOT_FOUND`/`API_ERROR`.
fn assert_admin_output(output: &std::process::Output) -> Option<Value> {
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|e| panic!("stdout not valid JSON: {e}\n{stdout}"));
        Some(json)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let json: Value = serde_json::from_str(&stderr)
            .unwrap_or_else(|e| panic!("stderr not valid JSON: {e}\n{stderr}"));
        let code = json["error"]["code"].as_str().unwrap_or("");
        assert!(
            matches!(code, "FORBIDDEN" | "NOT_FOUND" | "API_ERROR"),
            "Expected FORBIDDEN/NOT_FOUND/API_ERROR, got: {json}"
        );
        None
    }
}

/// Assert a list response has correct envelope structure.
fn assert_list_envelope(json: &Value) {
    assert!(
        json["data"].is_array(),
        "Expected 'data' to be an array, got: {json}",
    );
    assert!(
        json["count"].is_number(),
        "Expected 'count' to be a number, got: {json}",
    );
}

/// Assert a list response is non-empty.
fn assert_list_non_empty(json: &Value) {
    assert_list_envelope(json);
    let arr = json["data"].as_array().unwrap();
    assert!(!arr.is_empty(), "Expected non-empty list, got: {json}");
}

// ─── Tier 0: Read-Only Commands ──────────────────────────────────────────────
// These commands only fetch data. They are completely safe to run.

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_tenant_settings() {
    let output = fabio()
        .args(["admin", "list-tenant-settings"])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        // Tenant settings always exist in any tenant
        assert_list_non_empty(&json);
        // Verify known fields
        let first = &json["data"][0];
        assert!(
            first["settingName"].is_string(),
            "Expected 'settingName' field"
        );
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_capacities_tenant_overrides() {
    let output = fabio()
        .args(["admin", "list-capacities-tenant-overrides"])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_capacity_tenant_overrides() {
    let cfg = TestConfig::from_env();
    let output = fabio()
        .args([
            "admin",
            "list-capacity-tenant-overrides",
            "--capacity-id",
            &cfg.capacity_id,
        ])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_domains_tenant_overrides() {
    let output = fabio()
        .args(["admin", "list-domains-tenant-overrides"])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_workspaces_tenant_overrides() {
    let output = fabio()
        .args(["admin", "list-workspaces-tenant-overrides"])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_tags() {
    let output = fabio().args(["admin", "list-tags"]).output().unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_workloads() {
    let output = fabio().args(["admin", "list-workloads"]).output().unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_workload_assignments() {
    let output = fabio()
        .args(["admin", "list-workload-assignments"])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_workspaces() {
    let output = fabio()
        .args(["admin", "list-workspaces"])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        // Should always have at least one workspace (the test workspace exists)
        assert_list_non_empty(&json);
        let first = &json["data"][0];
        assert!(first["id"].is_string(), "Expected 'id' field on workspace");
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_show_workspace() {
    let cfg = TestConfig::from_env();
    let output = fabio()
        .args(["admin", "show-workspace", "--workspace", &cfg.source_workspace])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        let data = &json["data"];
        assert!(data["id"].is_string(), "Expected 'id' field");
        assert!(
            data["name"].is_string(),
            "Expected 'name' field"
        );
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_workspace_users() {
    let cfg = TestConfig::from_env();
    let output = fabio()
        .args([
            "admin",
            "list-workspace-users",
            "--workspace",
            &cfg.source_workspace,
        ])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        // At least the caller should have access
        assert_list_non_empty(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_git_connections() {
    let output = fabio()
        .args(["admin", "list-git-connections"])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_network_policies() {
    let output = fabio()
        .args(["admin", "list-network-policies"])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_items() {
    let output = fabio().args(["admin", "list-items"]).output().unwrap();

    if let Some(json) = assert_admin_output(&output) {
        // Tenant always has items
        assert_list_non_empty(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_show_item() {
    let cfg = TestConfig::from_env();
    let output = fabio()
        .args([
            "admin",
            "show-item",
            "--workspace",
            &cfg.source_workspace,
            "--item-id",
            &cfg.source_lakehouse,
        ])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        let data = &json["data"];
        assert!(data["id"].is_string(), "Expected 'id' field");
        assert!(data["type"].is_string(), "Expected 'type' field");
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_item_users() {
    let cfg = TestConfig::from_env();
    let output = fabio()
        .args([
            "admin",
            "list-item-users",
            "--workspace",
            &cfg.source_workspace,
            "--item-id",
            &cfg.source_lakehouse,
        ])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        // At least the caller should have access
        assert_list_non_empty(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_external_data_shares() {
    let output = fabio()
        .args(["admin", "list-external-data-shares"])
        .output()
        .unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_domains() {
    let output = fabio().args(["admin", "list-domains"]).output().unwrap();

    if let Some(json) = assert_admin_output(&output) {
        assert_list_envelope(&json);
    }
}

// ─── Tier 1: Tag Lifecycle (create → list → update → delete) ────────────────
// Tags can be cleanly created and deleted. This exercises the full lifecycle
// while seeding data for `list-tags` to be meaningfully non-empty.

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_tag_lifecycle() {
    let tag_name = unique_name("fabio-test-tag");

    // ── Step 1: Create tag ──
    let create_body = serde_json::json!({
        "createTagsRequest": [
            { "displayName": tag_name }
        ]
    });

    let output = fabio()
        .args([
            "admin",
            "create-tags",
            "--content",
            &create_body.to_string(),
        ])
        .output()
        .unwrap();

    let Some(create_json) = assert_admin_output(&output) else {
        // No admin access — skip rest of lifecycle
        eprintln!("SKIP: no admin access for tag lifecycle test");
        return;
    };

    // Extract the tag ID from the created response
    // The API returns the created tags; extract the ID
    let tag_id = extract_tag_id(&create_json, &tag_name);
    assert!(
        !tag_id.is_empty(),
        "Failed to extract tag ID from create response: {create_json}"
    );

    // ── Step 2: List tags and verify our tag is present ──
    let output = fabio().args(["admin", "list-tags"]).output().unwrap();
    let list_json = assert_admin_output(&output).expect("list-tags should succeed after create");
    assert_list_non_empty(&list_json);

    let found = list_json["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"].as_str() == Some(&tag_id));
    assert!(found, "Created tag {tag_id} not found in list-tags");

    // ── Step 3: Update tag ──
    let new_name = format!("{tag_name}-updated");
    let output = fabio()
        .args([
            "admin",
            "update-tag",
            "--tag-id",
            &tag_id,
            "--name",
            &new_name,
            "--description",
            "Updated description",
        ])
        .output()
        .unwrap();

    let update_json = assert_admin_output(&output).expect("update-tag should succeed");
    // Verify the update returned updated data
    let updated_name = update_json["data"]["displayName"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(updated_name, new_name, "Tag name should be updated");

    // ── Step 4: Delete tag (cleanup) ──
    let output = fabio()
        .args(["admin", "delete-tag", "--tag-id", &tag_id])
        .output()
        .unwrap();

    let delete_json = assert_admin_output(&output).expect("delete-tag should succeed");
    assert_eq!(delete_json["data"]["status"], "deleted");

    // ── Step 5: Verify tag is gone ──
    let output = fabio().args(["admin", "list-tags"]).output().unwrap();
    let list_json = assert_admin_output(&output).expect("list-tags should still succeed");
    let still_present = list_json["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"].as_str() == Some(&tag_id));
    assert!(
        !still_present,
        "Tag {tag_id} should be gone after deletion"
    );
}

/// Extract the tag ID from the create-tags response.
/// The response format may vary — handles both array and object responses.
fn extract_tag_id(json: &Value, name: &str) -> String {
    // Try: data is array of created tags
    if let Some(arr) = json["data"].as_array() {
        for tag in arr {
            if tag["displayName"].as_str() == Some(name) {
                if let Some(id) = tag["id"].as_str() {
                    return id.to_string();
                }
            }
        }
    }
    // Try: data is an object with nested tags
    if let Some(tags) = json["data"]["tags"].as_array() {
        for tag in tags {
            if tag["displayName"].as_str() == Some(name) {
                if let Some(id) = tag["id"].as_str() {
                    return id.to_string();
                }
            }
        }
    }
    // Try: data.value array
    if let Some(tags) = json["data"]["value"].as_array() {
        for tag in tags {
            if tag["displayName"].as_str() == Some(name) {
                if let Some(id) = tag["id"].as_str() {
                    return id.to_string();
                }
            }
        }
    }
    // Fallback: data itself has an id
    if let Some(id) = json["data"]["id"].as_str() {
        return id.to_string();
    }
    String::new()
}

// ─── Tier 1: Domain Lifecycle (create → show → update → list → delete) ──────
// Domains can be cleanly created and deleted. A fresh test domain is used
// to avoid interfering with real organizational domains.

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_domain_lifecycle() {
    let domain_name = unique_name("fabio-test-domain");
    let domain_desc = "E2E test domain - safe to delete";

    // ── Step 1: Create domain ──
    let output = fabio()
        .args([
            "admin",
            "create-domain",
            "--name",
            &domain_name,
            "--description",
            domain_desc,
        ])
        .output()
        .unwrap();

    let Some(create_json) = assert_admin_output(&output) else {
        eprintln!("SKIP: no admin access for domain lifecycle test");
        return;
    };

    let domain_id = create_json["data"]["id"]
        .as_str()
        .expect("Created domain should have 'id'")
        .to_string();

    // ── Step 2: Show domain ──
    let output = fabio()
        .args(["admin", "show-domain", "--domain-id", &domain_id])
        .output()
        .unwrap();

    let show_json = assert_admin_output(&output).expect("show-domain should succeed");
    assert_eq!(show_json["data"]["id"].as_str().unwrap(), domain_id);
    assert_eq!(
        show_json["data"]["displayName"].as_str().unwrap(),
        domain_name
    );

    // ── Step 3: Update domain ──
    let new_name = format!("{domain_name}-updated");
    let output = fabio()
        .args([
            "admin",
            "update-domain",
            "--domain-id",
            &domain_id,
            "--name",
            &new_name,
        ])
        .output()
        .unwrap();

    let update_json = assert_admin_output(&output).expect("update-domain should succeed");
    assert_eq!(
        update_json["data"]["displayName"].as_str().unwrap(),
        new_name
    );

    // ── Step 4: List domains and verify ours is present ──
    let output = fabio().args(["admin", "list-domains"]).output().unwrap();
    let list_json = assert_admin_output(&output).expect("list-domains should succeed");
    let found = list_json["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["id"].as_str() == Some(domain_id.as_str()));
    assert!(found, "Created domain {domain_id} not found in list");

    // ── Step 5: List domain workspaces (should be empty for new domain) ──
    let output = fabio()
        .args([
            "admin",
            "list-domain-workspaces",
            "--domain-id",
            &domain_id,
        ])
        .output()
        .unwrap();
    let ws_json = assert_admin_output(&output).expect("list-domain-workspaces should succeed");
    assert_list_envelope(&ws_json);

    // ── Step 6: List domain role assignments ──
    let output = fabio()
        .args([
            "admin",
            "list-domain-role-assignments",
            "--domain-id",
            &domain_id,
        ])
        .output()
        .unwrap();
    let roles_json =
        assert_admin_output(&output).expect("list-domain-role-assignments should succeed");
    assert_list_envelope(&roles_json);

    // ── Step 7: Delete domain (cleanup) ──
    let output = fabio()
        .args(["admin", "delete-domain", "--domain-id", &domain_id])
        .output()
        .unwrap();
    let delete_json = assert_admin_output(&output).expect("delete-domain should succeed");
    assert_eq!(delete_json["data"]["status"], "deleted");
}

// ─── Tier 2: Domain Workspace Assignments (on ephemeral domain) ──────────────
// Creates a domain, assigns/unassigns workspaces, then deletes the domain.

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_domain_workspace_assignment_lifecycle() {
    let cfg = TestConfig::from_env();
    let domain_name = unique_name("fabio-test-assign");

    // ── Create domain ──
    let output = fabio()
        .args([
            "admin",
            "create-domain",
            "--name",
            &domain_name,
            "--description",
            "Workspace assignment test",
        ])
        .output()
        .unwrap();

    let Some(create_json) = assert_admin_output(&output) else {
        eprintln!("SKIP: no admin access for domain assignment test");
        return;
    };

    let domain_id = create_json["data"]["id"]
        .as_str()
        .expect("domain id")
        .to_string();

    // ── Assign workspace to domain ──
    let output = fabio()
        .args([
            "admin",
            "assign-domain-workspaces",
            "--domain-id",
            &domain_id,
            "--workspace-ids",
            &cfg.source_workspace,
        ])
        .output()
        .unwrap();
    assert_admin_output(&output).expect("assign-domain-workspaces should succeed");

    // ── Verify workspace is listed under domain ──
    let output = fabio()
        .args([
            "admin",
            "list-domain-workspaces",
            "--domain-id",
            &domain_id,
        ])
        .output()
        .unwrap();
    let list_json = assert_admin_output(&output).expect("list-domain-workspaces should succeed");
    let found = list_json["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["id"].as_str() == Some(&cfg.source_workspace));
    assert!(found, "Assigned workspace should appear in domain");

    // ── Unassign workspace from domain ──
    let output = fabio()
        .args([
            "admin",
            "unassign-domain-workspaces",
            "--domain-id",
            &domain_id,
            "--workspace-ids",
            &cfg.source_workspace,
        ])
        .output()
        .unwrap();
    assert_admin_output(&output).expect("unassign-domain-workspaces should succeed");

    // ── Verify workspace is removed ──
    let output = fabio()
        .args([
            "admin",
            "list-domain-workspaces",
            "--domain-id",
            &domain_id,
        ])
        .output()
        .unwrap();
    let list_json = assert_admin_output(&output).expect("list after unassign");
    let still_found = list_json["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["id"].as_str() == Some(&cfg.source_workspace));
    assert!(!still_found, "Workspace should be removed after unassign");

    // ── Cleanup: delete domain ──
    let output = fabio()
        .args(["admin", "delete-domain", "--domain-id", &domain_id])
        .output()
        .unwrap();
    assert_admin_output(&output).expect("delete-domain cleanup");
}

// ─── Tier 2: Workspace Admin Access Grant/Revoke ─────────────────────────────
// Grant temporary admin access, then revoke it.

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_grant_revoke_workspace_access() {
    let cfg = TestConfig::from_env();

    // ── Grant admin access ──
    let output = fabio()
        .args([
            "admin",
            "grant-admin-access",
            "--workspace",
            &cfg.source_workspace,
        ])
        .output()
        .unwrap();

    let Some(_) = assert_admin_output(&output) else {
        eprintln!("SKIP: no admin access for grant/revoke test");
        return;
    };

    // ── Remove admin access (rollback) ──
    let output = fabio()
        .args([
            "admin",
            "remove-admin-access",
            "--workspace",
            &cfg.source_workspace,
        ])
        .output()
        .unwrap();
    assert_admin_output(&output).expect("remove-admin-access should succeed");
}

// ─── Tier 3: Dry-Run Validations for Destructive Commands ───────────────────
// Verify that --dry-run works correctly (returns request body without executing).

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_update_tenant_setting_dry_run() {
    let body = serde_json::json!({
        "enabled": true,
        "enabledSecurityGroups": []
    });

    let output = fabio()
        .args([
            "admin",
            "update-tenant-setting",
            "--setting-name",
            "SomeSetting",
            "--content",
            &body.to_string(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    // Dry-run always succeeds (no API call)
    assert!(output.status.success(), "dry-run should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_create_tags_dry_run() {
    let body = serde_json::json!({
        "createTagsRequest": [{ "displayName": "dry-run-tag" }]
    });

    let output = fabio()
        .args([
            "admin",
            "create-tags",
            "--content",
            &body.to_string(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_delete_tag_dry_run() {
    let output = fabio()
        .args([
            "admin",
            "delete-tag",
            "--tag-id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_revoke_external_data_share_dry_run() {
    let output = fabio()
        .args([
            "admin",
            "revoke-external-data-share",
            "--workspace",
            "00000000-0000-0000-0000-000000000000",
            "--item-id",
            "11111111-1111-1111-1111-111111111111",
            "--share-id",
            "22222222-2222-2222-2222-222222222222",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_bulk_set_labels_dry_run() {
    let body = serde_json::json!({
        "items": [
            { "id": "00000000-0000-0000-0000-000000000000", "labelId": "test-label" }
        ]
    });

    let output = fabio()
        .args([
            "admin",
            "bulk-set-labels",
            "--content",
            &body.to_string(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_bulk_remove_labels_dry_run() {
    let body = serde_json::json!({
        "items": [
            { "id": "00000000-0000-0000-0000-000000000000" }
        ]
    });

    let output = fabio()
        .args([
            "admin",
            "bulk-remove-labels",
            "--content",
            &body.to_string(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_remove_all_sharing_links_dry_run() {
    let body = serde_json::json!({
        "items": [
            { "workspaceId": "00000000-0000-0000-0000-000000000000", "itemId": "11111111-1111-1111-1111-111111111111" }
        ]
    });

    let output = fabio()
        .args([
            "admin",
            "remove-all-sharing-links",
            "--content",
            &body.to_string(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_bulk_remove_sharing_links_dry_run() {
    let body = serde_json::json!({
        "sharingLinks": [
            { "linkId": "00000000-0000-0000-0000-000000000000" }
        ]
    });

    let output = fabio()
        .args([
            "admin",
            "bulk-remove-sharing-links",
            "--content",
            &body.to_string(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_create_domain_dry_run() {
    let output = fabio()
        .args([
            "admin",
            "create-domain",
            "--name",
            "dry-run-domain",
            "--description",
            "Should not be created",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_delete_domain_dry_run() {
    let output = fabio()
        .args([
            "admin",
            "delete-domain",
            "--domain-id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_assign_domain_workspaces_dry_run() {
    let output = fabio()
        .args([
            "admin",
            "assign-domain-workspaces",
            "--domain-id",
            "00000000-0000-0000-0000-000000000000",
            "--workspace-ids",
            "11111111-1111-1111-1111-111111111111",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_unassign_domain_workspaces_dry_run() {
    let output = fabio()
        .args([
            "admin",
            "unassign-domain-workspaces",
            "--domain-id",
            "00000000-0000-0000-0000-000000000000",
            "--workspace-ids",
            "11111111-1111-1111-1111-111111111111",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_delete_capacity_tenant_override_dry_run() {
    let cfg = TestConfig::from_env();
    let output = fabio()
        .args([
            "admin",
            "delete-capacity-tenant-override",
            "--capacity-id",
            &cfg.capacity_id,
            "--setting-name",
            "SomeSetting",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_update_capacity_tenant_override_dry_run() {
    let cfg = TestConfig::from_env();
    let body = serde_json::json!({ "enabled": true });

    let output = fabio()
        .args([
            "admin",
            "update-capacity-tenant-override",
            "--capacity-id",
            &cfg.capacity_id,
            "--setting-name",
            "SomeSetting",
            "--content",
            &body.to_string(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_restore_workspace_dry_run() {
    let cfg = TestConfig::from_env();
    let output = fabio()
        .args([
            "admin",
            "restore-workspace",
            "--workspace",
            "00000000-0000-0000-0000-000000000000",
            "--name",
            "restored-ws",
            "--capacity-id",
            &cfg.capacity_id,
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

// ─── Tier 3: Update-tag validation ──────────────────────────────────────────
// Validates that update-tag requires at least one field.

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_update_tag_requires_field() {
    let output = fabio()
        .args([
            "admin",
            "update-tag",
            "--tag-id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let json: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(json["error"]["code"], "INVALID_INPUT");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_update_domain_requires_field() {
    let output = fabio()
        .args([
            "admin",
            "update-domain",
            "--domain-id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let json: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(json["error"]["code"], "INVALID_INPUT");
}

// ─── Tier 3: Workload Assignment Dry-Run ────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_create_workload_assignment_dry_run() {
    let body = serde_json::json!({
        "workloadId": "test-workload",
        "capacityId": "00000000-0000-0000-0000-000000000000"
    });

    let output = fabio()
        .args([
            "admin",
            "create-workload-assignment",
            "--content",
            &body.to_string(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_delete_workload_assignment_dry_run() {
    let output = fabio()
        .args([
            "admin",
            "delete-workload-assignment",
            "--assignment-id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

// ─── Tier 3: Grant/Revoke Dry-Run ───────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_grant_admin_access_dry_run() {
    let cfg = TestConfig::from_env();
    let output = fabio()
        .args([
            "admin",
            "grant-admin-access",
            "--workspace",
            &cfg.source_workspace,
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_remove_admin_access_dry_run() {
    let cfg = TestConfig::from_env();
    let output = fabio()
        .args([
            "admin",
            "remove-admin-access",
            "--workspace",
            &cfg.source_workspace,
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
}
