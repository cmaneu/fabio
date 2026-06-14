use assert_cmd::Command;

fn fabio() -> Command {
    Command::cargo_bin("fabio").unwrap()
}

#[test]
fn selfupdate_help_shows_usage() {
    fabio()
        .args(["selfupdate", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Update fabio to the latest release",
        ));
}

#[test]
fn selfupdate_dry_run_shows_plan() {
    let assert = fabio()
        .args(["selfupdate", "--dry-run", "--force"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["dry_run"], true);
    assert!(
        json["data"]["would_execute"]
            .as_str()
            .unwrap()
            .contains("selfupdate")
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
fn selfupdate_check_reports_version() {
    let assert = fabio().args(["selfupdate", "--check"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Should have current_version and latest_version fields
    assert!(json["data"]["current_version"].is_string());
    assert!(json["data"]["latest_version"].is_string());
    assert!(json["data"]["update_available"].is_boolean());
}

#[test]
fn selfupdate_dry_run_specific_version() {
    // --force needed because 0.23.0 < current version (would be a downgrade)
    let assert = fabio()
        .args([
            "selfupdate",
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
fn selfupdate_dry_run_with_v_prefix_version() {
    // Should strip the v prefix gracefully (--force needed for downgrade)
    let assert = fabio()
        .args([
            "selfupdate",
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
fn selfupdate_json_output() {
    let assert = fabio()
        .args(["--output", "json", "selfupdate", "--check"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Should be valid JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"].is_object());
}

#[test]
fn selfupdate_refuses_downgrade_without_force() {
    // Targeting an older version without --force should refuse to proceed
    let assert = fabio()
        .args(["selfupdate", "--target-version", "0.0.1"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["status"], "up_to_date");
    assert!(
        json["data"]["message"]
            .as_str()
            .unwrap()
            .contains("--force to downgrade")
    );
}

#[test]
fn selfupdate_check_reports_not_available_for_older() {
    // --check with an older release should report update_available: false
    let assert = fabio().args(["selfupdate", "--check"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Since the latest release (0.1.0) is older than our dev version (0.24.0)
    assert_eq!(json["data"]["update_available"], false);
}
