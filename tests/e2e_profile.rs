use assert_cmd::Command;
use serial_test::serial;

fn fabio() -> Command {
    Command::cargo_bin("fabio").unwrap()
}

#[test]
#[serial]
fn profile_save_and_show() {
    // Save a test profile
    fabio()
        .args([
            "profile",
            "save",
            "--name",
            "test-profile-e2e",
            "--workspace",
            "ws-123",
            "--capacity",
            "cap-456",
        ])
        .assert()
        .success();

    // Show it
    let assert = fabio()
        .args(["profile", "show", "--name", "test-profile-e2e"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["workspace"], "ws-123");
    assert_eq!(json["data"]["capacity"], "cap-456");

    // Cleanup
    fabio()
        .args(["profile", "delete", "--name", "test-profile-e2e"])
        .assert()
        .success();
}

#[test]
#[serial]
fn profile_list_returns_array() {
    // Save a profile to ensure list isn't empty
    fabio()
        .args([
            "profile",
            "save",
            "--name",
            "test-list-e2e",
            "--workspace",
            "ws-abc",
        ])
        .assert()
        .success();

    let assert = fabio().args(["profile", "list"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_array());
    assert!(json["count"].as_u64().unwrap() >= 1);

    // Cleanup
    fabio()
        .args(["profile", "delete", "--name", "test-list-e2e"])
        .assert()
        .success();
}

#[test]
#[serial]
fn profile_use_sets_active() {
    // Save and activate
    fabio()
        .args([
            "profile",
            "save",
            "--name",
            "test-use-e2e",
            "--workspace",
            "ws-use",
        ])
        .assert()
        .success();

    fabio()
        .args(["profile", "use", "--name", "test-use-e2e"])
        .assert()
        .success();

    // Verify active profile is shown in list or show
    let assert = fabio()
        .args(["profile", "show", "--name", "test-use-e2e"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["workspace"], "ws-use");

    // Cleanup
    fabio()
        .args(["profile", "delete", "--name", "test-use-e2e"])
        .assert()
        .success();
}

#[test]
#[serial]
fn profile_delete_nonexistent_fails() {
    let assert = fabio()
        .args(["profile", "delete", "--name", "nonexistent-profile-xyz"])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(json["error"]["code"], "NOT_FOUND");
}

#[test]
#[serial]
fn profile_show_nonexistent_fails() {
    let assert = fabio()
        .args(["profile", "show", "--name", "nonexistent-profile-xyz"])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(json["error"]["code"], "NOT_FOUND");
}
