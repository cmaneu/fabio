//! End-to-end integration tests for `fabio rti` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

// ─── NL-to-KQL ──────────────────────────────────────────────────────────────

#[test]
fn rti_nl_to_kql_dry_run() {
    let assert = fabio()
        .args([
            "--dry-run",
            "rti",
            "nl-to-kql",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--item-id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--cluster-url",
            "https://test.kusto.fabric.microsoft.com",
            "--database-name",
            "TestDB",
            "--question",
            "Show me all events from yesterday",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "rti nl-to-kql");
    assert_eq!(
        data["details"]["naturalLanguage"],
        "Show me all events from yesterday"
    );
    assert_eq!(data["details"]["databaseName"], "TestDB");
}

#[test]
fn rti_nl_to_kql_dry_run_with_optional_fields() {
    let assert = fabio()
        .args([
            "--dry-run",
            "rti",
            "nl-to-kql",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--item-id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--cluster-url",
            "https://test.kusto.fabric.microsoft.com",
            "--database-name",
            "TestDB",
            "--question",
            "Count events by type",
            "--user-shots",
            r#"[{"question":"count rows","kql":"Events | count"}]"#,
            "--chat-messages",
            r#"[{"role":"user","content":"hello"}]"#,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data["details"]["userShots"].is_array());
    assert!(data["details"]["chatMessages"].is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn rti_nl_to_kql_invalid_item() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "rti",
            "nl-to-kql",
            "--workspace",
            &cfg.source_workspace,
            "--item-id",
            "00000000-0000-0000-0000-000000000000",
            "--cluster-url",
            "https://test.kusto.fabric.microsoft.com",
            "--database-name",
            "TestDB",
            "--question",
            "Show all events",
        ])
        .assert()
        .failure();
}
