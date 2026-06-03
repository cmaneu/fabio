//! E2E integration tests for the `fabio onelake-security` command group.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};

#[test]
#[ignore = "requires live Fabric tenant"]
fn onelake_security_list() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "onelake-security",
            "list",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn onelake_security_upsert_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "onelake-security",
            "upsert",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--roles",
            r#"[{"name":"TestRole","decisionRules":[{"effect":"Permit","permission":[{"attributeName":"Path","attributeValueIncludedIn":["/Tables/*"]}]}],"members":{"fabricItemMembers":[]}}]"#,
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
fn onelake_security_delete_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "onelake-security",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--role-name",
            "NonExistentRole",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}

// ─── Create ─────────────────────────────────────────────────────────────────

#[test]
fn onelake_security_create_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "onelake-security",
            "create",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--role",
            r#"{"name":"TestRole","decisionRules":[{"effect":"Permit","permission":[{"attributeName":"Path","attributeValueIncludedIn":["/Tables/*"]}]}],"members":{"fabricItemMembers":[]}}"#,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "onelake-security create");
}

#[test]
fn onelake_security_create_with_conflict_policy_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "onelake-security",
            "create",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--role",
            r#"{"name":"TestRole","decisionRules":[{"effect":"Permit","permission":[{"attributeName":"Path","attributeValueIncludedIn":["/Tables/*"]}]}],"members":{"fabricItemMembers":[]}}"#,
            "--conflict-policy",
            "Overwrite",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
}

// ─── Show (native endpoint) ─────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
fn onelake_security_show_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "onelake-security",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--role-name",
            "NonExistentRoleName12345",
        ])
        .assert()
        .failure();
}
