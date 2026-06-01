//! End-to-end integration tests for `fabio deploy` commands.
//!
//! Tests the plan/apply/export workflow against a live Fabric tenant.
//! Requires `FABIO_TEST_SOURCE_WORKSPACE` and `FABIO_TEST_CAPACITY_ID` env vars.

mod common;

use base64::Engine;
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

// ── Init-Params (local-only, no live tenant required) ────────────────────────

/// Helper: create a synthetic .platform item directory for init-params testing.
fn create_platform_item(
    base_dir: &std::path::Path,
    folder_name: &str,
    item_type: &str,
    display_name: &str,
    file_name: &str,
    content: &str,
) {
    let item_dir = base_dir.join(folder_name);
    std::fs::create_dir_all(&item_dir).unwrap();

    let platform = serde_json::json!({
        "$schema": "https://developer.microsoft.com/json-schemas/fabric/gitIntegration/platformProperties/2.0.0/schema.json",
        "metadata": {
            "type": item_type,
            "displayName": display_name
        },
        "config": {}
    });
    std::fs::write(
        item_dir.join(".platform"),
        serde_json::to_string_pretty(&platform).unwrap(),
    )
    .unwrap();

    std::fs::write(item_dir.join(file_name), content).unwrap();
}

#[test]
fn deploy_init_params_scan_mode() {
    let dir = tempfile::TempDir::new().unwrap();
    let source = dir.path().join("source");
    std::fs::create_dir_all(&source).unwrap();

    // Create items with GUIDs embedded in their definitions
    create_platform_item(
        &source,
        "MyPipeline.DataPipeline",
        "DataPipeline",
        "MyPipeline",
        "pipeline-content.json",
        r#"{"activities": [{"connectionId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890", "workspaceId": "12345678-aaaa-bbbb-cccc-ddddeeeeaaaa"}]}"#,
    );

    let out_file = dir.path().join("params.json");

    let assert = fabio()
        .args([
            "deploy",
            "init-params",
            "--source",
            source.to_str().unwrap(),
            "--out",
            out_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "generated");
    assert_eq!(data["mode"], "scan");
    assert_eq!(data["source_items"].as_u64().unwrap(), 1);
    assert!(data["rules_generated"].as_u64().unwrap() >= 2);
    assert!(data["guids_found"].as_u64().unwrap() >= 2);

    // Verify output file was written
    assert!(out_file.exists());
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out_file).unwrap()).unwrap();
    assert!(written["find_replace"].as_array().unwrap().len() >= 2);
}

#[test]
fn deploy_init_params_scan_skips_well_known_guids() {
    let dir = tempfile::TempDir::new().unwrap();
    let source = dir.path().join("source");
    std::fs::create_dir_all(&source).unwrap();

    create_platform_item(
        &source,
        "MyLakehouse.Lakehouse",
        "Lakehouse",
        "MyLakehouse",
        "definition.json",
        r#"{"nullId": "00000000-0000-0000-0000-000000000000", "realId": "abcdef12-3456-7890-abcd-ef1234567890"}"#,
    );

    let assert = fabio()
        .args([
            "deploy",
            "init-params",
            "--source",
            source.to_str().unwrap(),
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Only the real GUID should be found (not the null/well-known one)
    assert_eq!(data["guids_found"].as_u64().unwrap(), 1);
    assert_eq!(data["rules_generated"].as_u64().unwrap(), 1);
}

#[test]
fn deploy_init_params_diff_mode() {
    let dir = tempfile::TempDir::new().unwrap();
    let source = dir.path().join("source");
    let compare = dir.path().join("compare");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&compare).unwrap();

    // Same item in both, with different GUIDs (env-specific)
    create_platform_item(
        &source,
        "MyPipeline.DataPipeline",
        "DataPipeline",
        "MyPipeline",
        "pipeline-content.json",
        r#"{"connectionId": "aaaaaaaa-1111-2222-3333-444444444444"}"#,
    );
    create_platform_item(
        &compare,
        "MyPipeline.DataPipeline",
        "DataPipeline",
        "MyPipeline",
        "pipeline-content.json",
        r#"{"connectionId": "bbbbbbbb-5555-6666-7777-888888888888"}"#,
    );

    let out_file = dir.path().join("params.json");

    let assert = fabio()
        .args([
            "deploy",
            "init-params",
            "--source",
            source.to_str().unwrap(),
            "--compare",
            compare.to_str().unwrap(),
            "--source-env",
            "dev",
            "--compare-env",
            "prod",
            "--out",
            out_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "generated");
    assert_eq!(data["mode"], "diff");
    assert_eq!(data["source_items"].as_u64().unwrap(), 1);
    assert_eq!(data["compare_items"].as_u64().unwrap(), 1);
    assert!(data["rules_generated"].as_u64().unwrap() >= 1);

    // Verify generated rules map the GUIDs correctly
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out_file).unwrap()).unwrap();
    let rules = written["find_replace"].as_array().unwrap();
    assert!(!rules.is_empty());

    let guid_rule = rules
        .iter()
        .find(|r| {
            r["find_value"]
                .as_str()
                .is_some_and(|v| v.contains("aaaaaaaa"))
        })
        .expect("Should find rule for source GUID");
    assert_eq!(
        guid_rule["replace_value"]["dev"].as_str().unwrap(),
        "aaaaaaaa-1111-2222-3333-444444444444"
    );
    assert_eq!(
        guid_rule["replace_value"]["prod"].as_str().unwrap(),
        "bbbbbbbb-5555-6666-7777-888888888888"
    );
}

#[test]
fn deploy_init_params_diff_no_common_items() {
    let dir = tempfile::TempDir::new().unwrap();
    let source = dir.path().join("source");
    let compare = dir.path().join("compare");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&compare).unwrap();

    // Different items in each directory
    create_platform_item(
        &source,
        "ItemA.Notebook",
        "Notebook",
        "ItemA",
        "notebook-content.py",
        r#"print("hello from dev")"#,
    );
    create_platform_item(
        &compare,
        "ItemB.Notebook",
        "Notebook",
        "ItemB",
        "notebook-content.py",
        r#"print("hello from prod")"#,
    );

    let assert = fabio()
        .args([
            "deploy",
            "init-params",
            "--source",
            source.to_str().unwrap(),
            "--compare",
            compare.to_str().unwrap(),
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["mode"], "diff");
    assert_eq!(data["rules_generated"].as_u64().unwrap(), 0);
}

#[test]
fn deploy_init_params_diff_string_differences() {
    let dir = tempfile::TempDir::new().unwrap();
    let source = dir.path().join("source");
    let compare = dir.path().join("compare");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&compare).unwrap();

    // Items with different connection strings
    create_platform_item(
        &source,
        "Config.DataPipeline",
        "DataPipeline",
        "Config",
        "pipeline-content.json",
        r#"{"server": "myserver-dev.database.windows.net", "port": 1433}"#,
    );
    create_platform_item(
        &compare,
        "Config.DataPipeline",
        "DataPipeline",
        "Config",
        "pipeline-content.json",
        r#"{"server": "myserver-prod.database.windows.net", "port": 1433}"#,
    );

    let assert = fabio()
        .args([
            "deploy",
            "init-params",
            "--source",
            source.to_str().unwrap(),
            "--compare",
            compare.to_str().unwrap(),
            "--source-env",
            "dev",
            "--compare-env",
            "prod",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data["rules_generated"].as_u64().unwrap() >= 1);

    // Verify the rule captures the string difference
    let params = &data["parameters"];
    let rules = params["find_replace"].as_array().unwrap();
    let server_rule = rules.iter().find(|r| {
        r["find_value"]
            .as_str()
            .is_some_and(|v| v.contains("myserver-dev"))
    });
    assert!(
        server_rule.is_some(),
        "Should detect server name difference"
    );
}

#[test]
fn deploy_init_params_empty_source_returns_zero_rules() {
    let dir = tempfile::TempDir::new().unwrap();
    let source = dir.path().join("empty_source");
    std::fs::create_dir_all(&source).unwrap();

    let assert = fabio()
        .args([
            "deploy",
            "init-params",
            "--source",
            source.to_str().unwrap(),
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["source_items"].as_u64().unwrap(), 0);
    assert_eq!(data["rules_generated"].as_u64().unwrap(), 0);
}

#[test]
fn deploy_init_params_nonexistent_source_fails() {
    let dir = tempfile::TempDir::new().unwrap();
    let source = dir.path().join("does_not_exist");

    fabio()
        .args([
            "deploy",
            "init-params",
            "--source",
            source.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

// ── Plan with Parameters (requires live tenant) ──────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_with_parameters_requires_env() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");

    // Export first
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

    // Create a minimal parameters file
    let params_file = dir.path().join("params.json");
    std::fs::write(
        &params_file,
        r#"{"find_replace": [{"find_value": "placeholder", "replace_value": {"_ALL_": "replaced"}}]}"#,
    )
    .unwrap();

    // Plan with --parameters but no --env should fail
    fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
            "--parameters",
            params_file.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_with_parameters_and_env() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");

    // Export first
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

    // Create a parameters file with a no-op rule (won't match anything)
    let params_file = dir.path().join("params.json");
    std::fs::write(
        &params_file,
        r#"{"find_replace": [{"find_value": "nonexistent_value_xyz_123", "replace_value": {"prod": "replaced_value"}}]}"#,
    )
    .unwrap();

    // Plan with --parameters and --env should succeed
    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
            "--parameters",
            params_file.to_str().unwrap(),
            "--env",
            "prod",
            "--force-all",
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should succeed (plan completes with parameter substitution applied)
    assert!(data["workspace_id"].is_string());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_with_key_value_replace_parameters() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");

    // Export first
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

    // Create a parameters file with key_value_replace rules
    let params_file = dir.path().join("params.json");
    std::fs::write(
        &params_file,
        r#"{
            "find_replace": [],
            "key_value_replace": [
                {
                    "find_key": "$.nonexistent_path_xyz",
                    "replace_value": {"prod": "replaced_value"}
                }
            ]
        }"#,
    )
    .unwrap();

    // Plan with key_value_replace should succeed
    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
            "--parameters",
            params_file.to_str().unwrap(),
            "--env",
            "prod",
            "--force-all",
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data["workspace_id"].is_string());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_with_spark_pool_parameters() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");

    // Export first
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

    // Create parameters with spark_pool rules
    let params_file = dir.path().join("params.json");
    std::fs::write(
        &params_file,
        r#"{
            "find_replace": [],
            "spark_pool": [
                {
                    "instance_pool_id": "00000000-0000-0000-0000-000000000099",
                    "replace_value": {
                        "prod": {
                            "type": "Workspace",
                            "name": "prod-pool"
                        }
                    }
                }
            ]
        }"#,
    )
    .unwrap();

    // Plan with spark_pool should succeed (pool won't match anything)
    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
            "--parameters",
            params_file.to_str().unwrap(),
            "--env",
            "prod",
            "--force-all",
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data["workspace_id"].is_string());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_with_semantic_model_binding_parameters() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let export_dir = dir.path().join("exported");

    // Export first
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

    // Create parameters with semantic_model_binding
    let params_file = dir.path().join("params.json");
    std::fs::write(
        &params_file,
        r#"{
            "find_replace": [],
            "semantic_model_binding": {
                "default": {
                    "connection_id": {
                        "prod": "99999999-aaaa-bbbb-cccc-ddddeeee1111"
                    }
                }
            }
        }"#,
    )
    .unwrap();

    // Plan with semantic_model_binding should succeed
    let assert = fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            export_dir.to_str().unwrap(),
            "--workspace",
            &cfg.source_workspace,
            "--parameters",
            params_file.to_str().unwrap(),
            "--env",
            "prod",
            "--force-all",
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data["workspace_id"].is_string());
}

// ── Phase 4: Plan File Roundtrip ─────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_file_roundtrip_apply() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let source_dir = dir.path().join("source");
    let plan_file = dir.path().join("plan.json");
    let name = unique_name("deploy_rt");

    // Create a minimal notebook source
    let nb_dir = source_dir.join(format!("{name}.Notebook"));
    std::fs::create_dir_all(&nb_dir).unwrap();

    let platform = serde_json::json!({
        "metadata": {
            "type": "Notebook",
            "displayName": name
        },
        "config": {
            "version": "2.0",
            "logicalId": "rt-lid-001",
            "definitionFormat": "ipynb"
        }
    });
    std::fs::write(
        nb_dir.join(".platform"),
        serde_json::to_string_pretty(&platform).unwrap(),
    )
    .unwrap();

    let ipynb = serde_json::json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": { "language_info": { "name": "python" } },
        "cells": [{
            "cell_type": "code",
            "source": ["# roundtrip test\n"],
            "metadata": {},
            "outputs": []
        }]
    });
    std::fs::write(
        nb_dir.join("notebook-content.py"),
        serde_json::to_string(&ipynb).unwrap(),
    )
    .unwrap();

    // Step 1: Plan --out (save plan to file)
    fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            source_dir.to_str().unwrap(),
            "--workspace",
            &cfg.dest_workspace,
            "--out",
            plan_file.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    assert!(plan_file.exists(), "Plan file should exist");
    let plan_content = std::fs::read_to_string(&plan_file).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&plan_content).unwrap();
    assert_eq!(plan["version"], 1);
    assert!(plan["workspace_fingerprint"].is_string());

    // Step 2: Apply from plan file
    let assert = fabio()
        .args(["deploy", "apply", "--plan", plan_file.to_str().unwrap()])
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
    assert!(created.is_some(), "Expected deployed notebook '{name}'");

    // Cleanup
    let nb_id = created.unwrap()["id"].as_str().unwrap();
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

// ── Phase 4: Plan Staleness Detection ────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_staleness_detection() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let source_dir = dir.path().join("source");
    let plan_file = dir.path().join("plan.json");
    let name = unique_name("deploy_stale");
    let stale_name = unique_name("stale_item");

    // Create a minimal notebook source
    let nb_dir = source_dir.join(format!("{name}.Notebook"));
    std::fs::create_dir_all(&nb_dir).unwrap();

    let platform = serde_json::json!({
        "metadata": {
            "type": "Notebook",
            "displayName": name
        },
        "config": {
            "version": "2.0",
            "logicalId": "stale-lid-001",
            "definitionFormat": "ipynb"
        }
    });
    std::fs::write(
        nb_dir.join(".platform"),
        serde_json::to_string_pretty(&platform).unwrap(),
    )
    .unwrap();

    let ipynb = serde_json::json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": { "language_info": { "name": "python" } },
        "cells": [{
            "cell_type": "code",
            "source": ["# staleness test\n"],
            "metadata": {},
            "outputs": []
        }]
    });
    std::fs::write(
        nb_dir.join("notebook-content.py"),
        serde_json::to_string(&ipynb).unwrap(),
    )
    .unwrap();

    // Step 1: Save plan (captures workspace fingerprint)
    fabio()
        .args([
            "deploy",
            "plan",
            "--source",
            source_dir.to_str().unwrap(),
            "--workspace",
            &cfg.dest_workspace,
            "--out",
            plan_file.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();

    // Step 2: Modify the workspace state (create a new item to change fingerprint)
    let assert = fabio()
        .args([
            "item",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &stale_name,
            "--type",
            "Notebook",
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();
    let json = parse_json(&assert);
    let stale_item_id = extract_data(&json)["id"].as_str().unwrap().to_owned();

    // Step 3: Apply from plan file WITHOUT --force → should FAIL (stale)
    let assert = fabio()
        .args(["deploy", "apply", "--plan", plan_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("Workspace state has changed")
            || stderr.contains("workspace_fingerprint")
            || stderr.contains("fingerprint"),
        "Expected staleness error, got: {stderr}"
    );

    // Step 4: Apply with --force → should SUCCEED despite stale fingerprint
    let assert = fabio()
        .args([
            "deploy",
            "apply",
            "--plan",
            plan_file.to_str().unwrap(),
            "--force",
        ])
        .timeout(Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "succeeded");

    // Cleanup: delete both items
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
    if let Some(deployed) = items.iter().find(|i| i["displayName"] == name) {
        let id = deployed["id"].as_str().unwrap();
        fabio()
            .args([
                "item",
                "delete",
                "--workspace",
                &cfg.dest_workspace,
                "--id",
                id,
            ])
            .timeout(Duration::from_secs(60))
            .assert()
            .success();
    }
    fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &stale_item_id,
        ])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();
}

// ── Phase 4: Logical ID Resolution (Live) ────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_apply_logical_id_resolution() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let source_dir = dir.path().join("source");
    let lh_name = unique_name("deploy_lh");
    let nb_name = unique_name("deploy_nb_ref");

    // Use a stable logical ID for the lakehouse that we'll embed in the notebook
    let lh_logical_id = "e2e-logical-id-lakehouse-001";

    // Create a lakehouse source item
    let lh_dir = source_dir.join(format!("{lh_name}.Lakehouse"));
    std::fs::create_dir_all(&lh_dir).unwrap();
    let lh_platform = serde_json::json!({
        "metadata": {
            "type": "Lakehouse",
            "displayName": lh_name
        },
        "config": {
            "version": "2.0",
            "logicalId": lh_logical_id
        }
    });
    std::fs::write(
        lh_dir.join(".platform"),
        serde_json::to_string_pretty(&lh_platform).unwrap(),
    )
    .unwrap();
    // Lakehouse has no definition content (empty definition creates shell)

    // Create a notebook source item whose definition references the lakehouse's logical ID
    let nb_dir = source_dir.join(format!("{nb_name}.Notebook"));
    std::fs::create_dir_all(&nb_dir).unwrap();

    let nb_platform = serde_json::json!({
        "metadata": {
            "type": "Notebook",
            "displayName": nb_name
        },
        "config": {
            "version": "2.0",
            "logicalId": "e2e-logical-id-notebook-001",
            "definitionFormat": "ipynb"
        }
    });
    std::fs::write(
        nb_dir.join(".platform"),
        serde_json::to_string_pretty(&nb_platform).unwrap(),
    )
    .unwrap();

    // The notebook content references the lakehouse logical ID
    // (This simulates a notebook that uses the lakehouse's ID in its metadata)
    let ipynb = serde_json::json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": {
            "language_info": { "name": "python" },
            "trident": {
                "lakehouse": {
                    "default_lakehouse": lh_logical_id,
                    "default_lakehouse_name": lh_name,
                    "known_lakehouses": [{
                        "id": lh_logical_id
                    }]
                }
            }
        },
        "cells": [{
            "cell_type": "code",
            "source": [&format!("# Uses lakehouse: {lh_logical_id}\n")],
            "metadata": {},
            "outputs": []
        }]
    });
    std::fs::write(
        nb_dir.join("notebook-content.py"),
        serde_json::to_string(&ipynb).unwrap(),
    )
    .unwrap();

    // Plan should show 2 creates
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
    assert_eq!(
        data["summary"]["create"].as_u64().unwrap(),
        2,
        "Expected 2 creates (lakehouse + notebook)"
    );

    // Apply (real deployment) — lakehouse deploys first (lower priority number),
    // then notebook gets the resolved lakehouse ID in its definition
    let assert = fabio()
        .args([
            "deploy",
            "apply",
            "--source",
            source_dir.to_str().unwrap(),
            "--workspace",
            &cfg.dest_workspace,
        ])
        .timeout(Duration::from_secs(300))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "succeeded");
    assert_eq!(
        data["succeeded"].as_u64().unwrap(),
        2,
        "Both items should deploy successfully"
    );
    assert_eq!(data["failed"].as_u64().unwrap(), 0);

    // Verify both items exist
    let assert = fabio()
        .args(["item", "list", "--workspace", &cfg.dest_workspace])
        .assert()
        .success();
    let json = parse_json(&assert);
    let items = extract_data(&json).as_array().unwrap().clone();

    let lh_item = items.iter().find(|i| i["displayName"] == lh_name);
    assert!(lh_item.is_some(), "Lakehouse '{lh_name}' should exist");
    let lh_id = lh_item.unwrap()["id"].as_str().unwrap();

    let nb_item = items.iter().find(|i| i["displayName"] == nb_name);
    assert!(nb_item.is_some(), "Notebook '{nb_name}' should exist");
    let nb_id = nb_item.unwrap()["id"].as_str().unwrap();

    // Verify the notebook's definition has the REAL lakehouse ID (not the logical ID)
    let assert = fabio()
        .args([
            "item",
            "get-definition",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            nb_id,
        ])
        .timeout(Duration::from_secs(120))
        .assert()
        .success();
    let json = parse_json(&assert);
    let def_data = extract_data(&json);

    // Find the notebook content part and check it contains the real lakehouse ID
    let parts = def_data["definition"]["parts"].as_array().unwrap();
    let content_part = parts
        .iter()
        .find(|p| {
            p["path"]
                .as_str()
                .is_some_and(|path| path.contains("notebook"))
        })
        .expect("Should find notebook content part");

    let payload_b64 = content_part["payload"].as_str().unwrap();
    let payload_bytes = base64::engine::general_purpose::STANDARD
        .decode(payload_b64)
        .unwrap();
    let payload_str = String::from_utf8(payload_bytes).unwrap();

    // The logical ID should be resolved to the actual deployed lakehouse ID
    assert!(
        payload_str.contains(lh_id),
        "Notebook definition should contain the real lakehouse ID '{lh_id}', but got: {}...",
        &payload_str[..payload_str.len().min(200)]
    );
    assert!(
        !payload_str.contains(lh_logical_id),
        "Notebook definition should NOT contain the logical ID '{lh_logical_id}'"
    );

    // Cleanup: delete both items (notebook first to avoid dependency issues)
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
    fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            lh_id,
        ])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();
}

// ── Phase 4: Plan workspace_fingerprint field validation ─────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_file_contains_fingerprint() {
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

    // Plan with --out
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

    // Parse plan file and verify structure
    let content = std::fs::read_to_string(&plan_file).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(plan["version"], 1, "Plan version should be 1");
    assert!(
        plan["workspace_id"].is_string(),
        "Plan should have workspace_id"
    );
    assert!(
        plan["workspace_fingerprint"].is_string(),
        "Plan should have workspace_fingerprint"
    );

    let fingerprint = plan["workspace_fingerprint"].as_str().unwrap();
    assert!(
        fingerprint.starts_with("sha256:"),
        "Fingerprint should start with 'sha256:', got: {fingerprint}"
    );
    assert_eq!(
        fingerprint.len(),
        7 + 64,
        "Fingerprint should be sha256: + 64 hex chars"
    );

    assert!(
        plan["changeset"]["changes"].is_array(),
        "Plan should have changeset.changes array"
    );
    assert!(
        plan["source_path"].is_string(),
        "Plan should have source_path"
    );
}

// ── creationPayload ──────────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_apply_creation_payload_lakehouse_with_schemas() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let source_dir = dir.path().join("source");
    std::fs::create_dir_all(&source_dir).unwrap();

    let name = unique_name("DeploySchemaLH");

    // Create a Lakehouse source with creationPayload.json (enableSchemas)
    let lh_dir = source_dir.join(format!("{name}.Lakehouse"));
    std::fs::create_dir_all(&lh_dir).unwrap();

    let platform = serde_json::json!({
        "metadata": {
            "type": "Lakehouse",
            "displayName": name
        },
        "config": {
            "version": "2.0",
            "logicalId": "e2e-creation-payload-001"
        }
    });
    std::fs::write(
        lh_dir.join(".platform"),
        serde_json::to_string_pretty(&platform).unwrap(),
    )
    .unwrap();

    // This is the key part: creationPayload.json triggers enableSchemas
    let creation_payload = serde_json::json!({
        "enableSchemas": true
    });
    std::fs::write(
        lh_dir.join("creationPayload.json"),
        serde_json::to_string_pretty(&creation_payload).unwrap(),
    )
    .unwrap();

    // Plan — should show Create
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

    // Apply
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
            "Lakehouse",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    let items = extract_data(&json).as_array().unwrap().clone();
    let created = items.iter().find(|i| i["displayName"] == name);
    assert!(
        created.is_some(),
        "Expected to find deployed lakehouse '{name}'"
    );

    let lh_id = created.unwrap()["id"].as_str().unwrap();

    // Cleanup
    fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            lh_id,
        ])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();
}

// ── Rename Detection ─────────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_plan_detects_rename() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let source_dir = dir.path().join("source");
    std::fs::create_dir_all(&source_dir).unwrap();

    let original_name = unique_name("DeployRenOrig");
    let renamed_name = unique_name("DeployRenNew");

    // Step 1: Create item via deploy apply with original name
    let nb_dir = source_dir.join(format!("{original_name}.Notebook"));
    std::fs::create_dir_all(&nb_dir).unwrap();

    let logical_id = "e2e-rename-detection-lid-001";
    let platform = serde_json::json!({
        "metadata": {
            "type": "Notebook",
            "displayName": original_name
        },
        "config": {
            "version": "2.0",
            "logicalId": logical_id,
            "definitionFormat": "ipynb"
        }
    });
    std::fs::write(
        nb_dir.join(".platform"),
        serde_json::to_string_pretty(&platform).unwrap(),
    )
    .unwrap();

    let ipynb = serde_json::json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": { "language_info": { "name": "python" } },
        "cells": [{
            "cell_type": "code",
            "source": ["# Rename test\n"],
            "metadata": {},
            "outputs": []
        }]
    });
    std::fs::write(
        nb_dir.join("notebook-content.py"),
        serde_json::to_string(&ipynb).unwrap(),
    )
    .unwrap();

    // Deploy with original name
    fabio()
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

    // Verify item deployed
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
    let created = items.iter().find(|i| i["displayName"] == original_name);
    assert!(
        created.is_some(),
        "Expected deployed notebook '{original_name}'"
    );
    let nb_id = created.unwrap()["id"].as_str().unwrap().to_owned();

    // Step 2: Rename the source (simulate renaming in source code)
    std::fs::remove_dir_all(&nb_dir).unwrap();
    let new_nb_dir = source_dir.join(format!("{renamed_name}.Notebook"));
    std::fs::create_dir_all(&new_nb_dir).unwrap();

    let renamed_platform = serde_json::json!({
        "metadata": {
            "type": "Notebook",
            "displayName": renamed_name
        },
        "config": {
            "version": "2.0",
            "logicalId": logical_id,  // Same logical ID → rename detection
            "definitionFormat": "ipynb"
        }
    });
    std::fs::write(
        new_nb_dir.join(".platform"),
        serde_json::to_string_pretty(&renamed_platform).unwrap(),
    )
    .unwrap();
    std::fs::write(
        new_nb_dir.join("notebook-content.py"),
        serde_json::to_string(&ipynb).unwrap(),
    )
    .unwrap();

    // Step 3: Plan — should detect rename
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
    let changes = data["changeset"]["changes"].as_array().unwrap();

    // Should have exactly one rename action
    let renames: Vec<_> = changes.iter().filter(|c| c["action"] == "rename").collect();
    assert_eq!(
        renames.len(),
        1,
        "Expected exactly 1 rename, got: {changes:?}"
    );
    assert_eq!(renames[0]["name"], renamed_name);
    assert_eq!(renames[0]["previous_name"], original_name);

    // Step 4: Apply the rename
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

    // Verify renamed item exists
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
    let renamed = items.iter().find(|i| i["displayName"] == renamed_name);
    assert!(
        renamed.is_some(),
        "Expected renamed notebook '{renamed_name}'"
    );
    let old = items.iter().find(|i| i["displayName"] == original_name);
    assert!(old.is_none(), "Original name should no longer exist");

    // Cleanup
    fabio()
        .args([
            "item",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &nb_id,
        ])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();
}

// ── --no-post-hooks flag ─────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn deploy_apply_no_post_hooks_flag_accepted() {
    let cfg = TestConfig::from_env();
    let dir = tempfile::TempDir::new().unwrap();
    let source_dir = dir.path().join("source");
    std::fs::create_dir_all(&source_dir).unwrap();

    let name = unique_name("DeployNoHooks");

    // Create a simple notebook source
    let nb_dir = source_dir.join(format!("{name}.Notebook"));
    std::fs::create_dir_all(&nb_dir).unwrap();

    let platform = serde_json::json!({
        "metadata": {
            "type": "Notebook",
            "displayName": name
        },
        "config": {
            "version": "2.0",
            "logicalId": "e2e-no-hooks-lid-001",
            "definitionFormat": "ipynb"
        }
    });
    std::fs::write(
        nb_dir.join(".platform"),
        serde_json::to_string_pretty(&platform).unwrap(),
    )
    .unwrap();

    let ipynb = serde_json::json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": { "language_info": { "name": "python" } },
        "cells": [{
            "cell_type": "code",
            "source": ["# no-hooks test\n"],
            "metadata": {},
            "outputs": []
        }]
    });
    std::fs::write(
        nb_dir.join("notebook-content.py"),
        serde_json::to_string(&ipynb).unwrap(),
    )
    .unwrap();

    // Deploy with --no-post-hooks
    let assert = fabio()
        .args([
            "deploy",
            "apply",
            "--source",
            source_dir.to_str().unwrap(),
            "--workspace",
            &cfg.dest_workspace,
            "--no-post-hooks",
        ])
        .timeout(Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "succeeded");
    assert_eq!(data["succeeded"].as_u64().unwrap(), 1);
    // post_hooks should be empty array (no hooks executed)
    assert_eq!(
        data["post_hooks"].as_array().map_or(0, Vec::len),
        0,
        "Expected no post-hooks with --no-post-hooks flag"
    );

    // Verify item exists and cleanup
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
    assert!(created.is_some(), "Expected deployed notebook '{name}'");

    let nb_id = created.unwrap()["id"].as_str().unwrap();

    // Cleanup
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
