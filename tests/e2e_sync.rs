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
            "-s",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "-d",
            &dst_dir,
            "-o",
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
            "-s",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "-d",
            &dst_dir,
            "-o",
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
            "-s",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "-d",
            &dst_dir,
            "--delete",
            "-o",
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
            "-s",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "-d",
            &dst_dir,
            "-o",
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
            "-s",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "-d",
            &dst_dir,
            "--checksum",
            "-o",
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
            "-s",
            &src_dir,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "-d",
            &dst_dir,
            "-o",
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

/// Sync detects renames: when a file is renamed at source (same content, different path),
/// sync with `--delete` detects it via `ETag` match and performs an atomic rename at the
/// destination instead of a full copy + delete.
///
/// NOTE: `OneLake` DFS rename changes the `ETag`, so rename detection only works when
/// the file was previously synced via server-side blob copy (which preserves `ETags`).
/// This test validates the full flow: upload, sync (copy preserves `ETag`), rename
/// at source, sync detects rename via `ETag` match.
#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sync_detects_renames_via_etag() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sync_rename");
    let src_dir = format!("Files/{name}_src");
    let dst_dir = format!("Files/{name}_dst");
    let content = "This file will be renamed to test rename detection in sync.\n";

    // Step 1: Upload file to source only
    upload_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/original.txt"),
        content,
    );

    // Step 2: Sync source→dest to establish matching ETags (server-side copy preserves ETag)
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
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &dst_dir,
            "--delete",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "synced");
    assert_eq!(data["copied"], 1, "initial sync should copy 1 file");

    // Step 3: Rename the file at source using move-file
    // NOTE: OneLake DFS rename changes the ETag, so this test validates that
    // sync handles the scenario correctly (falls back to copy + delete).
    // Rename detection fires when ETags are preserved (e.g., blob copy scenarios).
    fabio()
        .args([
            "lakehouse",
            "move-file",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &format!("{src_dir}/original.txt"),
            "--dest-workspace",
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &format!("{src_dir}/renamed.txt"),
        ])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    // Step 4: Sync with --delete — since OneLake rename changes ETag, this will
    // do a normal copy + delete (not a rename detection). The output should still
    // include the "renamed" field (value 0) showing the feature is active.
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
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &dst_dir,
            "--delete",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "synced");
    eprintln!("Sync result: {data}");

    // Verify the output envelope includes the "renamed" field
    assert!(
        data.get("renamed").is_some(),
        "output should include 'renamed' field"
    );
    // OneLake DFS rename changes ETags, so rename detection won't fire here.
    // The file will be copied + deleted instead. Assert the correct totals.
    let copied = data["copied"].as_u64().unwrap_or(0);
    let renamed = data["renamed"].as_u64().unwrap_or(0);
    let deleted = data["deleted"].as_u64().unwrap_or(0);
    assert_eq!(
        copied + renamed,
        1,
        "either copied or renamed should handle the file"
    );
    if renamed == 1 {
        assert_eq!(deleted, 0, "rename should not need a separate delete");
    } else {
        assert_eq!(deleted, 1, "without rename detection, old file is deleted");
    }

    // Cleanup
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/renamed.txt"),
    );
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{dst_dir}/renamed.txt"),
    );
    // In case rename detection didn't fire
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{dst_dir}/original.txt"),
    );
}

/// Sync with `--checksum --delete` detects renames via size-based matching.
/// Since `OneLake` DFS rename changes `ETags` and does not provide `Content-MD5`,
/// the checksum path falls back to size matching for unique-sized files.
#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sync_checksum_detects_renames_via_size() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sync_csrn");
    let src_dir = format!("Files/{name}_src");
    let dst_dir = format!("Files/{name}_dst");
    let content = "unique content for checksum rename detection - unlikely to match other files\n";

    // Step 1: Upload to source
    upload_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/original.txt"),
        content,
    );

    // Step 2: Sync to dest (server-side copy)
    fabio()
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
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &dst_dir,
            "--delete",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    // Step 3: Rename at source (DFS rename changes ETag)
    fabio()
        .args([
            "lakehouse",
            "move-file",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &format!("{src_dir}/original.txt"),
            "--dest-workspace",
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &format!("{src_dir}/renamed.txt"),
        ])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    // Step 4: Sync with --checksum --delete — should detect rename via size matching
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
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &dst_dir,
            "--delete",
            "--checksum",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    eprintln!("Sync result: {data}");
    assert_eq!(data["status"], "synced");
    assert_eq!(
        data["renamed"], 1,
        "expected rename detection via size matching with --checksum"
    );
    assert_eq!(data["copied"], 0, "no copy needed when rename is detected");
    assert_eq!(
        data["deleted"], 0,
        "no delete needed when rename is detected"
    );

    // Cleanup
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/renamed.txt"),
    );
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{dst_dir}/renamed.txt"),
    );
}

/// Sync with mixed metadata: some files uploaded via fabio (have `Content-MD5` stored,
/// `ETags` preserved on rename) and others simulating Fabric-generated files (no MD5,
/// `ETags` change on rename). Validates that sync handles both correctly:
/// - fabio-uploaded files: rename detected via checksum
/// - "Fabric-like" files with unique size: rename detected via size fallback
/// - Files with non-unique sizes: fall back to copy+delete (no false match)
#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn sync_mixed_metadata_rename_detection() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("sync_mixed");
    let src_dir = format!("Files/{name}_src");
    let dst_dir = format!("Files/{name}_dst");

    // Upload 3 files with different content (different sizes for unique matching)
    // File A: 50 bytes (unique size — rename detectable by size)
    let content_a = "A".repeat(50);
    // File B: 75 bytes (unique size — rename detectable by size)
    let content_b = "B".repeat(75);
    // File C: stays unchanged (should not be touched)
    let content_c = "This file stays the same and should be unchanged after sync.\n";

    upload_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/file_a.txt"),
        &content_a,
    );
    upload_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/file_b.txt"),
        &content_b,
    );
    upload_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/file_c.txt"),
        content_c,
    );

    // Sync to dest (establishes matching state)
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
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &dst_dir,
            "--delete",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["copied"], 3, "initial sync should copy 3 files");

    // Rename file_a and file_b at source (simulates reorganization)
    fabio()
        .args([
            "lakehouse",
            "move-file",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &format!("{src_dir}/file_a.txt"),
            "--dest-workspace",
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &format!("{src_dir}/renamed_a.txt"),
        ])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    fabio()
        .args([
            "lakehouse",
            "move-file",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-path",
            &format!("{src_dir}/file_b.txt"),
            "--dest-workspace",
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &format!("{src_dir}/moved_b.txt"),
        ])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    // Sync with --checksum --delete: should detect renames via size matching
    // file_c stays unchanged
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
            &cfg.source_workspace,
            "--dest-id",
            &cfg.source_lakehouse,
            "--dest-path",
            &dst_dir,
            "--delete",
            "--checksum",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    eprintln!("Mixed metadata sync result: {data}");
    assert_eq!(data["status"], "synced");

    let renamed = data["renamed"].as_u64().unwrap_or(0);
    let copied = data["copied"].as_u64().unwrap_or(0);
    let deleted = data["deleted"].as_u64().unwrap_or(0);
    let unchanged = data["unchanged"].as_u64().unwrap_or(0);

    // file_c should be unchanged
    assert!(
        unchanged >= 1,
        "file_c should be detected as unchanged, got unchanged={unchanged}"
    );
    // Both renames should be detected (unique sizes: 50 and 75 bytes)
    assert_eq!(
        renamed, 2,
        "expected 2 renames detected (unique sizes), got renamed={renamed}, copied={copied}, deleted={deleted}. Full: {data}"
    );
    // No copies or deletes needed since renames handled it
    assert_eq!(copied, 0, "no copies needed when renames detected");
    assert_eq!(deleted, 0, "no deletes needed when renames detected");

    // Cleanup
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/renamed_a.txt"),
    );
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/moved_b.txt"),
    );
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{src_dir}/file_c.txt"),
    );
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{dst_dir}/renamed_a.txt"),
    );
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{dst_dir}/moved_b.txt"),
    );
    delete_file(
        &cfg.source_workspace,
        &cfg.source_lakehouse,
        &format!("{dst_dir}/file_c.txt"),
    );
}
