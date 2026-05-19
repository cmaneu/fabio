//! End-to-end integration tests for `fabio ontology` commands.
//!
//! Tests exercise the compiled binary against a live Microsoft Fabric tenant.
//! Requires valid Azure credentials and `FABIO_TEST_*` environment variables.

mod common;

use common::{TestConfig, extract_count, extract_data, fabio, parse_json, unique_name};
use serial_test::serial;

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_list_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["ontology", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should be an array (possibly empty)
    assert!(data.is_array());
    // count field must be present
    let _ = extract_count(&json);
}

// ---------------------------------------------------------------------------
// Create + Show + Update + Delete lifecycle
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_create_show_update_delete() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_test");

    // Create ontology
    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
            "--description",
            "Test ontology for E2E",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Show ontology
    let assert = fabio()
        .args([
            "ontology",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["id"], ont_id);
    assert_eq!(data["displayName"], name);

    // Update name and description
    let new_name = unique_name("ont_renamed");
    let assert = fabio()
        .args([
            "ontology",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--name",
            &new_name,
            "--description",
            "Updated description",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], new_name);

    // Delete (soft)
    let assert = fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
    assert_eq!(data["id"], ont_id);
}

// ---------------------------------------------------------------------------
// Create with --definition (JSON parts format)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_create_with_definition_json() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_def");

    // Create a definition JSON file with the mandatory definition.json part + TTL payload
    let ttl_content = "@prefix ex: <http://example.org/> .\nex:Thing a ex:Class .";
    let ttl_encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        ttl_content.as_bytes(),
    );
    let def_json_encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        b"{}",
    );

    let def = serde_json::json!({
        "parts": [
            {
                "path": "definition.json",
                "payload": def_json_encoded,
                "payloadType": "InlineBase64"
            },
            {
                "path": "ontology.ttl",
                "payload": ttl_encoded,
                "payloadType": "InlineBase64"
            }
        ]
    });

    let dir = tempfile::tempdir().unwrap();
    let def_path = dir.path().join("definition.json");
    std::fs::write(&def_path, serde_json::to_string(&def).unwrap()).unwrap();

    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
            "--definition",
            def_path.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Cleanup
    fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Create with --rdf (auto-wraps TTL into definition)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_create_with_rdf_ttl() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_rdf");

    let dir = tempfile::tempdir().unwrap();
    let ttl_path = dir.path().join("schema.ttl");
    std::fs::write(
        &ttl_path,
        "@prefix ex: <http://example.org/> .\nex:Thing a ex:Class .",
    )
    .unwrap();

    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
            "--rdf",
            ttl_path.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Cleanup
    fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Create with --rdf OWL format
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_create_with_rdf_owl() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_owl");

    let dir = tempfile::tempdir().unwrap();
    let owl_path = dir.path().join("ontology.owl");
    std::fs::write(
        &owl_path,
        r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:owl="http://www.w3.org/2002/07/owl#">
  <owl:Ontology rdf:about="http://example.org/test"/>
</rdf:RDF>"#,
    )
    .unwrap();

    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
            "--rdf",
            owl_path.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Cleanup
    fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Create with --rdf JSON-LD format
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_create_with_rdf_jsonld() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_jld");

    let dir = tempfile::tempdir().unwrap();
    let jsonld_path = dir.path().join("ontology.jsonld");
    std::fs::write(
        &jsonld_path,
        r#"{"@context": {"@vocab": "http://example.org/"}, "@type": "Ontology"}"#,
    )
    .unwrap();

    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
            "--rdf",
            jsonld_path.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Cleanup
    fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Hard delete
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_hard_delete() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_hard");

    // Create
    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Hard delete
    let assert = fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--hard",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");

    // Verify it's gone (show should fail)
    fabio()
        .args([
            "ontology",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// Get definition and update definition
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_get_and_update_definition() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_defn");

    // Create with initial RDF definition
    let dir = tempfile::tempdir().unwrap();
    let ttl_path = dir.path().join("initial.ttl");
    std::fs::write(
        &ttl_path,
        "@prefix ex: <http://example.org/> .\nex:A a ex:Class .",
    )
    .unwrap();

    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
            "--rdf",
            ttl_path.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Get definition
    let assert = fabio()
        .args([
            "ontology",
            "get-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should contain a definition field or parts
    assert!(data.get("definition").is_some() || data.get("parts").is_some());

    // Update definition with new RDF via --rdf
    let updated_path = dir.path().join("updated.ttl");
    std::fs::write(
        &updated_path,
        "@prefix ex: <http://example.org/> .\nex:B a ex:Class .\nex:C a ex:Class .",
    )
    .unwrap();

    fabio()
        .args([
            "ontology",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--rdf",
            updated_path.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Update definition with JSON format (using --definition)
    let def_json_path = dir.path().join("def.json");
    let ttl_bytes = b"@prefix ex: <http://example.org/> .\nex:D a ex:Class .";
    let ttl_encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        ttl_bytes,
    );
    let def_json_encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        b"{}",
    );
    let def = serde_json::json!({
        "parts": [
            {
                "path": "definition.json",
                "payload": def_json_encoded,
                "payloadType": "InlineBase64"
            },
            {
                "path": "ontology.ttl",
                "payload": ttl_encoded,
                "payloadType": "InlineBase64"
            }
        ]
    });
    std::fs::write(&def_json_path, serde_json::to_string(&def).unwrap()).unwrap();

    fabio()
        .args([
            "ontology",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--definition",
            def_json_path.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Cleanup
    fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Update definition via stdin
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_update_definition_from_stdin() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_stdin");

    // Create
    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Update definition from stdin (using - as path)
    let ttl_bytes = b"@prefix ex: <http://example.org/> .\nex:StdinTest a ex:Class .";
    let ttl_encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        ttl_bytes,
    );
    let def_json_encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        b"{}",
    );
    let def_json = serde_json::json!({
        "parts": [
            {
                "path": "definition.json",
                "payload": def_json_encoded,
                "payloadType": "InlineBase64"
            },
            {
                "path": "ontology.ttl",
                "payload": ttl_encoded,
                "payloadType": "InlineBase64"
            }
        ]
    });
    let stdin_content = serde_json::to_string(&def_json).unwrap();

    fabio()
        .args([
            "ontology",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--definition",
            "-",
        ])
        .write_stdin(stdin_content)
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Cleanup
    fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Update requires at least one field
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_update_requires_field() {
    let cfg = TestConfig::from_env();

    // Update without --name or --description should fail
    fabio()
        .args([
            "ontology",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// Update-definition requires --definition or --rdf
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_update_definition_requires_source() {
    let cfg = TestConfig::from_env();

    // update-definition without --definition or --rdf should fail
    fabio()
        .args([
            "ontology",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// --definition and --rdf are mutually exclusive (create)
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn ontology_create_definition_and_rdf_conflict() {
    // This doesn't need a live tenant - clap should reject it
    fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            "fake-ws",
            "--name",
            "test",
            "--definition",
            "def.json",
            "--rdf",
            "schema.ttl",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

// ---------------------------------------------------------------------------
// --definition and --rdf are mutually exclusive (update-definition)
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn ontology_update_definition_and_rdf_conflict() {
    // This doesn't need a live tenant - clap should reject it
    fabio()
        .args([
            "ontology",
            "update-definition",
            "--workspace",
            "fake-ws",
            "--id",
            "fake-id",
            "--definition",
            "def.json",
            "--rdf",
            "schema.ttl",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

// ---------------------------------------------------------------------------
// Show non-existent ontology returns error
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_show_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "ontology",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// List with --output table format
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_list_table_format() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "ontology",
            "list",
            "--workspace",
            &cfg.source_workspace,
            "--output",
            "table",
        ])
        .assert()
        .success();

    // Table output should contain header columns
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Table header should appear (or be empty for no items)
    assert!(stdout.contains("NAME") || stdout.is_empty() || stdout.contains("No items"));
}

// ---------------------------------------------------------------------------
// Create with --rdf unsupported extension
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_create_with_rdf_unsupported_extension() {
    let cfg = TestConfig::from_env();

    let dir = tempfile::tempdir().unwrap();
    let bad_path = dir.path().join("data.csv");
    std::fs::write(&bad_path, "a,b,c").unwrap();

    fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "should_fail",
            "--rdf",
            bad_path.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// Update only description
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_update_description_only() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_desc");

    // Create
    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Update description only
    let assert = fabio()
        .args([
            "ontology",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--description",
            "A new description",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Name should remain unchanged
    assert_eq!(data["displayName"], name);

    // Cleanup
    fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Update definition with --rdf (no --update-metadata to avoid needing .platform)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_update_definition_with_rdf() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_upd");

    // Create
    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Update definition with --rdf
    let dir = tempfile::tempdir().unwrap();
    let ttl_path = dir.path().join("meta.ttl");
    std::fs::write(
        &ttl_path,
        "@prefix ex: <http://example.org/> .\nex:Meta a ex:Class .",
    )
    .unwrap();

    fabio()
        .args([
            "ontology",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--rdf",
            ttl_path.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Cleanup
    fabio()
        .args([
            "ontology",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
        ])
        .assert()
        .success();
}
