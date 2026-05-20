//! End-to-end integration tests for `fabio git` commands.
//!
//! Tests exercise the compiled binary against a live Microsoft Fabric tenant.
//! Requires valid Azure credentials and `FABIO_TEST_*` environment variables.
//!
//! Git tests cover:
//! - Connection show (always works)
//! - Status on unconnected workspace (expected error)
//! - Connect → Init → Status → Disconnect lifecycle
//!
//! The lifecycle test uses `iemejia/fabio-test-connection` on GitHub.
//! It auto-discovers a GitHub connection from the tenant (via `fabio connection list`)
//! or falls back to `FABIO_TEST_GIT_CONNECTION_ID` env var.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

/// Discover a GitHub connection ID from the tenant.
///
/// Tries (in order):
/// 1. `FABIO_TEST_GIT_CONNECTION_ID` environment variable
/// 2. First connection with type "GitHub" from `fabio connection list`
///
/// Returns `None` if no GitHub connection is available.
fn find_github_connection_id() -> Option<String> {
    // Check env var first
    if let Ok(id) = std::env::var("FABIO_TEST_GIT_CONNECTION_ID") {
        if !id.is_empty() {
            return Some(id);
        }
    }

    // Auto-discover from tenant
    let output = fabio()
        .args(["connection", "list", "-o", "json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).ok()?;
    let connections = json.get("data")?.as_array()?;

    connections
        .iter()
        .find(|c| {
            c.get("connectionDetails")
                .and_then(|d| d.get("type"))
                .and_then(|t| t.as_str())
                == Some("GitHub")
        })
        .and_then(|c| c.get("id"))
        .and_then(|id| id.as_str())
        .map(String::from)
}

// ---------------------------------------------------------------------------
// Connection show (works regardless of connection state)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_connection_show() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "git",
            "connection",
            "show",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should have gitConnectionState field
    assert!(data.get("gitConnectionState").is_some());
}

// ---------------------------------------------------------------------------
// Status on unconnected workspace returns error
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_status_unconnected_workspace_fails() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["git", "status", "--workspace", &cfg.source_workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("not connected") || stderr.contains("Not"),
        "Expected 'not connected' error, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Commit on unconnected workspace fails
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_commit_unconnected_workspace_fails() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "git",
            "commit",
            "--workspace",
            &cfg.source_workspace,
            "--all",
            "--message",
            "test commit",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// Pull on unconnected workspace fails
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_pull_unconnected_workspace_fails() {
    let cfg = TestConfig::from_env();

    fabio()
        .args(["git", "pull", "--workspace", &cfg.source_workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// Connect → Init → Status → Disconnect lifecycle
// Uses the dest workspace (to avoid disrupting source workspace)
// Auto-discovers GitHub connection from tenant or uses FABIO_TEST_GIT_CONNECTION_ID
// Target repo: iemejia/fabio-test-connection
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_connect_init_status_disconnect_lifecycle() {
    let cfg = TestConfig::from_env();
    let workspace = &cfg.dest_workspace;

    // Auto-discover or use env var for GitHub connection
    let Some(connection_id) = find_github_connection_id() else {
        eprintln!(
            "No GitHub connection found in tenant and FABIO_TEST_GIT_CONNECTION_ID not set.\n\
             Create a GitHub connection in the Fabric UI (Settings > Manage connections) \
             or set FABIO_TEST_GIT_CONNECTION_ID to skip this test."
        );
        return;
    };

    // First ensure workspace is disconnected (ignore error if already disconnected)
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();

    // Verify disconnected state
    let assert = fabio()
        .args(["git", "connection", "show", "--workspace", workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(
        data["gitConnectionState"], "NotConnected",
        "Workspace should be disconnected before test"
    );

    // Connect to GitHub repo (iemejia/fabio-test-connection)
    let assert = fabio()
        .args([
            "git",
            "connect",
            "--workspace",
            workspace,
            "--provider",
            "github",
            "--owner",
            "iemejia",
            "--repo",
            "fabio-test-connection",
            "--branch",
            "main",
            "--connection-id",
            &connection_id,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "connected");

    // Verify connected state
    let assert = fabio()
        .args(["git", "connection", "show", "--workspace", workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["gitConnectionState"], "Connected");
    assert!(
        data.get("gitProviderDetails")
            .and_then(|d| d.get("repositoryName"))
            .is_some()
    );
    assert_eq!(
        data["gitProviderDetails"]["repositoryName"],
        "fabio-test-connection"
    );

    // Initialize connection
    let assert = fabio()
        .args(["git", "init", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "initialized");

    // Get status (should work now)
    let assert = fabio()
        .args(["git", "status", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should have workspaceHead or changes array
    assert!(
        data.get("workspaceHead").is_some() || data.get("changes").is_some(),
        "Status should contain workspaceHead or changes: {data}"
    );

    // Disconnect
    let assert = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "disconnected");

    // Verify disconnected
    let assert = fabio()
        .args(["git", "connection", "show", "--workspace", workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["gitConnectionState"], "NotConnected");
}

// ---------------------------------------------------------------------------
// Checkout (switch) requires connected workspace
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_checkout_unconnected_fails() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "git",
            "checkout",
            "--workspace",
            &cfg.source_workspace,
            "--branch",
            "some-branch",
            "--provider",
            "github",
            "--owner",
            "iemejia",
            "--repo",
            "fabio-test-connection",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// Switch alias works same as checkout
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_switch_alias_works() {
    let cfg = TestConfig::from_env();

    // switch on unconnected workspace should fail the same way as checkout
    let assert = fabio()
        .args([
            "git",
            "switch",
            "--workspace",
            &cfg.source_workspace,
            "--branch",
            "some-branch",
            "--provider",
            "github",
            "--owner",
            "iemejia",
            "--repo",
            "fabio-test-connection",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    // Should fail because workspace is not connected (disconnect step will fail/succeed)
    assert!(!stderr.is_empty());
}

// ---------------------------------------------------------------------------
// Credentials show (may return error if no credentials configured)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_credentials_show() {
    let cfg = TestConfig::from_env();

    // Credentials show - may succeed or fail depending on configuration
    // We just verify the command runs and returns structured output
    let assert = fabio()
        .args([
            "git",
            "credentials",
            "show",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should produce valid JSON on either stdout or stderr
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_ok()
            || serde_json::from_str::<serde_json::Value>(&stderr).is_ok(),
        "Expected JSON output, got stdout={stdout}, stderr={stderr}"
    );
}

// ---------------------------------------------------------------------------
// Connect with Azure DevOps requires --org and --project
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn git_connect_azdo_requires_org_and_project() {
    // Clap should accept the command but the API will reject without proper auth
    // This tests that the command structure is correct
    fabio()
        .args([
            "git",
            "connect",
            "--workspace",
            "fake-workspace-id",
            "--provider",
            "azure-devops",
            "--repo",
            "test-repo",
            "--branch",
            "main",
            // Missing --org and --project: command should still parse
            // but fail at runtime with a validation error
        ])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// Connect with GitHub requires --owner
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn git_connect_github_requires_owner() {
    // Missing --owner should fail at runtime validation
    fabio()
        .args([
            "git",
            "connect",
            "--workspace",
            "fake-workspace-id",
            "--provider",
            "github",
            "--repo",
            "test-repo",
            "--branch",
            "main",
        ])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .failure();
}
