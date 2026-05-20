//! E2E integration tests for the `fabio spark` command group.
//!
//! Tests workspace-level Spark settings and custom pool operations.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};

#[test]
#[ignore = "requires live Fabric tenant"]
fn spark_get_settings() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "spark",
            "get-settings",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    // Should return an object with Spark settings data
    assert!(json.get("data").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn spark_update_settings_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "spark",
            "update-settings",
            "--workspace",
            &cfg.source_workspace,
            "--settings",
            r#"{"automaticLog":{"enabled":true}}"#,
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn spark_list_pools() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args(["spark", "list-pools", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&output);
    // Should return a list (possibly empty)
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn spark_create_pool_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "spark",
            "create-pool",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "test-pool",
            "--node-size",
            "Small",
            "--max-node-count",
            "3",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn spark_delete_pool_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "spark",
            "delete-pool",
            "--workspace",
            &cfg.source_workspace,
            "--pool-id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}
