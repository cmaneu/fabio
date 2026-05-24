//! End-to-end integration tests for `fabio semantic-model` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json, unique_name};
use serial_test::serial;
use std::io::Write;
use tempfile::NamedTempFile;

// ─── List / Show / Update / Delete (basic) ───────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn semantic_model_list_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "semantic-model",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn semantic_model_update_requires_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "semantic-model",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err_json["error"]["code"], "INVALID_INPUT");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn semantic_model_show_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "semantic-model",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn semantic_model_delete_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "semantic-model",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();
}

// ─── Full Lifecycle: Create (model.bim) → Show → Get-Definition → Delete ────

/// Minimal model.bim JSON for an Import-mode semantic model.
fn minimal_model_bim() -> String {
    serde_json::json!({
        "compatibilityLevel": 1604,
        "model": {
            "culture": "en-US",
            "defaultPowerBIDataSourceVersion": "powerBI_V3",
            "tables": [
                {
                    "name": "TestTable",
                    "columns": [
                        {
                            "name": "ID",
                            "dataType": "int64",
                            "sourceColumn": "ID"
                        },
                        {
                            "name": "Name",
                            "dataType": "string",
                            "sourceColumn": "Name"
                        }
                    ],
                    "partitions": [
                        {
                            "name": "TestTable",
                            "source": {
                                "type": "m",
                                "expression": "let Source = #table({\"ID\", \"Name\"}, {{1, \"Test\"}}) in Source"
                            }
                        }
                    ]
                }
            ]
        }
    })
    .to_string()
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn semantic_model_create_show_get_definition_delete() {
    let cfg = TestConfig::from_env();
    let name = unique_name("sm_bim");

    // Write model.bim to a temp file
    let mut tmp = NamedTempFile::with_suffix(".bim").unwrap();
    tmp.write_all(minimal_model_bim().as_bytes()).unwrap();
    let file_path = tmp.path().to_str().unwrap().to_string();

    // ── Create ───────────────────────────────────────────────────────────
    let assert = fabio()
        .args([
            "semantic-model",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--description",
            "E2E test semantic model (model.bim)",
            "--file",
            &file_path,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let sm_id = data["id"].as_str().unwrap().to_string();

    // ── Show ─────────────────────────────────────────────────────────────
    let assert = fabio()
        .args([
            "semantic-model",
            "show",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &sm_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["id"], sm_id);
    assert_eq!(data["displayName"], name);

    // ── Get Definition ───────────────────────────────────────────────────
    let assert = fabio()
        .args([
            "semantic-model",
            "get-definition",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &sm_id,
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Definition should have parts
    let parts = data["definition"]["parts"].as_array();
    assert!(
        parts.is_some(),
        "expected 'definition.parts' array in response"
    );
    let parts = parts.unwrap();
    assert!(!parts.is_empty(), "expected at least one definition part");

    // Should contain model.bim or definition.pbism
    let paths: Vec<&str> = parts.iter().filter_map(|p| p["path"].as_str()).collect();
    assert!(
        paths
            .iter()
            .any(|p| p.contains("model.bim") || p.contains(".pbism")),
        "expected model.bim or definition.pbism in parts, got: {paths:?}"
    );

    // ── Delete ───────────────────────────────────────────────────────────
    let assert = fabio()
        .args([
            "semantic-model",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &sm_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
}

// ─── Create with TMDL Format ─────────────────────────────────────────────────

/// Minimal TMDL model definition (Import mode, single table).
fn minimal_model_tmdl() -> String {
    r#"model Model
	culture: en-US
	defaultPowerBIDataSourceVersion: powerBI_V3

	table TestTable
		lineageTag: 00000000-0000-0000-0000-000000000002

		column ID
			dataType: int64
			sourceColumn: ID
			lineageTag: 00000000-0000-0000-0000-000000000003

		column Name
			dataType: string
			sourceColumn: Name
			lineageTag: 00000000-0000-0000-0000-000000000004

		partition TestTable = m
			expression = let Source = #table({"ID", "Name"}, {{1, "Test"}}) in Source
"#
    .to_string()
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn semantic_model_create_tmdl_and_delete() {
    let cfg = TestConfig::from_env();
    let name = unique_name("sm_tmdl");

    // Write model.tmdl to a temp file
    let mut tmp = NamedTempFile::with_suffix(".tmdl").unwrap();
    tmp.write_all(minimal_model_tmdl().as_bytes()).unwrap();
    let file_path = tmp.path().to_str().unwrap().to_string();

    // ── Create (TMDL format auto-detected from extension) ────────────────
    let assert = fabio()
        .args([
            "semantic-model",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
            "--description",
            "E2E test semantic model (TMDL)",
            "--file",
            &file_path,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let sm_id = data["id"].as_str().unwrap().to_string();

    // ── Verify it shows up in list ───────────────────────────────────────
    let assert = fabio()
        .args(["semantic-model", "list", "--workspace", &cfg.dest_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let models = data.as_array().unwrap();
    assert!(
        models.iter().any(|m| m["id"] == sm_id),
        "created model should appear in list"
    );

    // ── Delete ───────────────────────────────────────────────────────────
    let assert = fabio()
        .args([
            "semantic-model",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &sm_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
}

// ─── Update + Update-Definition ──────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn semantic_model_update_name_and_description() {
    let cfg = TestConfig::from_env();
    let original_name = unique_name("sm_upd_o");
    let updated_name = unique_name("sm_upd_n");

    // Create
    let mut tmp = NamedTempFile::with_suffix(".bim").unwrap();
    tmp.write_all(minimal_model_bim().as_bytes()).unwrap();
    let file_path = tmp.path().to_str().unwrap().to_string();

    let assert = fabio()
        .args([
            "semantic-model",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &original_name,
            "--file",
            &file_path,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let sm_id = data["id"].as_str().unwrap().to_string();

    // Update name and description
    let assert = fabio()
        .args([
            "semantic-model",
            "update",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &sm_id,
            "--name",
            &updated_name,
            "--description",
            "Updated via E2E test",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], updated_name);
    assert_eq!(data["description"], "Updated via E2E test");

    // Cleanup
    fabio()
        .args([
            "semantic-model",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &sm_id,
        ])
        .assert()
        .success();
}

// ─── Dry Run ─────────────────────────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn semantic_model_create_dry_run() {
    let cfg = TestConfig::from_env();

    // Write a temp file
    let mut tmp = NamedTempFile::with_suffix(".bim").unwrap();
    tmp.write_all(minimal_model_bim().as_bytes()).unwrap();
    let file_path = tmp.path().to_str().unwrap().to_string();

    let assert = fabio()
        .args([
            "--dry-run",
            "semantic-model",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            "dry_run_sm",
            "--file",
            &file_path,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "semantic-model create");
}
