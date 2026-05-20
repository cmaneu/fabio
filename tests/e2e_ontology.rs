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
    let def_json_encoded =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"{}");

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
// Create with --file (auto-wraps TTL into definition)
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
        r#"@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix sales: <http://example.org/sales#> .

sales:SalesOntology a owl:Ontology ;
    rdfs:label "Sales Domain Ontology" ;
    rdfs:comment "Models customers, products, and orders for a retail domain." .

sales:Customer a owl:Class ;
    rdfs:label "Customer" ;
    rdfs:comment "A person or organization that purchases products." .

sales:Product a owl:Class ;
    rdfs:label "Product" ;
    rdfs:comment "An item available for sale." .

sales:Order a owl:Class ;
    rdfs:label "Order" ;
    rdfs:comment "A purchase transaction linking a customer to products." .

sales:hasName a owl:DatatypeProperty ;
    rdfs:domain sales:Customer ;
    rdfs:range xsd:string ;
    rdfs:label "name" .

sales:hasEmail a owl:DatatypeProperty ;
    rdfs:domain sales:Customer ;
    rdfs:range xsd:string ;
    rdfs:label "email" .

sales:hasPrice a owl:DatatypeProperty ;
    rdfs:domain sales:Product ;
    rdfs:range xsd:decimal ;
    rdfs:label "price" .

sales:placedBy a owl:ObjectProperty ;
    rdfs:domain sales:Order ;
    rdfs:range sales:Customer ;
    rdfs:label "placed by" .

sales:containsProduct a owl:ObjectProperty ;
    rdfs:domain sales:Order ;
    rdfs:range sales:Product ;
    rdfs:label "contains product" .

sales:orderDate a owl:DatatypeProperty ;
    rdfs:domain sales:Order ;
    rdfs:range xsd:date ;
    rdfs:label "order date" .
"#,
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
            "--file",
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
// Create with --file OWL format
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
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
         xmlns:owl="http://www.w3.org/2002/07/owl#"
         xmlns:xsd="http://www.w3.org/2001/XMLSchema#"
         xmlns:inv="http://example.org/inventory#">

  <owl:Ontology rdf:about="http://example.org/inventory">
    <rdfs:label>Inventory Management Ontology</rdfs:label>
    <rdfs:comment>Models warehouses, stock items, and supply chain relationships.</rdfs:comment>
  </owl:Ontology>

  <owl:Class rdf:about="http://example.org/inventory#Warehouse">
    <rdfs:label>Warehouse</rdfs:label>
    <rdfs:comment>A physical location where inventory is stored.</rdfs:comment>
  </owl:Class>

  <owl:Class rdf:about="http://example.org/inventory#StockItem">
    <rdfs:label>Stock Item</rdfs:label>
    <rdfs:comment>A product unit held in inventory.</rdfs:comment>
  </owl:Class>

  <owl:Class rdf:about="http://example.org/inventory#Supplier">
    <rdfs:label>Supplier</rdfs:label>
    <rdfs:comment>An entity that provides stock items.</rdfs:comment>
  </owl:Class>

  <owl:DatatypeProperty rdf:about="http://example.org/inventory#sku">
    <rdfs:domain rdf:resource="http://example.org/inventory#StockItem"/>
    <rdfs:range rdf:resource="http://www.w3.org/2001/XMLSchema#string"/>
    <rdfs:label>SKU</rdfs:label>
  </owl:DatatypeProperty>

  <owl:DatatypeProperty rdf:about="http://example.org/inventory#quantity">
    <rdfs:domain rdf:resource="http://example.org/inventory#StockItem"/>
    <rdfs:range rdf:resource="http://www.w3.org/2001/XMLSchema#integer"/>
    <rdfs:label>quantity on hand</rdfs:label>
  </owl:DatatypeProperty>

  <owl:ObjectProperty rdf:about="http://example.org/inventory#storedIn">
    <rdfs:domain rdf:resource="http://example.org/inventory#StockItem"/>
    <rdfs:range rdf:resource="http://example.org/inventory#Warehouse"/>
    <rdfs:label>stored in</rdfs:label>
  </owl:ObjectProperty>

  <owl:ObjectProperty rdf:about="http://example.org/inventory#suppliedBy">
    <rdfs:domain rdf:resource="http://example.org/inventory#StockItem"/>
    <rdfs:range rdf:resource="http://example.org/inventory#Supplier"/>
    <rdfs:label>supplied by</rdfs:label>
  </owl:ObjectProperty>

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
            "--file",
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
// Create with --file JSON-LD format
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
        r#"{
  "@context": {
    "rdf": "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
    "owl": "http://www.w3.org/2002/07/owl#",
    "xsd": "http://www.w3.org/2001/XMLSchema#",
    "hr": "http://example.org/hr#"
  },
  "@graph": [
    {
      "@id": "hr:HROntology",
      "@type": "owl:Ontology",
      "rdfs:label": "Human Resources Ontology",
      "rdfs:comment": "Models employees, departments, and organizational structure."
    },
    {
      "@id": "hr:Employee",
      "@type": "owl:Class",
      "rdfs:label": "Employee",
      "rdfs:comment": "A person employed by the organization."
    },
    {
      "@id": "hr:Department",
      "@type": "owl:Class",
      "rdfs:label": "Department",
      "rdfs:comment": "An organizational unit within the company."
    },
    {
      "@id": "hr:Role",
      "@type": "owl:Class",
      "rdfs:label": "Role",
      "rdfs:comment": "A job function or position title."
    },
    {
      "@id": "hr:employeeId",
      "@type": "owl:DatatypeProperty",
      "rdfs:domain": {"@id": "hr:Employee"},
      "rdfs:range": {"@id": "xsd:string"},
      "rdfs:label": "employee ID"
    },
    {
      "@id": "hr:belongsToDepartment",
      "@type": "owl:ObjectProperty",
      "rdfs:domain": {"@id": "hr:Employee"},
      "rdfs:range": {"@id": "hr:Department"},
      "rdfs:label": "belongs to department"
    },
    {
      "@id": "hr:hasRole",
      "@type": "owl:ObjectProperty",
      "rdfs:domain": {"@id": "hr:Employee"},
      "rdfs:range": {"@id": "hr:Role"},
      "rdfs:label": "has role"
    },
    {
      "@id": "hr:reportsTo",
      "@type": "owl:ObjectProperty",
      "rdfs:domain": {"@id": "hr:Employee"},
      "rdfs:range": {"@id": "hr:Employee"},
      "rdfs:label": "reports to"
    }
  ]
}"#,
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
            "--file",
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
            "--file",
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

    // Update definition with new RDF via --file
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
            "--file",
            updated_path.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Update definition with JSON format (using --definition)
    let def_json_path = dir.path().join("def.json");
    let ttl_bytes = b"@prefix ex: <http://example.org/> .\nex:D a ex:Class .";
    let ttl_encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, ttl_bytes);
    let def_json_encoded =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"{}");
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
    let ttl_encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, ttl_bytes);
    let def_json_encoded =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"{}");
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
// Update-definition requires --definition or --file
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_update_definition_requires_source() {
    let cfg = TestConfig::from_env();

    // update-definition without --definition or --file should fail
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
// --definition and --file are mutually exclusive (create)
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
            "--file",
            "schema.ttl",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

// ---------------------------------------------------------------------------
// --definition and --file are mutually exclusive (update-definition)
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
            "--file",
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
// Create with --file unsupported extension
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
            "--file",
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
// Update definition with --file (no --update-metadata to avoid needing .platform)
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

    // Update definition with --file
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
            "--file",
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
