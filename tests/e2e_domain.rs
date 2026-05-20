//! E2E integration tests for the `fabio domain` command group.
//!
//! Tests domain CRUD and workspace assignment operations.

mod common;

use common::{extract_data, fabio, parse_json, unique_name};

#[test]
#[ignore = "requires live Fabric tenant"]
fn domain_list() {
    let output = fabio().args(["domain", "list"]).assert().success();

    let json = parse_json(&output);
    // Should return a list envelope
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn domain_create_dry_run() {
    let name = unique_name("test-domain");

    let output = fabio()
        .args([
            "domain",
            "create",
            "--name",
            &name,
            "--description",
            "E2E test domain",
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
fn domain_update_requires_fields() {
    // Should fail with no --name or --description
    fabio()
        .args([
            "domain",
            "update",
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn domain_delete_dry_run() {
    let output = fabio()
        .args([
            "domain",
            "delete",
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

#[test]
#[ignore = "requires live Fabric tenant"]
fn domain_assign_workspaces_dry_run() {
    let output = fabio()
        .args([
            "domain",
            "assign-workspaces",
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--workspaces",
            "11111111-1111-1111-1111-111111111111,22222222-2222-2222-2222-222222222222",
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
fn domain_unassign_workspaces_dry_run() {
    let output = fabio()
        .args([
            "domain",
            "unassign-workspaces",
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--workspaces",
            "11111111-1111-1111-1111-111111111111",
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
fn domain_assign_by_capacity_dry_run() {
    let output = fabio()
        .args([
            "domain",
            "assign-by-capacity",
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--capacities",
            "11111111-1111-1111-1111-111111111111",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}
