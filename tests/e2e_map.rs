//! End-to-end integration tests for `fabio map` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn map_list_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["map", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn map_create_show_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("map_crud");

    // Create
    let assert = fabio()
        .args([
            "map",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--description",
            "Test map for e2e",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    assert_eq!(data["type"], "Map");
    let map_id = data["id"].as_str().unwrap().to_string();

    // Show
    let assert = fabio()
        .args([
            "map",
            "show",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &map_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    assert_eq!(data["description"], "Test map for e2e");

    // Delete
    let assert = fabio()
        .args([
            "map",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &map_id,
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
fn map_update_name_and_description() {
    let cfg = TestConfig::from_env();
    let original = common::unique_name("map_upd_o");
    let updated = common::unique_name("map_upd_n");

    // Create
    let assert = fabio()
        .args([
            "map",
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
    let map_id = data["id"].as_str().unwrap().to_string();

    // Update
    let assert = fabio()
        .args([
            "map",
            "update",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &map_id,
            "--name",
            &updated,
            "--description",
            "Updated description",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], updated);
    assert_eq!(data["description"], "Updated description");

    // Cleanup
    fabio()
        .args([
            "map",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &map_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn map_get_definition_returns_map_json() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("map_def");

    // Create
    let assert = fabio()
        .args([
            "map",
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
    let map_id = data["id"].as_str().unwrap().to_string();

    // Get definition
    let assert = fabio()
        .args([
            "map",
            "get-definition",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &map_id,
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let parts = data["definition"]["parts"].as_array().unwrap();
    // Should have map.json and .platform
    let paths: Vec<&str> = parts.iter().map(|p| p["path"].as_str().unwrap()).collect();
    assert!(paths.contains(&"map.json"), "Expected map.json part");
    assert!(paths.contains(&".platform"), "Expected .platform part");

    // Decode map.json and verify structure
    let map_part = parts.iter().find(|p| p["path"] == "map.json").unwrap();
    let payload = map_part["payload"].as_str().unwrap();
    let decoded = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        payload,
    )
    .unwrap();
    let map_def: serde_json::Value = serde_json::from_slice(&decoded).unwrap();
    assert!(map_def["$schema"].as_str().unwrap().contains("map/definition"));
    assert!(map_def["dataSources"].is_array());
    assert!(map_def["layerSources"].is_array());
    assert!(map_def["layerSettings"].is_array());

    // Cleanup
    fabio()
        .args([
            "map",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &map_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn map_update_definition_with_layers() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("map_upddef");

    // Create
    let assert = fabio()
        .args([
            "map",
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
    let map_id = data["id"].as_str().unwrap().to_string();

    // Build a map definition with basemap config and a data source
    let map_def = serde_json::json!({
        "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/map/definition/2.0.0/schema.json",
        "basemap": {
            "options": {
                "center": [0, 20],
                "zoom": 2,
                "style": "road",
                "showLabels": true
            },
            "controls": {
                "zoom": true,
                "compass": true
            }
        },
        "dataSources": [
            {
                "itemType": "Lakehouse",
                "workspaceId": &cfg.source_workspace,
                "itemId": &cfg.source_lakehouse
            }
        ],
        "iconSources": [],
        "layerSources": [
            {
                "id": "a0000001-0001-0001-0001-000000000001",
                "name": "test_layer_source",
                "type": "table",
                "itemId": &cfg.source_lakehouse,
                "relativePath": "Tables/cities_sales"
            }
        ],
        "layerSettings": [
            {
                "id": "b0000001-0001-0001-0001-000000000001",
                "name": "Test Bubble Layer",
                "sourceId": "a0000001-0001-0001-0001-000000000001",
                "options": {
                    "type": "vector",
                    "visible": true,
                    "latitudeColumnName": "latitude",
                    "longitudeColumnName": "longitude",
                    "pointLayerType": "bubble",
                    "bubbleOptions": {
                        "color": "#FF6600",
                        "radius": 8,
                        "opacity": 0.7
                    }
                },
                "latitudeColumnName": "latitude",
                "longitudeColumnName": "longitude"
            }
        ]
    });

    let dir = tempfile::tempdir().unwrap();
    let def_path = dir.path().join("map_def.json");
    std::fs::write(&def_path, serde_json::to_string(&map_def).unwrap()).unwrap();

    // Update definition
    let assert = fabio()
        .args([
            "map",
            "update-definition",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &map_id,
            "--file",
            def_path.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // updateDefinition returns the item on success (non-LRO result)
    assert_eq!(data["displayName"], name);

    // Verify definition was saved
    let assert = fabio()
        .args([
            "map",
            "get-definition",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &map_id,
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let parts = data["definition"]["parts"].as_array().unwrap();
    let map_part = parts.iter().find(|p| p["path"] == "map.json").unwrap();
    let payload = map_part["payload"].as_str().unwrap();
    let decoded = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        payload,
    )
    .unwrap();
    let saved_def: serde_json::Value = serde_json::from_slice(&decoded).unwrap();
    // Verify the layer source was persisted
    assert_eq!(saved_def["layerSources"][0]["name"], "test_layer_source");
    assert_eq!(
        saved_def["layerSettings"][0]["name"],
        "Test Bubble Layer"
    );
    assert_eq!(
        saved_def["layerSettings"][0]["options"]["pointLayerType"],
        "bubble"
    );

    // Cleanup
    fabio()
        .args([
            "map",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &map_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn map_update_requires_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "map",
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
fn map_dry_run_create() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "map",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "test-dry-run",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["would_execute"], "map create");
    assert!(data["dry_run"].as_bool().unwrap());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn map_dry_run_delete() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "map",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["would_execute"], "map delete");
    assert!(data["dry_run"].as_bool().unwrap());
}
