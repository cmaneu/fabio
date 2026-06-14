use assert_cmd::Command;

fn fabio() -> Command {
    Command::cargo_bin("fabio").unwrap()
}

#[test]
fn upgrade_help_shows_usage() {
    fabio()
        .args(["upgrade", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Upgrade fabio to the latest release",
        ));
}

#[test]
fn upgrade_dry_run_shows_plan() {
    let assert = fabio()
        .args(["upgrade", "--dry-run", "--force"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
    assert!(
        json["data"]["would_execute"]
            .as_str()
            .unwrap()
            .contains("upgrade")
    );
    // Details should include the artifact name
    assert!(
        json["data"]["details"]["artifact"]
            .as_str()
            .unwrap()
            .starts_with("fabio-")
    );
}

#[test]
fn upgrade_check_reports_version() {
    let assert = fabio().args(["upgrade", "--check"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Should have current_version and latest_version fields
    assert!(json["data"]["current_version"].is_string());
    assert!(json["data"]["latest_version"].is_string());
    assert!(json["data"]["update_available"].is_boolean());
}

#[test]
fn upgrade_dry_run_specific_version() {
    // --force needed because 0.23.0 < current version (would be a downgrade)
    let assert = fabio()
        .args([
            "upgrade",
            "--dry-run",
            "--force",
            "--target-version",
            "0.23.0",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
    assert!(
        json["data"]["details"]["target_version"]
            .as_str()
            .unwrap()
            .contains("0.23.0")
    );
}

#[test]
fn upgrade_dry_run_with_v_prefix_version() {
    // Should strip the v prefix gracefully (--force needed for downgrade)
    let assert = fabio()
        .args([
            "upgrade",
            "--dry-run",
            "--force",
            "--target-version",
            "v0.23.0",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["details"]["target_version"], "0.23.0");
}

#[test]
fn upgrade_json_output() {
    let assert = fabio()
        .args(["--output", "json", "upgrade", "--check"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Should be valid JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_object());
}

#[test]
fn upgrade_refuses_on_dev_build_even_with_target_version() {
    // On dev builds, any upgrade attempt (even with --target-version) hits the dev guard
    let assert = fabio()
        .args(["upgrade", "--target-version", "0.0.1"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["status"], "dev_build");
    assert!(
        json["data"]["message"]
            .as_str()
            .unwrap()
            .contains("development build")
    );
}

#[test]
fn upgrade_check_reports_not_available_for_older() {
    // --check with an older release should report update_available: false
    let assert = fabio().args(["upgrade", "--check"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Since the latest release (0.1.0) is older than our dev version (0.25.0-dev)
    assert_eq!(json["data"]["update_available"], false);
}

#[test]
fn upgrade_dev_build_refuses_without_force() {
    // Dev builds (version contains -dev) should refuse upgrade
    let assert = fabio().args(["upgrade"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["status"], "dev_build");
    assert!(
        json["data"]["message"]
            .as_str()
            .unwrap()
            .contains("development build")
    );
}

#[test]
fn upgrade_dev_build_check_still_works() {
    // --check should still work on dev builds (informational, no mutation)
    let assert = fabio().args(["upgrade", "--check"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        json["data"]["current_version"]
            .as_str()
            .unwrap()
            .contains("-dev")
    );
}
