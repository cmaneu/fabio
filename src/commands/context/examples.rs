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
        "workspace/list",
        include_str!("data/examples/workspace_list.json"),
    ),
    ("item/list", include_str!("data/examples/item_list.json")),
    (
        "deploy/plan",
        include_str!("data/examples/deploy_plan.json"),
    ),
];
