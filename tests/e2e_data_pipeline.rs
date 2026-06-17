//! End-to-end integration tests for `fabio data-pipeline` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_list_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "data-pipeline",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_create_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("dp_test");

    // Create
    let assert = fabio()
        .args([
            "data-pipeline",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let dp_id = data["id"].as_str().unwrap().to_string();

    // Delete
    let assert = fabio()
        .args([
            "data-pipeline",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &dp_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_update_name() {
    let cfg = TestConfig::from_env();
    let original = common::unique_name("dp_upd_o");
    let updated = common::unique_name("dp_upd_n");

    // Create
    let assert = fabio()
        .args([
            "data-pipeline",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &original,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let dp_id = data["id"].as_str().unwrap().to_string();

    // Update
    let assert = fabio()
        .args([
            "data-pipeline",
            "update",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &dp_id,
            "--name",
            &updated,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], updated);

    // Cleanup
    fabio()
        .args([
            "data-pipeline",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &dp_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_update_requires_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "data-pipeline",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err_json["error"]["code"], "INVALID_INPUT");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_list_schedules_returns_array() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("dp_sched");

    // Create a pipeline to test schedule listing
    let assert = fabio()
        .args([
            "data-pipeline",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let dp_id = json["data"]["id"].as_str().unwrap().to_string();

    // List schedules (should be empty initially)
    let assert = fabio()
        .args([
            "data-pipeline",
            "list-schedules",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &dp_id,
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_array());

    // Cleanup
    fabio()
        .args([
            "data-pipeline",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &dp_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_list_instances_returns_array() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("dp_inst");

    // Create a pipeline to test instance listing
    let assert = fabio()
        .args([
            "data-pipeline",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let dp_id = json["data"]["id"].as_str().unwrap().to_string();

    // List instances (should be empty initially)
    let assert = fabio()
        .args([
            "data-pipeline",
            "list-instances",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &dp_id,
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_array());

    // Cleanup
    fabio()
        .args([
            "data-pipeline",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &dp_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_delete_schedule_dry_run() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "data-pipeline",
            "delete-schedule",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
            "--schedule-id",
            "00000000-0000-0000-0000-000000000002",
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        json["data"]["would_execute"],
        "data-pipeline delete-schedule"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_update_schedule_requires_body() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "data-pipeline",
            "update-schedule",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
            "--schedule-id",
            "00000000-0000-0000-0000-000000000002",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("INVALID_INPUT")
            || stderr.contains("--file")
            || stderr.contains("--content"),
        "Expected error about missing body, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_update_schedule_dry_run() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "data-pipeline",
            "update-schedule",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
            "--schedule-id",
            "00000000-0000-0000-0000-000000000002",
            "--content",
            r#"{"enabled":true,"configuration":{"type":"Cron","interval":10}}"#,
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        json["data"]["would_execute"],
        "data-pipeline update-schedule"
    );
    // Verify the body content is in the dry-run details
    assert_eq!(json["data"]["details"]["enabled"], true);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_get_schedule_invalid_returns_error() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "data-pipeline",
            "get-schedule",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
            "--schedule-id",
            "00000000-0000-0000-0000-ffffffffffff",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("NOT_FOUND") || stderr.contains("API_ERROR") || stderr.contains("error"),
        "Expected error for invalid schedule ID, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn data_pipeline_get_instance_invalid_returns_error() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "data-pipeline",
            "get-instance",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000001",
            "--instance-id",
            "00000000-0000-0000-0000-ffffffffffff",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("NOT_FOUND") || stderr.contains("API_ERROR") || stderr.contains("error"),
        "Expected error for invalid instance ID, got: {stderr}"
    );
}
