//! End-to-end integration tests for `fabio warehouse` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn warehouse_list_returns_json() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["warehouse", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    // Should have data array and count
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn warehouse_show_returns_details() {
    let cfg = TestConfig::from_env();

    // First list warehouses to get an ID
    let assert = fabio()
        .args(["warehouse", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let items = extract_data(&json).as_array().unwrap().clone();
    if items.is_empty() {
        eprintln!("No warehouses found in source workspace, skipping show test");
        return;
    }

    let wh_id = items[0]["id"].as_str().unwrap();

    // Show the warehouse
    let assert = fabio()
        .args([
            "warehouse",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            wh_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["id"], wh_id);
    assert!(data.get("displayName").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn warehouse_query_resolves_endpoint() {
    let cfg = TestConfig::from_env();

    // Query against the lakehouse SQL endpoint
    let assert = fabio()
        .args([
            "warehouse",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--sql",
            "SELECT 1 AS test",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Should resolve the endpoint (ODBC execution may not be implemented)
    assert!(
        data.get("endpoint").is_some() || data.get("rows").is_some(),
        "expected endpoint or query results"
    );

    // Verify the endpoint was resolved correctly
    if let Some(endpoint) = data.get("endpoint").and_then(|e| e.as_str()) {
        assert!(
            endpoint.contains("datawarehouse.fabric.microsoft.com"),
            "unexpected endpoint: {endpoint}"
        );
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn warehouse_query_from_stdin() {
    let cfg = TestConfig::from_env();

    // Pipe SQL via stdin
    let assert = fabio()
        .args([
            "warehouse",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
        ])
        .write_stdin("SELECT 1 AS test")
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["sql"], "SELECT 1 AS test");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn warehouse_query_table_output() {
    let cfg = TestConfig::from_env();

    // Table output should not crash even if ODBC isn't available
    fabio()
        .args([
            "--output",
            "table",
            "warehouse",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--sql",
            "SELECT 1 AS test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// ---------------------------------------------------------------------------
// warehouse show for non-existent ID returns error
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn warehouse_show_not_found() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "warehouse",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
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

// ---------------------------------------------------------------------------
// warehouse query with --sql from @file
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn warehouse_query_from_file() {
    let cfg = TestConfig::from_env();
    let tmp_dir = tempfile::TempDir::new().unwrap();
    let sql_file = tmp_dir.path().join("query.sql");
    std::fs::write(&sql_file, "SELECT 42 AS answer").unwrap();

    let sql_arg = format!("@{}", sql_file.to_str().unwrap());
    let assert = fabio()
        .args([
            "warehouse",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--sql",
            &sql_arg,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["sql"], "SELECT 42 AS answer");
}

// ===========================================================================
// warehouse create / update / delete
// ===========================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn warehouse_create_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("wh_crud");

    // Create
    let assert = fabio()
        .args([
            "warehouse",
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
    let wh_id = data["id"].as_str().unwrap().to_string();

    // Delete
    let assert = fabio()
        .args([
            "warehouse",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &wh_id,
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
fn warehouse_update_name() {
    let cfg = TestConfig::from_env();
    let original = common::unique_name("wh_upd_o");
    let updated = common::unique_name("wh_upd_n");

    // Create
    let assert = fabio()
        .args([
            "warehouse",
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
    let wh_id = data["id"].as_str().unwrap().to_string();

    // Update
    let assert = fabio()
        .args([
            "warehouse",
            "update",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &wh_id,
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
            "warehouse",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &wh_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn warehouse_update_requires_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "warehouse",
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
