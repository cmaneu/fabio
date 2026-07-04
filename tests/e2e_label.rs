//! End-to-end tests for `fabio label` commands.

mod common;

use common::fabio;

#[test]
#[ignore = "requires live Microsoft Graph access with InformationProtection.Read"]
fn label_list_returns_array() {
    let output = fabio().args(["label", "list"]).assert().success();

    let json = common::parse_json(&output);
    let data = common::extract_data(&json);
    let arr = data.as_array().expect("data should be an array");
    assert!(!arr.is_empty(), "Expected at least one sensitivity label");

    // Each label should have id and name
    let first = &arr[0];
    assert!(first.get("id").is_some(), "Label should have id");
    assert!(first.get("name").is_some(), "Label should have name");
}

#[test]
fn label_list_help_shows_usage() {
    fabio()
        .args(["label", "list", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("sensitivity labels"));
}

#[test]
fn label_help_shows_subcommands() {
    fabio()
        .args(["label", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("list"));
}
