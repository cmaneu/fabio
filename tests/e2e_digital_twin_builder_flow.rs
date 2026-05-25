use assert_cmd::Command;
use serial_test::serial;

mod common;
use common::TestConfig;

fn fabio() -> Command {
    Command::cargo_bin("fabio").unwrap()
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn digital_twin_builder_flow_list_returns_array() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "digital-twin-builder-flow",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn digital_twin_builder_flow_dry_run_create() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "digital-twin-builder-flow",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "test_dtb_flow",
            "--dtb-id",
            "00000000-0000-0000-0000-000000000001",
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        json["data"]["would_execute"],
        "digital-twin-builder-flow create"
    );
}
