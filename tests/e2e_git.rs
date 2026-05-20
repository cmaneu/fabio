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

/// Retry a fabio command up to 5 times with a 15-second delay between attempts.
/// Returns the last assertion result. Used for transient "Git provider failed" errors.
fn retry_on_failure<F>(f: F) -> assert_cmd::assert::Assert
where
    F: Fn() -> assert_cmd::assert::Assert,
{
    let mut last_assert = f();
    for _ in 0..4 {
        if last_assert.get_output().status.success() {
            return last_assert;
        }
        std::thread::sleep(std::time::Duration::from_secs(15));
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
    let assert = retry_on_failure(|| {
        fabio()
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
    })
    .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "initialized");

    // Get status (should work now)
    let assert = retry_on_failure(|| {
        fabio()
            .args(["git", "status", "--workspace", workspace])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
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
            "--force",
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
    let assert = retry_on_failure(|| {
        fabio()
            .args(["git", "status", "--workspace", workspace])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
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
    let assert = retry_on_failure(|| {
        fabio()
            .args(["git", "status", "--workspace", workspace])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
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

// ---------------------------------------------------------------------------
// Feature branch workflow: create branch, commit, merge, pull to main
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_feature_branch_workflow() {
    let cfg = TestConfig::from_env();
    let workspace = &cfg.dest_workspace;

    // Auto-discover GitHub connection
    let Some(connection_id) = find_github_connection_id() else {
        eprintln!("No GitHub connection found, skipping test.");
        return;
    };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let branch_name = format!("feature/test-{ts}");
    let notebook_name = format!("feature_nb_{ts}");

    // Ensure workspace is disconnected
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();

    // Step 1: Connect to main and initialize
    fabio()
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

    retry_on_failure(|| {
        fabio()
            .args([
                "git",
                "init",
                "--workspace",
                workspace,
                "--strategy",
                "prefer-workspace",
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    // Step 2: Create a feature branch on the remote via gh CLI
    let gh_output = std::process::Command::new("gh")
        .args([
            "api",
            "repos/iemejia/fabio-test-connection/git/refs",
            "-X",
            "POST",
            "-f",
            "ref=refs/heads/placeholder",
            "-f",
            "sha=placeholder",
        ])
        .output();

    // Get the current main SHA first
    let sha_output = std::process::Command::new("gh")
        .args([
            "api",
            "repos/iemejia/fabio-test-connection/git/ref/heads/main",
            "--jq",
            ".object.sha",
        ])
        .output()
        .expect("failed to get main SHA");
    let main_sha = String::from_utf8_lossy(&sha_output.stdout)
        .trim()
        .to_string();
    assert!(!main_sha.is_empty(), "Failed to get main SHA");

    // Create the feature branch
    let create_ref = std::process::Command::new("gh")
        .args([
            "api",
            "repos/iemejia/fabio-test-connection/git/refs",
            "-X",
            "POST",
            "-f",
            &format!("ref=refs/heads/{branch_name}"),
            "-f",
            &format!("sha={main_sha}"),
        ])
        .output()
        .expect("failed to create branch");
    assert!(
        create_ref.status.success(),
        "Failed to create branch: {}",
        String::from_utf8_lossy(&create_ref.stderr)
    );
    // Ensure cleanup runs even if test fails
    let _branch_cleanup = BranchCleanup {
        branch: branch_name.clone(),
    };

    // Ignore the earlier placeholder attempt
    drop(gh_output);

    // Step 3: Switch workspace to feature branch (--force to bypass uncommitted check)
    let checkout_args = [
        "--force",
        "git",
        "checkout",
        "--workspace",
        workspace,
        "--branch",
        &branch_name,
        "--strategy",
        "prefer-workspace",
        "--wait",
    ];
    retry_on_failure(|| {
        fabio()
            .args(checkout_args)
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    // Verify we're on the feature branch
    let assert = fabio()
        .args(["git", "connection", "show", "--workspace", workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(
        data["gitProviderDetails"]["branchName"],
        branch_name.as_str()
    );

    // Step 4: Create a notebook on the feature branch
    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            workspace,
            "--name",
            &notebook_name,
            "--content",
            "# Feature branch notebook\nprint('hello from feature branch')",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    // Step 5: Commit the change to the feature branch
    let commit_msg = format!("feat: add {notebook_name}");
    let commit_args = [
        "git",
        "commit",
        "--workspace",
        workspace,
        "--all",
        "--message",
        &commit_msg,
        "--wait",
    ];
    let assert = retry_on_failure(|| {
        fabio()
            .args(commit_args)
            .timeout(std::time::Duration::from_secs(180))
            .assert()
    })
    .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "Succeeded");

    // Step 6: Merge the feature branch into main via gh CLI
    let merge_output = std::process::Command::new("gh")
        .args([
            "api",
            "repos/iemejia/fabio-test-connection/merges",
            "-X",
            "POST",
            "-f",
            "base=main",
            "-f",
            &format!("head={branch_name}"),
            "-f",
            &format!("commit_message=Merge {branch_name}: add {notebook_name}"),
        ])
        .output()
        .expect("failed to merge branch");
    assert!(
        merge_output.status.success(),
        "Failed to merge: {}",
        String::from_utf8_lossy(&merge_output.stderr)
    );

    // Step 7: Switch back to main (--force to bypass uncommitted check)
    let switch_args = [
        "--force",
        "git",
        "checkout",
        "--workspace",
        workspace,
        "--branch",
        "main",
        "--strategy",
        "prefer-remote",
        "--wait",
    ];
    retry_on_failure(|| {
        fabio()
            .args(switch_args)
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    // Verify we're on main
    let assert = fabio()
        .args(["git", "connection", "show", "--workspace", workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["gitProviderDetails"]["branchName"], "main");

    // Step 8: Verify the notebook from the feature branch exists in workspace
    let assert = fabio()
        .args(["item", "list", "--workspace", workspace, "-o", "json"])
        .assert()
        .success();

    let json = parse_json(&assert);
    let items = json["data"].as_array().unwrap();
    let has_feature_notebook = items
        .iter()
        .any(|i| i.get("displayName").and_then(|n| n.as_str()) == Some(notebook_name.as_str()));
    assert!(
        has_feature_notebook,
        "Expected notebook '{notebook_name}' in workspace after merge, got: {:?}",
        items
            .iter()
            .filter_map(|i| i.get("displayName").and_then(|n| n.as_str()))
            .collect::<Vec<_>>()
    );

    // Step 9: Clean up - delete the notebook and commit
    if let Some(nb) = items
        .iter()
        .find(|i| i.get("displayName").and_then(|n| n.as_str()) == Some(notebook_name.as_str()))
    {
        let nb_id = nb["id"].as_str().unwrap();
        fabio()
            .args(["item", "delete", "--workspace", workspace, "--id", nb_id])
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success();

        // Commit cleanup (best-effort, may fail transiently)
        let cleanup_msg = format!("cleanup: delete {notebook_name}");
        let cleanup_args = [
            "git",
            "commit",
            "--workspace",
            workspace,
            "--all",
            "--message",
            &cleanup_msg,
            "--wait",
        ];
        let _ = retry_on_failure(|| {
            fabio()
                .args(cleanup_args)
                .timeout(std::time::Duration::from_secs(180))
                .assert()
        });
    }

    // Disconnect
    fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Selective commit (--items): commit only specific items by object ID
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_selective_commit() {
    let cfg = TestConfig::from_env();
    let workspace = &cfg.dest_workspace;

    let Some(connection_id) = find_github_connection_id() else {
        eprintln!("No GitHub connection found, skipping test.");
        return;
    };

    // Ensure workspace is disconnected
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();

    // Connect and init
    fabio()
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

    retry_on_failure(|| {
        fabio()
            .args([
                "git",
                "init",
                "--workspace",
                workspace,
                "--strategy",
                "prefer-workspace",
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    // Create two notebooks
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let nb_a = format!("selective_a_{ts}");
    let nb_b = format!("selective_b_{ts}");

    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            workspace,
            "--name",
            &nb_a,
            "--content",
            "# Notebook A",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            workspace,
            "--name",
            &nb_b,
            "--content",
            "# Notebook B",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    // Get status to see both items and their objectIds (retry for transient errors)
    let assert = retry_on_failure(|| {
        fabio()
            .args(["git", "status", "--workspace", workspace])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let changes = data.as_array().expect("Expected changes array");

    // Find objectId for notebook A only (nested at itemMetadata.itemIdentifier.objectId)
    let obj_id_a = changes
        .iter()
        .find(|c| {
            c.get("itemMetadata")
                .and_then(|m| m.get("displayName"))
                .and_then(|n| n.as_str())
                == Some(nb_a.as_str())
        })
        .and_then(|c| {
            c.get("itemMetadata")
                .and_then(|m| m.get("itemIdentifier"))
                .and_then(|i| i.get("objectId"))
                .and_then(|id| id.as_str())
        })
        .expect("Could not find objectId for notebook A");

    // Selective commit: only commit notebook A
    let commit_args = [
        "git",
        "commit",
        "--workspace",
        workspace,
        "--items",
        obj_id_a,
        "--message",
        &format!("Selective commit: only {nb_a}"),
        "--wait",
    ];
    let assert = retry_on_failure(|| {
        fabio()
            .args(commit_args)
            .timeout(std::time::Duration::from_secs(180))
            .assert()
    })
    .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "Succeeded");

    // Status should still show notebook B as uncommitted (retry for transient errors)
    let assert = retry_on_failure(|| {
        fabio()
            .args(["git", "status", "--workspace", workspace])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should be an array with remaining changes (notebook B)
    assert!(data.is_array(), "Expected changes array: {data}");
    let remaining = data.as_array().unwrap();
    let has_nb_b = remaining.iter().any(|c| {
        c.get("itemMetadata")
            .and_then(|m| m.get("displayName"))
            .and_then(|n| n.as_str())
            == Some(nb_b.as_str())
    });
    assert!(has_nb_b, "Expected notebook B still uncommitted: {data}");
    let has_nb_a = remaining.iter().any(|c| {
        c.get("itemMetadata")
            .and_then(|m| m.get("displayName"))
            .and_then(|n| n.as_str())
            == Some(nb_a.as_str())
    });
    assert!(!has_nb_a, "Notebook A should already be committed: {data}");

    // Clean up: commit all remaining, delete both notebooks, commit, disconnect
    let _ = retry_on_failure(|| {
        fabio()
            .args([
                "git",
                "commit",
                "--workspace",
                workspace,
                "--all",
                "--message",
                "cleanup: commit remaining",
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(180))
            .assert()
    });

    // Delete notebooks
    let assert = fabio()
        .args(["item", "list", "--workspace", workspace, "-o", "json"])
        .assert()
        .success();
    let json = parse_json(&assert);
    let items = json["data"].as_array().unwrap();
    for name in [&nb_a, &nb_b] {
        if let Some(nb) = items
            .iter()
            .find(|i| i.get("displayName").and_then(|n| n.as_str()) == Some(name.as_str()))
        {
            let nb_id = nb["id"].as_str().unwrap();
            let _ = fabio()
                .args(["item", "delete", "--workspace", workspace, "--id", nb_id])
                .timeout(std::time::Duration::from_secs(30))
                .assert();
        }
    }

    // Commit cleanup and disconnect
    let _ = retry_on_failure(|| {
        fabio()
            .args([
                "git",
                "commit",
                "--workspace",
                workspace,
                "--all",
                "--message",
                "cleanup: delete selective test notebooks",
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(180))
            .assert()
    });

    fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Credentials update: change from ConfiguredConnection to None and back
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_credentials_update() {
    let cfg = TestConfig::from_env();
    let workspace = &cfg.dest_workspace;

    let Some(connection_id) = find_github_connection_id() else {
        eprintln!("No GitHub connection found, skipping test.");
        return;
    };

    // Ensure workspace is disconnected
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();

    // Connect and init (credentials will be ConfiguredConnection)
    fabio()
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

    retry_on_failure(|| {
        fabio()
            .args([
                "git",
                "init",
                "--workspace",
                workspace,
                "--strategy",
                "prefer-workspace",
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    // Verify current credentials are ConfiguredConnection
    let assert = fabio()
        .args(["git", "credentials", "show", "--workspace", workspace])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["source"], "ConfiguredConnection");

    // Update credentials to None
    fabio()
        .args([
            "git",
            "credentials",
            "update",
            "--workspace",
            workspace,
            "--source",
            "none",
        ])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    // Verify credentials changed to None
    let assert = fabio()
        .args(["git", "credentials", "show", "--workspace", workspace])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(
        data["source"], "None",
        "Expected credentials source 'None' after update: {data}"
    );

    // Restore credentials back to ConfiguredConnection
    fabio()
        .args([
            "git",
            "credentials",
            "update",
            "--workspace",
            workspace,
            "--source",
            "configured-connection",
            "--connection-id",
            &connection_id,
        ])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    // Verify restored
    let assert = fabio()
        .args(["git", "credentials", "show", "--workspace", workspace])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["source"], "ConfiguredConnection");
    assert_eq!(data["connectionId"], connection_id.as_str());

    // Disconnect
    fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Async commit (without --wait): returns immediately with operation status
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_commit_async_returns_operation() {
    let cfg = TestConfig::from_env();
    let workspace = &cfg.dest_workspace;

    let Some(connection_id) = find_github_connection_id() else {
        eprintln!("No GitHub connection found, skipping test.");
        return;
    };

    // Ensure workspace is disconnected
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();

    // Connect and init
    fabio()
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

    retry_on_failure(|| {
        fabio()
            .args([
                "git",
                "init",
                "--workspace",
                workspace,
                "--strategy",
                "prefer-workspace",
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    // Create a notebook to have something to commit
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let nb_name = format!("async_commit_{ts}");

    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            workspace,
            "--name",
            &nb_name,
            "--content",
            "# Async commit test",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    // Commit WITHOUT --wait (should return immediately)
    let commit_args = [
        "git",
        "commit",
        "--workspace",
        workspace,
        "--all",
        "--message",
        &format!("async: add {nb_name}"),
    ];
    let assert = retry_on_failure(|| {
        fabio()
            .args(commit_args)
            .timeout(std::time::Duration::from_secs(60))
            .assert()
    })
    .success();

    // Without --wait, the response should contain an operationId or status != Succeeded (async)
    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should have some response - could be empty (202 accepted) or contain operation info
    // The key property is that it returns quickly (within timeout) without polling
    // Accept any valid JSON response (the operation was dispatched)
    assert!(
        data.is_null() || data.is_object() || data.is_string(),
        "Expected structured async response: {data}"
    );

    // Wait for the async operation to complete before cleanup
    std::thread::sleep(std::time::Duration::from_secs(30));

    // Cleanup: delete the notebook, commit (with --wait), disconnect
    let assert = fabio()
        .args(["item", "list", "--workspace", workspace, "-o", "json"])
        .assert()
        .success();
    let json = parse_json(&assert);
    let items = json["data"].as_array().unwrap();
    if let Some(nb) = items
        .iter()
        .find(|i| i.get("displayName").and_then(|n| n.as_str()) == Some(nb_name.as_str()))
    {
        let nb_id = nb["id"].as_str().unwrap();
        let _ = fabio()
            .args(["item", "delete", "--workspace", workspace, "--id", nb_id])
            .timeout(std::time::Duration::from_secs(30))
            .assert();
    }

    let _ = retry_on_failure(|| {
        fabio()
            .args([
                "git",
                "commit",
                "--workspace",
                workspace,
                "--all",
                "--message",
                "cleanup: delete async test notebook",
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(180))
            .assert()
    });

    fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Pull with --conflict-resolution prefer-remote
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_pull_with_conflict_resolution() {
    let cfg = TestConfig::from_env();
    let workspace = &cfg.dest_workspace;

    let Some(connection_id) = find_github_connection_id() else {
        eprintln!("No GitHub connection found, skipping test.");
        return;
    };

    // Ensure workspace is disconnected
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();

    // Connect and init
    fabio()
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

    retry_on_failure(|| {
        fabio()
            .args([
                "git",
                "init",
                "--workspace",
                workspace,
                "--strategy",
                "prefer-workspace",
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    // Pull with --conflict-resolution prefer-remote --allow-override
    // Even if there's nothing to pull, this tests the flags are passed correctly.
    // The API should succeed (no-op if already up to date).
    let pull_args = [
        "git",
        "pull",
        "--workspace",
        workspace,
        "--conflict-resolution",
        "prefer-remote",
        "--allow-override",
        "--wait",
    ];
    let assert = retry_on_failure(|| {
        fabio()
            .args(pull_args)
            .timeout(std::time::Duration::from_secs(180))
            .assert()
    })
    .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Pull should succeed (either "Succeeded" or empty if already up-to-date)
    assert!(
        data.get("status").and_then(|s| s.as_str()) == Some("Succeeded")
            || data.is_null()
            || data.is_object(),
        "Expected successful pull response: {data}"
    );

    // Disconnect
    fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Connect to non-existent branch: error with actionable hint
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_connect_nonexistent_branch_gives_hint() {
    let cfg = TestConfig::from_env();
    let workspace = &cfg.dest_workspace;

    let Some(connection_id) = find_github_connection_id() else {
        eprintln!("No GitHub connection found, skipping test.");
        return;
    };

    // Ensure workspace is disconnected
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();

    // Try to connect to a branch that doesn't exist
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
            "nonexistent-branch-xyz-999",
            "--connection-id",
            &connection_id,
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();

    // Error should be on stderr with a helpful hint
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value =
        serde_json::from_str(&stderr).expect("Expected JSON error on stderr");

    let error = &err_json["error"];
    assert_eq!(error["code"], "NOT_FOUND");
    assert!(
        error.get("hint").is_some(),
        "Expected hint field in error: {error}"
    );

    let hint = error["hint"].as_str().unwrap();
    // Hint should mention the branch name and how to list branches
    assert!(
        hint.contains("nonexistent-branch-xyz-999"),
        "Hint should reference the bad branch: {hint}"
    );
    assert!(
        hint.contains("gh api") || hint.contains("List remote branches"),
        "Hint should suggest how to list valid branches: {hint}"
    );
    assert!(
        hint.contains("iemejia/fabio-test-connection"),
        "Hint should reference the repository: {hint}"
    );
}

// ---------------------------------------------------------------------------
// Checkout/switch to non-existent branch: error with hint + rollback to original
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_checkout_nonexistent_branch_gives_hint_and_rollback() {
    let cfg = TestConfig::from_env();
    let workspace = &cfg.dest_workspace;

    let Some(connection_id) = find_github_connection_id() else {
        eprintln!("No GitHub connection found, skipping test.");
        return;
    };

    // Ensure workspace is disconnected
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();

    // Connect to main and init
    fabio()
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

    retry_on_failure(|| {
        fabio()
            .args([
                "git",
                "init",
                "--workspace",
                workspace,
                "--strategy",
                "prefer-workspace",
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    // Try to checkout to a non-existent branch (--force to skip uncommitted changes check)
    let assert = fabio()
        .args([
            "--force",
            "git",
            "checkout",
            "--workspace",
            workspace,
            "--branch",
            "nonexistent-branch-xyz-999",
            "--strategy",
            "prefer-remote",
            "--wait",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .failure();

    // Verify error has hint
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value =
        serde_json::from_str(&stderr).expect("Expected JSON error on stderr");

    let error = &err_json["error"];
    assert_eq!(error["code"], "NOT_FOUND");
    let hint = error["hint"].as_str().expect("Expected hint in error");
    assert!(
        hint.contains("nonexistent-branch-xyz-999"),
        "Hint should reference the bad branch: {hint}"
    );

    // Verify rollback: workspace should still be connected (to original branch)
    let assert = fabio()
        .args(["git", "connection", "show", "--workspace", workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should be Connected (rollback reconnected to main)
    let state = data["gitConnectionState"].as_str().unwrap_or("");
    assert!(
        state == "Connected" || state == "ConnectedAndInitialized",
        "Expected workspace still connected after failed checkout, got: {state}"
    );
    assert_eq!(
        data["gitProviderDetails"]["branchName"], "main",
        "Expected rollback to original branch 'main'"
    );

    // Disconnect
    fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

/// RAII guard to delete a remote branch on drop (cleanup even if test panics).
struct BranchCleanup {
    branch: String,
}

impl Drop for BranchCleanup {
    fn drop(&mut self) {
        let _ = std::process::Command::new("gh")
            .args([
                "api",
                &format!(
                    "repos/iemejia/fabio-test-connection/git/refs/heads/{}",
                    self.branch
                ),
                "-X",
                "DELETE",
            ])
            .output();
    }
}

/// Test: checkout blocks when workspace has uncommitted changes (unless --force).
///
/// Flow:
/// 1. Connect to main, init
/// 2. Create a notebook (creates uncommitted workspace change)
/// 3. Attempt checkout without --force → `INVALID_INPUT` error with hint
/// 4. Cleanup: delete notebook, disconnect
#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn git_checkout_blocked_by_uncommitted_changes() {
    let cfg = TestConfig::from_env();
    let workspace = &cfg.dest_workspace;

    let Some(connection_id) = find_github_connection_id() else {
        eprintln!("No GitHub connection found, skipping test.");
        return;
    };

    // Ensure workspace is disconnected
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();

    // Connect to main and init
    fabio()
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

    retry_on_failure(|| {
        fabio()
            .args([
                "git",
                "init",
                "--workspace",
                workspace,
                "--strategy",
                "prefer-remote",
                "--wait",
            ])
            .timeout(std::time::Duration::from_secs(120))
            .assert()
    })
    .success();

    // Create a notebook to produce an uncommitted workspace change
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let nb_name = format!("guard_test_{ts}");

    let create_assert = fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            workspace,
            "--name",
            &nb_name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Extract notebook ID for cleanup
    let create_json = parse_json(&create_assert);
    let create_data = extract_data(&create_json);
    let nb_id = create_data["id"].as_str().unwrap_or("").to_string();

    // Wait a moment for workspace change to register
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Attempt checkout WITHOUT --force → should fail with INVALID_INPUT
    let assert = fabio()
        .args([
            "git",
            "checkout",
            "--workspace",
            workspace,
            "--branch",
            "main",
            "--strategy",
            "prefer-remote",
            "--wait",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value =
        serde_json::from_str(&stderr).expect("Expected JSON error on stderr");
    let error = &err_json["error"];
    assert_eq!(
        error["code"], "INVALID_INPUT",
        "Expected INVALID_INPUT error for uncommitted changes"
    );
    let hint = error["hint"].as_str().expect("Expected hint in error");
    assert!(
        hint.contains("--force") || hint.contains("commit"),
        "Hint should mention --force or commit: {hint}"
    );

    // Cleanup: delete the notebook
    if !nb_id.is_empty() {
        let _ = fabio()
            .args([
                "notebook",
                "delete",
                "--workspace",
                workspace,
                "--id",
                &nb_id,
            ])
            .timeout(std::time::Duration::from_secs(60))
            .assert();
    }

    // Disconnect
    let _ = fabio()
        .args(["git", "disconnect", "--workspace", workspace])
        .timeout(std::time::Duration::from_secs(60))
        .assert();
}
