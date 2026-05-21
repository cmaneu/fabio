use assert_cmd::Command;
use serial_test::serial;

mod common;

fn fabio() -> Command {
    Command::cargo_bin("fabio").unwrap()
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn catalog_search_succeeds() {
    let assert = fabio()
        .args(["catalog", "search", "--query", "test"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Result may be null (no matches), an array, or an object
    assert!(json.get("data").is_some());
}
