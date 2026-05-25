//! End-to-end integration tests for `fabio data-agent` commands.

mod common;

use base64::Engine;
use common::{TestConfig, extract_count, extract_data, fabio, parse_json, unique_name};
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_list_returns_json() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args(["data-agent", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    // Should have data array and count
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_list_via_alias() {
    let cfg = TestConfig::from_env();

    // Should work with the 'da' alias
    let assert = fabio()
        .args(["da", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    assert!(json.get("data").is_some());
    assert!(json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_create_show_update_delete() {
    let cfg = TestConfig::from_env();
    let name = unique_name("da_test");
    let description = "Test data agent created by fabio e2e tests";

    // Create a data agent
    let assert = fabio()
        .args([
            "data-agent",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
            "--description",
            description,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    assert_eq!(data["type"], "DataAgent");
    let agent_id = data["id"].as_str().unwrap().to_string();

    // Show the data agent
    let assert = fabio()
        .args([
            "data-agent",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["id"], agent_id);
    assert_eq!(data["displayName"], name);

    // Update the data agent
    let new_name = unique_name("da_updated");
    let new_desc = "Updated description";
    let assert = fabio()
        .args([
            "data-agent",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
            "--name",
            &new_name,
            "--description",
            new_desc,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], new_name);
    assert_eq!(data["description"], new_desc);

    // Delete the data agent
    let assert = fabio()
        .args([
            "data-agent",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["status"], "deleted");
    assert_eq!(data["id"], agent_id);

    // Verify it's gone (show should fail)
    fabio()
        .args([
            "data-agent",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .failure();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_create_without_description() {
    let cfg = TestConfig::from_env();
    let name = unique_name("da_nodesc");

    // Create without description
    let assert = fabio()
        .args([
            "data-agent",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["displayName"], name);
    let agent_id = data["id"].as_str().unwrap().to_string();

    // Cleanup
    fabio()
        .args([
            "data-agent",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_update_requires_at_least_one_field() {
    let cfg = TestConfig::from_env();

    // Update without --name or --description should fail
    fabio()
        .args([
            "data-agent",
            "update",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("INVALID_INPUT"));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_show_nonexistent_returns_not_found() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "data-agent",
            "show",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NOT_FOUND").or(predicate::str::contains("API_ERROR")));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_list_table_output() {
    let cfg = TestConfig::from_env();

    fabio()
        .args([
            "--output",
            "table",
            "data-agent",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_list_with_query_projection() {
    let cfg = TestConfig::from_env();

    let assert = fabio()
        .args([
            "--query",
            "displayName",
            "data-agent",
            "list",
            "--workspace",
            &cfg.source_workspace,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    // Query projection should return array of names (or empty)
    let data = extract_data(&json);
    if let Some(arr) = data.as_array() {
        for item in arr {
            // Each item should be a string (projected displayName)
            assert!(item.is_string(), "expected string, got: {item}");
        }
    }
}

#[test]
#[ignore = "requires live Fabric tenant and published data agent"]
#[serial]
fn dataagent_query_with_prompt() {
    let cfg = TestConfig::from_env();

    // This test requires a published data agent. Skip if no agent exists.
    let assert = fabio()
        .args(["data-agent", "list", "--workspace", &cfg.source_workspace])
        .assert()
        .success();

    let json = parse_json(&assert);
    let count = extract_count(&json);
    if count == 0 {
        eprintln!("No data agents in workspace, skipping query test");
        return;
    }

    let items = extract_data(&json).as_array().unwrap().clone();
    let agent_id = items[0]["id"].as_str().unwrap();

    // Try querying the data agent
    let assert = fabio()
        .args([
            "data-agent",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            agent_id,
            "--prompt",
            "What data sources do you have access to?",
        ])
        .timeout(std::time::Duration::from_secs(300))
        .assert();

    // Query may fail if agent isn't published - that's OK for this test
    let output = assert.get_output();
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let data = extract_data(&json);
        assert!(data.get("answer").is_some());
        assert!(data.get("question").is_some());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Query failed (agent may not be published): {stderr}");
    }
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_query_no_prompt_no_stdin_fails() {
    let cfg = TestConfig::from_env();

    // Without --prompt and with empty stdin, should fail
    fabio()
        .args([
            "data-agent",
            "query",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("INVALID_INPUT"));
}

// ─── Definition & Publish Tests ──────────────────────────────────────────────

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_get_definition() {
    let cfg = TestConfig::from_env();
    let name = unique_name("da_getdef");

    // Create a data agent first
    let assert = fabio()
        .args([
            "data-agent",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let agent_id = data["id"].as_str().unwrap().to_string();

    // Get definition - should succeed (even if empty/minimal)
    let assert = fabio()
        .args([
            "data-agent",
            "get-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Should have a definition with parts
    assert!(
        data.get("definition").is_some(),
        "expected definition field in response"
    );

    // Cleanup
    fabio()
        .args([
            "data-agent",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_update_definition_with_lakehouse_datasource() {
    let cfg = TestConfig::from_env();
    let name = unique_name("da_upddef");

    // Create a data agent
    let assert = fabio()
        .args([
            "data-agent",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let agent_id = data["id"].as_str().unwrap().to_string();

    // Build a definition that configures the data agent with a lakehouse data source
    let data_agent_json = serde_json::json!({
        "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/dataAgent/2.1.0/schema.json"
    });
    let stage_config_json = serde_json::json!({
        "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/stageConfiguration/1.0.0/schema.json",
        "aiInstructions": "You are a helpful data assistant for sales analytics."
    });
    let datasource_json = serde_json::json!({
        "$schema": "1.0.0",
        "artifactId": &cfg.source_lakehouse,
        "workspaceId": &cfg.source_workspace,
        "displayName": "SalesLakehouse",
        "type": "lakehouse_tables",
        "userDescription": "Sales data lakehouse",
        "dataSourceInstructions": "This contains sales transaction data"
    });

    let encode = |v: &serde_json::Value| {
        base64::engine::general_purpose::STANDARD.encode(v.to_string().as_bytes())
    };

    let definition = serde_json::json!({
        "definition": {
            "parts": [
                {
                    "path": "Files/Config/data_agent.json",
                    "payload": encode(&data_agent_json),
                    "payloadType": "InlineBase64"
                },
                {
                    "path": "Files/Config/draft/stage_config.json",
                    "payload": encode(&stage_config_json),
                    "payloadType": "InlineBase64"
                },
                {
                    "path": "Files/Config/draft/lakehouse-SalesLakehouse/datasource.json",
                    "payload": encode(&datasource_json),
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    let def_content = definition.to_string();

    // Update definition with lakehouse data source
    let assert = fabio()
        .args([
            "data-agent",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
            "--content",
            &def_content,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Response is either {"status":"definition_updated"} or the full item object
    assert!(
        data["status"] == "definition_updated" || data["id"] == *agent_id,
        "expected definition_updated status or item with matching id, got: {data}"
    );

    // Verify the definition was updated by fetching it back
    let assert = fabio()
        .args([
            "data-agent",
            "get-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let parts = data["definition"]["parts"].as_array().unwrap();
    // Should have at least the parts we set
    assert!(
        parts.len() >= 3,
        "expected at least 3 definition parts, got {}",
        parts.len()
    );

    // Cleanup
    fabio()
        .args([
            "data-agent",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_update_definition_requires_file_or_content() {
    let cfg = TestConfig::from_env();

    // Should fail without --file or --content
    fabio()
        .args([
            "data-agent",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("INVALID_INPUT"));
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_publish_lifecycle() {
    let cfg = TestConfig::from_env();
    let name = unique_name("da_pub");

    // Create a data agent
    let assert = fabio()
        .args([
            "data-agent",
            "create",
            "--workspace",
            &cfg.source_workspace,
            "--name",
            &name,
        ])
        .timeout(std::time::Duration::from_secs(180))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let agent_id = data["id"].as_str().unwrap().to_string();

    // First set up a draft definition
    let data_agent_json = serde_json::json!({
        "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/dataAgent/2.1.0/schema.json"
    });
    let stage_config_json = serde_json::json!({
        "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/stageConfiguration/1.0.0/schema.json",
        "aiInstructions": "You are a helpful data assistant."
    });
    let datasource_json = serde_json::json!({
        "$schema": "1.0.0",
        "artifactId": &cfg.source_lakehouse,
        "workspaceId": &cfg.source_workspace,
        "displayName": "TestLH",
        "type": "lakehouse_tables",
        "userDescription": "Test lakehouse"
    });

    let encode = |v: &serde_json::Value| {
        base64::engine::general_purpose::STANDARD.encode(v.to_string().as_bytes())
    };

    let definition = serde_json::json!({
        "definition": {
            "parts": [
                {
                    "path": "Files/Config/data_agent.json",
                    "payload": encode(&data_agent_json),
                    "payloadType": "InlineBase64"
                },
                {
                    "path": "Files/Config/draft/stage_config.json",
                    "payload": encode(&stage_config_json),
                    "payloadType": "InlineBase64"
                },
                {
                    "path": "Files/Config/draft/lakehouse-TestLH/datasource.json",
                    "payload": encode(&datasource_json),
                    "payloadType": "InlineBase64"
                }
            ]
        }
    });

    let def_content = definition.to_string();

    // Set draft definition
    fabio()
        .args([
            "data-agent",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
            "--content",
            &def_content,
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    // Publish the data agent
    let assert = fabio()
        .args([
            "data-agent",
            "publish",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
            "--description",
            "Test publish from e2e",
        ])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    // Status is "definition_promoted" (V3 not enabled) or "published" (V3 enabled)
    assert!(
        data["status"] == "definition_promoted" || data["status"] == "published",
        "expected definition_promoted or published, got: {}",
        data["status"]
    );
    assert_eq!(data["id"], agent_id);

    // Verify the definition now has published parts
    let assert = fabio()
        .args([
            "data-agent",
            "get-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    let parts = data["definition"]["parts"].as_array().unwrap();
    let paths: Vec<&str> = parts.iter().filter_map(|p| p["path"].as_str()).collect();

    // Should now have published paths
    assert!(
        paths
            .iter()
            .any(|p| p.starts_with("Files/Config/published/")),
        "expected published parts after publish, got paths: {paths:?}"
    );
    assert!(
        paths.contains(&"Files/Config/publish_info.json"),
        "expected publish_info.json, got paths: {paths:?}"
    );

    // Cleanup
    fabio()
        .args([
            "data-agent",
            "delete",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &agent_id,
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_publish_dry_run() {
    let cfg = TestConfig::from_env();

    // --dry-run should succeed without making changes
    let assert = fabio()
        .args([
            "--dry-run",
            "data-agent",
            "publish",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--description",
            "dry run test",
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "data-agent publish");
}

#[test]
#[ignore = "requires live Fabric tenant"]
#[serial]
fn dataagent_update_definition_dry_run() {
    let cfg = TestConfig::from_env();

    let definition = serde_json::json!({"definition": {"parts": []}}).to_string();

    let assert = fabio()
        .args([
            "--dry-run",
            "data-agent",
            "update-definition",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--content",
            &definition,
        ])
        .assert()
        .success();

    let json = parse_json(&assert);
    let data = extract_data(&json);
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["would_execute"], "data-agent update-definition");
}
