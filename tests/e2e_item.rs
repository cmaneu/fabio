//! End-to-end integration tests for `fabio item` commands.

mod common;

use common::{TestConfig, extract_count, extract_data, fabio, parse_json};
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn item_list_returns_items() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["item", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let count = extract_count(&json);

    assert!(count > 0);
    let arr = data.as_array().unwrap();
    assert!(!arr.is_empty());

    // Each item should have id, displayName, type
    let first = &arr[0];
    assert!(first.get("id").is_some());
    assert!(first.get("displayName").is_some());
    assert!(first.get("type").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn item_list_with_type_filter() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "item",
            "list",
            "--workspace",
            &cfg.source_workspace,
            "--type",
            "Lakehouse",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let arr = data.as_array().unwrap();

    // All returned items should be Lakehouses
    for item in arr {
        assert_eq!(item["type"], "Lakehouse");
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn item_list_without_type_filter_returns_all() {
    let cfg = TestConfig::from_env();

    // Without type filter, should return items of multiple types
    let assert = fabio()
        .args(["item", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let arr = data.as_array().unwrap();

    // Collect unique types
    let types: std::collections::HashSet<&str> =
        arr.iter().filter_map(|i| i["type"].as_str()).collect();

    // Should have at least Lakehouse and Notebook
    assert!(
        types.len() >= 2,
        "expected multiple item types, got: {types:?}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn item_show_returns_details() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "item",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    assert_eq!(data["id"], cfg.source_lakehouse);
    assert_eq!(data["type"], "Lakehouse");
    assert!(data.get("displayName").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn item_create_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("item_test");

    // Create a Lakehouse item
    let assert = fabio()
        .args([
            "item",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--type",
            "Lakehouse",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    assert_eq!(data["type"], "Lakehouse");
    let new_id = data["id"].as_str().unwrap().to_string();

    // Delete the item
    let assert = fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &new_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
    assert_eq!(data["id"], new_id);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn item_copy_with_custom_name_and_delete() {
    let cfg = TestConfig::from_env();
    let copy_name = common::unique_name("test_copy");

    // Copy the notebook to dest workspace with explicit name
    let assert = fabio()
        .args([
            "item",
            "copy",
            "--source-workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.notebook_id,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--name",
            &copy_name,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    assert_eq!(data["displayName"], copy_name);
    assert_eq!(data["type"], "Notebook");

    let new_id = data["id"].as_str().unwrap();

    // Delete the copy
    fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            new_id,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted"));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn item_copy_without_name_inherits_source_name() {
    let cfg = TestConfig::from_env();

    // First create a uniquely named notebook in source to avoid conflicts
    let src_name = common::unique_name("cp_src_nb");
    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &src_name,
            "--content",
            "print('copy inherit name test')",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Find the notebook ID
    let assert = fabio()
        .args([
            "item",
            "list",
            "--workspace",
            &cfg.source_workspace,
            "--type",
            "Notebook",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let items = extract_data(&json).as_array().unwrap().clone();
    let nb = items.iter().find(|i| i["displayName"] == src_name).unwrap();
    let nb_id = nb["id"].as_str().unwrap().to_string();

    // Copy without specifying --name (should inherit from source)
    let assert = fabio()
        .args([
            "item",
            "copy",
            "--source-workspace",
            &cfg.source_workspace,
            "--id",
            &nb_id,
            "--dest-workspace",
            &cfg.dest_workspace,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Name should be the source notebook's name
    assert_eq!(data["displayName"], src_name);
    assert_eq!(data["type"], "Notebook");

    let new_id = data["id"].as_str().unwrap();

    // Cleanup: delete copy from dest
    fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            new_id,
        ])
        .assert()
        .success();

    // Cleanup: delete source
    fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &nb_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn item_move_to_dest_workspace() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("move_test");

    // First create a notebook in dest workspace (we'll move it back to source)
    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--content",
            "print('move test')",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

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
        .expect("created notebook not found");
    let nb_id = nb["id"].as_str().unwrap().to_string();

    // Move from dest to source workspace
    let move_name = common::unique_name("moved_nb");
    let assert = fabio()
        .args([
            "item",
            "move",
            "--source-workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
            "--dest-workspace",
            &cfg.source_workspace,
            "--name",
            &move_name,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "moved");
    assert_eq!(data["displayName"], move_name);
    assert_eq!(data["type"], "Notebook");

    let moved_id = data["id"].as_str().unwrap();

    // Verify original is gone (should fail)
    fabio()
        .args([
            "item",
            "show",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
        ])
        .assert()
        .failure();

    // Cleanup: delete from source workspace
    fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            moved_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn item_move_without_name() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("move_noname");

    // Create notebook
    fabio()
        .args([
            "notebook",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--content",
            "print('move no rename')",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Find ID
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
    let nb = items.iter().find(|i| i["displayName"] == name).unwrap();
    let nb_id = nb["id"].as_str().unwrap().to_string();

    // Move without --name (should keep original name)
    let assert = fabio()
        .args([
            "item",
            "move",
            "--source-workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
            "--dest-workspace",
            &cfg.source_workspace,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "moved");
    assert_eq!(data["displayName"], name);

    let moved_id = data["id"].as_str().unwrap();

    // Cleanup
    fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            moved_id,
        ])
        .assert()
        .success();
}
