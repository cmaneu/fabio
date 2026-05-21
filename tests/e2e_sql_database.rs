//! End-to-end integration tests for `fabio sql-database` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sql_database_list_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["sql-database", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sql_database_create_show_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sqldb_test");

    // Create
    let assert = fabio()
        .args([
            "sql-database",
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
    let db_id = data["id"].as_str().unwrap().to_string();

    // Show
    let assert = fabio()
        .args([
            "sql-database",
            "show",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &db_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["id"], db_id);
    assert_eq!(data["displayName"], name);
    // Verify properties are returned
    assert!(data.get("properties").is_some());

    // Delete
    let assert = fabio()
        .args([
            "sql-database",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &db_id,
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
fn sql_database_update_description() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sqldb_upd");

    // Create
    let assert = fabio()
        .args([
            "sql-database",
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
    let db_id = data["id"].as_str().unwrap().to_string();

    // Update description
    let assert = fabio()
        .args([
            "sql-database",
            "update",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &db_id,
            "--description",
            "Updated via E2E test",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["description"], "Updated via E2E test");

    // Cleanup
    fabio()
        .args([
            "sql-database",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &db_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sql_database_update_requires_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "sql-database",
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
fn sql_database_list_deleted_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "sql-database",
            "list-deleted",
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
fn sql_database_dry_run_create() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "sql-database",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            "dry_run_test",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "sql-database create");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sql_database_get_audit_settings() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sqldb_aud");

    // Create
    let assert = fabio()
        .args([
            "sql-database",
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
    let db_id = data["id"].as_str().unwrap().to_string();

    // Get audit settings
    let assert = fabio()
        .args([
            "sql-database",
            "get-audit-settings",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &db_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should have state field
    assert!(data.get("state").is_some());

    // Cleanup
    fabio()
        .args([
            "sql-database",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &db_id,
        ])
        .assert()
        .success();
}
