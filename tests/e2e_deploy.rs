//! End-to-end integration tests for `fabio deploy` commands.
//!
//! Tests the plan/apply/export workflow against a live Fabric tenant.
//! Requires `FABIO_TEST_SOURCE_WORKSPACE` and `FABIO_TEST_CAPACITY_ID` env vars.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json, unique_name};
use serial_test::serial;
use std::time::Duration;

// ── Export ────────────────────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_export_workspace_to_directory() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let output_dir = dir.path().join("export");

    let assert = fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            output_dir.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(300))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "exported");
    assert!(data["total_items"].as_u64().unwrap() > 0);
    assert!(data["exported"].as_u64().unwrap() > 0);

    // Verify directory structure was created
    assert!(output_dir.exists());
    // Should have at least one subdirectory with a .platform file
    let entries: Vec<_> = std::fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().is_dir())
        .collect();
    assert!(
        !entries.is_empty(),
        "Expected at least one item directory in export"
    );

    // Check first item dir has .platform file
    let first_item_dir = &entries[0].path();
    assert!(
        first_item_dir.join(".platform").exists(),
        "Expected .platform file in {}",
        first_item_dir.display()
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_export_with_item_type_filter() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let output_dir = dir.path().join("export_filtered");

    let assert = fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            output_dir.to_str().unwrap(),
            "--item-types",
            "Lakehouse",
        ])
        .timeout(Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "exported");

    // All exported directories should be Lakehouse type
    if let Some(exported) = data["exported"].as_u64() {
        if exported > 0 {
            for entry in std::fs::read_dir(&output_dir).unwrap().flatten() {
                if entry.path().is_dir() {
                    let platform_path = entry.path().join(".platform");
                    if platform_path.exists() {
                        let content = std::fs::read_to_string(&platform_path).unwrap();
                        let meta: serde_json::Value = serde_json::from_str(&content).unwrap();
                        assert_eq!(meta["metadata"]["type"], "Lakehouse");
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_export_dry_run() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let output_dir = dir.path().join("export_dry");

    let assert = fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            output_dir.to_str().unwrap(),
            "--dry-run",
        ])
        .timeout(Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");

    // Directory should NOT be populated in dry-run mode
    // (the export may create the dir but not write item files)
}

// ── Plan ─────────────────────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_exported_workspace_shows_skip_or_update() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");

    // Step 1: Export the workspace
    fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            export_dir.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(300))
        .assert()
        .success();

    // Step 2: Plan from exported dir back to same workspace
    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
        ])
        .timeout(Duration::from_secs(300))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Since we just exported and plan back, items should be skip (unchanged)
    // or update (if hash algorithm differs from API normalization)
    let summary = &data["summary"];
    let total = summary["create"].as_u64().unwrap_or(0)
        + summary["update"].as_u64().unwrap_or(0)
        + summary["skip"].as_u64().unwrap_or(0);
    assert!(total > 0, "Expected at least one item in plan");

    // Should NOT have any create (items already exist)
    // Note: may have updates due to hash normalization differences
    assert_eq!(
        summary["create"].as_u64().unwrap_or(0),
        0,
        "Expected no creates when planning against same workspace"
    );

    // Should NOT have deletes (not using --delete-orphans)
    assert_eq!(
        summary["delete"].as_u64().unwrap_or(0),
        0,
        "Expected no deletes without --delete-orphans"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_force_all_shows_all_updates() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");

    // Step 1: Export
    fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            export_dir.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(300))
        .assert()
        .success();

    // Step 2: Plan with --force-all
    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
            "--force-all",
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    let summary = &data["summary"];
    // With --force-all, all items should be marked as update (no skip)
    assert_eq!(
        summary["skip"].as_u64().unwrap_or(0),
        0,
        "Expected no skips with --force-all"
    );
    assert!(
        summary["update"].as_u64().unwrap_or(0) > 0,
        "Expected updates with --force-all"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_with_item_type_filter() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");

    // Export
    fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            export_dir.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(300))
        .assert()
        .success();

    // Plan with item-type filter
    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
            "--item-types",
            "Lakehouse",
            "--force-all",
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // All changes should be Lakehouse type
    if let Some(changes) = data["changes"].as_array() {
        for change in changes {
            assert_eq!(
                change["item_type"], "Lakehouse",
                "Expected only Lakehouse items when filtered"
            );
        }
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_save_to_file() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");
    let plan_file = dir.path().join("plan.json");

    // Export
    fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            export_dir.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(300))
        .assert()
        .success();

    // Plan with --out (save to file)
    fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
            "--force-all",
            "--out",
            plan_file.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    // Verify plan file exists and is valid JSON
    assert!(plan_file.exists(), "Plan file should have been written");
    let content = std::fs::read_to_string(&plan_file).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(plan["version"], 1);
    assert!(plan["workspace_id"].is_string());
    assert!(plan["changeset"].is_object());
    assert!(plan["changeset"]["changes"].is_array());
}

// ── Apply (dry-run only to avoid mutation) ────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_apply_dry_run() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");

    // Export
    fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            export_dir.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(300))
        .assert()
        .success();

    // Apply with --dry-run (no actual mutations)
    let assert = fabio()
        .args([
            "deploy",
            "apply",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
            "--force-all",
            "--dry-run",
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
    assert!(data["summary"].is_object());
}

// ── Apply (real create/update cycle on dest workspace) ────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_apply_create_notebook_and_cleanup() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let source_dir = dir.path().join("source");
    let name = unique_name("deploy_nb");

    // Create a minimal notebook source directory
    let nb_dir = source_dir.join(format!("{name}.Notebook"));
    std::fs::create_dir_all(&nb_dir).unwrap();

    // Write .platform file
    let platform = serde_json::json!({
        "metadata": {
            "type": "Notebook",
            "displayName": name
        },
        "config": {
            "version": "2.0",
            "logicalId": "e2e-test-lid-001",
            "definitionFormat": "ipynb"
        }
    });
    std::fs::write(
        nb_dir.join(".platform"),
        serde_json::to_string_pretty(&platform).unwrap(),
    )
    .unwrap();

    // Write notebook content (minimal ipynb)
    let ipynb = serde_json::json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": {
            "language_info": { "name": "python" }
        },
        "cells": [
            {
                "cell_type": "code",
                "source": ["# Deploy test\n", "print('hello')\n"],
                "metadata": {},
                "outputs": []
            }
        ]
    });
    std::fs::write(
        nb_dir.join("notebook-content.py"),
        serde_json::to_string(&ipynb).unwrap(),
    )
    .unwrap();

    // Plan first
    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            source_dir.to_str().unwrap(),
            "--workspace",
            &cfg.dest_workspace,
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["summary"]["create"].as_u64().unwrap(), 1);

    // Apply (real deployment)
    let assert = fabio()
        .args([
            "deploy",
            "apply",
            "--source",
            source_dir.to_str().unwrap(),
            "--workspace",
            &cfg.dest_workspace,
        ])
        .timeout(Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "succeeded");
    assert_eq!(data["succeeded"].as_u64().unwrap(), 1);

    // Verify item exists
    let assert = fabio()
        .args([
            "item",
            "list",
            "--workspace",
            &cfg.dest_workspace,
            "--type",
            "Notebook",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let items = extract_data(&json).as_array().unwrap().clone();
    let created = items.iter().find(|i| i["displayName"] == name);
    assert!(
        created.is_some(),
        "Expected to find deployed notebook '{name}'"
    );

    let nb_id = created.unwrap()["id"].as_str().unwrap();

    // Plan again — should show Skip (idempotent)
    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            source_dir.to_str().unwrap(),
            "--workspace",
            &cfg.dest_workspace,
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should either be skip (if hashes match) or update (if normalization differs)
    // But definitely NOT create
    assert_eq!(data["summary"]["create"].as_u64().unwrap_or(0), 0);

    // Cleanup: delete the created notebook
    fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            nb_id,
        ])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();
}

// ── Error Cases ──────────────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_nonexistent_workspace_fails() {
    let dir = tempfile::TempDir::new().unwrap();
    let source_dir = dir.path().join("source");
    std::fs::create_dir_all(&source_dir).unwrap();

    // Create minimal .platform so source isn't empty... actually, it should error
    // because workspace doesn't exist
    let nb_dir = source_dir.join("Test.Notebook");
    std::fs::create_dir_all(&nb_dir).unwrap();
    std::fs::write(
        nb_dir.join(".platform"),
        r#"{"metadata":{"type":"Notebook","displayName":"Test"},"config":{"version":"2.0"}}"#,
    )
    .unwrap();
    std::fs::write(nb_dir.join("notebook-content.py"), "# test").unwrap();

    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            source_dir.to_str().unwrap(),
            "--workspace",
            "NonExistentWorkspace12345",
        ])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("NOT_FOUND"),
        "Expected not-found error, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_empty_source_directory_fails() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let source_dir = dir.path().join("empty_source");
    std::fs::create_dir_all(&source_dir).unwrap();

    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            source_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("No items found"),
        "Expected 'No items found' error, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_nonexistent_source_directory_fails() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            "/nonexistent/path/that/does/not/exist",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("does not exist"),
        "Expected 'does not exist' error, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_export_non_empty_dir_without_overwrite_fails() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let output_dir = dir.path().join("nonempty");
    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(output_dir.join("existing_file.txt"), "data").unwrap();

    let assert = fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("not empty") || stderr.contains("--overwrite"),
        "Expected non-empty dir error, got: {stderr}"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_export_overwrite_flag_works() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let output_dir = dir.path().join("overwrite_test");
    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(output_dir.join("old_file.txt"), "old data").unwrap();

    fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            output_dir.to_str().unwrap(),
            "--overwrite",
        ])
        .timeout(Duration::from_secs(300))
        .assert()
        .success();
}

// ── Workspace name resolution ────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_workspace_name_resolution() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");

    // First export to get content (using ID)
    fabio()
        .args([
            "deploy",
            "export",
            "--workspace",
            &cfg.source_workspace,
            "--dir",
            export_dir.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(300))
        .assert()
        .success();

    // Get workspace name
    let assert = fabio()
        .args(["workspace", "show", "--id", &cfg.source_workspace])
        .assert()
        .success();
    let json = parse_json(&assert);
    let ws_name = extract_data(&json)["displayName"]
        .as_str()
        .unwrap()
        .to_owned();

    // Plan using workspace name instead of ID
    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &ws_name,
            "--force-all",
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should succeed and find items
    assert!(data["workspace_id"].is_string());
}
