//! End-to-end integration tests for `fabio context` commands.

mod common;

use common::{TestConfig, fabio, parse_json};
use serial_test::serial;

// ── Dry-run tests (offline, no live tenant needed) ──────────────────────────

#[test]
fn context_extract_dry_run_succeeds() {
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            "00000000-0000-0000-0000-000000000001",
            "--dry-run",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "context extract");
}

#[test]
fn context_extract_dry_run_with_deep() {
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            "00000000-0000-0000-0000-000000000001",
            "--deep",
            "--dry-run",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert_eq!(data["dry_run"], true);
    // Verify deep flag is reflected in dry-run output
    assert_eq!(data["details"]["deep"], true);
}

#[test]
fn context_extract_dry_run_with_connections() {
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            "00000000-0000-0000-0000-000000000001",
            "--include-connections",
            "--dry-run",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["details"]["includeConnections"], true);
}

#[test]
fn context_extract_dry_run_with_item_types() {
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            "00000000-0000-0000-0000-000000000001",
            "--item-types",
            "Notebook,Lakehouse",
            "--dry-run",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["details"]["itemTypes"], "Notebook,Lakehouse");
}

#[test]
fn context_extract_dry_run_multiple_workspaces() {
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            "00000000-0000-0000-0000-000000000001",
            "--workspace",
            "00000000-0000-0000-0000-000000000002",
            "--dry-run",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert_eq!(data["dry_run"], true);
    let workspaces = data["details"]["workspaces"].as_array().unwrap();
    assert_eq!(workspaces.len(), 2);
}

#[test]
fn context_extract_dry_run_concurrency_flag() {
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            "00000000-0000-0000-0000-000000000001",
            "--concurrency",
            "4",
            "--dry-run",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert_eq!(data["details"]["concurrency"], 4);
}

#[test]
fn context_extract_dry_run_no_properties() {
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            "00000000-0000-0000-0000-000000000001",
            "--no-properties",
            "--dry-run",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["details"]["noProperties"], true);
}

#[test]
fn context_extract_dry_run_output_file() {
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            "00000000-0000-0000-0000-000000000001",
            "--output-file",
            "/tmp/opencode/test_context_output.json",
            "--dry-run",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert_eq!(data["dry_run"], true);
    assert!(
        data["details"]["outputFile"]
            .as_str()
            .unwrap_or_default()
            .contains("test_context_output.json")
    );
}

#[test]
fn context_extract_dry_run_merge() {
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            "00000000-0000-0000-0000-000000000001",
            "--merge",
            "/tmp/opencode/existing_graph.json",
            "--dry-run",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert_eq!(data["dry_run"], true);
    assert!(
        data["details"]["merge"]
            .as_str()
            .unwrap_or_default()
            .contains("existing_graph.json")
    );
}

#[test]
fn context_extract_dry_run_format_jsonld() {
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            "00000000-0000-0000-0000-000000000001",
            "--format",
            "jsonld",
            "--dry-run",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    assert_eq!(data["dry_run"], true);
}

// ── Live tenant tests ───────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn context_extract_returns_graph_structure() {
    let config = TestConfig::from_env();
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            &config.source_workspace,
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");

    // Verify graph structure
    assert!(data.get("nodes").is_some(), "missing nodes");
    assert!(data.get("edges").is_some(), "missing edges");
    assert!(data.get("workspaces").is_some(), "missing workspaces");
    assert!(data.get("summary").is_some(), "missing summary");

    // Verify summary fields
    let summary = &data["summary"];
    assert!(summary["totalNodes"].as_u64().unwrap() > 0);
    assert!(summary["workspacesScanned"].as_u64().unwrap() == 1);
    assert!(summary["itemTypes"].is_object());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn context_extract_nodes_have_required_fields() {
    let config = TestConfig::from_env();
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            &config.source_workspace,
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    let nodes = data["nodes"].as_array().expect("nodes is not an array");

    // Every node must have id, type, name, workspaceId
    for node in nodes {
        assert!(
            node.get("id").and_then(|v| v.as_str()).is_some(),
            "node missing id"
        );
        assert!(
            node.get("type").and_then(|v| v.as_str()).is_some(),
            "node missing type"
        );
        assert!(
            node.get("name").and_then(|v| v.as_str()).is_some(),
            "node missing name"
        );
        assert!(
            node.get("workspaceId").and_then(|v| v.as_str()).is_some(),
            "node missing workspaceId"
        );
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn context_extract_with_item_type_filter() {
    let config = TestConfig::from_env();
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            &config.source_workspace,
            "--item-types",
            "Lakehouse",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    let nodes = data["nodes"].as_array().expect("nodes is not an array");

    // All nodes should be Lakehouse type
    for node in nodes {
        let node_type = node["type"].as_str().unwrap_or_default();
        assert_eq!(
            node_type.to_lowercase(),
            "lakehouse",
            "expected Lakehouse, got {node_type}"
        );
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn context_extract_deep_discovers_more_edges() {
    let config = TestConfig::from_env();

    // Without deep
    let assert_shallow = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            &config.source_workspace,
        ])
        .assert()
        .success();
    let json_shallow = parse_json(&assert_shallow);
    let shallow_edges = json_shallow["data"]["summary"]["totalEdges"]
        .as_u64()
        .unwrap_or(0);

    // With deep
    let assert_deep = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            &config.source_workspace,
            "--deep",
        ])
        .assert()
        .success();
    let json_deep = parse_json(&assert_deep);
    let deep_edges = json_deep["data"]["summary"]["totalEdges"]
        .as_u64()
        .unwrap_or(0);

    // Deep should find at least as many edges (likely more)
    assert!(
        deep_edges >= shallow_edges,
        "deep mode ({deep_edges}) should find >= shallow mode ({shallow_edges}) edges"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn context_extract_workspace_by_name() {
    // This test uses the workspace ID env var but resolves it as a name
    // (only works if the workspace listing returns it)
    let config = TestConfig::from_env();
    let assert = fabio()
        .args([
            "context",
            "extract",
            "--workspace",
            &config.source_workspace,
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let data = json.get("data").expect("missing data");
    let workspaces = data["workspaces"].as_array().expect("missing workspaces");
    assert_eq!(workspaces.len(), 1);
    assert_eq!(workspaces[0]["id"], config.source_workspace);
}
