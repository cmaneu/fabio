//! End-to-end integration tests for `fabio lakehouse` basic commands
//! (tables, files, upload, download).

mod common;

use common::{TestConfig, extract_count, extract_data, fabio, parse_json};
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_tables_returns_list() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "lakehouse",
            "tables",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let count = extract_count(&json);

    assert!(count > 0, "expected at least one table in source lakehouse");
    let arr = data.as_array().unwrap();
    let first = &arr[0];
    assert!(first.get("name").is_some());
    assert!(first.get("format").is_some());
    assert_eq!(first["format"], "delta");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_tables_table_format() {
    let cfg = TestConfig::from_env();

    // Table output for lakehouse tables
    fabio()
        .args([
            "--output",
            "table",
            "lakehouse",
            "tables",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_files_returns_entries() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "lakehouse",
            "files",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let count = extract_count(&json);

    assert!(count > 0, "expected at least one file entry");
    let arr = data.as_array().unwrap();
    let first = &arr[0];
    assert!(first.get("name").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_files_with_path_filter() {
    let cfg = TestConfig::from_env();

    // Upload a file into a subdirectory first
    let tmp_dir = TempDir::new().unwrap();
    let upload_path = tmp_dir.path().join("subdir_test.txt");
    fs::write(&upload_path, "path filter test").unwrap();

    let subdir = format!("Files/{}", common::unique_name("subdir"));
    let remote_path = format!("{subdir}/test.txt");

    fabio()
        .args([
            "lakehouse",
            "upload",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            upload_path.to_str().unwrap(),
            "--dest-path",
            &remote_path,
        ])
        .assert()
        .success();

    // List with --path filter pointing to the subdirectory
    let assert = fabio()
        .args([
            "lakehouse",
            "files",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--path",
            &subdir,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let arr = data.as_array().unwrap();

    // Should find our file
    let found = arr.iter().any(|f| {
        f.get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| n.contains("test.txt"))
    });
    assert!(found, "uploaded file not found with --path filter");

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
            &remote_path,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_upload_and_download_roundtrip() {
    let cfg = TestConfig::from_env();
    let tmp_dir = TempDir::new().unwrap();
    let file_name = common::unique_name("upload_test");
    let content = format!("test content from {file_name}");
    let remote_path = format!("Files/{file_name}.txt");

    // Create a local file
    let upload_path = tmp_dir.path().join("upload.txt");
    fs::write(&upload_path, &content).unwrap();

    // Upload
    let assert = fabio()
        .args([
            "lakehouse",
            "upload",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            upload_path.to_str().unwrap(),
            "--dest-path",
            &remote_path,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "uploaded");
    assert_eq!(data["size"], content.len());

    // Download
    let download_path = tmp_dir.path().join("download.txt");
    let assert = fabio()
        .args([
            "lakehouse",
            "download",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            &remote_path,
            "--dest-path",
            download_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "downloaded");

    // Verify content matches
    let downloaded = fs::read_to_string(&download_path).unwrap();
    assert_eq!(downloaded, content);

    // Cleanup: delete remote file
    fabio()
        .args([
            "lakehouse",
            "delete-file",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--path",
            &remote_path,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_upload_nonexistent_source_errors() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "lakehouse",
            "upload",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            "/tmp/nonexistent_file_xyz_12345.txt",
            "--dest-path",
            "Files/should_not_exist.txt",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("INVALID_INPUT").or(predicate::str::contains("error")));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_download_nonexistent_source_errors() {
    let cfg = TestConfig::from_env();
    let tmp_dir = TempDir::new().unwrap();
    let local_path = tmp_dir.path().join("should_not_be_created.txt");

    fabio()
        .args([
            "lakehouse",
            "download",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            "Files/absolutely_nonexistent_file_xyz.txt",
            "--dest-path",
            local_path.to_str().unwrap(),
        ])
        .assert()
        .failure();

    // File should not have been created
    assert!(!local_path.exists());
}
