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
    // Should report credential source
    let source = data["credential_source"].as_str().unwrap_or("");
    assert!(
        [
            "environment",
            "managed_identity",
            "azure_cli",
            "azure_developer_cli"
        ]
        .contains(&source),
        "Unexpected credential_source: {source}"
    );
    // Message should mention the source
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Token acquired successfully via"),
        "Expected token message, got: {msg}"
    );
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
fn auth_login_validates_credentials() {
    // Login now actually validates credentials (attempts token acquisition)
    let assert = fabio().args(["auth", "login"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "logged_in");
    // Should report credential source
    assert!(data["credential_source"].is_string());
    // Message should say successful
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Successfully authenticated via"),
        "Expected success message, got: {msg}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_login_device_code_flag() {
    // --device-code is accepted but doesn't change behavior (no interactive flow)
    let assert = fabio()
        .args(["auth", "login", "--device-code"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "logged_in");
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

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_status_reports_credential_source() {
    let assert = fabio()
        .args(["auth", "status", "--query", "credential_source"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should be one of the valid credential sources
    let source = data.as_str().unwrap_or("");
    assert!(
        [
            "environment",
            "managed_identity",
            "azure_cli",
            "azure_developer_cli"
        ]
        .contains(&source),
        "Unexpected credential_source: {source}"
    );
}
