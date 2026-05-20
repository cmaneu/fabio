//! End-to-end integration tests for `fabio lakehouse` table operations
//! (load-table, copy-table, delete-table).

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

/// Helper: upload a CSV and load it as a Delta table.
/// Returns the table name.
fn create_test_table(cfg: &TestConfig, table_name: &str) -> String {
    let tmp_dir = TempDir::new().unwrap();
    let csv_content = "id,name,value\n1,alpha,100\n2,beta,200\n3,gamma,300\n";
    let local_path = tmp_dir.path().join("data.csv");
    fs::write(&local_path, csv_content).unwrap();

    let remote_csv = format!("Files/{table_name}.csv");

    // Upload CSV
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
            &remote_csv,
        ])
        .assert()
        .success();

    // Load into table
    fabio()
        .args([
            "lakehouse",
            "load-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            &remote_csv,
            "--table",
            table_name,
            "--mode",
            "Overwrite",
            "--format",
            "Csv",
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    // Cleanup the CSV
    fabio()
        .args([
            "lakehouse",
            "delete-file",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--path",
            &remote_csv,
        ])
        .assert()
        .success();

    table_name.to_string()
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_load_table_succeeds() {
    let cfg = TestConfig::from_env();
    let table_name = common::unique_name("load_test");

    let tmp_dir = TempDir::new().unwrap();
    let csv_content = "x,y\n1,hello\n2,world\n";
    let local_path = tmp_dir.path().join("data.csv");
    fs::write(&local_path, csv_content).unwrap();

    let remote_csv = format!("Files/{table_name}.csv");

    // Upload
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
            &remote_csv,
        ])
        .assert()
        .success();

    // Load table
    let assert = fabio()
        .args([
            "lakehouse",
            "load-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            &remote_csv,
            "--table",
            &table_name,
            "--mode",
            "Overwrite",
            "--format",
            "Csv",
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "Succeeded");

    // Verify table appears in listing
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
    let found = data
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["name"].as_str() == Some(table_name.as_str()));
    assert!(found, "table not found in listing");

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
            &remote_csv,
        ])
        .assert()
        .success();

    fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--table",
            &table_name,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_copy_table_and_delete() {
    let cfg = TestConfig::from_env();
    let src_table = common::unique_name("copy_src");
    let dst_table = common::unique_name("copy_dst");

    // Create source table
    create_test_table(&cfg, &src_table);

    // Copy table to dest lakehouse
    let assert = fabio()
        .args([
            "lakehouse",
            "copy-table",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-table",
            &src_table,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "--dest-table",
            &dst_table,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "copied");
    assert!(data["filesCopied"].as_u64().unwrap() > 0);

    // Delete destination table
    fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--table",
            &dst_table,
        ])
        .assert()
        .success();

    // Delete source table
    fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--table",
            &src_table,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_delete_table_succeeds() {
    let cfg = TestConfig::from_env();
    let table_name = common::unique_name("del_test");

    // Create a table
    create_test_table(&cfg, &table_name);

    // Delete it
    let assert = fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--table",
            &table_name,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
    assert_eq!(data["table"], table_name);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_move_table_across_workspaces() {
    let cfg = TestConfig::from_env();
    let src_table = common::unique_name("mv_src");
    let dst_table = common::unique_name("mv_dst");

    // Create source table
    create_test_table(&cfg, &src_table);

    // Move table from source to dest lakehouse with a new name
    let assert = fabio()
        .args([
            "lakehouse",
            "move-table",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-table",
            &src_table,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
            "--dest-table",
            &dst_table,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "moved");
    assert_eq!(data["sourceTable"], src_table);
    assert_eq!(data["destTable"], dst_table);

    // Verify source table is gone (delete should fail)
    fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--table",
            &src_table,
        ])
        .assert()
        .failure();

    // Cleanup: delete destination table
    fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--table",
            &dst_table,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Delete non-existent table returns error
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_delete_table_nonexistent_returns_error() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--table",
            "nonexistent_table_xyz_12345",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    let code = err_json["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "NOT_FOUND" || code == "API_ERROR",
        "Expected NOT_FOUND or API_ERROR, got: {code}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_move_table_without_dest_name() {
    let cfg = TestConfig::from_env();
    let table_name = common::unique_name("mv_same");

    // Create source table
    create_test_table(&cfg, &table_name);

    // Move without --dest-table (should keep same name)
    let assert = fabio()
        .args([
            "lakehouse",
            "move-table",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-table",
            &table_name,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "moved");
    assert_eq!(data["sourceTable"], table_name);
    assert_eq!(data["destTable"], table_name);

    // Cleanup
    fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--table",
            &table_name,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_copy_table_without_dest_name() {
    let cfg = TestConfig::from_env();
    let src_table = common::unique_name("cp_same");

    // Create source table
    create_test_table(&cfg, &src_table);

    // Copy without --dest-table (should keep same name)
    let assert = fabio()
        .args([
            "lakehouse",
            "copy-table",
            "--source-workspace",
            &cfg.source_workspace,
            "--source-id",
            &cfg.source_lakehouse,
            "--source-table",
            &src_table,
            "--dest-workspace",
            &cfg.dest_workspace,
            "--dest-id",
            &cfg.dest_lakehouse,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "copied");
    assert_eq!(data["sourceTable"], src_table);
    assert_eq!(data["destTable"], src_table); // Same name as source

    // Cleanup
    fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &cfg.dest_lakehouse,
            "--table",
            &src_table,
        ])
        .assert()
        .success();

    fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--table",
            &src_table,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn lakehouse_load_table_with_append_mode() {
    let cfg = TestConfig::from_env();
    let table_name = common::unique_name("append_test");

    let tmp_dir = tempfile::TempDir::new().unwrap();
    let csv1 = "x,y\n1,a\n2,b\n";
    let csv2 = "x,y\n3,c\n4,d\n";

    let path1 = tmp_dir.path().join("data1.csv");
    let path2 = tmp_dir.path().join("data2.csv");
    std::fs::write(&path1, csv1).unwrap();
    std::fs::write(&path2, csv2).unwrap();

    let remote1 = format!("Files/{table_name}_1.csv");
    let remote2 = format!("Files/{table_name}_2.csv");

    // Upload both CSVs
    fabio()
        .args([
            "lakehouse",
            "upload",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            path1.to_str().unwrap(),
            "--dest-path",
            &remote1,
        ])
        .assert()
        .success();

    fabio()
        .args([
            "lakehouse",
            "upload",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            path2.to_str().unwrap(),
            "--dest-path",
            &remote2,
        ])
        .assert()
        .success();

    // Load first CSV with Overwrite
    fabio()
        .args([
            "lakehouse",
            "load-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            &remote1,
            "--table",
            &table_name,
            "--mode",
            "Overwrite",
            "--format",
            "Csv",
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    // Load second CSV with Append
    let assert = fabio()
        .args([
            "lakehouse",
            "load-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--source-path",
            &remote2,
            "--table",
            &table_name,
            "--mode",
            "Append",
            "--format",
            "Csv",
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should succeed with loaded or Succeeded status
    let status = data.get("status").and_then(|s| s.as_str()).unwrap_or("");
    assert!(
        status == "loaded" || status == "Succeeded",
        "unexpected status: {status}"
    );

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
            &remote1,
        ])
        .assert()
        .success();

    fabio()
        .args([
            "lakehouse",
            "delete-file",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--path",
            &remote2,
        ])
        .assert()
        .success();

    fabio()
        .args([
            "lakehouse",
            "delete-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--table",
            &table_name,
        ])
        .assert()
        .success();
}
