//! End-to-end tests for CLI safety features:
//! --readonly, --enable-commands, --disable-commands.
//!
//! These tests are all offline (no live tenant needed) because the safety
//! features block commands before any HTTP/auth calls are made.
//! Commands that succeed use `context agent` which requires no auth.
//! Commands that should be blocked are validated by checking the error JSON on stderr.

mod common;

use common::fabio;
use serde_json::Value;

/// Parse stderr as a JSON error envelope.
fn parse_stderr_error(output: &assert_cmd::assert::Assert) -> Value {
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    serde_json::from_str(&stderr).expect("failed to parse stderr as JSON")
}

// =============================================================================
// --readonly tests
// =============================================================================

#[test]
fn readonly_allows_get_operations() {
    // `context agent` is a read-only command that requires no auth.
    fabio()
        .args(["--readonly", "context", "agent"])
        .assert()
        .success();
}

#[test]
fn readonly_blocks_mutations() {
    // `workspace create` triggers a POST call which --readonly blocks
    // before any network/auth call is made (guard_readonly runs before require_auth).
    let assert = fabio()
        .args(["--readonly", "workspace", "create", "--name", "test"])
        .assert()
        .failure();

    let json = parse_stderr_error(&assert);
    let error = json.get("error").expect("stderr should have 'error' field");
    assert_eq!(
        error["code"].as_str().unwrap(),
        "READONLY_MODE",
        "should produce READONLY_MODE error code"
    );
}

#[test]
fn readonly_env_var() {
    // FABIO_READONLY=true should have the same effect as --readonly flag.
    // (clap bool flags require "true"/"false" string values from env vars)
    let assert = fabio()
        .env("FABIO_READONLY", "true")
        .args(["workspace", "create", "--name", "test-env"])
        .assert()
        .failure();

    let json = parse_stderr_error(&assert);
    let error = &json["error"];
    assert_eq!(error["code"].as_str().unwrap(), "READONLY_MODE");
}

#[test]
fn readonly_allows_help() {
    // --help should always work regardless of --readonly.
    fabio().args(["--readonly", "--help"]).assert().success();
}

#[test]
fn readonly_error_has_hint() {
    let assert = fabio()
        .args(["--readonly", "workspace", "create", "--name", "x"])
        .assert()
        .failure();

    let json = parse_stderr_error(&assert);
    let error = &json["error"];
    let hint = error["hint"]
        .as_str()
        .expect("error should have a 'hint' field");
    assert!(
        hint.contains("FABIO_READONLY") || hint.contains("--readonly"),
        "hint should mention how to disable readonly mode, got: {hint}"
    );
}

// =============================================================================
// --enable-commands tests
// =============================================================================

#[test]
fn enable_commands_allows_listed_group() {
    // `--enable-commands context` should allow `context agent` to run.
    fabio()
        .args(["--enable-commands", "context", "context", "agent"])
        .assert()
        .success();
}

#[test]
fn enable_commands_blocks_unlisted_group() {
    // `--enable-commands context` should block `workspace list`.
    // `workspace list` has no required args, so clap parses it fine and
    // check_command_policy triggers the FORBIDDEN before any auth call.
    let assert = fabio()
        .args(["--enable-commands", "context", "workspace", "list"])
        .assert()
        .failure();

    let json = parse_stderr_error(&assert);
    let error = &json["error"];
    assert_eq!(
        error["code"].as_str().unwrap(),
        "FORBIDDEN",
        "should produce FORBIDDEN error code"
    );
}

#[test]
fn enable_commands_parent_allows_children() {
    // Parent path "context" should allow child "context agent".
    // This verifies the parent-prefix matching logic.
    fabio()
        .args(["--enable-commands", "context", "context", "agent"])
        .assert()
        .success();
}

#[test]
fn enable_commands_env_var() {
    // FABIO_ENABLE_COMMANDS=context should allow `context agent`.
    fabio()
        .env("FABIO_ENABLE_COMMANDS", "context")
        .args(["context", "agent"])
        .assert()
        .success();

    // But should block `workspace list` (no required args, triggers policy check).
    let assert = fabio()
        .env("FABIO_ENABLE_COMMANDS", "context")
        .args(["workspace", "list"])
        .assert()
        .failure();

    let json = parse_stderr_error(&assert);
    assert_eq!(json["error"]["code"].as_str().unwrap(), "FORBIDDEN");
}

// =============================================================================
// --disable-commands tests
// =============================================================================

#[test]
fn disable_commands_blocks_listed_group() {
    // `--disable-commands workspace` should block `workspace list`.
    let assert = fabio()
        .args(["--disable-commands", "workspace", "workspace", "list"])
        .assert()
        .failure();

    let json = parse_stderr_error(&assert);
    assert_eq!(json["error"]["code"].as_str().unwrap(), "FORBIDDEN");
}

#[test]
fn disable_commands_allows_unlisted() {
    // `--disable-commands workspace` should NOT block `context agent`.
    fabio()
        .args(["--disable-commands", "workspace", "context", "agent"])
        .assert()
        .success();
}

#[test]
fn disable_commands_deny_overrides_allow() {
    // Deny should override allow: workspace is in both lists, deny wins.
    let assert = fabio()
        .args([
            "--enable-commands",
            "workspace",
            "--disable-commands",
            "workspace",
            "workspace",
            "list",
        ])
        .assert()
        .failure();

    let json = parse_stderr_error(&assert);
    assert_eq!(json["error"]["code"].as_str().unwrap(), "FORBIDDEN");
    let msg = json["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("disable-commands"),
        "error should mention disable-commands policy, got: {msg}"
    );
}

#[test]
fn disable_commands_env_var() {
    // FABIO_DISABLE_COMMANDS=workspace should block workspace commands.
    let assert = fabio()
        .env("FABIO_DISABLE_COMMANDS", "workspace")
        .args(["workspace", "list"])
        .assert()
        .failure();

    let json = parse_stderr_error(&assert);
    assert_eq!(json["error"]["code"].as_str().unwrap(), "FORBIDDEN");
}

// =============================================================================
// Combined safety tests
// =============================================================================

#[test]
fn readonly_plus_enable_commands() {
    // Both flags together: readonly + enable-commands context.
    // `context agent` is read-only and in the allowlist — should succeed.
    fabio()
        .args([
            "--readonly",
            "--enable-commands",
            "context",
            "context",
            "agent",
        ])
        .assert()
        .success();
}

#[test]
fn readonly_plus_enable_commands_blocks_mutation() {
    // Even if the command group is in the allowlist, --readonly blocks
    // the actual HTTP mutation at the client level.
    let assert = fabio()
        .args([
            "--readonly",
            "--enable-commands",
            "workspace",
            "workspace",
            "create",
            "--name",
            "x",
        ])
        .assert()
        .failure();

    let json = parse_stderr_error(&assert);
    let code = json["error"]["code"].as_str().unwrap();
    assert_eq!(
        code, "READONLY_MODE",
        "readonly should block mutation even when command is in allowlist"
    );
}
