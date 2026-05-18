//! End-to-end integration tests for `fabio lakehouse` file operations
//! (copy-file, move-file, delete-file).

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

/// Helper: upload a test file and return its remote path.
fn upload_test_file(cfg: &TestConfig, name: &str, content: &str) -> String {
    let tmp_dir = TempDir::new().unwrap();
    let local_path = tmp_dir.path().join("file.txt");
    fs::write(&local_path, content).unwrap();
    let remote_path = format!("Files/{name}");

    fabio()
        .args([
            "lakehouse",
            "upload",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            local_path.to_str().unwrap(),
            "--dest-path",
            &remote_path,
        ])
        .assert()
        .success();

    remote_path
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_copy_file_across_workspaces() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("copy_src");
    let src_path = upload_test_file(&cfg, &format!("{name}.txt"), "copy test content");
    let dst_path = format!("Files/{name}_dest.txt");

    // Copy from source to dest lakehouse
    let assert = fabio()
        .args([
            "lakehouse",
            "copy-file",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &src_path,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "--dest-path",
            &dst_path,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "copied");

    // Cleanup
    fabio()
        .args([
            "lakehouse",
            "delete-file",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--path",
            &src_path,
        ])
        .assert()
        .success();

    fabio()
        .args([
            "lakehouse",
            "delete-file",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--path",
            &dst_path,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_move_file_within_workspace() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("move_src");
    let src_path = upload_test_file(&cfg, &format!("{name}.txt"), "move test content");
    let dst_path = format!("Files/{name}_moved.txt");

    // Move within same workspace/lakehouse
    let assert = fabio()
        .args([
            "lakehouse",
            "move-file",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &src_path,
            "--dest-workspace",
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &dst_path,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "moved");

    // Cleanup destination (source was already deleted by move)
    fabio()
        .args([
            "lakehouse",
            "delete-file",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--path",
            &dst_path,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_delete_file_nonexistent_returns_error() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "lakehouse",
            "delete-file",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--path",
            "Files/nonexistent_file_12345.txt",
        ])
        .assert()
        .failure();
}
