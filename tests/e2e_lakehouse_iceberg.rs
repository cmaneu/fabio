//! End-to-end integration tests for `fabio lakehouse iceberg-*` commands
//! (`OneLake` Table API / Iceberg REST Catalog).

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

// ─── Offline tests (no live tenant required) ─────────────────────────────────

#[test]
fn iceberg_config_missing_workspace() {
    fabio()
        .args([
            "lakehouse",
            "iceberg-config",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
        ])
        .assert()
        .failure();
}

#[test]
fn iceberg_config_missing_id() {
    fabio()
        .args([
            "lakehouse",
            "iceberg-config",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
        ])
        .assert()
        .failure();
}

#[test]
fn iceberg_namespaces_missing_workspace() {
    fabio()
        .args([
            "lakehouse",
            "iceberg-namespaces",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
        ])
        .assert()
        .failure();
}

#[test]
fn iceberg_tables_missing_namespace() {
    fabio()
        .args([
            "lakehouse",
            "iceberg-tables",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
        ])
        .assert()
        .failure();
}

#[test]
fn iceberg_table_missing_table() {
    fabio()
        .args([
            "lakehouse",
            "iceberg-table",
            "--workspace",
            "aaaaaaaa-1111-2222-3333-444444444444",
            "--id",
            "bbbbbbbb-1111-2222-3333-444444444444",
            "--namespace",
            "dbo",
        ])
        .assert()
        .failure();
}

#[test]
fn iceberg_help_shows_commands() {
    let assert = fabio().args(["lakehouse", "--help"]).assert().success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("iceberg-config"));
    assert!(stdout.contains("iceberg-namespaces"));
    assert!(stdout.contains("iceberg-namespace"));
    assert!(stdout.contains("iceberg-tables"));
    assert!(stdout.contains("iceberg-table"));
}

// ─── Live tests (require Fabric tenant) ──────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn iceberg_config_returns_endpoints() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "lakehouse",
            "iceberg-config",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // The config response should include available endpoints
    let endpoints = data.get("endpoints").expect("should have endpoints field");
    let arr = endpoints.as_array().expect("endpoints should be array");
    assert!(!arr.is_empty(), "should have at least one endpoint");

    // Should include the namespaces endpoint
    let has_namespaces = arr
        .iter()
        .any(|e| e.as_str().is_some_and(|s| s.contains("namespaces")));
    assert!(has_namespaces, "should include namespaces endpoint");

    // Should include the tables endpoint
    let has_tables = arr
        .iter()
        .any(|e| e.as_str().is_some_and(|s| s.contains("tables")));
    assert!(has_tables, "should include tables endpoint");

    // Should have an overrides.prefix matching workspace/item
    let prefix = data
        .get("overrides")
        .and_then(|o| o.get("prefix"))
        .and_then(|p| p.as_str())
        .expect("should have overrides.prefix");
    assert!(
        prefix.contains(&cfg.source_workspace),
        "prefix should contain workspace ID"
    );
    assert!(
        prefix.contains(&cfg.source_lakehouse),
        "prefix should contain lakehouse ID"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn iceberg_namespaces_returns_dbo() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "lakehouse",
            "iceberg-namespaces",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Should have namespaces array
    let namespaces = data
        .get("namespaces")
        .expect("should have namespaces field");
    let arr = namespaces.as_array().expect("namespaces should be array");
    assert!(!arr.is_empty(), "should have at least one namespace");

    // Standard lakehouses have the "dbo" namespace
    let has_dbo = arr
        .iter()
        .any(|ns| ns.as_array().is_some_and(|a| a.iter().any(|v| v == "dbo")));
    assert!(has_dbo, "should contain dbo namespace");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn iceberg_namespace_shows_dbo_metadata() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "lakehouse",
            "iceberg-namespace",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--namespace",
            "dbo",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Should have namespace field echoing back "dbo"
    let ns = data.get("namespace").expect("should have namespace field");
    let ns_arr = ns.as_array().expect("namespace should be array");
    assert_eq!(ns_arr[0].as_str().unwrap(), "dbo");

    // Should have properties with location
    let props = data.get("properties").expect("should have properties");
    assert!(
        props.get("location").is_some(),
        "should have location property"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn iceberg_tables_lists_tables_in_namespace() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "lakehouse",
            "iceberg-tables",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--namespace",
            "dbo",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Should have identifiers array
    let identifiers = data
        .get("identifiers")
        .expect("should have identifiers field");
    let arr = identifiers.as_array().expect("identifiers should be array");

    // Source lakehouse should have at least one table (from other tests)
    assert!(!arr.is_empty(), "should have at least one table");

    // Each identifier should have name and namespace fields
    let first = &arr[0];
    assert!(first.get("name").is_some(), "table should have name");
    assert!(
        first.get("namespace").is_some(),
        "table should have namespace"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn iceberg_table_returns_full_schema() {
    let cfg = TestConfig::from_env();

    // First, list tables to get a real table name
    let list_assert = fabio()
        .args([
            "lakehouse",
            "iceberg-tables",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--namespace",
            "dbo",
        ])
        .assert()
        .success();

    let list_json = parse_json(&list_assert);
    let list_data = extract_data(&list_json);
    let identifiers = list_data["identifiers"]
        .as_array()
        .expect("should have identifiers");
    assert!(!identifiers.is_empty(), "need at least one table to test");

    let table_name = identifiers[0]["name"]
        .as_str()
        .expect("table should have name");

    // Now get the full table definition
    let assert = fabio()
        .args([
            "lakehouse",
            "iceberg-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--namespace",
            "dbo",
            "--table",
            table_name,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);

    // Should have metadata with schema information
    let metadata = data.get("metadata").expect("should have metadata field");

    // format-version should be 1 or 2
    let format_version = metadata
        .get("format-version")
        .expect("should have format-version");
    let version = format_version.as_u64().unwrap();
    assert!(
        version == 1 || version == 2,
        "format-version should be 1 or 2, got {version}"
    );

    // Should have schemas array with at least one schema
    let schemas = metadata
        .get("schemas")
        .expect("should have schemas")
        .as_array()
        .expect("schemas should be array");
    assert!(!schemas.is_empty(), "should have at least one schema");

    // The active schema should have fields
    let current_schema_id = metadata
        .get("current-schema-id")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let active_schema = schemas
        .iter()
        .find(|s| {
            s.get("schema-id")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                == current_schema_id
        })
        .expect("should find active schema");
    let fields = active_schema
        .get("fields")
        .expect("schema should have fields")
        .as_array()
        .expect("fields should be array");
    assert!(!fields.is_empty(), "schema should have at least one field");

    // Each field should have id, name, type, required
    let first_field = &fields[0];
    assert!(first_field.get("id").is_some(), "field should have id");
    assert!(first_field.get("name").is_some(), "field should have name");
    assert!(first_field.get("type").is_some(), "field should have type");
    assert!(
        first_field.get("required").is_some(),
        "field should have required"
    );

    // Should have metadata-location pointing to an abfss:// path
    let metadata_location = data
        .get("metadata-location")
        .and_then(|v| v.as_str())
        .expect("should have metadata-location");
    assert!(
        metadata_location.starts_with("abfss://"),
        "metadata-location should be an abfss:// path"
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn iceberg_config_with_json_output() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "--output",
            "json",
            "lakehouse",
            "iceberg-config",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    // Should be a valid JSON envelope with "data" key
    assert!(json.get("data").is_some(), "should have data envelope");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn iceberg_namespace_not_found() {
    let cfg = TestConfig::from_env();

    // Request a namespace that doesn't exist
    fabio()
        .args([
            "lakehouse",
            "iceberg-namespace",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--namespace",
            "nonexistent_schema_xyz_999",
        ])
        .assert()
        .failure();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn iceberg_table_not_found() {
    let cfg = TestConfig::from_env();

    // Request a table that doesn't exist
    fabio()
        .args([
            "lakehouse",
            "iceberg-table",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.source_lakehouse,
            "--namespace",
            "dbo",
            "--table",
            "nonexistent_table_xyz_999",
        ])
        .assert()
        .failure();
}
