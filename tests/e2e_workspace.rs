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
    fabio()
        .args([
            "workspace",
            "show",
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NOT_FOUND"));
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
