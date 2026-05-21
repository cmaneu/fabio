//! End-to-end integration tests for `fabio lakehouse` shortcut commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_shortcut_create_get_delete() {
    let cfg = TestConfig::from_env();
    let shortcut_name = common::unique_name("sc_test");

    let target_json = format!(
        r#"{{"workspaceId":"{}","itemId":"{}","path":"Files"}}"#,
        cfg.source_workspace, cfg.source_lakehouse
    );

    // Create shortcut
    let assert = fabio()
        .args([
            "lakehouse",
            "create-shortcut",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--name",
            &shortcut_name,
            "--path",
            "Files",
            "--target-type",
            "oneLake",
            "--target",
            &target_json,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["name"], shortcut_name);
    assert_eq!(data["path"], "Files");
    assert!(data.get("target").is_some());
    assert_eq!(data["target"]["type"], "OneLake");

    // Get shortcut
    let assert = fabio()
        .args([
            "lakehouse",
            "get-shortcut",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--name",
            &shortcut_name,
            "--path",
            "Files",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["name"], shortcut_name);
    assert_eq!(
        data["target"]["oneLake"]["workspaceId"],
        cfg.source_workspace
    );

    // Delete shortcut
    let assert = fabio()
        .args([
            "lakehouse",
            "delete-shortcut",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--name",
            &shortcut_name,
            "--path",
            "Files",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
    assert_eq!(data["name"], shortcut_name);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_get_shortcut_not_found() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "lakehouse",
            "get-shortcut",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--name",
            "nonexistent_shortcut_xyz",
            "--path",
            "Files",
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

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_delete_shortcut_not_found() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "lakehouse",
            "delete-shortcut",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--name",
            "nonexistent_shortcut_abc",
            "--path",
            "Files",
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

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_shortcut_create_in_tables_path() {
    let cfg = TestConfig::from_env();
    let shortcut_name = common::unique_name("sc_tbl");

    let target_json = format!(
        r#"{{"workspaceId":"{}","itemId":"{}","path":"Tables"}}"#,
        cfg.source_workspace, cfg.source_lakehouse
    );

    // Create shortcut in Tables path
    let assert = fabio()
        .args([
            "lakehouse",
            "create-shortcut",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--name",
            &shortcut_name,
            "--path",
            "Tables",
            "--target-type",
            "oneLake",
            "--target",
            &target_json,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["name"], shortcut_name);
    assert_eq!(data["path"], "Tables");

    // Delete shortcut
    fabio()
        .args([
            "lakehouse",
            "delete-shortcut",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--name",
            &shortcut_name,
            "--path",
            "Tables",
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_bulk_create_shortcuts_dry_run() {
    let cfg = TestConfig::from_env();

    let content = r#"[{"name":"sc1","path":"Files","target":{"oneLake":{"workspaceId":"00000000-0000-0000-0000-000000000000","itemId":"00000000-0000-0000-0000-000000000001","path":"Files"}}}]"#;

    let assert = fabio()
        .args([
            "lakehouse",
            "bulk-create-shortcuts",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--content",
            content,
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["would_execute"], "lakehouse bulk-create-shortcuts");
    assert!(data["details"]["createShortcutRequests"].is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_bulk_create_shortcuts_with_conflict_policy_dry_run() {
    let cfg = TestConfig::from_env();

    let content = r#"{"createShortcutRequests":[{"name":"sc1","path":"Files","target":{"oneLake":{"workspaceId":"00000000-0000-0000-0000-000000000000","itemId":"00000000-0000-0000-0000-000000000001","path":"Files"}}}]}"#;

    let assert = fabio()
        .args([
            "lakehouse",
            "bulk-create-shortcuts",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--content",
            content,
            "--conflict-policy",
            "GenerateUniqueName",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["would_execute"], "lakehouse bulk-create-shortcuts");
}
