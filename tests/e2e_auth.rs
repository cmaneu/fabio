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

// ── Service principal login validation tests (offline) ──────────────────────

#[test]
fn sp_login_requires_tenant() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--client-id",
            "abc",
            "--client-secret",
            "xyz",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--tenant is required"),
        "expected tenant required error, got: {stderr}"
    );
}

#[test]
fn sp_login_requires_client_id() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-secret",
            "xyz",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--client-id is required"),
        "expected client-id required error, got: {stderr}"
    );
}

#[test]
fn sp_login_requires_credential_type() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--client-secret, --certificate, or --federated-token"),
        "expected credential type required error, got: {stderr}"
    );
}

#[test]
fn sp_login_rejects_multiple_credential_types() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
            "--client-secret",
            "secret",
            "--certificate",
            "/tmp/cert.pem",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("Only one credential type allowed"),
        "expected mutual exclusion error, got: {stderr}"
    );
}

#[test]
fn sp_login_certificate_file_not_found() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
            "--certificate",
            "/nonexistent/path/cert.pem",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("Failed to read certificate file"),
        "expected file not found error, got: {stderr}"
    );
}

#[test]
fn sp_login_federated_token_file_not_found() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
            "--federated-token-file",
            "/nonexistent/path/token.txt",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("Failed to read federated token file"),
        "expected file not found error, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sp_login_invalid_secret_returns_auth_error() {
    // Use fake tenant/client to verify the flow reaches Azure and fails gracefully
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "00000000-0000-0000-0000-000000000001",
            "--client-id",
            "00000000-0000-0000-0000-000000000002",
            "--client-secret",
            "fake-secret-value",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("AUTH_REQUIRED"),
        "expected AUTH_REQUIRED error code, got: {stderr}"
    );
    assert!(
        stderr.contains("authentication failed")
            || stderr.contains("AADSTS")
            || stderr.contains("not found"),
        "expected Azure error details, got: {stderr}"
    );
}

// ── Empty string validation tests (offline) ─────────────────────────────────

#[test]
fn sp_login_empty_tenant_treated_as_missing() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "",
            "--client-id",
            "abc",
            "--client-secret",
            "xyz",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--tenant is required"),
        "empty --tenant should be treated as missing, got: {stderr}"
    );
}

#[test]
fn sp_login_empty_client_id_treated_as_missing() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "",
            "--client-secret",
            "xyz",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--client-id is required"),
        "empty --client-id should be treated as missing, got: {stderr}"
    );
}

#[test]
fn sp_login_empty_secret_treated_as_no_credential() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
            "--client-secret",
            "",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("requires one of"),
        "empty --client-secret should be treated as no credential, got: {stderr}"
    );
}

#[test]
fn sp_login_empty_certificate_path_treated_as_no_credential() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
            "--certificate",
            "",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("requires one of"),
        "empty --certificate should be treated as no credential, got: {stderr}"
    );
}

#[test]
fn sp_login_empty_federated_token_treated_as_no_credential() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
            "--federated-token",
            "",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("requires one of"),
        "empty --federated-token should be treated as no credential, got: {stderr}"
    );
}

// ── File content edge case tests (offline) ──────────────────────────────────

#[test]
fn sp_login_empty_certificate_file() {
    // Create a temp empty file
    let temp = std::env::temp_dir().join("fabio_test_empty_cert.pem");
    std::fs::write(&temp, "").unwrap();

    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
            "--certificate",
            temp.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("is empty"),
        "empty certificate file should produce clear error, got: {stderr}"
    );

    std::fs::remove_file(&temp).ok();
}

#[test]
fn sp_login_empty_federated_token_file() {
    // Create a temp empty file
    let temp = std::env::temp_dir().join("fabio_test_empty_token.txt");
    std::fs::write(&temp, "   \n  ").unwrap(); // whitespace only

    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
            "--federated-token-file",
            temp.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("is empty"),
        "whitespace-only federated token file should be treated as empty, got: {stderr}"
    );

    std::fs::remove_file(&temp).ok();
}

#[test]
fn sp_login_invalid_certificate_content() {
    // Create a temp file with garbage content (not valid PEM/PFX)
    let temp = std::env::temp_dir().join("fabio_test_bad_cert.pem");
    std::fs::write(&temp, "this is not a certificate").unwrap();

    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
            "--certificate",
            temp.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("AUTH_REQUIRED") || stderr.contains("certificate"),
        "invalid cert content should produce auth/cert error, got: {stderr}"
    );

    std::fs::remove_file(&temp).ok();
}

// ── Structured output validation (offline) ──────────────────────────────────

#[test]
fn sp_login_error_has_structured_json_envelope() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "abc",
            "--client-id",
            "def",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    // Should be valid JSON with error envelope
    let json: serde_json::Value =
        serde_json::from_str(&stderr).expect("stderr should be valid JSON");
    assert!(json.get("error").is_some(), "should have error field");
    assert_eq!(json["error"]["code"].as_str().unwrap(), "INVALID_INPUT");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("requires one of"),
        "message should explain the issue"
    );
}

#[test]
fn sp_login_error_includes_hint() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--client-id",
            "abc",
            "--client-secret",
            "xyz",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let json: serde_json::Value =
        serde_json::from_str(&stderr).expect("stderr should be valid JSON");
    assert!(
        json["error"]["hint"].as_str().is_some(),
        "error should include a hint with example command"
    );
    assert!(
        json["error"]["hint"].as_str().unwrap().contains("Example:"),
        "hint should contain an example"
    );
}

// ── Live tests with real Azure (invalid credentials) ────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sp_login_invalid_tenant_returns_descriptive_error() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "not-a-real-tenant-id",
            "--client-id",
            "00000000-0000-0000-0000-000000000002",
            "--client-secret",
            "fake-secret",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("AUTH_REQUIRED"),
        "expected AUTH_REQUIRED code, got: {stderr}"
    );
    // Azure should report tenant not found
    assert!(
        stderr.contains("AADSTS") || stderr.contains("not found") || stderr.contains("Tenant"),
        "expected tenant-related error, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sp_login_invalid_federated_token_returns_auth_error() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "00000000-0000-0000-0000-000000000001",
            "--client-id",
            "00000000-0000-0000-0000-000000000002",
            "--federated-token",
            "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.invalid.token",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("AUTH_REQUIRED"),
        "expected AUTH_REQUIRED code, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sp_login_error_output_is_valid_json() {
    let assert = fabio()
        .args([
            "auth",
            "login",
            "--service-principal",
            "--tenant",
            "00000000-0000-0000-0000-000000000001",
            "--client-id",
            "00000000-0000-0000-0000-000000000002",
            "--client-secret",
            "bad",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let json: serde_json::Value =
        serde_json::from_str(&stderr).expect("live error should be valid JSON on stderr");
    assert_eq!(json["error"]["code"].as_str().unwrap(), "AUTH_REQUIRED");
    assert!(json["error"]["message"].as_str().unwrap().len() > 20);
    assert!(json["error"]["hint"].as_str().is_some());
}
