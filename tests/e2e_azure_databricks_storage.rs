//! End-to-end integration tests for `fabio azure-databricks-storage` commands.

use assert_cmd::Command;
use serial_test::serial;

mod common;
use common::TestConfig;

fn fabio() -> Command {
    Command::cargo_bin("fabio").unwrap()
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_list_returns_array() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_dry_run_create() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "test-ads",
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        json["data"]["would_execute"],
        "azure-databricks-storage create"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_dry_run_delete() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        json["data"]["would_execute"],
        "azure-databricks-storage delete"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_dry_run_delete_hard() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
            "--hard-delete",
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        json["data"]["would_execute"],
        "azure-databricks-storage delete"
    );
    assert_eq!(json["data"]["details"]["hardDelete"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_update_requires_field() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("INVALID_INPUT")
            || stderr.contains("--name")
            || stderr.contains("--description"),
        "Expected error about missing fields, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_dry_run_update_definition() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
            "--content",
            r#"{"key":"value"}"#,
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        json["data"]["would_execute"],
        "azure-databricks-storage update-definition"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_create_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("ads_test");

    // Create (use source_workspace — feature may not be enabled on all workspaces)
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_mins(2))
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let item_id = json["data"]["id"].as_str().unwrap().to_string();
    assert!(!item_id.is_empty());

    // Delete
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &item_id,
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["status"], "deleted");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_dry_run_update() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
            "--name",
            "new-name",
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        json["data"]["would_execute"],
        "azure-databricks-storage update"
    );
    assert_eq!(json["data"]["details"]["displayName"], "new-name");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_show_invalid_id_returns_error() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-ffffffffffff",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    // Should return NOT_FOUND or API_ERROR for non-existent item
    assert!(
        stderr.contains("NOT_FOUND") || stderr.contains("API_ERROR") || stderr.contains("error"),
        "Expected error for invalid ID, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_get_definition_invalid_id_returns_error() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "get-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-ffffffffffff",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("NOT_FOUND") || stderr.contains("API_ERROR") || stderr.contains("error"),
        "Expected error for invalid ID, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn azure_databricks_storage_update_definition_requires_input() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "azure-databricks-storage",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("INVALID_INPUT")
            || stderr.contains("--file")
            || stderr.contains("--content"),
        "Expected error about missing input, got: {stderr}"
    );
}
