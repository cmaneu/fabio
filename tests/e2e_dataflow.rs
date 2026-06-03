//! End-to-end integration tests for `fabio dataflow` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataflow_list_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["dataflow", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataflow_create_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("df_test");

    // Create
    let assert = fabio()
        .args([
            "dataflow",
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
    let id = data["id"].as_str().unwrap().to_string();

    // Delete
    let assert = fabio()
        .args([
            "dataflow",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &id,
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
fn dataflow_show_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "dataflow",
            "show",
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
#[serial]
fn dataflow_update_requires_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "dataflow",
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
fn dataflow_dry_run_create() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "dataflow",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "test-dry-run",
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["would_execute"], "dataflow create");
}

// ─── Discover Parameters ─────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataflow_discover_parameters_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "dataflow",
            "discover-parameters",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();
}

// ─── Hard Delete ─────────────────────────────────────────────────────────────

#[test]
fn dataflow_delete_hard_delete_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "dataflow",
            "delete",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--hard-delete",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["details"]["hardDelete"], true);
}

// ─── Run ─────────────────────────────────────────────────────────────────────

#[test]
fn dataflow_run_dry_run_execute() {
    let assert = fabio()
        .args([
            "--dry-run",
            "dataflow",
            "run",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["details"]["jobType"], "execute");
}

#[test]
fn dataflow_run_dry_run_apply_changes() {
    let assert = fabio()
        .args([
            "--dry-run",
            "dataflow",
            "run",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--job-type",
            "apply-changes",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["details"]["jobType"], "applyChanges");
}

#[test]
fn dataflow_run_dry_run_with_parameters() {
    let assert = fabio()
        .args([
            "--dry-run",
            "dataflow",
            "run",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--execute-option",
            "ApplyChangesIfNeeded",
            "--parameters",
            r#"[{"parameterName":"X","type":"Automatic","value":25}]"#,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    let body = &data["details"]["body"];
    assert_eq!(
        body["executionData"]["executeOption"],
        "ApplyChangesIfNeeded"
    );
    assert!(body["executionData"]["parameters"].is_array());
}

#[test]
fn dataflow_run_invalid_job_type() {
    let assert = fabio()
        .args([
            "dataflow",
            "run",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--job-type",
            "invalid",
        ])
        .assert()
        .failure();

    let json: serde_json::Value = serde_json::from_slice(&assert.get_output().stderr).unwrap();
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Invalid --job-type")
    );
}

#[test]
fn dataflow_run_apply_changes_rejects_parameters() {
    let assert = fabio()
        .args([
            "dataflow",
            "run",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--job-type",
            "apply-changes",
            "--execute-option",
            "SkipApplyChanges",
        ])
        .assert()
        .failure();

    let json: serde_json::Value = serde_json::from_slice(&assert.get_output().stderr).unwrap();
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("only supported for execute")
    );
}

// ─── Execute Query ──────────────────────────────────────────────────────────

#[test]
fn dataflow_execute_query_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "dataflow",
            "execute-query",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--query-name",
            "MyTable",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "dataflow execute-query");
    assert_eq!(data["details"]["queryName"], "MyTable");
}

#[test]
fn dataflow_execute_query_dry_run_with_mashup() {
    let assert = fabio()
        .args([
            "--dry-run",
            "dataflow",
            "execute-query",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--query-name",
            "MyTable",
            "--mashup",
            "let Source = Sql.Database(\"server\", \"db\") in Source",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["details"]["queryName"], "MyTable");
    assert!(
        data["details"]["customMashupDocument"]
            .as_str()
            .unwrap()
            .contains("Sql.Database")
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataflow_execute_query_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "dataflow",
            "execute-query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--query-name",
            "NonExistentQuery",
        ])
        .assert()
        .failure();
}
