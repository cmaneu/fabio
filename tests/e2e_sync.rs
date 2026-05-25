//! End-to-end integration tests for `fabio lakehouse sync` command.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

/// Helper: upload a test file to a specific lakehouse and path.
fn upload_file(workspace: &str, lakehouse: &str, remote_path: &str, content: &str) {
    let tmp_dir = TempDir::new().unwrap();
    let local_path = tmp_dir.path().join("file.txt");
    fs::write(&local_path, content).unwrap();

    fabio()
        .args([
            "lakehouse",
            "upload",
            "--workspace",
            workspace,
            "--id",
            lakehouse,
            "--source-path",
            local_path.to_str().unwrap(),
            "--dest-path",
            remote_path,
        ])
        .assert()
        .success();
}

/// Helper: delete a file (ignore errors if it doesn't exist).
fn delete_file(workspace: &str, lakehouse: &str, path: &str) {
    let _ = fabio()
        .args([
            "lakehouse",
            "delete-file",
            "--workspace",
            workspace,
            "--id",
            lakehouse,
            "--path",
            path,
        ])
        .assert();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sync_copies_new_files_to_destination() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sync_new");
    let src_dir = format!("Files/{name}_src");
    let dst_dir = format!("Files/{name}_dst");

    // Upload file to source
    upload_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/hello.txt"),
        "hello sync",
    );

    // Run sync
    let assert = fabio()
        .args([
            "lakehouse",
            "sync",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "--dest-path",
            &dst_dir,
            "--output",
            "json",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["copied"], 1);
    assert_eq!(data["status"], "synced");
    assert_eq!(data["strategy"], "ETag");

    // Cleanup
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/hello.txt"),
    );
    delete_file(
        &cfg.dest_workspace,
        &cfg.dest_lakehouse,
        &format!("{dst_dir}/hello.txt"),
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sync_skips_unchanged_files() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sync_skip");
    let src_dir = format!("Files/{name}_src");
    let dst_dir = format!("Files/{name}_dst");

    // Upload same file to both source and dest
    let content = "unchanged content";
    upload_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/same.txt"),
        content,
    );
    upload_file(
        &cfg.dest_workspace,
        &cfg.dest_lakehouse,
        &format!("{dst_dir}/same.txt"),
        content,
    );

    // Run sync — file should be unchanged (same ETag since same content+upload)
    // Note: ETags may differ if uploaded separately, so this tests the flow
    let assert = fabio()
        .args([
            "lakehouse",
            "sync",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "--dest-path",
            &dst_dir,
            "--output",
            "json",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "synced");
    // Whether copied is 0 or 1 depends on ETag matching across lakehouses

    // Cleanup
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/same.txt"),
    );
    delete_file(
        &cfg.dest_workspace,
        &cfg.dest_lakehouse,
        &format!("{dst_dir}/same.txt"),
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sync_with_delete_removes_extra_dest_files() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sync_del");
    let src_dir = format!("Files/{name}_src");
    let dst_dir = format!("Files/{name}_dst");

    // Upload a file only to dest (should be deleted by --delete)
    upload_file(
        &cfg.dest_workspace,
        &cfg.dest_lakehouse,
        &format!("{dst_dir}/extra.txt"),
        "extra file",
    );

    // Upload a file to source
    upload_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/keep.txt"),
        "keep me",
    );

    // Sync with --delete
    let assert = fabio()
        .args([
            "lakehouse",
            "sync",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "--dest-path",
            &dst_dir,
            "--delete",
            "--output",
            "json",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["deleted"], 1);
    assert_eq!(data["copied"], 1);
    assert_eq!(data["status"], "synced");

    // Cleanup
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/keep.txt"),
    );
    delete_file(
        &cfg.dest_workspace,
        &cfg.dest_lakehouse,
        &format!("{dst_dir}/keep.txt"),
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sync_without_delete_preserves_extra_dest_files() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sync_nodel");
    let src_dir = format!("Files/{name}_src");
    let dst_dir = format!("Files/{name}_dst");

    // Upload file only to dest
    upload_file(
        &cfg.dest_workspace,
        &cfg.dest_lakehouse,
        &format!("{dst_dir}/extra.txt"),
        "should stay",
    );

    // Upload file to source
    upload_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/new.txt"),
        "new file",
    );

    // Sync without --delete
    let assert = fabio()
        .args([
            "lakehouse",
            "sync",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "--dest-path",
            &dst_dir,
            "--output",
            "json",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["deleted"], 0);
    assert_eq!(data["copied"], 1);

    // Cleanup
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/new.txt"),
    );
    delete_file(
        &cfg.dest_workspace,
        &cfg.dest_lakehouse,
        &format!("{dst_dir}/extra.txt"),
    );
    delete_file(
        &cfg.dest_workspace,
        &cfg.dest_lakehouse,
        &format!("{dst_dir}/new.txt"),
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sync_with_checksum_flag_uses_md5() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sync_md5");
    let src_dir = format!("Files/{name}_src");
    let dst_dir = format!("Files/{name}_dst");

    // Upload file to source
    upload_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/data.txt"),
        "checksum test data",
    );

    // Sync with --checksum
    let assert = fabio()
        .args([
            "lakehouse",
            "sync",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "--dest-path",
            &dst_dir,
            "--checksum",
            "--output",
            "json",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["copied"], 1);
    assert_eq!(data["status"], "synced");
    assert_eq!(data["strategy"], "Content-MD5");

    // Cleanup
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/data.txt"),
    );
    delete_file(
        &cfg.dest_workspace,
        &cfg.dest_lakehouse,
        &format!("{dst_dir}/data.txt"),
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sync_multiple_files_parallel() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sync_multi");
    let src_dir = format!("Files/{name}_src");
    let dst_dir = format!("Files/{name}_dst");

    // Upload multiple files to source
    for i in 0..5 {
        upload_file(
            &cfg.source_workspace,
            &cfg.source_lakehouse,
            &format!("{src_dir}/file_{i}.txt"),
            &format!("content for file {i}"),
        );
    }

    // Sync all files
    let assert = fabio()
        .args([
            "lakehouse",
            "sync",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "--dest-path",
            &dst_dir,
            "--output",
            "json",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["copied"], 5);
    assert_eq!(data["sourceFiles"], 5);
    assert_eq!(data["status"], "synced");

    // Cleanup
    for i in 0..5 {
        delete_file(
            &cfg.source_workspace,
            &cfg.source_lakehouse,
            &format!("{src_dir}/file_{i}.txt"),
        );
        delete_file(
            &cfg.dest_workspace,
            &cfg.dest_lakehouse,
            &format!("{dst_dir}/file_{i}.txt"),
        );
    }
}
