//! End-to-end integration tests for `fabio auth` commands.

mod common;

use common::{extract_data, fabio, parse_json};
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_status_returns_authenticated() {
    let assert = fabio().args(["auth", "status"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    assert_eq!(data["status"], "authenticated");
    assert_eq!(data["message"], "Token acquired successfully");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_status_json_envelope() {
    let assert = fabio()
        .args(["auth", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"data\""));

    let json = parse_json(&assert);
    assert!(json.get("data").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_status_table_format() {
    // auth status with --output table should produce human-readable output
    fabio()
        .args(["auth", "status", "-o", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("authenticated"));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_login_shows_credential_chain() {
    // Login is a no-op (relies on DefaultAzureCredential) but should succeed
    let assert = fabio().args(["auth", "login"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "logged_in");
    assert_eq!(data["method"], "browser");
    // Should mention DefaultAzureCredential
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("DefaultAzureCredential"),
        "Expected credential chain info: {msg}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_login_device_code_flag() {
    let assert = fabio()
        .args(["auth", "login", "--device-code"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "logged_in");
    assert_eq!(data["method"], "device_code");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_logout_succeeds() {
    let assert = fabio().args(["auth", "logout"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "logged_out");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_status_with_query_extracts_field() {
    let assert = fabio()
        .args(["auth", "status", "--query", "status"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // --query status should extract just the status string
    assert_eq!(data, "authenticated");
}
