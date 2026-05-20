//! E2E integration tests for the `fabio mirrored-database` command group.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};

#[test]
#[ignore = "requires live Fabric tenant"]
fn mirrored_database_list() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "mirrored-database",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn mirrored_database_create_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "mirrored-database",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "test-mirror-db",
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
fn mirrored_database_update_requires_fields() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "mirrored-database",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn mirrored_database_delete_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "mirrored-database",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
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
fn mirrored_database_start_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "mirrored-database",
            "start",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
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
fn mirrored_database_stop_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "mirrored-database",
            "stop",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}
