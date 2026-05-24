//! End-to-end integration tests for `fabio eventstream` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn eventstream_list_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["eventstream", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn eventstream_create_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("es_test");

    // Create
    let assert = fabio()
        .args([
            "eventstream",
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
    assert_eq!(data["displayName"], name);
    let es_id = data["id"].as_str().unwrap().to_string();

    // Delete
    let assert = fabio()
        .args([
            "eventstream",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &es_id,
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
fn eventstream_update_requires_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "eventstream",
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
fn eventstream_show_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "eventstream",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn eventstream_dry_run_create() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "eventstream",
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
    assert_eq!(json["data"]["would_execute"], "eventstream create");
}

// ─── Topology tests ──────────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn eventstream_get_topology_returns_structure() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("es_topo");

    // Create eventstream
    let assert = fabio()
        .args([
            "eventstream",
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
    let es_id = data["id"].as_str().unwrap().to_string();

    // Get topology
    let assert = fabio()
        .args([
            "eventstream",
            "get-topology",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &es_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data["sources"].is_array());
    assert!(data["destinations"].is_array());
    assert!(data["streams"].is_array());
    assert!(data["operators"].is_array());
    assert!(data["compatibilityLevel"].is_string());

    // Cleanup
    fabio()
        .args([
            "eventstream",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &es_id,
        ])
        .assert()
        .success();
}

// ─── add-source / add-destination tests ──────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn eventstream_add_source_custom_endpoint() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("es_addsrc");

    // Create eventstream
    let assert = fabio()
        .args([
            "eventstream",
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
    let es_id = data["id"].as_str().unwrap().to_string();

    // Add source
    let assert = fabio()
        .args([
            "eventstream",
            "add-source",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &es_id,
            "--name",
            "my-source",
            "--source-type",
            "CustomEndpoint",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Topology should contain the new source
    let sources = data["sources"].as_array().unwrap();
    assert!(!sources.is_empty());
    assert_eq!(sources[0]["name"], "my-source");
    assert_eq!(sources[0]["type"], "CustomEndpoint");
    // Default stream should have been auto-created
    let streams = data["streams"].as_array().unwrap();
    assert!(!streams.is_empty());

    // Cleanup
    fabio()
        .args([
            "eventstream",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &es_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn eventstream_add_source_dry_run() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "eventstream",
            "add-source",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--name",
            "test-source",
            "--source-type",
            "CustomEndpoint",
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["would_execute"], "eventstream add-source");
    assert_eq!(json["data"]["details"]["source"]["name"], "test-source");
    assert_eq!(json["data"]["details"]["source"]["type"], "CustomEndpoint");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn eventstream_add_destination_dry_run() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "eventstream",
            "add-destination",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--name",
            "test-dest",
            "--destination-type",
            "Eventhouse",
            "--input-node",
            "my-stream",
            "--properties",
            r#"{"dataIngestionMode":"DirectIngestion","workspaceId":"ws-id","itemId":"item-id","tableName":"T1","connectionName":"conn","mappingRuleName":"map"}"#,
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["would_execute"], "eventstream add-destination");
    assert_eq!(json["data"]["details"]["destination"]["name"], "test-dest");
    assert_eq!(json["data"]["details"]["destination"]["type"], "Eventhouse");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn eventstream_add_source_invalid_properties_json() {
    let cfg = TestConfig::from_env();

    // Invalid JSON in --properties should fail
    fabio()
        .args([
            "eventstream",
            "add-source",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--name",
            "test-source",
            "--source-type",
            "CustomEndpoint",
            "--properties",
            "not-valid-json",
        ])
        .assert()
        .failure();
}

// ─── get-definition / update-definition tests ────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn eventstream_get_definition_returns_parts() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("es_def");

    // Create eventstream
    let assert = fabio()
        .args([
            "eventstream",
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
    let es_id = data["id"].as_str().unwrap().to_string();

    // Get definition
    let assert = fabio()
        .args([
            "eventstream",
            "get-definition",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &es_id,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let parts = data["definition"]["parts"].as_array().unwrap();
    assert!(!parts.is_empty());
    // Should contain eventstream.json
    let has_eventstream_json = parts
        .iter()
        .any(|p| p["path"].as_str() == Some("eventstream.json"));
    assert!(has_eventstream_json);

    // Cleanup
    fabio()
        .args([
            "eventstream",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &es_id,
        ])
        .assert()
        .success();
}

// ─── pause / resume tests ────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn eventstream_pause_resume_dry_run() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "eventstream",
            "pause",
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
    assert_eq!(json["data"]["would_execute"], "eventstream pause");

    let assert = fabio()
        .args([
            "eventstream",
            "resume",
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
    assert_eq!(json["data"]["would_execute"], "eventstream resume");
}
