//! End-to-end integration tests for agent-native CLI features:
//! agent-context, profile, jobs, feedback, --json, --dry-run, --limit, --force.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

// =============================================================================
// agent-context command (Principle 7: Three-layer introspection)
// =============================================================================

#[test]
fn agent_context_returns_schema() {
    let assert = fabio().args(["agent-context"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Should have required top-level schema fields
    assert!(
        data.get("schema_version").is_some(),
        "missing schema_version"
    );
    assert!(data.get("commands").is_some(), "missing commands field");
    assert!(data.get("global_flags").is_some(), "missing global_flags");

    // Commands should be a non-empty object
    let commands = data["commands"]
        .as_object()
        .expect("commands should be object");
    assert!(!commands.is_empty(), "commands should not be empty");

    // Each command should have description
    let workspace = commands
        .get("workspace")
        .expect("should have workspace command");
    assert!(
        workspace.get("description").is_some(),
        "command missing description"
    );
}

#[test]
fn agent_context_with_json_flag() {
    let assert = fabio().args(["--json", "agent-context"]).assert().success();

    let json = parse_json(&assert);
    assert!(json.get("data").is_some());
}

#[test]
fn agent_context_with_query_extracts_version() {
    let assert = fabio()
        .args(["--query", "schema_version", "agent-context"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should be a version string
    assert!(data.is_string(), "version should be a string");
}

// =============================================================================
// --json global flag (Principle 2: Structured output)
// =============================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn json_flag_produces_json_output() {
    let assert = fabio()
        .args(["--json", "workspace", "list"])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn json_flag_overrides_table_output() {
    // --json should override --output table
    let assert = fabio()
        .args(["--output", "table", "--json", "workspace", "list"])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert!(json.get("data").is_some(), "--json should override table");
}

// =============================================================================
// --dry-run global flag (Principle 4: Safe retries)
// =============================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dry_run_workspace_create_does_not_create() {
    let assert = fabio()
        .args([
            "--dry-run",
            "workspace",
            "create",
            "--name",
            "test-should-not-exist",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "workspace create");
    assert!(data.get("details").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dry_run_workspace_delete_does_not_delete() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "--dry-run",
            "workspace",
            "delete",
            "--id",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "workspace delete");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dry_run_item_create_does_not_create() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "--dry-run",
            "item",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "dry-run-item",
            "--type",
            "Notebook",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "item create");
}

// =============================================================================
// --limit global flag (Principle 5: Bounded responses)
// =============================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn limit_flag_truncates_list() {
    let assert = fabio()
        .args(["--limit", "1", "workspace", "list"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let count = json["count"].as_u64().unwrap();
    assert_eq!(count, 1, "should return exactly 1 item");

    // If there are more workspaces, truncated should be true
    if json.get("truncated").is_some() {
        assert_eq!(json["truncated"], true);
        assert!(json["total_available"].as_u64().unwrap() > 1);
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn all_flag_fetches_all_pages() {
    // --all should return all items without a continuationToken
    let assert = fabio()
        .args(["--all", "workspace", "list"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let count = json["count"].as_u64().unwrap();
    assert!(count >= 1, "should return at least 1 workspace");
    // With --all, no continuationToken should be present (all pages fetched)
    assert!(
        json.get("continuationToken").is_none(),
        "should not have continuationToken when --all is used"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn all_flag_with_limit_truncates_after_fetching_all() {
    // --all --limit 1: fetches all pages, then truncates to 1
    let assert = fabio()
        .args(["--all", "--limit", "1", "workspace", "list"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let count = json["count"].as_u64().unwrap();
    assert_eq!(count, 1, "should return exactly 1 item with --limit 1");
}

// =============================================================================
// profile command (Principle 9: Persistent identity)
// =============================================================================

#[test]
fn profile_list_empty() {
    // With no profiles, should return empty list
    let assert = fabio().args(["profile", "list"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
fn profile_save_and_show_and_delete() {
    // Save a test profile
    let assert = fabio()
        .args([
            "profile",
            "save",
            "--name",
            "test-ci-profile",
            "--default-output",
            "table",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["name"], "test-ci-profile");

    // Show it
    let assert = fabio()
        .args(["profile", "show", "--name", "test-ci-profile"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["name"], "test-ci-profile");
    assert_eq!(data["output"], "table");

    // Delete it
    let assert = fabio()
        .args(["profile", "delete", "--name", "test-ci-profile"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
}

// =============================================================================
// jobs command (Principle 8: Async-aware execution)
// =============================================================================

#[test]
fn jobs_list_empty() {
    let assert = fabio().args(["jobs", "list"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
fn jobs_get_nonexistent_fails() {
    let assert = fabio()
        .args(["jobs", "get", "--id", "nonexistent-id-12345"])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("NOT_FOUND") || stderr.contains("not found"));
}

#[test]
fn jobs_prune_succeeds_on_empty() {
    let assert = fabio().args(["jobs", "prune"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.get("pruned").is_some());
}

// =============================================================================
// feedback command (Principle 10: Two-way I/O)
// =============================================================================

#[test]
fn feedback_send_records_message() {
    let assert = fabio()
        .args(["feedback", "send", "test feedback from e2e - please ignore"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "recorded");
}

#[test]
fn feedback_list_returns_array() {
    let assert = fabio().args(["feedback", "list"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

// =============================================================================
// --quiet flag
// =============================================================================

#[test]
fn quiet_flag_suppresses_output() {
    let assert = fabio()
        .args(["--quiet", "agent-context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.is_empty(), "stdout should be empty with --quiet");
}

// =============================================================================
// Error hint in structured errors (Principle 3)
// =============================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn error_includes_hint_field() {
    // Attempt to load table with invalid mode → should get hint
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "lakehouse",
            "load-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--table",
            "nonexistent",
            "--source-path",
            "Files/test.csv",
            "--mode",
            "invalid_mode",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value =
        serde_json::from_str(stderr.trim()).expect("stderr should be JSON");
    let error = &err_json["error"];
    assert_eq!(error["code"], "INVALID_INPUT");
    assert!(error.get("hint").is_some(), "error should include hint");
    let hint = error["hint"].as_str().unwrap();
    assert!(
        hint.contains("Overwrite"),
        "hint should enumerate valid values"
    );
}

// =============================================================================
// Integration: notebook run writes to job ledger (Principle 8)
// =============================================================================

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn notebook_run_dry_run_does_not_start() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "--dry-run",
            "notebook",
            "run",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.notebook_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "notebook run");
}
