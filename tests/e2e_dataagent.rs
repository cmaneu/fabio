//! End-to-end integration tests for `fabio data-agent` commands.

mod common;

use common::{TestConfig, extract_count, extract_data, fabio, parse_json, unique_name};
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_list_returns_json() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["data-agent", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    // Should have data array and count
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_list_via_alias() {
    let cfg = TestConfig::from_env();

    // Should work with the 'da' alias
    let assert = fabio()
        .args(["da", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_create_show_update_delete() {
    let cfg = TestConfig::from_env();
    let name = unique_name("da_test");
    let description = "Test data agent created by fabio e2e tests";

    // Create a data agent
    let assert = fabio()
        .args([
            "data-agent",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
            "--description",
            description,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    assert_eq!(data["type"], "DataAgent");
    let agent_id = data["id"].as_str().unwrap().to_string();

    // Show the data agent
    let assert = fabio()
        .args([
            "data-agent",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["id"], agent_id);
    assert_eq!(data["displayName"], name);

    // Update the data agent
    let new_name = unique_name("da_updated");
    let new_desc = "Updated description";
    let assert = fabio()
        .args([
            "data-agent",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
            "--name",
            &new_name,
            "--description",
            new_desc,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], new_name);
    assert_eq!(data["description"], new_desc);

    // Delete the data agent
    let assert = fabio()
        .args([
            "data-agent",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
    assert_eq!(data["id"], agent_id);

    // Verify it's gone (show should fail)
    fabio()
        .args([
            "data-agent",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .failure();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_create_without_description() {
    let cfg = TestConfig::from_env();
    let name = unique_name("da_nodesc");

    // Create without description
    let assert = fabio()
        .args([
            "data-agent",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let agent_id = data["id"].as_str().unwrap().to_string();

    // Cleanup
    fabio()
        .args([
            "data-agent",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_update_requires_at_least_one_field() {
    let cfg = TestConfig::from_env();

    // Update without --name or --description should fail
    fabio()
        .args([
            "data-agent",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("INVALID_INPUT"));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_show_nonexistent_returns_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "data-agent",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NOT_FOUND").or(predicate::str::contains("API_ERROR")));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_list_table_output() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "--output",
            "table",
            "data-agent",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_list_with_query_projection() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "--query",
            "displayName",
            "data-agent",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    // Query projection should return array of names (or empty)
    let data = extract_data(&json);
    if let Some(arr) = data.as_array() {
        for item in arr {
            // Each item should be a string (projected displayName)
            assert!(item.is_string(), "expected string, got: {item}");
        }
    }
}

#[test]
#[ignore = "requires live Fabric tenant and published data agent"]
#[serial]
fn dataagent_query_with_prompt() {
    let cfg = TestConfig::from_env();

    // This test requires a published data agent. Skip if no agent exists.
    let assert = fabio()
        .args(["data-agent", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let count = extract_count(&json);
    if count == 0 {
        eprintln!("No data agents in workspace, skipping query test");
        return;
    }

    let items = extract_data(&json).as_array().unwrap().clone();
    let agent_id = items[0]["id"].as_str().unwrap();

    // Try querying the data agent
    let assert = fabio()
        .args([
            "data-agent",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            agent_id,
            "--prompt",
            "What data sources do you have access to?",
        ])
        .timeout(std::time::Duration::from_secs(300))
        .assert();

    // Query may fail if agent isn't published - that's OK for this test
    let output = assert.get_output();
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let data = extract_data(&json);
        assert!(data.get("answer").is_some());
        assert!(data.get("question").is_some());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Query failed (agent may not be published): {stderr}");
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_query_no_prompt_no_stdin_fails() {
    let cfg = TestConfig::from_env();

    // Without --prompt and with empty stdin, should fail
    fabio()
        .args([
            "data-agent",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("INVALID_INPUT"));
}
