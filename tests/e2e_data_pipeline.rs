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
