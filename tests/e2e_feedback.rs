use assert_cmd::Command;
use serial_test::serial;

fn fabio() -> Command {
    Command::cargo_bin("fabio").unwrap()
}

#[test]
#[serial]
fn feedback_send_records_message() {
    let assert = fabio()
        .args([
            "feedback",
            "send",
            "This is a test feedback message from E2E",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["status"], "recorded");
}

#[test]
#[serial]
fn feedback_list_returns_array() {
    // Send one first to ensure list isn't empty
    fabio()
        .args(["feedback", "send", "E2E test feedback entry"])
        .assert()
        .success();

    let assert = fabio().args(["feedback", "list"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_array());
    assert!(json["count"].as_u64().unwrap() >= 1);
}
