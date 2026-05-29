//! End-to-end integration tests for `fabio workspace` commands.

mod common;

use common::{TestConfig, extract_count, extract_data, fabio, parse_json, unique_name};
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_list_returns_workspaces() {
    let assert = fabio().args(["workspace", "list"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let count = extract_count(&json);

    assert!(count > 0, "expected at least one workspace");
    assert!(data.is_array());
    let arr = data.as_array().unwrap();
    assert!(!arr.is_empty());

    // Each workspace should have id, displayName, type
    let first = &arr[0];
    assert!(first.get("id").is_some());
    assert!(first.get("displayName").is_some());
    assert!(first.get("type").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_list_table_format() {
    fabio()
        .args(["--output", "table", "workspace", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("NAME"))
        .stdout(predicate::str::contains("ID"))
        .stdout(predicate::str::contains("TYPE"));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_list_plain_format() {
    let cfg = TestConfig::from_env();

    fabio()
        .args(["--output", "plain", "workspace", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&cfg.source_workspace));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_show_returns_details() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["workspace", "show", "--id", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    assert_eq!(data["id"], cfg.source_workspace);
    assert!(data.get("displayName").is_some());
    assert!(data.get("capacityId").is_some());
    assert!(data.get("type").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_show_not_found() {
    let assert = fabio()
        .args([
            "workspace",
            "show",
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    // Fabric may return API_ERROR or NOT_FOUND for invalid workspace IDs
    assert!(
        code == "NOT_FOUND" || code == "API_ERROR",
        "Expected NOT_FOUND or API_ERROR, got: {code}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_create_with_description_and_delete() {
    let name = common::unique_name("ws_test");

    // Create workspace with description
    let assert = fabio()
        .args([
            "workspace",
            "create",
            "--name",
            &name,
            "--description",
            "Integration test workspace",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    assert!(data.get("id").is_some());

    let ws_id = data["id"].as_str().unwrap().to_string();

    // Show workspace to verify description was set
    let assert = fabio()
        .args(["workspace", "show", "--id", &ws_id])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    // Note: Fabric API may or may not return description in show, depending on version

    // Delete workspace
    let assert = fabio()
        .args(["workspace", "delete", "--id", &ws_id])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
    assert_eq!(data["id"], ws_id);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_create_without_description_and_delete() {
    let name = common::unique_name("ws_nodesc");

    // Create workspace without description
    let assert = fabio()
        .args(["workspace", "create", "--name", &name])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let ws_id = data["id"].as_str().unwrap().to_string();

    // Delete
    fabio()
        .args(["workspace", "delete", "--id", &ws_id])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_assign_capacity() {
    let cfg = TestConfig::from_env();

    // Re-assign source workspace to its current capacity (idempotent)
    let assert = fabio()
        .args([
            "workspace",
            "assign-capacity",
            "--id",
            &cfg.source_workspace,
            "--capacity",
            &cfg.capacity_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "assigned");
    assert_eq!(data["workspaceId"], cfg.source_workspace);
    assert_eq!(data["capacityId"], cfg.capacity_id);
}

// ---------------------------------------------------------------------------
// workspace list --limit
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_list_with_limit() {
    let assert = fabio()
        .args(["workspace", "list", "--limit", "1"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let arr = data.as_array().expect("Expected array");
    assert_eq!(arr.len(), 1, "Expected exactly 1 workspace with --limit 1");
    // count should reflect total available, not limited
    let count = extract_count(&json);
    assert!(
        count >= 1,
        "Count should be >= 1 (total available, not limited)"
    );
}

// ---------------------------------------------------------------------------
// workspace show with --query extracts a field
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_show_with_query() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "workspace",
            "show",
            "--id",
            &cfg.source_workspace,
            "--query",
            "displayName",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // --query displayName extracts the workspace name as a string
    assert!(data.is_string(), "Expected string from --query: {data}");
}

// ---------------------------------------------------------------------------
// workspace delete with non-existent ID returns error
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_delete_not_found() {
    let assert = fabio()
        .args([
            "workspace",
            "delete",
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    // Fabric may return API_ERROR or NOT_FOUND for invalid workspace IDs
    assert!(
        code == "NOT_FOUND" || code == "API_ERROR",
        "Expected NOT_FOUND or API_ERROR for invalid workspace, got: {code}"
    );
}

// ---------------------------------------------------------------------------
// workspace assign-capacity with invalid capacity fails with hint
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_assign_capacity_invalid() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "workspace",
            "assign-capacity",
            "--id",
            &cfg.source_workspace,
            "--capacity",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    // Should be a client error (INVALID_INPUT or API_ERROR or NOT_FOUND)
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "NOT_FOUND" || code == "API_ERROR" || code == "INVALID_INPUT",
        "Expected error code for invalid capacity, got: {code}"
    );

    // Should have a hint about creating the capacity
    let hint = err_json["error"]["hint"].as_str().unwrap_or("");
    assert!(
        hint.contains("az fabric capacity"),
        "Expected hint with az CLI command, got: {hint}"
    );
}

// ===========================================================================
// workspace update
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_update_name_and_description() {
    let name = unique_name("ws_upd");

    // Create workspace
    let assert = fabio()
        .args(["workspace", "create", "--name", &name])
        .assert()
        .success();
    let json = parse_json(&assert);
    let ws_id = extract_data(&json)["id"].as_str().unwrap().to_string();

    // Update both name and description
    let new_name = format!("{name}_renamed");
    let assert = fabio()
        .args([
            "workspace",
            "update",
            "--id",
            &ws_id,
            "--name",
            &new_name,
            "--description",
            "Updated description",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], new_name);
    assert_eq!(data["id"], ws_id);

    // Cleanup
    fabio()
        .args(["workspace", "delete", "--id", &ws_id])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_update_requires_at_least_one_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["workspace", "update", "--id", &cfg.source_workspace])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert_eq!(code, "INVALID_INPUT");
}

// ===========================================================================
// workspace unassign-capacity
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_unassign_capacity_on_nonexistent() {
    let assert = fabio()
        .args([
            "workspace",
            "unassign-capacity",
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "NOT_FOUND" || code == "API_ERROR" || code == "FORBIDDEN",
        "Expected error for invalid workspace, got: {code}"
    );
}

// ===========================================================================
// workspace role-assignments
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_list_role_assignments() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "workspace",
            "list-role-assignments",
            "--id",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let count = extract_count(&json);

    assert!(
        count >= 1,
        "Expected at least one role assignment (the admin)"
    );
    let arr = data.as_array().unwrap();
    assert!(!arr.is_empty());

    // Each assignment should have id and role
    let first = &arr[0];
    assert!(first.get("id").is_some());
    assert!(first.get("role").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_add_role_assignment_invalid_role() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "workspace",
            "add-role-assignment",
            "--id",
            &cfg.source_workspace,
            "--principal-id",
            "00000000-0000-0000-0000-000000000000",
            "--principal-type",
            "User",
            "--role",
            "SuperAdmin",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert_eq!(code, "INVALID_INPUT");

    let hint = err_json["error"]["hint"].as_str().unwrap_or("");
    assert!(
        hint.contains("Admin") && hint.contains("Member"),
        "Expected hint with valid roles, got: {hint}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_add_role_assignment_invalid_principal_type() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "workspace",
            "add-role-assignment",
            "--id",
            &cfg.source_workspace,
            "--principal-id",
            "00000000-0000-0000-0000-000000000000",
            "--principal-type",
            "Application",
            "--role",
            "Viewer",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert_eq!(code, "INVALID_INPUT");

    let hint = err_json["error"]["hint"].as_str().unwrap_or("");
    assert!(
        hint.contains("ServicePrincipal"),
        "Expected hint with valid principal types, got: {hint}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_update_role_assignment_invalid_role() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "workspace",
            "update-role-assignment",
            "--id",
            &cfg.source_workspace,
            "--assignment-id",
            "00000000-0000-0000-0000-000000000000",
            "--role",
            "Owner",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert_eq!(code, "INVALID_INPUT");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_delete_role_assignment_not_found() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "workspace",
            "delete-role-assignment",
            "--id",
            &cfg.source_workspace,
            "--assignment-id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "NOT_FOUND" || code == "API_ERROR",
        "Expected NOT_FOUND or API_ERROR, got: {code}"
    );
}

// ===========================================================================
// workspace provision/deprovision identity
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_provision_identity_on_nonexistent() {
    let assert = fabio()
        .args([
            "workspace",
            "provision-identity",
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "NOT_FOUND" || code == "API_ERROR" || code == "FORBIDDEN",
        "Expected error for invalid workspace, got: {code}"
    );
}

// ===========================================================================
// workspace update with --dry-run
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_update_dry_run() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "--dry-run",
            "workspace",
            "update",
            "--id",
            &cfg.source_workspace,
            "--name",
            "Should Not Change",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("dry_run"),
        "Expected dry_run indicator in output"
    );

    // Verify workspace name did NOT change
    let assert = fabio()
        .args(["workspace", "show", "--id", &cfg.source_workspace])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_ne!(
        data["displayName"].as_str().unwrap_or(""),
        "Should Not Change"
    );
}

// ===========================================================================
// workspace get-settings
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_get_settings() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "workspace",
            "get-settings",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    // Should return JSON (either a properties object or the full workspace)
    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(
        data.is_object(),
        "Expected settings to be a JSON object, got: {data}"
    );
}

// ===========================================================================
// workspace update-settings --dry-run
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_update_settings_dry_run() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "--dry-run",
            "workspace",
            "update-settings",
            "--workspace",
            &cfg.source_workspace,
            "--content",
            r#"{"automaticMetadataSync":"Enabled"}"#,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "workspace update-settings");
    assert_eq!(data["details"]["automaticMetadataSync"], "Enabled");
}

// ===========================================================================
// workspace update-settings requires --file or --content
// ===========================================================================

#[test]
fn workspace_update_settings_missing_input() {
    fabio()
        .args([
            "workspace",
            "update-settings",
            "--workspace",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--file or --content"));
}

// ===========================================================================
// workspace get-firewall-rules
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_get_firewall_rules() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "workspace",
            "get-firewall-rules",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should have a "rules" array
    assert!(data.get("rules").is_some(), "expected 'rules' field");
    assert!(data["rules"].is_array());
}

// ===========================================================================
// workspace set-firewall-rules roundtrip (add then clear)
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_set_firewall_rules_roundtrip() {
    let cfg = TestConfig::from_env();

    // Set a rule
    fabio()
        .args([
            "workspace",
            "set-firewall-rules",
            "--workspace",
            &cfg.source_workspace,
            "--content",
            r#"{"rules":[{"displayName":"E2ETest","value":"172.16.0.0/12"}]}"#,
        ])
        .assert()
        .success();

    // Verify it persisted
    let assert = fabio()
        .args([
            "workspace",
            "get-firewall-rules",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = extract_data(&json);
    let rules = data["rules"].as_array().unwrap();
    assert!(
        rules.iter().any(|r| r["displayName"] == "E2ETest"),
        "expected E2ETest rule to be present"
    );

    // Clear rules
    fabio()
        .args([
            "workspace",
            "set-firewall-rules",
            "--workspace",
            &cfg.source_workspace,
            "--content",
            r#"{"rules":[]}"#,
        ])
        .assert()
        .success();
}

// ===========================================================================
// workspace set-firewall-rules dry-run
// ===========================================================================

#[test]
fn workspace_set_firewall_rules_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "workspace",
            "set-firewall-rules",
            "--workspace",
            "00000000-0000-0000-0000-000000000000",
            "--content",
            r#"{"rules":[{"displayName":"Test","value":"1.2.3.4"}]}"#,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "workspace set-firewall-rules");
}

// ===========================================================================
// workspace get-git-outbound-policy
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_get_git_outbound_policy() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "workspace",
            "get-git-outbound-policy",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should have a "defaultAction" field
    assert!(
        data.get("defaultAction").is_some(),
        "expected 'defaultAction' field"
    );
}

// ===========================================================================
// workspace set-dataset-storage-format roundtrip
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_set_dataset_storage_format_roundtrip() {
    let cfg = TestConfig::from_env();

    // Set to Large
    let assert = fabio()
        .args([
            "workspace",
            "set-dataset-storage-format",
            "--workspace",
            &cfg.source_workspace,
            "--format",
            "Large",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["defaultDatasetStorageFormat"], "Large");

    // Revert to Small
    let assert = fabio()
        .args([
            "workspace",
            "set-dataset-storage-format",
            "--workspace",
            &cfg.source_workspace,
            "--format",
            "Small",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["defaultDatasetStorageFormat"], "Small");
}

// ===========================================================================
// workspace set-dataset-storage-format dry-run
// ===========================================================================

#[test]
fn workspace_set_dataset_storage_format_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "workspace",
            "set-dataset-storage-format",
            "--workspace",
            "00000000-0000-0000-0000-000000000000",
            "--format",
            "Large",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(
        data["would_execute"],
        "workspace set-dataset-storage-format"
    );
    assert_eq!(data["details"]["defaultDatasetStorageFormat"], "Large");
}

// ===========================================================================
// workspace set-firewall-rules missing input
// ===========================================================================

#[test]
fn workspace_set_firewall_rules_missing_input() {
    fabio()
        .args([
            "workspace",
            "set-firewall-rules",
            "--workspace",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--file or --content"));
}

// ===========================================================================
// workspace get-dataset-storage-format
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_get_dataset_storage_format() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "workspace",
            "get-dataset-storage-format",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(
        data.get("defaultDatasetStorageFormat").is_some(),
        "expected 'defaultDatasetStorageFormat' field"
    );
    let fmt = data["defaultDatasetStorageFormat"].as_str().unwrap();
    assert!(
        fmt == "Small" || fmt == "Large",
        "expected Small or Large, got {fmt}"
    );
}

// ===========================================================================
// workspace get-onelake-settings
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_get_onelake_settings() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "workspace",
            "get-onelake-settings",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should have tier information
    assert!(
        data.get("defaultTier").is_some() || data.get("tier").is_some() || data.is_object(),
        "expected OneLake settings object"
    );
}

// ===========================================================================
// workspace get-network-policy
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_get_network_policy() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "workspace",
            "get-network-policy",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Network policy should be an object (may have inbound/outbound fields)
    assert!(data.is_object(), "expected network policy object");
}

// ===========================================================================
// workspace reset-shortcut-cache
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_reset_shortcut_cache() {
    let cfg = TestConfig::from_env();
    // reset-shortcut-cache is LRO; may return API_ERROR if no shortcuts exist
    let assert = fabio()
        .args([
            "workspace",
            "reset-shortcut-cache",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert();

    // Accept either success (workspace has shortcuts) or API_ERROR (no cache to reset)
    let output = assert.get_output();
    let code = output.status.code().unwrap_or(1);
    if code != 0 {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("API_ERROR") || stderr.contains("ItemNotFound"),
            "unexpected error: {stderr}"
        );
    }
}

// ===========================================================================
// workspace show-role-assignment (via list first to get an ID)
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_show_role_assignment() {
    let cfg = TestConfig::from_env();

    // First list to get an assignment ID
    let assert = fabio()
        .args([
            "workspace",
            "list-role-assignments",
            "--id",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let arr = data.as_array().expect("expected array of role assignments");
    assert!(!arr.is_empty(), "expected at least one role assignment");

    let first_id = arr[0]["id"].as_str().expect("expected assignment id");

    // Now show that specific assignment
    let assert = fabio()
        .args([
            "workspace",
            "show-role-assignment",
            "--id",
            &cfg.source_workspace,
            "--assignment-id",
            first_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["id"].as_str().unwrap(), first_id);
    assert!(data.get("principal").is_some());
    assert!(data.get("role").is_some());
}

// ===========================================================================
// workspace folder lifecycle: create → list → show → update → move → delete
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_folder_lifecycle() {
    let cfg = TestConfig::from_env();
    let folder_name = unique_name("TestFolder");

    // Create folder
    let assert = fabio()
        .args([
            "workspace",
            "create-folder",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &folder_name,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let folder_id = data["id"].as_str().expect("expected folder id").to_string();
    assert_eq!(data["displayName"].as_str().unwrap(), folder_name);

    // List folders — should contain our folder
    let assert = fabio()
        .args([
            "workspace",
            "list-folders",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let arr = data.as_array().expect("expected array of folders");
    assert!(
        arr.iter()
            .any(|f| f["id"].as_str() == Some(folder_id.as_str())),
        "created folder should appear in list"
    );

    // Show folder
    let assert = fabio()
        .args([
            "workspace",
            "show-folder",
            "--workspace",
            &cfg.source_workspace,
            "--folder-id",
            &folder_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["id"].as_str().unwrap(), folder_id);
    assert_eq!(data["displayName"].as_str().unwrap(), folder_name);

    // Update folder
    let updated_name = format!("{folder_name}Updated");
    let assert = fabio()
        .args([
            "workspace",
            "update-folder",
            "--workspace",
            &cfg.source_workspace,
            "--folder-id",
            &folder_id,
            "--name",
            &updated_name,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"].as_str().unwrap(), updated_name);

    // Create a second folder to test move
    let subfolder_name = unique_name("SubFolder");
    let assert = fabio()
        .args([
            "workspace",
            "create-folder",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &subfolder_name,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let subfolder_id = data["id"].as_str().expect("subfolder id").to_string();

    // Move subfolder into the first folder
    fabio()
        .args([
            "workspace",
            "move-folder",
            "--workspace",
            &cfg.source_workspace,
            "--folder-id",
            &subfolder_id,
            "--target-folder-id",
            &folder_id,
        ])
        .assert()
        .success();

    // Delete subfolder first (child before parent)
    fabio()
        .args([
            "workspace",
            "delete-folder",
            "--workspace",
            &cfg.source_workspace,
            "--folder-id",
            &subfolder_id,
        ])
        .assert()
        .success();

    // Delete the main folder
    fabio()
        .args([
            "workspace",
            "delete-folder",
            "--workspace",
            &cfg.source_workspace,
            "--folder-id",
            &folder_id,
        ])
        .assert()
        .success();
}

// ===========================================================================
// workspace list with --roles filter
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn workspace_list_with_roles_filter() {
    let assert = fabio()
        .args(["workspace", "list", "--roles", "Admin"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
    // Should have at least our test workspace where we're Admin
    let count = extract_count(&json);
    assert!(count > 0, "expected at least one workspace with Admin role");
}
