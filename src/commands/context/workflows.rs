//! Multi-step workflow recipes for AI agents.

use serde_json::{Value, json};

use crate::cli::Cli;
use crate::output;

use super::find_entry;

pub(super) fn execute(cli: &Cli, name: &str) {
    let normalized = name.to_lowercase().replace(['-', '_'], "");
    if let Some(content) = find_entry(WORKFLOWS, &normalized) {
        let val: Value =
            serde_json::from_str(content).unwrap_or_else(|_| json!({"content": content}));
        output::render_object(cli, &val, "name");
    } else {
        let available: Vec<&str> = WORKFLOWS.iter().map(|(name, _)| *name).collect();
        let result = json!({
            "error": format!("No workflow found for '{name}'"),
            "available_workflows": available,
            "hint": "Use 'fabio context list' to see all available workflows"
        });
        output::render_object(cli, &result, "error");
    }
}

pub(super) fn list_names() -> Vec<&'static str> {
    WORKFLOWS.iter().map(|(name, _)| *name).collect()
}

const WORKFLOWS: &[(&str, &str)] = &[
    (
        "rti-pipeline",
        include_str!("data/workflows/rti_pipeline.json"),
    ),
    (
        "direct-lake-report",
        include_str!("data/workflows/direct_lake_report.json"),
    ),
    (
        "cicd-deploy",
        include_str!("data/workflows/cicd_deploy.json"),
    ),
    (
        "lakehouse-etl",
        include_str!("data/workflows/lakehouse_etl.json"),
    ),
    (
        "data-agent-setup",
        include_str!("data/workflows/data_agent_setup.json"),
    ),
];
