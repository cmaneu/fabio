//! Best-practices guidance for AI agents using fabio.

use serde_json::{Value, json};

use crate::cli::Cli;
use crate::output;

use super::find_entry;

pub(super) fn execute(cli: &Cli, topic: &str) {
    let normalized = topic.to_lowercase().replace(['-', '_'], "");
    if let Some(content) = find_entry(BEST_PRACTICES, &normalized) {
        let val: Value =
            serde_json::from_str(content).unwrap_or_else(|_| json!({"content": content}));
        output::render_object(cli, &val, "topic");
    } else {
        let available: Vec<&str> = BEST_PRACTICES.iter().map(|(name, _)| *name).collect();
        let result = json!({
            "error": format!("No best-practices topic found for '{topic}'"),
            "available_topics": available,
            "hint": "Use 'fabio context list' to see all available topics"
        });
        output::render_object(cli, &result, "error");
    }
}

pub(super) fn list_names() -> Vec<&'static str> {
    BEST_PRACTICES.iter().map(|(name, _)| *name).collect()
}

const BEST_PRACTICES: &[(&str, &str)] = &[
    (
        "throttling",
        include_str!("data/best_practices/throttling.json"),
    ),
    ("lro", include_str!("data/best_practices/lro.json")),
    (
        "pagination",
        include_str!("data/best_practices/pagination.json"),
    ),
    (
        "admin-apis",
        include_str!("data/best_practices/admin_apis.json"),
    ),
];
