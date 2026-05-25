//! E2E integration tests for the `fabio kql-database` command group.
//!
//! Live query tests require:
//! - `FABIO_TEST_SOURCE_WORKSPACE` (workspace with an Eventhouse + KQL Database)
//! - `FABIO_TEST_KQL_DATABASE_ID` (ID of the KQL database to query)
//! - `FABIO_TEST_KUSTO_QUERY_URI` (Kusto query URI, e.g. `https://xxx.z4.kusto.fabric.microsoft.com`)

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serde_json::Value;

// ─── CRUD Tests ──────────────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
fn kql_database_list() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args(["kql-database", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&output);
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn kql_database_create_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "kql-database",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "test-kql-db",
            "--eventhouse-id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "kql-database create");
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn kql_database_update_requires_fields() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "kql-database",
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
fn kql_database_delete_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "kql-database",
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
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "kql-database delete");
}

// ─── Query Tests (no tenant required) ────────────────────────────────────────

#[test]
fn kql_database_query_no_input_fails() {
    // Without --kql and without stdin, should fail with helpful error
    fabio()
        .args([
            "kql-database",
            "query",
            "--workspace",
            "00000000-0000-0000-0000-000000000000",
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--kql",
            "",
        ])
        .assert()
        .failure();
}

// ─── Query Tests (live tenant) ───────────────────────────────────────────────

/// Helper to get KQL test config from environment.
fn kql_test_config() -> (TestConfig, String, String) {
    let cfg = TestConfig::from_env();
    let kql_db_id =
        std::env::var("FABIO_TEST_KQL_DATABASE_ID").expect("FABIO_TEST_KQL_DATABASE_ID required");
    let query_uri =
        std::env::var("FABIO_TEST_KUSTO_QUERY_URI").expect("FABIO_TEST_KUSTO_QUERY_URI required");
    (cfg, kql_db_id, query_uri)
}

#[test]
#[ignore = "requires live Fabric tenant with Eventhouse"]
fn kql_database_query_print_literal() {
    let (cfg, kql_db_id, _query_uri) = kql_test_config();

    // Auto-discovers query URI from database properties
    let output = fabio()
        .args([
            "kql-database",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &kql_db_id,
            "--kql",
            "print message='hello from fabio'",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    let rows = data.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["message"], "hello from fabio");
}

#[test]
#[ignore = "requires live Fabric tenant with Eventhouse"]
fn kql_database_query_with_explicit_uri() {
    let (cfg, kql_db_id, query_uri) = kql_test_config();

    let output = fabio()
        .args([
            "kql-database",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &kql_db_id,
            "--query-uri",
            &query_uri,
            "--kql",
            "print x=42, y=3.14, z=true",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    let rows = data.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["x"], 42);
    assert_eq!(rows[0]["z"], true);
}

#[test]
#[ignore = "requires live Fabric tenant with Eventhouse"]
fn kql_database_query_multiple_rows() {
    let (cfg, kql_db_id, _) = kql_test_config();

    let output = fabio()
        .args([
            "kql-database",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &kql_db_id,
            "--kql",
            "range i from 1 to 5 step 1 | extend squared = i * i",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    assert_eq!(json["count"], 5);
    let data = extract_data(&json);
    let rows = data.as_array().unwrap();
    assert_eq!(rows[0]["i"], 1);
    assert_eq!(rows[0]["squared"], 1);
    assert_eq!(rows[4]["i"], 5);
    assert_eq!(rows[4]["squared"], 25);
}

#[test]
#[ignore = "requires live Fabric tenant with Eventhouse"]
fn kql_database_query_show_tables_mgmt() {
    let (cfg, kql_db_id, _) = kql_test_config();

    // .show tables uses the /v1/rest/mgmt endpoint
    let output = fabio()
        .args([
            "kql-database",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &kql_db_id,
            "--kql",
            ".show tables",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    // Should have at least the SalesEvents table we created
    let data = extract_data(&json);
    if let Some(rows) = data.as_array() {
        if !rows.is_empty() {
            assert!(rows[0].get("TableName").is_some());
        }
    }
}

#[test]
#[ignore = "requires live Fabric tenant with Eventhouse"]
fn kql_database_query_table_data() {
    let (cfg, kql_db_id, _) = kql_test_config();

    // Query the SalesEvents table (created during setup)
    let output = fabio()
        .args([
            "kql-database",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &kql_db_id,
            "--kql",
            "SalesEvents | summarize TotalAmount=sum(Amount), Count=count() by Region | order by TotalAmount desc",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    let rows = data.as_array().unwrap();
    // We know from setup: EU=99.0, US=59.98, APAC=15.0
    assert!(rows.len() >= 3);
    assert!(rows[0].get("Region").is_some());
    assert!(rows[0].get("TotalAmount").is_some());
    assert!(rows[0].get("Count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant with Eventhouse"]
fn kql_database_query_from_file() {
    let (cfg, kql_db_id, _) = kql_test_config();

    // Write a query to a temp file
    let tmp_dir = std::env::temp_dir();
    let kql_file = tmp_dir.join("fabio_test_query.kql");
    std::fs::write(&kql_file, "print source='file', value=123").unwrap();
    let file_arg = format!("@{}", kql_file.display());

    let output = fabio()
        .args([
            "kql-database",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &kql_db_id,
            "--kql",
            &file_arg,
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    let rows = data.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["source"], "file");
    assert_eq!(rows[0]["value"], 123);

    // Cleanup
    let _ = std::fs::remove_file(&kql_file);
}

#[test]
#[ignore = "requires live Fabric tenant with Eventhouse"]
fn kql_database_query_stdin() {
    let (cfg, kql_db_id, _) = kql_test_config();

    let output = fabio()
        .args([
            "kql-database",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &kql_db_id,
        ])
        .write_stdin("print source='stdin', value=456")
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    let rows = data.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["source"], "stdin");
    assert_eq!(rows[0]["value"], 456);
}

#[test]
#[ignore = "requires live Fabric tenant with Eventhouse"]
fn kql_database_query_invalid_syntax_fails() {
    let (cfg, kql_db_id, _) = kql_test_config();

    let output = fabio()
        .args([
            "kql-database",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &kql_db_id,
            "--kql",
            "INVALID SYNTAX @@@ |||",
        ])
        .assert()
        .failure();

    // Errors are written to stderr in fabio
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    let err: Value = serde_json::from_str(&stderr).expect("stderr should be JSON error");
    assert_eq!(err["error"]["code"], "API_ERROR");
    assert!(err["error"]["message"].as_str().unwrap().contains("400"));
}

#[test]
#[ignore = "requires live Fabric tenant with Eventhouse"]
fn kql_database_query_table_output_format() {
    let (cfg, kql_db_id, _) = kql_test_config();

    fabio()
        .args([
            "kql-database",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &kql_db_id,
            "--kql",
            "range i from 1 to 3 step 1",
            "-o",
            "table",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("| i |"));
}
