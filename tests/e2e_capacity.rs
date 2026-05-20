//! E2E integration tests for the `fabio capacity` command group.

mod common;

use common::{extract_count, fabio, parse_json};

#[test]
#[ignore = "requires live Fabric tenant"]
fn capacity_list() {
    let output = fabio().args(["capacity", "list"]).assert().success();

    let json = parse_json(&output);
    let data = json
        .get("data")
        .and_then(|d| d.as_array())
        .expect("data should be array");
    assert!(!data.is_empty(), "expected at least one capacity");

    let first = &data[0];
    assert!(first.get("id").is_some());
    assert!(first.get("displayName").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn capacity_list_with_limit() {
    let output = fabio()
        .args(["capacity", "list", "--limit", "1"])
        .assert()
        .success();

    let json = parse_json(&output);
    let count = extract_count(&json);
    assert!(count <= 1);
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn capacity_show() {
    // First list to get a capacity ID
    let list_output = fabio().args(["capacity", "list"]).assert().success();
    let list_json = parse_json(&list_output);
    let capacities = list_json["data"].as_array().expect("data should be array");
    let first_id = capacities[0]["id"].as_str().expect("id should be string");

    let output = fabio()
        .args(["capacity", "show", "--id", first_id])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = json.get("data").expect("should have data");
    assert_eq!(data["id"].as_str().unwrap(), first_id);
}
