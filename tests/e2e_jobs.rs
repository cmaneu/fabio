use assert_cmd::Command;
use serial_test::serial;

fn fabio() -> Command {
    Command::cargo_bin("fabio").unwrap()
}

#[test]
#[serial]
fn jobs_list_returns_array() {
    let assert = fabio().args(["jobs", "list"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_array());
}

#[test]
#[serial]
fn jobs_list_with_status_filter() {
    let assert = fabio()
        .args(["jobs", "list", "--status", "completed"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_array());
}

#[test]
#[serial]
fn jobs_get_nonexistent_fails() {
    let assert = fabio()
        .args(["jobs", "get", "--id", "nonexistent-job-xyz-000"])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(json["error"]["code"], "NOT_FOUND");
}

#[test]
#[serial]
fn jobs_prune_succeeds() {
    // Prune should always succeed (even with empty ledger)
    let assert = fabio().args(["jobs", "prune"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"]["pruned"].is_number());
}
