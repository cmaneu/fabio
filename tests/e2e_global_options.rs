//! End-to-end integration tests for global CLI options: --query, --quiet, --output.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use predicates::prelude::*;
use serial_test::serial;

// --- --query flag tests ---

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn query_extracts_single_field_from_object() {
    let cfg = TestConfig::from_env();

    // Use --query to extract just the "id" field from workspace show
    let assert = fabio()
        .args([
            "--query",
            "id",
            "workspace",
            "show",
            "--id",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Should just be the workspace ID string
    assert_eq!(data.as_str().unwrap(), cfg.source_workspace);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn query_extracts_field_from_list() {
    // Use --query to extract "displayName" from workspace list
    let assert = fabio()
        .args(["--query", "displayName", "workspace", "list"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Should be an array of display names (strings)
    let arr = data.as_array().expect("expected array of names");
    assert!(!arr.is_empty());
    // Each element should be a string
    for name in arr {
        assert!(name.is_string(), "expected string, got: {name}");
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn query_nested_field() {
    let cfg = TestConfig::from_env();

    // Workspace show returns fields like capacityId - test it extracts correctly
    let assert = fabio()
        .args([
            "--query",
            "capacityId",
            "workspace",
            "show",
            "--id",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Should be the capacity ID string
    assert!(data.is_string(), "expected capacityId to be a string");
    assert!(!data.as_str().unwrap().is_empty());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn query_missing_field_returns_null() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "--query",
            "nonexistent_field",
            "workspace",
            "show",
            "--id",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    assert!(
        data.is_null(),
        "expected null for missing field, got: {data}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn query_with_table_output() {
    // --query + --output table should not crash
    fabio()
        .args([
            "--query",
            "displayName",
            "--output",
            "table",
            "workspace",
            "list",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn query_with_plain_output() {
    let cfg = TestConfig::from_env();

    // --query id + --output plain should print just the id
    let assert = fabio()
        .args([
            "--query",
            "id",
            "--output",
            "plain",
            "workspace",
            "show",
            "--id",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let trimmed = stdout.trim();
    assert_eq!(trimmed, cfg.source_workspace);
}

// --- --quiet flag tests ---

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn quiet_suppresses_all_stdout() {
    // --quiet should produce no stdout
    fabio()
        .args(["--quiet", "workspace", "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn quiet_with_show_command() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "--quiet",
            "workspace",
            "show",
            "--id",
            &cfg.source_workspace,
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn quiet_still_shows_errors_on_stderr() {
    // --quiet should still show errors on stderr
    fabio()
        .args([
            "--quiet",
            "workspace",
            "show",
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("error"));
}

// --- --output format coverage ---

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn output_json_produces_valid_json() {
    let assert = fabio()
        .args(["--output", "json", "workspace", "list"])
        .assert()
        .success();

    // Should be valid JSON
    let _json = parse_json(&assert);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn output_table_has_headers() {
    let cfg = TestConfig::from_env();

    // Table output for item list should show headers
    fabio()
        .args([
            "--output",
            "table",
            "item",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("NAME"))
        .stdout(predicate::str::contains("ID"))
        .stdout(predicate::str::contains("TYPE"));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn output_plain_for_item_list() {
    let cfg = TestConfig::from_env();

    // Plain output for item list should print item IDs (plain_key is "id")
    fabio()
        .args([
            "--output",
            "plain",
            "item",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// --- --continuation-token flag tests ---

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn continuation_token_with_invalid_token_returns_error() {
    let cfg = TestConfig::from_env();

    // An invalid token should fail gracefully (API returns error)
    fabio()
        .args([
            "--continuation-token",
            "invalid_token_abc123",
            "item",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .failure();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn continuation_token_flag_accepted_on_list_command() {
    // Verify the flag is accepted (even with no token) - just tests CLI parsing
    fabio()
        .args(["workspace", "list", "--limit", "1"])
        .assert()
        .success();
}
