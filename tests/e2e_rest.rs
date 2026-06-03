//! End-to-end tests for `fabio rest call` (raw REST passthrough).

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use predicates::prelude::*;
use serial_test::serial;

// --- GET requests ---

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn rest_get_workspaces() {
    let assert = fabio()
        .args(["rest", "call", "--method", "get", "--path", "/workspaces"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should have a "value" array of workspaces
    assert!(data.get("value").unwrap().is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn rest_get_with_query_params() {
    let cfg = TestConfig::from_env();

    let path = format!("/workspaces/{}/items", cfg.source_workspace);
    let assert = fabio()
        .args([
            "rest",
            "call",
            "--method",
            "get",
            "--path",
            &path,
            "--query-params",
            "type=Lakehouse",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let items = data.get("value").unwrap().as_array().unwrap();
    // All returned items should be lakehouses
    for item in items {
        assert_eq!(item["type"], "Lakehouse");
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn rest_get_single_resource() {
    let cfg = TestConfig::from_env();

    let path = format!("/workspaces/{}", cfg.source_workspace);
    let assert = fabio()
        .args(["rest", "call", "--method", "get", "--path", &path])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["id"], cfg.source_workspace);
}

// --- POST dry-run ---

#[test]
#[serial]
fn rest_post_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "rest",
            "call",
            "--method",
            "post",
            "--path",
            "/workspaces",
            "--body",
            r#"{"displayName":"DryRunTest"}"#,
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("dry_run"));
    assert!(stdout.contains("DryRunTest"));
}

// --- DELETE dry-run ---

#[test]
#[serial]
fn rest_delete_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "rest",
            "call",
            "--method",
            "delete",
            "--path",
            "/workspaces/00000000-0000-0000-0000-000000000000/items/00000000-0000-0000-0000-000000000001",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("dry_run"));
}

// --- Body from file ---

#[test]
#[serial]
fn rest_body_from_file_dry_run() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), r#"{"displayName":"FromFile"}"#).unwrap();

    let body_arg = format!("@{}", tmp.path().display());
    let assert = fabio()
        .args([
            "--dry-run",
            "rest",
            "call",
            "--method",
            "post",
            "--path",
            "/workspaces",
            "--body",
            &body_arg,
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("FromFile"));
}

// --- Invalid JSON body ---

#[test]
#[serial]
fn rest_invalid_json_body_fails() {
    fabio()
        .args([
            "rest",
            "call",
            "--method",
            "post",
            "--path",
            "/workspaces",
            "--body",
            "not valid json {{{",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON body"));
}

// --- Global options work with rest ---

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn rest_with_output_table() {
    let cfg = TestConfig::from_env();

    let path = format!("/workspaces/{}", cfg.source_workspace);
    fabio()
        .args([
            "--output", "table", "rest", "call", "--method", "get", "--path", &path,
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn rest_with_query_flag() {
    let cfg = TestConfig::from_env();

    let path = format!("/workspaces/{}", cfg.source_workspace);
    let assert = fabio()
        .args([
            "--query",
            "displayName",
            "rest",
            "call",
            "--method",
            "get",
            "--path",
            &path,
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Should have extracted the displayName field
    assert!(stdout.contains("fabio-demo-source"));
}

// --- Power BI API (--api powerbi) dry-run tests ---

#[test]
#[serial]
fn rest_powerbi_post_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "rest",
            "call",
            "--method",
            "post",
            "--path",
            "/groups/abc/datasets/def/refreshes",
            "--api",
            "powerbi",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["details"]["api"], "powerbi");
    assert_eq!(data["details"]["method"], "POST");
    assert!(
        data["details"]["path"]
            .as_str()
            .unwrap()
            .contains("datasets/def/refreshes")
    );
}

#[test]
#[serial]
fn rest_powerbi_delete_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "rest",
            "call",
            "--method",
            "delete",
            "--path",
            "/groups/abc/datasets/def/users/user123",
            "--api",
            "powerbi",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["details"]["api"], "powerbi");
    assert_eq!(data["details"]["method"], "DELETE");
}

#[test]
#[serial]
fn rest_powerbi_patch_dry_run_with_body() {
    let assert = fabio()
        .args([
            "--dry-run",
            "rest",
            "call",
            "--method",
            "patch",
            "--path",
            "/groups/abc/datasets/def/Default.UpdateDatasources",
            "--api",
            "powerbi",
            "--body",
            r#"{"updateDetails":[{"datasourceSelector":{"datasourceType":"Sql"}}]}"#,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["details"]["api"], "powerbi");
    assert_eq!(data["details"]["method"], "PATCH");
    assert!(data["details"]["body"]["updateDetails"].is_array());
}

// --- Power BI API live tests ---

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn rest_powerbi_get_datasets() {
    let cfg = TestConfig::from_env();

    let path = format!("/groups/{}/datasets", cfg.source_workspace);
    let assert = fabio()
        .args([
            "rest", "call", "--method", "get", "--path", &path, "--api", "powerbi",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Power BI API returns a "value" array of datasets
    assert!(data.get("value").unwrap().is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn rest_powerbi_get_workspaces() {
    let assert = fabio()
        .args([
            "rest", "call", "--method", "get", "--path", "/groups", "--api", "powerbi",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Power BI API returns a "value" array of workspaces (groups)
    assert!(data.get("value").unwrap().is_array());
}
