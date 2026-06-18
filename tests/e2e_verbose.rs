//! End-to-end integration tests for the --verbose / -v global flag.
//!
//! Tests verify:
//! - Flag is accepted without error (offline)
//! - HTTP request/response traces appear on stderr (live)
//! - Auth traces appear on stderr (live)
//! - stdout remains valid JSON (not polluted by verbose)
//! - --quiet suppresses verbose output (live)
//! - --dry-run + --verbose produces no HTTP traces (offline)
//! - -v short form works the same as --verbose (live)

mod common;

use common::{TestConfig, fabio, parse_json};
use serial_test::serial;

// ── Offline tests (no live tenant required) ─────────────────────────────────

#[test]
fn verbose_flag_accepted_with_help() {
    fabio().args(["--verbose", "--help"]).assert().success();
}

#[test]
fn verbose_short_flag_accepted_with_help() {
    fabio().args(["-v", "--help"]).assert().success();
}

#[test]
fn verbose_with_dry_run_produces_no_http_trace() {
    // --dry-run returns before making HTTP calls, so no verbose output
    let assert = fabio()
        .args([
            "--verbose",
            "--dry-run",
            "workspace",
            "create",
            "--name",
            "test-verbose-dryrun",
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        !stderr.contains("[verbose][http]"),
        "dry-run should not produce HTTP traces, got: {stderr}"
    );
}

#[test]
fn verbose_does_not_affect_stdout_of_context_agent() {
    // context agent is offline (no HTTP) — stdout should still be valid JSON
    let assert = fabio()
        .args(["--verbose", "context", "agent"])
        .assert()
        .success();

    let json = parse_json(&assert);
    // Should still produce the schema
    assert!(json.get("commands").is_some() || json.get("data").is_some());

    // stderr should not have HTTP traces (context agent is local)
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        !stderr.contains("[verbose][http] -->"),
        "context agent should not produce HTTP traces"
    );
}

#[test]
fn verbose_and_quiet_together_suppresses_verbose_offline() {
    // Even with --verbose, --quiet should suppress all stderr
    let assert = fabio()
        .args(["--verbose", "--quiet", "context", "agent"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        !stderr.contains("[verbose]"),
        "--quiet should suppress all verbose output"
    );
}

// ── Live tests (require Fabric tenant — workspace list needs no env vars) ───

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn verbose_emits_http_request_trace_on_stderr() {
    let assert = fabio()
        .args(["--verbose", "workspace", "list", "--limit", "1"])
        .assert()
        .success();

    // stdout should still be valid JSON envelope
    let json = parse_json(&assert);
    assert!(
        json.get("data").is_some(),
        "stdout should have data envelope"
    );

    // stderr should contain HTTP request trace
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("[verbose][http] -->"),
        "expected HTTP request trace on stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("GET"),
        "expected GET method in HTTP trace, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn verbose_emits_http_response_trace_on_stderr() {
    let assert = fabio()
        .args(["--verbose", "workspace", "list", "--limit", "1"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("[verbose][http] <--"),
        "expected HTTP response trace on stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("200"),
        "expected 200 status in response trace, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn verbose_emits_auth_trace_on_stderr() {
    let assert = fabio()
        .args(["--verbose", "workspace", "list", "--limit", "1"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("[verbose][auth]"),
        "expected auth trace on stderr, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn verbose_shows_response_timing_ms() {
    let assert = fabio()
        .args(["--verbose", "workspace", "list", "--limit", "1"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    // Response line has format: [verbose][http] <-- 200 <url> (NNms)
    assert!(
        stderr.contains("ms)"),
        "expected timing in response trace, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn verbose_shows_fabric_api_url() {
    let assert = fabio()
        .args(["--verbose", "workspace", "list", "--limit", "1"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("api.fabric.microsoft.com"),
        "expected Fabric API URL in trace, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn verbose_short_flag_works_same_as_long() {
    let assert = fabio()
        .args(["-v", "workspace", "list", "--limit", "1"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("[verbose][http] -->"),
        "-v should produce same traces as --verbose"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn quiet_suppresses_verbose_on_live_request() {
    let assert = fabio()
        .args(["--verbose", "--quiet", "workspace", "list", "--limit", "1"])
        .assert()
        .success();

    // stdout is suppressed by --quiet
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.is_empty(), "stdout should be empty with --quiet");

    // stderr should NOT have verbose traces
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        !stderr.contains("[verbose]"),
        "--quiet should suppress verbose even on live requests, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn verbose_does_not_corrupt_json_output() {
    let assert = fabio()
        .args(["--verbose", "workspace", "list", "--limit", "2"])
        .assert()
        .success();

    // stdout must still be valid parseable JSON
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert!(data.is_array(), "workspace list data should be an array");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn verbose_shows_pagination_traces_with_all() {
    // With --all, multiple GET requests should appear
    let assert = fabio()
        .args(["--verbose", "--all", "workspace", "list"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    // Should have at least one GET trace for the list
    assert!(
        stderr.contains("[verbose][http] --> GET"),
        "expected GET trace for pagination, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn verbose_on_not_found_shows_trace_before_error() {
    // A request that will fail with 404 should still show the HTTP trace
    let assert = fabio()
        .args([
            "--verbose",
            "workspace",
            "show",
            "--id",
            "00000000-0000-0000-0000-000000000099",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("[verbose][http] --> GET"),
        "expected request trace even on failure, got: {stderr}"
    );
    assert!(
        stderr.contains("[verbose][http] <--"),
        "expected response trace even on failure, got: {stderr}"
    );
}

// ── Live tests requiring TestConfig (specific workspace/lakehouse) ───────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn verbose_on_lakehouse_list_tables() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "--verbose",
            "lakehouse",
            "list-tables",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("[verbose][http]"),
        "expected HTTP traces on lakehouse list-tables, got: {stderr}"
    );
}
