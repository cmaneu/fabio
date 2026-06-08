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

#[test]
#[serial]
fn profile_workspace_default_used_when_flag_omitted() {
    // Save and activate a profile with a workspace
    fabio()
        .args([
            "profile",
            "save",
            "--name",
            "test-ws-default",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
        ])
        .assert()
        .success();
    fabio()
        .args(["profile", "use", "--name", "test-ws-default"])
        .assert()
        .success();

    // Run a dry-run command WITHOUT --workspace; it should pick up from profile
    let assert = fabio()
        .args([
            "--dry-run",
            "item",
            "create",
            "--name",
            "test-item",
            "--type",
            "Notebook",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let data = &json["data"];
    assert_eq!(data["dry_run"], true);
    // The workspace in the dry-run details should match the profile value
    assert_eq!(
        data["details"]["workspace"],
        "aaaaaaaa-1111-2222-3333-444444444444"
    );

    // Cleanup
    fabio()
        .args(["profile", "delete", "--name", "test-ws-default"])
        .assert()
        .success();
}

#[test]
#[serial]
fn profile_output_default_applied() {
    // Save and activate a profile with output=table
    fabio()
        .args([
            "profile",
            "save",
            "--name",
            "test-output-default",
            "--workspace",
            "ws-out-test",
            "--default-output",
            "table",
        ])
        .assert()
        .success();
    fabio()
        .args(["profile", "use", "--name", "test-output-default"])
        .assert()
        .success();

    // Run profile list WITHOUT --output; should produce table format (not JSON)
    let assert = fabio().args(["profile", "list"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Table format uses +---+ separators and column headers
    assert!(
        stdout.contains("+--"),
        "Expected table format from profile output default, got: {stdout}"
    );

    // Cleanup
    fabio()
        .args([
            "--output",
            "json",
            "profile",
            "delete",
            "--name",
            "test-output-default",
        ])
        .assert()
        .success();
}

#[test]
#[serial]
fn profile_explicit_flag_overrides_profile_default() {
    // Save and activate a profile with output=table
    fabio()
        .args([
            "profile",
            "save",
            "--name",
            "test-override",
            "--workspace",
            "ws-override",
            "--default-output",
            "table",
        ])
        .assert()
        .success();
    fabio()
        .args(["profile", "use", "--name", "test-override"])
        .assert()
        .success();

    // Explicit --output json should override the profile's table default
    let assert = fabio()
        .args(["--output", "json", "profile", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Should be valid JSON (not table format)
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_array());

    // Cleanup
    fabio()
        .args([
            "--output",
            "json",
            "profile",
            "delete",
            "--name",
            "test-override",
        ])
        .assert()
        .success();
}

#[test]
#[serial]
fn profile_env_var_overrides_profile_default() {
    // Save and activate a profile with output=table
    fabio()
        .args([
            "profile",
            "save",
            "--name",
            "test-env-override",
            "--workspace",
            "ws-env",
            "--default-output",
            "table",
        ])
        .assert()
        .success();
    fabio()
        .args(["profile", "use", "--name", "test-env-override"])
        .assert()
        .success();

    // External env var should override profile default
    let assert = fabio()
        .env("FABIO_OUTPUT", "csv")
        .args(["profile", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // CSV format has comma-separated values
    assert!(
        stdout.contains(','),
        "Expected CSV format from env var override, got: {stdout}"
    );

    // Cleanup
    fabio()
        .args([
            "--output",
            "json",
            "profile",
            "delete",
            "--name",
            "test-env-override",
        ])
        .assert()
        .success();
}
