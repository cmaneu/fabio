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

// ---------------------------------------------------------------------------
// Get definition with --decode flag
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_get_definition_decode() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_decode");

    // Create with entity type definition via --definition
    let entity_def = serde_json::json!({
        "id": "5550000000001",
        "namespace": "usertypes",
        "baseEntityTypeId": null,
        "name": "TestEntity",
        "entityIdParts": ["5550000000011"],
        "displayNamePropertyId": "5550000000011",
        "namespaceType": "Custom",
        "visibility": "Visible",
        "properties": [{
            "id": "5550000000011",
            "name": "DisplayName",
            "redefines": null,
            "baseTypeNamespaceType": null,
            "valueType": "String"
        }],
        "timeseriesProperties": []
    });

    let entity_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&entity_def).unwrap().as_bytes(),
    );
    let def_json_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"{}");

    let def = serde_json::json!({
        "parts": [
            {"path": "definition.json", "payload": def_json_b64, "payloadType": "InlineBase64"},
            {"path": "EntityTypes/5550000000001/definition.json", "payload": entity_b64, "payloadType": "InlineBase64"}
        ]
    });

    let dir = tempfile::tempdir().unwrap();
    let def_path = dir.path().join("definition.json");
    std::fs::write(&def_path, serde_json::to_string(&def).unwrap()).unwrap();

    // Create ontology with definition
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
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Get definition with --decode
    let assert = fabio()
        .args([
            "ontology",
            "get-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--decode",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let parts = data["definition"]["parts"].as_array().unwrap();

    // Verify decoded payloads are present
    let mut found_entity = false;
    for part in parts {
        if part["path"].as_str().unwrap_or("").contains("EntityTypes/") {
            let decoded = &part["decodedPayload"];
            assert!(
                decoded.is_object(),
                "decodedPayload should be a JSON object"
            );
            assert_eq!(decoded["name"], "TestEntity");
            found_entity = true;
        }
    }
    assert!(found_entity, "Should find decoded entity type in parts");

    // Cleanup
    fabio()
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
}

// ---------------------------------------------------------------------------
// Create and update with --dir (directory-based definition)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_create_with_dir() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_dir");

    let dir = tempfile::tempdir().unwrap();
    let ont_dir = dir.path().join("ontology");
    std::fs::create_dir_all(ont_dir.join("EntityTypes").join("7770000000001")).unwrap();

    // definition.json
    std::fs::write(ont_dir.join("definition.json"), "{}").unwrap();

    // Entity type
    std::fs::write(
        ont_dir
            .join("EntityTypes")
            .join("7770000000001")
            .join("definition.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "id": "7770000000001",
            "namespace": "usertypes",
            "baseEntityTypeId": null,
            "name": "Machine",
            "entityIdParts": ["7770000000011"],
            "displayNamePropertyId": "7770000000011",
            "namespaceType": "Custom",
            "visibility": "Visible",
            "properties": [{
                "id": "7770000000011",
                "name": "DisplayName",
                "redefines": null,
                "baseTypeNamespaceType": null,
                "valueType": "String"
            }, {
                "id": "7770000000012",
                "name": "SerialNumber",
                "redefines": null,
                "baseTypeNamespaceType": null,
                "valueType": "String"
            }],
            "timeseriesProperties": []
        }))
        .unwrap(),
    )
    .unwrap();

    // Create with --dir
    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
            "--description",
            "Created from directory structure",
            "--dir",
            ont_dir.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let ont_id = data["id"].as_str().unwrap().to_string();

    // Get definition and verify entity type was stored
    let assert = fabio()
        .args([
            "ontology",
            "get-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--decode",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let parts = data["definition"]["parts"].as_array().unwrap();

    let entity_part = parts
        .iter()
        .find(|p| {
            p["path"]
                .as_str()
                .unwrap_or("")
                .contains("EntityTypes/7770000000001/definition.json")
        })
        .expect("EntityType part should exist");
    assert_eq!(entity_part["decodedPayload"]["name"], "Machine");

    // Cleanup
    fabio()
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
}

// ---------------------------------------------------------------------------
// Update definition with --dir (entity types + relationship + data binding)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_update_definition_with_dir() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_updir");

    // Create empty ontology
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

    // Build directory with entity types, data binding, and relationship
    let dir = tempfile::tempdir().unwrap();
    let ont_dir = dir.path();

    // Entity type 1: Equipment
    let et1_dir = ont_dir.join("EntityTypes").join("8880000000001");
    std::fs::create_dir_all(et1_dir.join("DataBindings")).unwrap();
    std::fs::write(
        et1_dir.join("definition.json"),
        serde_json::to_string(&serde_json::json!({
            "id": "8880000000001",
            "namespace": "usertypes",
            "name": "Equipment",
            "entityIdParts": ["8880000000011"],
            "displayNamePropertyId": "8880000000011",
            "namespaceType": "Custom",
            "visibility": "Visible",
            "properties": [{
                "id": "8880000000011",
                "name": "DisplayName",
                "valueType": "String"
            }],
            "timeseriesProperties": []
        }))
        .unwrap(),
    )
    .unwrap();

    // Data binding for Equipment → sales table
    std::fs::write(
        et1_dir
            .join("DataBindings")
            .join("b0000001-0001-0001-0001-000000000001.json"),
        serde_json::to_string(&serde_json::json!({
            "id": "b0000001-0001-0001-0001-000000000001",
            "dataBindingConfiguration": {
                "dataBindingType": "NonTimeSeries",
                "propertyBindings": [{
                    "sourceColumnName": "country",
                    "targetPropertyId": "8880000000011"
                }],
                "sourceTableProperties": {
                    "sourceType": "LakehouseTable",
                    "workspaceId": cfg.source_workspace,
                    "itemId": cfg.source_lakehouse,
                    "sourceTableName": "sales",
                    "sourceSchema": "dbo"
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    // Entity type 2: Sensor
    let et2_dir = ont_dir.join("EntityTypes").join("8880000000002");
    std::fs::create_dir_all(&et2_dir).unwrap();
    std::fs::write(
        et2_dir.join("definition.json"),
        serde_json::to_string(&serde_json::json!({
            "id": "8880000000002",
            "namespace": "usertypes",
            "name": "Sensor",
            "entityIdParts": ["8880000000021"],
            "displayNamePropertyId": "8880000000021",
            "namespaceType": "Custom",
            "visibility": "Visible",
            "properties": [{
                "id": "8880000000021",
                "name": "DisplayName",
                "valueType": "String"
            }],
            "timeseriesProperties": []
        }))
        .unwrap(),
    )
    .unwrap();

    // Relationship: Equipment hasSensor Sensor
    let rel_dir = ont_dir.join("RelationshipTypes").join("9990000000001");
    std::fs::create_dir_all(&rel_dir).unwrap();
    std::fs::write(
        rel_dir.join("definition.json"),
        serde_json::to_string(&serde_json::json!({
            "namespace": "usertypes",
            "id": "9990000000001",
            "name": "hasSensor",
            "namespaceType": "Custom",
            "source": {"entityTypeId": "8880000000001"},
            "target": {"entityTypeId": "8880000000002"}
        }))
        .unwrap(),
    )
    .unwrap();

    // Update definition from directory
    fabio()
        .args([
            "ontology",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--dir",
            ont_dir.to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Verify with get-definition --decode
    let assert = fabio()
        .args([
            "ontology",
            "get-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--decode",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let parts = data["definition"]["parts"].as_array().unwrap();

    let paths: Vec<&str> = parts.iter().filter_map(|p| p["path"].as_str()).collect();

    assert!(
        paths.contains(&"EntityTypes/8880000000001/definition.json"),
        "Missing Equipment entity type"
    );
    assert!(
        paths.contains(&"EntityTypes/8880000000002/definition.json"),
        "Missing Sensor entity type"
    );
    assert!(
        paths.iter().any(|p| p.contains("DataBindings/")),
        "Missing data binding"
    );
    assert!(
        paths.contains(&"RelationshipTypes/9990000000001/definition.json"),
        "Missing relationship type"
    );

    // Verify Equipment entity type content
    let equipment_part = parts
        .iter()
        .find(|p| p["path"].as_str().unwrap_or("") == "EntityTypes/8880000000001/definition.json")
        .unwrap();
    assert_eq!(equipment_part["decodedPayload"]["name"], "Equipment");

    // Verify relationship content
    let rel_part = parts
        .iter()
        .find(|p| {
            p["path"].as_str().unwrap_or("") == "RelationshipTypes/9990000000001/definition.json"
        })
        .unwrap();
    assert_eq!(rel_part["decodedPayload"]["name"], "hasSensor");
    assert_eq!(
        rel_part["decodedPayload"]["source"]["entityTypeId"],
        "8880000000001"
    );

    // Cleanup
    fabio()
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
}

// ---------------------------------------------------------------------------
// --dir and --definition/--file are mutually exclusive (create)
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn ontology_create_dir_conflicts_with_definition() {
    fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            "fake-ws",
            "--name",
            "test",
            "--dir",
            "/tmp",
            "--definition",
            "def.json",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

#[test]
#[serial]
fn ontology_create_dir_conflicts_with_file() {
    fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            "fake-ws",
            "--name",
            "test",
            "--dir",
            "/tmp",
            "--file",
            "schema.ttl",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

// ---------------------------------------------------------------------------
// --dir and --definition/--file are mutually exclusive (update-definition)
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn ontology_update_definition_dir_conflicts_with_definition() {
    fabio()
        .args([
            "ontology",
            "update-definition",
            "--workspace",
            "fake-ws",
            "--id",
            "fake-id",
            "--dir",
            "/tmp",
            "--definition",
            "def.json",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

#[test]
#[serial]
fn ontology_update_definition_dir_conflicts_with_file() {
    fabio()
        .args([
            "ontology",
            "update-definition",
            "--workspace",
            "fake-ws",
            "--id",
            "fake-id",
            "--dir",
            "/tmp",
            "--file",
            "schema.ttl",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

// ---------------------------------------------------------------------------
// Full IoT scenario: create ontology with entity types + data bindings
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_iot_scenario_entity_types_and_data_bindings() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_iot");

    // Build ontology definition with entity types bound to lakehouse sales table
    let entity_def = serde_json::json!({
        "id": "6660000000001",
        "namespace": "usertypes",
        "baseEntityTypeId": null,
        "name": "SalesRecord",
        "entityIdParts": ["6660000000011"],
        "displayNamePropertyId": "6660000000011",
        "namespaceType": "Custom",
        "visibility": "Visible",
        "properties": [
            {"id": "6660000000011", "name": "DisplayName", "redefines": null, "baseTypeNamespaceType": null, "valueType": "String"},
            {"id": "6660000000012", "name": "Country", "redefines": null, "baseTypeNamespaceType": null, "valueType": "String"},
            {"id": "6660000000013", "name": "Revenue", "redefines": null, "baseTypeNamespaceType": null, "valueType": "Double"}
        ],
        "timeseriesProperties": []
    });

    let binding_def = serde_json::json!({
        "id": "c0000001-0001-0001-0001-000000000001",
        "dataBindingConfiguration": {
            "dataBindingType": "NonTimeSeries",
            "propertyBindings": [
                {"sourceColumnName": "country", "targetPropertyId": "6660000000011"},
                {"sourceColumnName": "country", "targetPropertyId": "6660000000012"},
                {"sourceColumnName": "revenue", "targetPropertyId": "6660000000013"}
            ],
            "sourceTableProperties": {
                "sourceType": "LakehouseTable",
                "workspaceId": &cfg.source_workspace,
                "itemId": &cfg.source_lakehouse,
                "sourceTableName": "sales",
                "sourceSchema": "dbo"
            }
        }
    });

    let def_json_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"{}");
    let entity_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&entity_def).unwrap().as_bytes(),
    );
    let binding_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&binding_def).unwrap().as_bytes(),
    );

    let full_def = serde_json::json!({
        "parts": [
            {"path": "definition.json", "payload": def_json_b64, "payloadType": "InlineBase64"},
            {"path": "EntityTypes/6660000000001/definition.json", "payload": entity_b64, "payloadType": "InlineBase64"},
            {"path": "EntityTypes/6660000000001/DataBindings/c0000001-0001-0001-0001-000000000001.json", "payload": binding_b64, "payloadType": "InlineBase64"}
        ]
    });

    let dir = tempfile::tempdir().unwrap();
    let def_path = dir.path().join("def.json");
    std::fs::write(&def_path, serde_json::to_string(&full_def).unwrap()).unwrap();

    // Create ontology with entity types + data bindings
    let assert = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
            "--description",
            "IoT scenario with entity types and data bindings",
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

    // Verify entity types exist in definition
    let assert = fabio()
        .args([
            "ontology",
            "get-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--decode",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let parts = data["definition"]["parts"].as_array().unwrap();

    // Verify SalesRecord entity type
    let entity_part = parts
        .iter()
        .find(|p| {
            p["path"]
                .as_str()
                .unwrap_or("")
                .contains("EntityTypes/6660000000001/definition.json")
        })
        .expect("SalesRecord entity type should exist");
    assert_eq!(entity_part["decodedPayload"]["name"], "SalesRecord");
    let properties = entity_part["decodedPayload"]["properties"]
        .as_array()
        .unwrap();
    assert_eq!(properties.len(), 3);

    // Verify data binding exists
    let binding_part = parts
        .iter()
        .find(|p| p["path"].as_str().unwrap_or("").contains("DataBindings/"))
        .expect("Data binding should exist");
    let binding_config = &binding_part["decodedPayload"]["dataBindingConfiguration"];
    assert_eq!(binding_config["dataBindingType"], "NonTimeSeries");
    assert_eq!(
        binding_config["sourceTableProperties"]["sourceTableName"],
        "sales"
    );

    // Cleanup
    fabio()
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
}

// ---------------------------------------------------------------------------
// Get definition without --decode (original behavior preserved)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_get_definition_without_decode() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_nodec");

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

    // Get definition WITHOUT --decode
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
    let parts = data["definition"]["parts"].as_array().unwrap();

    // Without --decode, parts should NOT have decodedPayload field
    for part in parts {
        assert!(
            part.get("decodedPayload").is_none(),
            "decodedPayload should not exist without --decode flag"
        );
        // But payload should be base64 string
        assert!(part["payload"].is_string());
    }

    // Cleanup
    fabio()
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
}

// ─── Import Tests ────────────────────────────────────────────────────────────

#[test]
fn ontology_import_rdf_to_directory() {
    let rdf = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
         xmlns:owl="http://www.w3.org/2002/07/owl#"
         xmlns:xsd="http://www.w3.org/2001/XMLSchema#"
         xmlns:ont="http://example.org/">
  <owl:Class rdf:about="http://example.org/Sensor"><rdfs:label>Sensor</rdfs:label></owl:Class>
  <owl:Class rdf:about="http://example.org/Reading"><rdfs:label>Reading</rdfs:label></owl:Class>
  <owl:DatatypeProperty rdf:about="http://example.org/sensor_id">
    <rdfs:label>sensorId</rdfs:label>
    <rdfs:domain rdf:resource="http://example.org/Sensor"/>
    <rdfs:range rdf:resource="http://www.w3.org/2001/XMLSchema#string"/>
    <ont:isIdentifier rdf:datatype="http://www.w3.org/2001/XMLSchema#boolean">true</ont:isIdentifier>
  </owl:DatatypeProperty>
  <owl:ObjectProperty rdf:about="http://example.org/produces">
    <rdfs:label>produces</rdfs:label>
    <rdfs:domain rdf:resource="http://example.org/Sensor"/>
    <rdfs:range rdf:resource="http://example.org/Reading"/>
  </owl:ObjectProperty>
</rdf:RDF>"#;

    let tmp_dir = std::env::temp_dir();
    let rdf_file = tmp_dir.join("fabio_test_import.rdf");
    let out_dir = tmp_dir.join("fabio_test_import_out");
    std::fs::write(&rdf_file, rdf).unwrap();
    let _ = std::fs::remove_dir_all(&out_dir);

    let output = fabio()
        .args([
            "ontology",
            "import",
            "--file",
            &rdf_file.display().to_string(),
            "--output-dir",
            &out_dir.display().to_string(),
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "exported");
    assert_eq!(data["entity_types"], 2);
    assert_eq!(data["relationship_types"], 1);
    assert!(out_dir.join("definition.json").exists());
    assert!(out_dir.join("EntityTypes").exists());
    assert!(out_dir.join("RelationshipTypes").exists());

    let _ = std::fs::remove_file(&rdf_file);
    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn ontology_import_jsonld_to_directory() {
    let jsonld = r#"{"@graph": [
        {"@id": "http://ex.org/Device", "@type": "owl:Class", "rdfs:label": "Device"},
        {"@id": "http://ex.org/Event", "@type": "owl:Class", "rdfs:label": "Event"},
        {"@id": "http://ex.org/emits", "@type": "owl:ObjectProperty", "rdfs:label": "emits",
         "rdfs:domain": {"@id": "http://ex.org/Device"}, "rdfs:range": {"@id": "http://ex.org/Event"}}
    ]}"#;

    let tmp_dir = std::env::temp_dir();
    let file = tmp_dir.join("fabio_test_import.jsonld");
    let out_dir = tmp_dir.join("fabio_test_import_jsonld_out");
    std::fs::write(&file, jsonld).unwrap();
    let _ = std::fs::remove_dir_all(&out_dir);

    let output = fabio()
        .args([
            "ontology",
            "import",
            "--file",
            &file.display().to_string(),
            "--output-dir",
            &out_dir.display().to_string(),
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "exported");
    assert_eq!(data["entity_types"], 2);
    assert_eq!(data["relationship_types"], 1);

    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn ontology_import_dry_run() {
    let rdf = r#"<?xml version="1.0"?><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:owl="http://www.w3.org/2002/07/owl#" xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"><owl:Class rdf:about="http://ex.org/T"><rdfs:label>T</rdfs:label></owl:Class></rdf:RDF>"#;
    let tmp = std::env::temp_dir().join("fabio_test_import_dryrun.rdf");
    std::fs::write(&tmp, rdf).unwrap();

    let output = fabio()
        .args([
            "ontology",
            "import",
            "--file",
            &tmp.display().to_string(),
            "--output-dir",
            "/tmp/unused",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["details"]["entity_types"], 1);

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn ontology_import_missing_file_fails() {
    fabio()
        .args([
            "ontology",
            "import",
            "--file",
            "/nonexistent/path.rdf",
            "--output-dir",
            "/tmp/x",
        ])
        .assert()
        .failure();
}

#[test]
fn ontology_import_no_output_no_workspace_fails() {
    let tmp = std::env::temp_dir().join("fabio_test_noout.rdf");
    std::fs::write(&tmp, "<?xml version=\"1.0\"?><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"/>").unwrap();

    let output = fabio()
        .args(["ontology", "import", "--file", &tmp.display().to_string()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(stderr.contains("--workspace") || stderr.contains("--output-dir"));
    let _ = std::fs::remove_file(&tmp);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn ontology_import_rdf_live() {
    let cfg = TestConfig::from_env();
    let name = unique_name("ont_import");

    let output = fabio()
        .args([
            "ontology",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .assert()
        .success();
    let json = parse_json(&output);
    let ont_id = extract_data(&json)["id"].as_str().unwrap().to_string();

    let rdf = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:owl="http://www.w3.org/2002/07/owl#" xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#" xmlns:xsd="http://www.w3.org/2001/XMLSchema#" xmlns:ont="http://example.org/">
  <owl:Class rdf:about="http://example.org/Vehicle"><rdfs:label>Vehicle</rdfs:label></owl:Class>
  <owl:Class rdf:about="http://example.org/Trip"><rdfs:label>Trip</rdfs:label></owl:Class>
  <owl:DatatypeProperty rdf:about="http://example.org/vehicle_vin">
    <rdfs:label>vin</rdfs:label>
    <rdfs:domain rdf:resource="http://example.org/Vehicle"/>
    <rdfs:range rdf:resource="http://www.w3.org/2001/XMLSchema#string"/>
    <ont:isIdentifier rdf:datatype="http://www.w3.org/2001/XMLSchema#boolean">true</ont:isIdentifier>
  </owl:DatatypeProperty>
  <owl:DatatypeProperty rdf:about="http://example.org/vehicle_name">
    <rdfs:label>name</rdfs:label>
    <rdfs:domain rdf:resource="http://example.org/Vehicle"/>
    <rdfs:range rdf:resource="http://www.w3.org/2001/XMLSchema#string"/>
  </owl:DatatypeProperty>
  <owl:DatatypeProperty rdf:about="http://example.org/trip_id">
    <rdfs:label>tripId</rdfs:label>
    <rdfs:domain rdf:resource="http://example.org/Trip"/>
    <rdfs:range rdf:resource="http://www.w3.org/2001/XMLSchema#string"/>
    <ont:isIdentifier rdf:datatype="http://www.w3.org/2001/XMLSchema#boolean">true</ont:isIdentifier>
  </owl:DatatypeProperty>
  <owl:DatatypeProperty rdf:about="http://example.org/trip_name">
    <rdfs:label>destination</rdfs:label>
    <rdfs:domain rdf:resource="http://example.org/Trip"/>
    <rdfs:range rdf:resource="http://www.w3.org/2001/XMLSchema#string"/>
  </owl:DatatypeProperty>
  <owl:ObjectProperty rdf:about="http://example.org/makes">
    <rdfs:label>makes</rdfs:label>
    <rdfs:domain rdf:resource="http://example.org/Vehicle"/>
    <rdfs:range rdf:resource="http://example.org/Trip"/>
  </owl:ObjectProperty>
</rdf:RDF>"#;
    let tmp = std::env::temp_dir().join("fabio_e2e_import.rdf");
    std::fs::write(&tmp, rdf).unwrap();

    let output = fabio()
        .args([
            "ontology",
            "import",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &ont_id,
            "--file",
            &tmp.display().to_string(),
        ])
        .assert()
        .success();
    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "imported");
    assert_eq!(data["entity_types"], 2);
    assert_eq!(data["relationship_types"], 1);

    // Cleanup
    fabio()
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
    let _ = std::fs::remove_file(&tmp);
}
