//! E2E integration tests for the `fabio managed-private-endpoint` command group.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};

#[test]
#[ignore = "requires live Fabric tenant"]
fn managed_private_endpoint_list() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "managed-private-endpoint",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn managed_private_endpoint_create_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "managed-private-endpoint",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "test-pe",
            "--target-resource-id",
            "/subscriptions/00000000-0000-0000-0000-000000000000/resourceGroups/rg/providers/Microsoft.Storage/storageAccounts/sa",
            "--target-subresource-type",
            "blob",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn managed_private_endpoint_delete_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "managed-private-endpoint",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}
