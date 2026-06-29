//! End-to-end tests for AI agent safety notices.
//!
//! Tests verify that when an AI agent is detected (via environment variables),
//! error output includes an `agentNotice` field warning the agent not to retry
//! with dangerous flags without user approval.
//!
//! Tests cover:
//! - `agentNotice` appears when agent env var is set AND hint contains dangerous flag
//! - `agentNotice` is absent when no agent env var is set
//! - `agentNotice` is absent when hint does not contain a dangerous flag
//! - `agentNotice` includes the detected agent provider name
//! - Multiple agent env vars: first match wins

mod common;

use common::{TestConfig, fabio};

// ── Helper functions ─────────────────────────────────────────────────────────

/// Helper: run `fabio deploy export` against a real workspace with a non-empty
/// output directory (no `--overwrite`). This triggers an `INVALID_INPUT` error
/// with hint "Use --overwrite to replace existing content." (a dangerous flag).
///
/// Uses `--item-types Datamart` to minimize API calls (Datamarts are list-only,
/// fast response, and every workspace supports them).
fn trigger_overwrite_error_with_env(
    workspace: &str,
    var_name: &str,
    var_value: &str,
) -> assert_cmd::assert::Assert {
    let tmp = tempfile::tempdir().expect("create temp dir");
    std::fs::write(tmp.path().join("dummy.txt"), "content").expect("write dummy file");

    fabio()
        .env(var_name, var_value)
        .args([
            "deploy",
            "export",
            "--workspace",
            workspace,
            "--dir",
            tmp.path().to_str().unwrap(),
            "--item-types",
            "Datamart",
        ])
        .assert()
        .failure()
}

/// Helper: parse stderr JSON error from an assert output.
fn parse_stderr_error(assert: &assert_cmd::assert::Assert) -> serde_json::Value {
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    for line in stderr.lines() {
        if line.starts_with('{') {
            return serde_json::from_str(line).expect("parse stderr JSON");
        }
    }
    panic!("No JSON found in stderr: {stderr}");
}

// ── Offline tests (no live tenant required) ─────────────────────────────────

#[test]
fn agent_notice_absent_when_no_agent_and_non_dangerous_hint() {
    // workspace update without --name or --description fails with INVALID_INPUT
    // and a non-dangerous hint. agentNotice should be absent.
    let assert = fabio()
        .env_remove("CLAUDE_CODE")
        .env_remove("CLAUDECODE")
        .env_remove("CURSOR_AGENT")
        .env_remove("CURSOR_TRACE_ID")
        .env_remove("CODEX")
        .env_remove("CODEX_CLI_AGENT")
        .env_remove("GITHUB_COPILOT")
        .env_remove("COPILOT_CLI")
        .env_remove("OPENCODE_AGENT")
        .env_remove("VSCODE_AGENT")
        .env_remove("WINDSURF_AGENT")
        .env_remove("AIDER_AGENT")
        .env_remove("CLINE_AGENT")
        .env_remove("CONTINUE_AGENT")
        .env_remove("DEVIN_AGENT")
        .args([
            "workspace",
            "update",
            "--id",
            "00000000-0000-0000-0000-000000000001",
        ])
        .assert()
        .failure();

    let error = parse_stderr_error(&assert);
    assert!(
        error["error"]["agentNotice"].is_null(),
        "agentNotice should be absent without agent env: {error}"
    );
}

#[test]
fn agent_notice_absent_when_agent_present_but_hint_not_dangerous() {
    // Even with CLAUDE_CODE set, if the hint doesn't suggest a dangerous flag,
    // agentNotice should be absent.
    let assert = fabio()
        .env("CLAUDE_CODE", "1")
        .args([
            "workspace",
            "update",
            "--id",
            "00000000-0000-0000-0000-000000000001",
        ])
        .assert()
        .failure();

    let error = parse_stderr_error(&assert);
    assert!(
        error["error"]["agentNotice"].is_null(),
        "agentNotice should be absent for non-dangerous hints, got: {error}"
    );
}

#[test]
fn no_agent_notice_for_help_commands() {
    // Successful commands produce no stderr error at all.
    let assert = fabio()
        .env("CLAUDE_CODE", "1")
        .args(["--help"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        !stderr.contains("agentNotice"),
        "No agentNotice on success: {stderr}"
    );
}

#[test]
fn no_agent_notice_for_dry_run_success() {
    // dry-run success should have no error output
    let assert = fabio()
        .env("CLAUDE_CODE", "1")
        .args(["--dry-run", "workspace", "create", "--name", "test-agent"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        !stderr.contains("agentNotice"),
        "No agentNotice on dry-run success: {stderr}"
    );
}

#[test]
fn agent_notice_absent_for_error_without_hint() {
    // deploy validate with non-existent source produces error without hint
    let assert = fabio()
        .env("CLAUDE_CODE", "1")
        .args(["deploy", "validate", "--source", "/nonexistent-path-xyz"])
        .assert()
        .failure();

    let error = parse_stderr_error(&assert);
    assert!(
        error["error"]["agentNotice"].is_null(),
        "agentNotice should be absent when no hint: {error}"
    );
}

// ── Live tests (require Fabric tenant auth) ─────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
fn agent_notice_present_when_agent_detected_and_dangerous_hint() {
    let config = TestConfig::from_env();
    let assert = trigger_overwrite_error_with_env(&config.source_workspace, "CLAUDE_CODE", "1");
    let error = parse_stderr_error(&assert);

    let notice = error["error"]["agentNotice"].as_str();
    assert!(
        notice.is_some(),
        "Expected agentNotice in error output, got: {error}"
    );
    let notice_text = notice.unwrap();
    assert!(
        notice_text.contains("Claude Code"),
        "Notice should mention provider: {notice_text}"
    );
    assert!(
        notice_text.contains("explicitly approved"),
        "Notice should warn about approval: {notice_text}"
    );
    assert!(
        notice_text.contains("irreversible"),
        "Notice should mention irreversibility: {notice_text}"
    );

    // Hint should still be present alongside the notice
    let hint = error["error"]["hint"].as_str().unwrap_or_default();
    assert!(
        hint.contains("--overwrite"),
        "Hint should suggest --overwrite: {hint}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn agent_notice_absent_when_no_agent_env_var_live() {
    let config = TestConfig::from_env();
    let tmp = tempfile::tempdir().expect("create temp dir");
    std::fs::write(tmp.path().join("dummy.txt"), "content").expect("write dummy file");

    let assert = fabio()
        .env_remove("CLAUDE_CODE")
        .env_remove("CLAUDECODE")
        .env_remove("CURSOR_AGENT")
        .env_remove("CURSOR_TRACE_ID")
        .env_remove("CODEX")
        .env_remove("CODEX_CLI_AGENT")
        .env_remove("GITHUB_COPILOT")
        .env_remove("COPILOT_CLI")
        .env_remove("OPENCODE_AGENT")
        .env_remove("VSCODE_AGENT")
        .env_remove("WINDSURF_AGENT")
        .env_remove("AIDER_AGENT")
        .env_remove("CLINE_AGENT")
        .env_remove("CONTINUE_AGENT")
        .env_remove("DEVIN_AGENT")
        .args([
            "deploy",
            "export",
            "--workspace",
            &config.source_workspace,
            "--dir",
            tmp.path().to_str().unwrap(),
            "--item-types",
            "Datamart",
        ])
        .assert()
        .failure();

    let error = parse_stderr_error(&assert);
    assert!(
        error["error"]["agentNotice"].is_null(),
        "agentNotice should be absent without agent: {error}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn agent_notice_with_cursor_agent_shows_cursor_provider() {
    let config = TestConfig::from_env();
    let assert = trigger_overwrite_error_with_env(&config.source_workspace, "CURSOR_AGENT", "1");
    let error = parse_stderr_error(&assert);

    let notice = error["error"]["agentNotice"].as_str();
    assert!(notice.is_some(), "Expected agentNotice: {error}");
    assert!(
        notice.unwrap().contains("Cursor"),
        "Should mention Cursor: {}",
        notice.unwrap()
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn agent_notice_with_opencode_agent_shows_opencode_provider() {
    let config = TestConfig::from_env();
    let assert = trigger_overwrite_error_with_env(&config.source_workspace, "OPENCODE_AGENT", "1");
    let error = parse_stderr_error(&assert);

    let notice = error["error"]["agentNotice"].as_str();
    assert!(notice.is_some(), "Expected agentNotice: {error}");
    assert!(
        notice.unwrap().contains("OpenCode"),
        "Should mention OpenCode: {}",
        notice.unwrap()
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn agent_notice_with_copilot_shows_github_copilot_provider() {
    let config = TestConfig::from_env();
    let assert = trigger_overwrite_error_with_env(&config.source_workspace, "COPILOT_CLI", "1");
    let error = parse_stderr_error(&assert);

    let notice = error["error"]["agentNotice"].as_str();
    assert!(notice.is_some(), "Expected agentNotice: {error}");
    assert!(
        notice.unwrap().contains("GitHub Copilot"),
        "Should mention GitHub Copilot: {}",
        notice.unwrap()
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn agent_notice_with_empty_env_value_still_detects() {
    let config = TestConfig::from_env();
    let assert = trigger_overwrite_error_with_env(&config.source_workspace, "CLAUDE_CODE", "");
    let error = parse_stderr_error(&assert);

    let notice = error["error"]["agentNotice"].as_str();
    assert!(
        notice.is_some(),
        "Empty env value should still detect agent: {error}"
    );
    assert!(notice.unwrap().contains("Claude Code"));
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn agent_notice_first_match_wins_with_multiple_env_vars() {
    let config = TestConfig::from_env();
    let tmp = tempfile::tempdir().expect("create temp dir");
    std::fs::write(tmp.path().join("dummy.txt"), "content").expect("write dummy file");

    let assert = fabio()
        .env("OPENCODE_AGENT", "1")
        .env("CLAUDE_CODE", "1")
        .env("CURSOR_AGENT", "1")
        .args([
            "deploy",
            "export",
            "--workspace",
            &config.source_workspace,
            "--dir",
            tmp.path().to_str().unwrap(),
            "--item-types",
            "Datamart",
        ])
        .assert()
        .failure();

    let error = parse_stderr_error(&assert);
    let notice = error["error"]["agentNotice"].as_str().unwrap();
    // CLAUDE_CODE has highest detection priority
    assert!(
        notice.contains("Claude Code"),
        "Highest priority agent should win: {notice}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn agent_notice_error_structure_preserved() {
    let config = TestConfig::from_env();
    let assert = trigger_overwrite_error_with_env(&config.source_workspace, "CLAUDE_CODE", "1");
    let error = parse_stderr_error(&assert);

    // Standard error fields must be present
    assert!(error["error"]["code"].is_string());
    assert!(error["error"]["message"].is_string());
    assert!(error["error"]["hint"].is_string());
    assert!(error["error"]["agentNotice"].is_string());
    assert_eq!(error["error"]["code"].as_str().unwrap(), "INVALID_INPUT");
}
