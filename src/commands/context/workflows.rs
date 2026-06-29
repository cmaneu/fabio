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

pub(super) const fn entries() -> &'static [(&'static str, &'static str)] {
    WORKFLOWS
}

include!(concat!(env!("OUT_DIR"), "/workflows.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_workflow_entries_are_valid_json() {
        for (name, content) in WORKFLOWS {
            let val: Result<serde_json::Value, _> = serde_json::from_str(content);
            assert!(
                val.is_ok(),
                "Workflow '{name}' contains invalid JSON: {}",
                val.unwrap_err()
            );
        }
    }

    #[test]
    fn all_workflow_entries_have_required_fields() {
        for (name, content) in WORKFLOWS {
            let val: serde_json::Value = serde_json::from_str(content).unwrap();
            assert!(
                val.get("name").is_some(),
                "Workflow '{name}' must have a 'name' field"
            );
            assert!(
                val.get("steps").is_some(),
                "Workflow '{name}' must have a 'steps' field"
            );
            assert!(
                val.get("description").is_some(),
                "Workflow '{name}' must have a 'description' field for discoverability"
            );
        }
    }

    #[test]
    fn workflows_is_non_empty() {
        assert!(
            !WORKFLOWS.is_empty(),
            "WORKFLOWS should have at least one entry"
        );
    }
}
