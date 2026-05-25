//! End-to-end integration tests for `fabio graph-model` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graph_model_list_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["graph-model", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graph_model_create_show_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("gm_test");

    // Create
    let assert = fabio()
        .args([
            "graph-model",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--description",
            "E2E test graph model",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    assert_eq!(data["type"], "GraphModel");
    let gm_id = data["id"].as_str().unwrap().to_string();

    // Show
    let assert = fabio()
        .args([
            "graph-model",
            "show",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    assert_eq!(data["id"], gm_id);
    assert_eq!(data["properties"]["queryReadiness"], "None");

    // Delete
    let assert = fabio()
        .args([
            "graph-model",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graph_model_create_with_ontology() {
    let cfg = TestConfig::from_env();
    let ont_name = common::unique_name("gm_ont");
    let gm_name = common::unique_name("gm_linked");

    // Create an ontology first
    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &ont_name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Create graph model linked to the ontology
    let assert = fabio()
        .args([
            "graph-model",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &gm_name,
            "--ontology",
            &ont_id,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], gm_name);
    let gm_id = data["id"].as_str().unwrap().to_string();

    // Cleanup
    fabio()
        .args([
            "graph-model",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
        ])
        .assert()
        .success();

    fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graph_model_update_name() {
    let cfg = TestConfig::from_env();
    let original = common::unique_name("gm_upd_o");
    let updated = common::unique_name("gm_upd_n");

    // Create
    let assert = fabio()
        .args([
            "graph-model",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &original,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let gm_id = data["id"].as_str().unwrap().to_string();

    // Update
    let assert = fabio()
        .args([
            "graph-model",
            "update",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
            "--name",
            &updated,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], updated);

    // Cleanup
    fabio()
        .args([
            "graph-model",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graph_model_update_requires_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "graph-model",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err_json["error"]["code"], "INVALID_INPUT");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graph_model_get_definition() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("gm_def");

    // Create
    let assert = fabio()
        .args([
            "graph-model",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let gm_id = data["id"].as_str().unwrap().to_string();

    // Get definition
    let assert = fabio()
        .args([
            "graph-model",
            "get-definition",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Definition should have parts with at least .platform
    let parts = data["definition"]["parts"].as_array().unwrap();
    assert!(!parts.is_empty());
    assert!(
        parts
            .iter()
            .any(|p| p["path"].as_str().unwrap() == ".platform")
    );

    // Cleanup
    fabio()
        .args([
            "graph-model",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graph_model_refresh_graph() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("gm_refresh");

    // Create
    let assert = fabio()
        .args([
            "graph-model",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let gm_id = data["id"].as_str().unwrap().to_string();

    // Refresh (no --wait, just trigger)
    let assert = fabio()
        .args([
            "graph-model",
            "refresh-graph",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "refresh_triggered");

    // Wait a moment and check status
    std::thread::sleep(std::time::Duration::from_secs(5));

    let assert = fabio()
        .args([
            "graph-model",
            "show",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // lastDataLoadingStatus should exist after refresh is triggered
    assert!(data["properties"]["lastDataLoadingStatus"].is_object());

    // Cleanup
    fabio()
        .args([
            "graph-model",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graph_model_execute_query_on_unloaded_graph() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("gm_query");

    // Create
    let assert = fabio()
        .args([
            "graph-model",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let gm_id = data["id"].as_str().unwrap().to_string();

    // Execute query should fail on unloaded graph
    let assert = fabio()
        .args([
            "graph-model",
            "execute-query",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
            "--query",
            "MATCH (n) RETURN n LIMIT 5",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err_json["error"]["code"], "API_ERROR");
    // Error message should indicate graph is not loaded
    let msg = err_json["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("GraphIsNotLoaded") || msg.contains("GraphNotQueryable"),
        "Expected graph-not-loaded error, got: {msg}"
    );

    // Cleanup
    fabio()
        .args([
            "graph-model",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &gm_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graph_model_dry_run_create() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "graph-model",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "test-dry-run",
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["would_execute"], "graph-model create");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graph_model_dry_run_refresh() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "graph-model",
            "refresh-graph",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["would_execute"], "graph-model refresh-graph");
}
