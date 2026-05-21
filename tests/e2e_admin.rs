use assert_cmd::Command;
use serial_test::serial;

mod common;

fn fabio() -> Command {
    Command::cargo_bin("fabio").unwrap()
}

/// Admin APIs require Fabric Admin role. Tests verify structured output
/// regardless of whether the caller has admin permissions.
#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_tenant_settings_structured_output() {
    let output = fabio()
        .args(["admin", "list-tenant-settings"])
        .output()
        .unwrap();

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(json["data"].is_array());
    } else {
        // FORBIDDEN is expected if caller lacks admin role
        let stderr = String::from_utf8_lossy(&output.stderr);
        let json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(json["error"]["code"], "FORBIDDEN");
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_workspaces_structured_output() {
    let output = fabio()
        .args(["admin", "list-workspaces"])
        .output()
        .unwrap();

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(json["data"].is_array());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(json["error"]["code"], "FORBIDDEN");
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn admin_list_tags_structured_output() {
    let output = fabio()
        .args(["admin", "list-tags"])
        .output()
        .unwrap();

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(json["data"].is_array());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(json["error"]["code"], "FORBIDDEN");
    }
}
