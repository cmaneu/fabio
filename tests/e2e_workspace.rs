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
