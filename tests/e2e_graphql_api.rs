//! End-to-end integration tests for `fabio graphql-api` commands.

mod common;

use common::{TestConfig, extract_data, fabio, parse_json};
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_list_returns_array() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["graphql-api", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data.is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_create_and_delete() {
    let cfg = TestConfig::from_env();
    let name = common::unique_name("gql_test");

    // Create
    let assert = fabio()
        .args([
            "graphql-api",
            "create",
            "--workspace",
            &cfg.dest_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_mins(2))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let id = data["id"].as_str().unwrap().to_string();

    // Delete
    let assert = fabio()
        .args([
            "graphql-api",
            "delete",
            "--workspace",
            &cfg.dest_workspace,
            "--id",
            &id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_show_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "graphql-api",
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
fn graphql_api_update_requires_field() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "graphql-api",
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
fn graphql_api_dry_run_create() {
    let cfg = TestConfig::from_env();
    let assert = fabio()
        .args([
            "graphql-api",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            "test-dry-run",
            "--dry-run",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["would_execute"], "graphql-api create");
}

// ─── Query tests ─────────────────────────────────────────────────────────────

/// Query the `SalesGraphQL` API (requires it to exist with data source configured)
/// Uses `FABIO_TEST_GRAPHQL_API_ID` and `FABIO_TEST_SOURCE_WORKSPACE`.
#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_customers() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            "{ customers { items { customer_id email city } } }",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let items = data["customers"]["items"].as_array().unwrap();
    assert!(!items.is_empty());
    // Check first item has expected fields
    assert!(items[0].get("customer_id").is_some());
    assert!(items[0].get("email").is_some());
    assert!(items[0].get("city").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_with_filter() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            r#"{ products(filter: {category: {eq: "Electronics"}}) { items { product_id category price } } }"#,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let items = data["products"]["items"].as_array().unwrap();
    assert!(!items.is_empty());
    // All returned items should be Electronics
    for item in items {
        assert_eq!(item["category"], "Electronics");
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_from_file() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    // Write query to temp file
    let tmp_file = std::env::temp_dir().join("fabio_test_query.graphql");
    std::fs::write(&tmp_file, "{ products { items { product_id price } } }").unwrap();
    let file_arg = format!("@{}", tmp_file.display());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            &file_arg,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let items = data["products"]["items"].as_array().unwrap();
    assert!(!items.is_empty());

    // Cleanup
    let _ = std::fs::remove_file(&tmp_file);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_from_stdin() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
        ])
        .write_stdin("{ products { items { product_id } } }")
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert!(data["products"]["items"].is_array());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_invalid_field_returns_error() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            "{ nonexistent_type { items { id } } }",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err_json["error"]["code"], "API_ERROR");
    assert!(
        err_json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("does not exist")
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_not_found() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--gql",
            "{ __typename }",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err_json["error"]["code"], "NOT_FOUND");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_products_all_fields() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            "{ products { items { product_id category price } } }",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let items = data["products"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);
    // Verify types
    assert!(items[0]["product_id"].is_number());
    assert!(items[0]["category"].is_string());
    assert!(items[0]["price"].is_number());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_filter_by_city() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            r#"{ customers(filter: {city: {eq: "Seattle"}}) { items { customer_id email city } } }"#,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let items = data["customers"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["city"], "Seattle");
    assert_eq!(items[0]["customer_id"], 1);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_filter_returns_empty() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            r#"{ customers(filter: {city: {eq: "Nonexistent City"}}) { items { customer_id } } }"#,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let items = data["customers"]["items"].as_array().unwrap();
    assert!(items.is_empty());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_multiple_root_fields() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    // Query both customers and products in one request
    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            "{ customers { items { customer_id } } products { items { product_id } } }",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Both root fields present
    assert!(data["customers"]["items"].is_array());
    assert!(data["products"]["items"].is_array());
    assert!(!data["customers"]["items"].as_array().unwrap().is_empty());
    assert!(!data["products"]["items"].as_array().unwrap().is_empty());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_with_field_projection() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    // Use fabio's --query/-q global option for field projection
    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            "{ products { items { product_id price } } }",
            "-q",
            "products.items",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Field projection extracts the nested path
    let items = json["data"].as_array().unwrap();
    assert!(!items.is_empty());
    assert!(items[0].get("product_id").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_table_output() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            "{ products { items { product_id category price } } }",
            "-o",
            "table",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Table output should contain key-value pairs
    assert!(stdout.contains("products"));
    assert!(stdout.contains("Electronics"));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_introspection_blocked() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    // Fabric blocks introspection by default
    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            "{ __schema { queryType { name } } }",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let err_json: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err_json["error"]["code"], "API_ERROR");
    assert!(
        err_json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Introspection")
    );
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_multiline_from_stdin() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let multiline_query = r"{
        customers(filter: {customer_id: {eq: 1}}) {
            items {
                customer_id
                email
                city
            }
        }
    }";

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
        ])
        .write_stdin(multiline_query)
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let items = data["customers"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["customer_id"], 1);
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_price_filter_gte() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            r"{ products(filter: {price: {gte: 40}}) { items { product_id price } } }",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let items = data["products"]["items"].as_array().unwrap();
    // products with price >= 40: product_id 2 (49.99), 5 (89.99)
    assert!(!items.is_empty());
    for item in items {
        let price = item["price"].as_f64().unwrap();
        assert!(price >= 40.0, "Expected price >= 40, got {price}");
    }
}

/// Test that --gql flag does not conflict with --quiet
#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn graphql_api_query_quiet_mode() {
    let cfg = TestConfig::from_env();
    let graphql_id = std::env::var("FABIO_TEST_GRAPHQL_API_ID")
        .unwrap_or_else(|_| "12310041-f5d0-4578-bf40-7aa461c79868".to_string());

    let assert = fabio()
        .args([
            "graphql-api",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &graphql_id,
            "--gql",
            "{ products { items { product_id } } }",
            "--quiet",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // --quiet suppresses all stdout
    assert!(stdout.is_empty());
}
