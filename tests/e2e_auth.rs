//! End-to-end integration tests for `fabio auth` commands.

mod common;

use common::{extract_data, fabio, parse_json};
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_status_returns_authenticated() {
    let assert = fabio().args(["auth", "status"]).assert().success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    assert_eq!(data["status"], "authenticated");
    assert_eq!(data["message"], "Token acquired successfully");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn auth_status_json_envelope() {
    let assert = fabio()
        .args(["auth", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"data\""));

    let json = parse_json(&assert);
    assert!(json.get("data").is_some());
}
