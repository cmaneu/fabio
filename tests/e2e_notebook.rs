//! End-to-end integration tests for `fabio notebook` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn notebook_create_get_definition_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("nb_test");

    // Create notebook
    let assert = fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--content",
            "print('integration test')",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "Succeeded");

    // Find the notebook ID
    let assert = fabio()
        .args([
            "item",
            "list",
            "--workspace",
            &cfg.dest_workspace,
            "--type",
            "Notebook",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let items = extract_data(&json).as_array().unwrap().clone();
    let nb = items
        .iter()
        .find(|i| i["displayName"] == name)
        .expect("created notebook not found in item list");
    let nb_id = nb["id"].as_str().unwrap().to_string();

    // Get definition
    let assert = fabio()
        .args([
            "notebook",
            "get-definition",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should have definition.parts
    let parts = data["definition"]["parts"].as_array().unwrap();
    assert!(!parts.is_empty());
    assert!(parts.iter().any(|p| p["path"] == "notebook-content.py"));

    // Delete
    fabio()
        .args([
            "notebook",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted"));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn notebook_run_status_stop() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("nb_run");

    // Create a simple notebook
    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--content",
            "import time; time.sleep(30); print('done')",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Find notebook ID
    let assert = fabio()
        .args([
            "item",
            "list",
            "--workspace",
            &cfg.dest_workspace,
            "--type",
            "Notebook",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let items = extract_data(&json).as_array().unwrap().clone();
    let nb = items
        .iter()
        .find(|i| i["displayName"] == name)
        .expect("notebook not found");
    let nb_id = nb["id"].as_str().unwrap().to_string();

    // Run
    let assert = fabio()
        .args([
            "notebook",
            "run",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "started");
    let job_id = data["jobId"].as_str().unwrap().to_string();
    assert!(!job_id.is_empty());

    // Status
    let assert = fabio()
        .args([
            "notebook",
            "status",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
            "--job-id",
            &job_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Status should be NotStarted or InProgress (Spark cold start)
    let status = data["status"].as_str().unwrap();
    assert!(
        ["NotStarted", "InProgress", "Completed"].contains(&status),
        "unexpected status: {status}"
    );

    // Stop (cancel)
    let assert = fabio()
        .args([
            "notebook",
            "stop",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
            "--job-id",
            &job_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "cancelled");

    // Delete notebook
    fabio()
        .args([
            "notebook",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn notebook_run_with_wait() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("nb_wait");

    // Create a quick notebook
    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--content",
            "print('quick run')",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Find notebook ID
    let assert = fabio()
        .args([
            "item",
            "list",
            "--workspace",
            &cfg.dest_workspace,
            "--type",
            "Notebook",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let items = extract_data(&json).as_array().unwrap().clone();
    let nb = items
        .iter()
        .find(|i| i["displayName"] == name)
        .expect("notebook not found");
    let nb_id = nb["id"].as_str().unwrap().to_string();

    // Run with --wait (use a generous timeout for Spark cold start)
    let assert = fabio()
        .args([
            "notebook",
            "run",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
            "--wait",
            "--timeout",
            "600",
        ])
        .timeout(std::time::Duration::from_secs(660))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "Completed");
    assert!(data.get("jobId").is_some());

    // Delete notebook
    fabio()
        .args([
            "notebook",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// notebook run --wait with a failing notebook (exception → Failed status)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn notebook_run_with_wait_fails() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("nb_fail");

    // Create a notebook that raises an exception
    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--content",
            "raise Exception('intentional failure for test')",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Find notebook ID
    let assert = fabio()
        .args([
            "item",
            "list",
            "--workspace",
            &cfg.dest_workspace,
            "--type",
            "Notebook",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let items = extract_data(&json).as_array().unwrap().clone();
    let nb = items
        .iter()
        .find(|i| i["displayName"] == name)
        .expect("notebook not found");
    let nb_id = nb["id"].as_str().unwrap().to_string();

    // Run with --wait — should complete but with Failed status
    let assert = fabio()
        .args([
            "notebook",
            "run",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
            "--wait",
            "--timeout",
            "600",
        ])
        .timeout(std::time::Duration::from_secs(660))
        .assert();

    // The command may succeed (reporting status=Failed) or fail outright
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let data = &json["data"];
        assert_eq!(
            data["status"], "Failed",
            "Expected Failed status for error notebook: {data}"
        );
    } else {
        // If command failed, error should mention failure
        assert!(
            stderr.contains("Failed") || stderr.contains("error"),
            "Expected failure indication in stderr: {stderr}"
        );
    }

    // Delete notebook
    fabio()
        .args([
            "notebook",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// notebook delete non-existent returns error
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn notebook_delete_not_found() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "notebook",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "NOT_FOUND" || code == "API_ERROR",
        "Expected NOT_FOUND or API_ERROR, got: {code}"
    );
}

// ---------------------------------------------------------------------------
// notebook get-definition for non-existent notebook returns error
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn notebook_get_definition_not_found() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "notebook",
            "get-definition",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "NOT_FOUND" || code == "API_ERROR",
        "Expected error for non-existent notebook, got: {code}"
    );
}
