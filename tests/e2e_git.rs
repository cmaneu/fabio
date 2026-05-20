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

/// Retry a fabio command up to 3 times with a 10-second delay between attempts.
/// Returns the last assertion result. Used for transient "Git provider failed" errors.
fn retry_on_failure<F>(f: F) -> assert_cmd::assert::Assert
where
    F: Fn() -> assert_cmd::assert::Assert,
{
    let mut last_assert = f();
    for _ in 0..2 {
        if last_assert.get_output().status.success() {
            return last_assert;
        }
        std::thread::sleep(std::time::Duration::from_secs(10));
        last_assert = f();
    }
    last_assert
}

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
            let conn_type = c
                .get("connectionDetails")
                .and_then(|d| d.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("");
            conn_type == "GitHubSourceControl" || conn_type == "GitHub"
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

    // Initialize connection (prefer-workspace to handle case where both sides have content)
    let assert = fabio()
        .args([
            "git",
            "init",
            "--workspace",
            workspace,
            "--strategy",
            "prefer-workspace",
        ])
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
    // Status renders as list (array of changes) or object (with workspaceHead)
    assert!(
        data.is_array() || data.get("workspaceHead").is_some() || data.get("changes").is_some(),
        "Status should contain workspaceHead, changes, or be a changes array: {data}"
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

// ---------------------------------------------------------------------------
// Full commit/pull lifecycle with real workspace changes
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_commit_pull_lifecycle() {
    let cfg = TestConfig::from_env();
    let workspace = &cfg.dest_workspace;

    // Auto-discover GitHub connection
    let Some(connection_id) = find_github_connection_id() else {
        eprintln!("No GitHub connection found, skipping test.");
        return;
    };

    // Ensure workspace is disconnected
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();

    // Connect
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

    // Initialize with prefer-workspace strategy (retry for transient errors)
    let init_args = [
        "git",
        "init",
        "--workspace",
        workspace,
        "--strategy",
        "prefer-workspace",
        "--wait",
    ];
    retry_on_failure(|| {
        fabio()
            .args(init_args)
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    // Create a notebook (to generate a workspace change)
    let test_name = format!(
        "git_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            workspace,
            "--name",
            &test_name,
            "--content",
            "# Test notebook for git commit test",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    // Status should show the new notebook as Added
    let assert = fabio()
        .args(["git", "status", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should be an array with at least one Added item
    assert!(data.is_array(), "Expected changes array: {data}");
    let changes = data.as_array().unwrap();
    let has_our_notebook = changes.iter().any(|c| {
        c.get("itemMetadata")
            .and_then(|m| m.get("displayName"))
            .and_then(|n| n.as_str())
            == Some(test_name.as_str())
    });
    assert!(has_our_notebook, "Expected our notebook in changes: {data}");

    // Commit the change (with retry for transient "Git provider failed" errors)
    let commit_args = [
        "git",
        "commit",
        "--workspace",
        workspace,
        "--all",
        "--message",
        &format!("Add {test_name} notebook"),
        "--wait",
    ];
    let assert = retry_on_failure(|| {
        fabio()
            .args(commit_args)
            .timeout(std::time::Duration::from_secs(180))
            .assert()
    });
    let assert = assert.success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "Succeeded");

    // Status should be clean after commit
    let assert = fabio()
        .args(["git", "status", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Clean status: object with workspaceHead and empty changes
    assert!(
        data.get("workspaceHead").is_some(),
        "Expected clean status with workspaceHead: {data}"
    );
    let changes = data.get("changes").and_then(|c| c.as_array());
    assert!(
        changes.is_none() || changes.unwrap().is_empty(),
        "Expected empty changes after commit: {data}"
    );

    // Credentials should show configured connection
    let assert = fabio()
        .args(["git", "credentials", "show", "--workspace", workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["source"], "ConfiguredConnection");
    assert_eq!(data["connectionId"], connection_id.as_str());

    // Clean up: delete the test notebook
    // First, find its ID from workspace items
    let assert = fabio()
        .args(["item", "list", "--workspace", workspace, "-o", "json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let items = json["data"].as_array().unwrap();
    if let Some(nb) = items
        .iter()
        .find(|i| i.get("displayName").and_then(|n| n.as_str()) == Some(test_name.as_str()))
    {
        let nb_id = nb["id"].as_str().unwrap();
        fabio()
            .args(["item", "delete", "--workspace", workspace, "--id", nb_id])
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success();

        // Commit the deletion (may fail transiently - don't assert)
        let _ = fabio()
            .args([
                "git",
                "commit",
                "--workspace",
                workspace,
                "--all",
                "--message",
                &format!("Clean up: delete {test_name}"),
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(180))
            .assert();
    }

    // Disconnect
    fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}
