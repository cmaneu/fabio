use assert_cmd::Command;

fn fabio() -> Command {
    Command::cargo_bin("fabio").unwrap()
}

#[test]
fn agent_context_returns_schema() {
    let assert = fabio()
        .args(["context", "agent", "--full"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Should have commands object
    assert!(json["data"]["commands"].is_object());
    let commands = json["data"]["commands"].as_object().unwrap();
    assert!(!commands.is_empty(), "context agent should list commands");
}

#[test]
fn agent_context_includes_workspace_command() {
    let assert = fabio()
        .args(["context", "agent", "--full"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let commands = json["data"]["commands"].as_object().unwrap();
    assert!(
        commands.contains_key("workspace"),
        "context agent should include 'workspace' command"
    );
}

#[test]
fn agent_context_output_table_format() {
    let assert = fabio()
        .args(["--output", "table", "context", "agent"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Table format should contain column headers or structured text
    assert!(!stdout.is_empty());
}

#[test]
fn agent_context_includes_app_backend_command_with_delete_flag() {
    let assert = fabio()
        .args(["context", "agent", "--full"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let commands = json["data"]["commands"].as_object().unwrap();

    let app_backend = commands
        .get("app-backend")
        .expect("context agent should include 'app-backend' command");
    let hard_delete_type = &app_backend["subcommands"]["delete"]["flags"]["--hard-delete"]["type"];
    assert_eq!(hard_delete_type, "bool");
}

// ── Phase 1: Hierarchical schema access ──────────────────────────────────────

#[test]
fn agent_context_default_returns_compact_index() {
    let assert = fabio().args(["context", "agent"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let commands = json["data"]["commands"].as_object().unwrap();
    // Should have all 74 groups
    assert!(commands.len() >= 70, "default should list all groups");
    // Each group should have description + subcommands array
    let lakehouse = &commands["lakehouse"];
    assert!(lakehouse["description"].is_string());
    assert!(lakehouse["subcommands"].is_array());
    let subs = lakehouse["subcommands"].as_array().unwrap();
    assert!(subs.len() > 30, "lakehouse should have 30+ subcommands");
    // Should NOT have full flag details (compact = names only)
    assert!(lakehouse.get("flags").is_none());
}

#[test]
fn agent_context_default_is_small() {
    let assert = fabio().args(["context", "agent"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Default compact output should be much smaller than full dump
    assert!(
        stdout.len() < 20_000,
        "default output should be under 20KB, got {} bytes",
        stdout.len()
    );
}

#[test]
fn agent_context_group_returns_single_group() {
    let assert = fabio()
        .args(["context", "agent", "--group", "lakehouse"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["data"]["group"], "lakehouse");
    assert!(json["data"]["group_details"]["subcommands"].is_object());
    // Should include global_flags for context
    assert!(json["data"]["global_flags"].is_array());
    // Should include error_codes for context
    assert!(json["data"]["error_codes"].is_array());
}

#[test]
fn agent_context_group_case_insensitive() {
    let assert = fabio()
        .args(["context", "agent", "--group", "KQL-Database"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["data"]["group"], "kql-database");
}

#[test]
fn agent_context_group_invalid_shows_available() {
    let assert = fabio()
        .args(["context", "agent", "--group", "nonexistent"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        json["data"]["error"]
            .as_str()
            .unwrap()
            .contains("nonexistent")
    );
    assert!(json["data"]["available_groups"].is_array());
    assert!(json["data"]["hint"].is_string());
}

#[test]
fn agent_context_group_much_smaller_than_full() {
    let full = fabio()
        .args(["context", "agent", "--full"])
        .assert()
        .success();
    let full_size = full.get_output().stdout.len();

    let group = fabio()
        .args(["context", "agent", "--group", "lakehouse"])
        .assert()
        .success();
    let group_size = group.get_output().stdout.len();

    // Group output should be at most 20% of full output
    assert!(
        group_size < full_size / 5,
        "group ({group_size}) should be <20% of full ({full_size})"
    );
}

// ── Describe subcommand ──────────────────────────────────────────────────────

#[test]
fn describe_returns_command_metadata() {
    let assert = fabio()
        .args(["context", "describe", "lakehouse", "sync"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["data"]["command"], "fabio lakehouse sync");
    assert!(json["data"]["description"].is_string());
    assert!(json["data"]["flags"].is_object());
    assert_eq!(json["data"]["mutates"], true);
    assert_eq!(json["data"]["returns"], "object");
}

#[test]
fn describe_includes_output_example_when_available() {
    let assert = fabio()
        .args(["context", "describe", "lakehouse", "sync"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // lakehouse/sync has an output example
    assert!(
        json["data"]["output_example"].is_object(),
        "describe should cross-reference the matching output example"
    );
    assert!(json["data"]["output_example"]["response"].is_object());
}

#[test]
fn describe_includes_auth_scope_from_group() {
    let assert = fabio()
        .args(["context", "describe", "lakehouse", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // auth_scope should be inherited from the group level
    assert!(json["data"]["auth_scope"].is_string());
}

#[test]
fn describe_invalid_group_shows_available() {
    let assert = fabio()
        .args(["context", "describe", "bogus", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["data"]["error"].as_str().unwrap().contains("bogus"));
    assert!(json["data"]["available_groups"].is_array());
}

#[test]
fn describe_invalid_subcommand_shows_available() {
    let assert = fabio()
        .args(["context", "describe", "lakehouse", "nonexistent"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        json["data"]["error"]
            .as_str()
            .unwrap()
            .contains("nonexistent")
    );
    assert!(json["data"]["available_subcommands"].is_array());
}

// ── Phase 2: Standard format emission ────────────────────────────────────────

#[test]
fn agent_format_mcp_emits_tools_array() {
    let assert = fabio()
        .args([
            "context",
            "agent",
            "--format",
            "mcp",
            "--group",
            "lakehouse",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let tools = json["data"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty());

    // Each tool should have MCP-standard fields.
    let tool = &tools[0];
    assert!(tool["name"].is_string());
    assert!(tool["description"].is_string());
    assert!(tool["inputSchema"].is_object());
    assert_eq!(tool["inputSchema"]["type"], "object");
    assert!(tool["inputSchema"]["properties"].is_object());
    assert!(tool["annotations"].is_object());
}

#[test]
fn agent_format_mcp_tool_names_use_underscores() {
    let assert = fabio()
        .args([
            "context",
            "agent",
            "--format",
            "mcp",
            "--group",
            "kql-database",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let tools = json["data"]["tools"].as_array().unwrap();
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        assert!(
            !name.contains('-'),
            "MCP tool name should not contain hyphens: {name}"
        );
        assert!(
            name.starts_with("fabio_kql_database_"),
            "tool name should start with fabio_kql_database_: {name}"
        );
    }
}

#[test]
fn agent_format_mcp_marks_mutations() {
    let assert = fabio()
        .args([
            "context",
            "agent",
            "--format",
            "mcp",
            "--group",
            "lakehouse",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let tools = json["data"]["tools"].as_array().unwrap();
    // Find a read-only tool (list).
    let list_tool = tools
        .iter()
        .find(|t| t["name"] == "fabio_lakehouse_list")
        .expect("should have list tool");
    assert_eq!(list_tool["annotations"]["readOnlyHint"], true);

    // Find a mutating tool (create).
    let create_tool = tools
        .iter()
        .find(|t| t["name"] == "fabio_lakehouse_create")
        .expect("should have create tool");
    assert_eq!(create_tool["annotations"]["readOnlyHint"], false);
}

#[test]
fn agent_format_mcp_includes_required_params() {
    let assert = fabio()
        .args([
            "context",
            "agent",
            "--format",
            "mcp",
            "--group",
            "lakehouse",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let tools = json["data"]["tools"].as_array().unwrap();
    let sync_tool = tools
        .iter()
        .find(|t| t["name"] == "fabio_lakehouse_sync")
        .expect("should have sync tool");
    let required = sync_tool["inputSchema"]["required"].as_array().unwrap();
    assert!(
        !required.is_empty(),
        "sync tool should have required params"
    );
}

#[test]
fn agent_format_openai_emits_functions_array() {
    let assert = fabio()
        .args([
            "context",
            "agent",
            "--format",
            "openai",
            "--group",
            "workspace",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let functions = json["data"]["functions"].as_array().unwrap();
    assert!(!functions.is_empty());

    // Each function should have OpenAI-standard structure.
    let func = &functions[0];
    assert_eq!(func["type"], "function");
    assert!(func["function"]["name"].is_string());
    assert!(func["function"]["description"].is_string());
    assert!(func["function"]["parameters"].is_object());
    assert_eq!(
        func["function"]["parameters"]["additionalProperties"],
        false
    );
}

#[test]
fn agent_format_openai_includes_mutation_hint_in_description() {
    let assert = fabio()
        .args([
            "context",
            "agent",
            "--format",
            "openai",
            "--group",
            "workspace",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let functions = json["data"]["functions"].as_array().unwrap();
    // Find a read-only function.
    let list_fn = functions
        .iter()
        .find(|f| f["function"]["name"] == "fabio_workspace_list")
        .expect("should have list function");
    assert!(
        list_fn["function"]["description"]
            .as_str()
            .unwrap()
            .contains("[read-only")
    );

    // Find a mutating function.
    let create_fn = functions
        .iter()
        .find(|f| f["function"]["name"] == "fabio_workspace_create")
        .expect("should have create function");
    assert!(
        create_fn["function"]["description"]
            .as_str()
            .unwrap()
            .contains("[mutates")
    );
}

#[test]
fn agent_format_mcp_full_emits_all_tools() {
    let assert = fabio()
        .args(["context", "agent", "--format", "mcp"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let tools = json["data"]["tools"].as_array().unwrap();
    // Should have 800+ tools (all 807 subcommands)
    assert!(
        tools.len() >= 800,
        "full MCP output should have 800+ tools, got {}",
        tools.len()
    );
}

// ── Phase 4: context find ────────────────────────────────────────────────────

#[test]
fn find_returns_relevant_results() {
    let assert = fabio()
        .args(["context", "find", "upload file lakehouse"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let results = json["data"]["results"].as_array().unwrap();
    assert!(!results.is_empty(), "find should return results");
    // Top result should be lakehouse upload
    assert_eq!(results[0]["command"], "fabio lakehouse upload");
}

#[test]
fn find_returns_empty_for_nonsense() {
    let assert = fabio()
        .args(["context", "find", "xyzzy frobulate quantum"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let results = json["data"]["results"].as_array().unwrap();
    assert!(
        results.is_empty(),
        "nonsense query should return no results"
    );
    assert!(json["data"]["hint"].is_string());
}

#[test]
fn find_results_have_required_fields() {
    let assert = fabio()
        .args(["context", "find", "workspace create"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let results = json["data"]["results"].as_array().unwrap();
    assert!(!results.is_empty());
    let first = &results[0];
    assert!(first["command"].is_string());
    assert!(first["score"].is_number());
    assert!(first["description"].is_string());
}
