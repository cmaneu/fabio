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
