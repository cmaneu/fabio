//! End-to-end integration tests for `fabio environment` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn environment_list_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["environment", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn environment_create_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("env_test");

    // Create
    let assert = fabio()
        .args([
            "environment",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_mins(2))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let env_id = data["id"].as_str().unwrap().to_string();

    // Delete
    let assert = fabio()
        .args([
            "environment",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &env_id,
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
fn environment_update_name() {
    let cfg = TestConfig::from_env();
    let original = common::unique_name("env_upd_o");
    let updated = common::unique_name("env_upd_n");

    // Create
    let assert = fabio()
        .args([
            "environment",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &original,
        ])
        .timeout(std::time::Duration::from_mins(2))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let env_id = data["id"].as_str().unwrap().to_string();

    // Update
    let assert = fabio()
        .args([
            "environment",
            "update",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &env_id,
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
            "environment",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &env_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn environment_update_requires_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "environment",
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
fn environment_dry_run_create() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "environment",
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
    assert_eq!(json["data"]["would_execute"], "environment create");
}

// ─── Upload Staging Library ─────────────────────────────────────────────────

#[test]
fn environment_upload_staging_library_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "environment",
            "upload-staging-library",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--file",
            "Cargo.toml",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "environment upload-staging-library");
    assert_eq!(data["details"]["libraryName"], "Cargo.toml");
    assert!(data["details"]["sizeBytes"].as_u64().unwrap() > 0);
}

#[test]
fn environment_upload_staging_library_custom_name_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "environment",
            "upload-staging-library",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--file",
            "Cargo.toml",
            "--library-name",
            "my_lib-1.0.0.whl",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["details"]["libraryName"], "my_lib-1.0.0.whl");
}

#[test]
fn environment_upload_staging_library_missing_file() {
    let assert = fabio()
        .args([
            "environment",
            "upload-staging-library",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--file",
            "/nonexistent/path/lib.whl",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("Failed to read file"));
}
