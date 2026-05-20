//! End-to-end integration tests for `fabio workspace` commands.

mod common;

use common::{TestConfig, extract_count, extract_data, fabio, parse_json};
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
// workspace assign-capacity with invalid capacity fails
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
}
