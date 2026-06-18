//! Example command outputs for AI agents (response shapes + `JMESPath` tips).

use serde_json::{Value, json};

use crate::cli::Cli;
use crate::output;

use super::find_entry;

pub(super) fn execute(cli: &Cli, group: &str, command: &str) {
    let key = format!("{group}/{command}");
    let normalized = key.to_lowercase().replace(['-', '_'], "");
    if let Some(content) = find_entry(OUTPUT_EXAMPLES, &normalized) {
        let val: Value =
            serde_json::from_str(content).unwrap_or_else(|_| json!({"content": content}));
        output::render_object(cli, &val, "command");
    } else {
        let available: Vec<&str> = OUTPUT_EXAMPLES.iter().map(|(name, _)| *name).collect();
        let result = json!({
            "error": format!("No output example found for '{group} {command}'"),
            "available_examples": available,
            "hint": "Use 'fabio context list' to see all available examples"
        });
        output::render_object(cli, &result, "error");
    }
}

pub(super) fn list_names() -> Vec<&'static str> {
    OUTPUT_EXAMPLES.iter().map(|(name, _)| *name).collect()
}

const OUTPUT_EXAMPLES: &[(&str, &str)] = &[
    (
        "lakehouse/list-tables",
        include_str!("data/examples/lakehouse_list_tables.json"),
    ),
    (
        "lakehouse/iceberg-table",
        include_str!("data/examples/lakehouse_iceberg_table.json"),
    ),
    (
        "lakehouse/iceberg-stats",
        include_str!("data/examples/lakehouse_iceberg_stats.json"),
    ),
    (
        "lakehouse/sync",
        include_str!("data/examples/lakehouse_sync.json"),
    ),
    (
        "workspace/list",
        include_str!("data/examples/workspace_list.json"),
    ),
    ("item/list", include_str!("data/examples/item_list.json")),
    (
        "deploy/plan",
        include_str!("data/examples/deploy_plan.json"),
    ),
    (
        "deploy/apply",
        include_str!("data/examples/deploy_apply.json"),
    ),
    (
        "notebook/run",
        include_str!("data/examples/notebook_run.json"),
    ),
    (
        "data-agent/query",
        include_str!("data/examples/data_agent_query.json"),
    ),
    (
        "context/tenant",
        include_str!("data/examples/context_tenant.json"),
    ),
    (
        "kql-database/list-entities",
        include_str!("data/examples/kql_database_list_entities.json"),
    ),
    (
        "kql-database/describe",
        include_str!("data/examples/kql_database_describe.json"),
    ),
    (
        "kql-database/sample",
        include_str!("data/examples/kql_database_sample.json"),
    ),
    (
        "kql-database/diagnostics",
        include_str!("data/examples/kql_database_diagnostics.json"),
    ),
    (
        "kql-database/deeplink",
        include_str!("data/examples/kql_database_deeplink.json"),
    ),
    (
        "kql-database/ingest",
        include_str!("data/examples/kql_database_ingest.json"),
    ),
    (
        "eventstream/validate",
        include_str!("data/examples/eventstream_validate.json"),
    ),
    (
        "eventstream/list-components",
        include_str!("data/examples/eventstream_list_components.json"),
    ),
    (
        "reflex/create-trigger",
        include_str!("data/examples/reflex_create_trigger.json"),
    ),
];
