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

    fabio()
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
}
