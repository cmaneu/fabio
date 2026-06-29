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

pub(super) const fn entries() -> &'static [(&'static str, &'static str)] {
    BEST_PRACTICES
}

include!(concat!(env!("OUT_DIR"), "/best_practices.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_best_practice_entries_are_valid_json() {
        for (name, content) in BEST_PRACTICES {
            let val: Result<serde_json::Value, _> = serde_json::from_str(content);
            assert!(
                val.is_ok(),
                "Best-practice '{name}' contains invalid JSON: {}",
                val.unwrap_err()
            );
        }
    }

    #[test]
    fn all_best_practice_entries_have_required_fields() {
        for (name, content) in BEST_PRACTICES {
            let val: serde_json::Value = serde_json::from_str(content).unwrap();
            assert!(
                val.get("topic").is_some() || val.get("title").is_some(),
                "Best-practice '{name}' must have 'topic' or 'title' field"
            );
            assert!(
                val.get("summary").is_some(),
                "Best-practice '{name}' must have a 'summary' field for discoverability"
            );
        }
    }

    #[test]
    fn best_practices_is_non_empty() {
        assert!(
            !BEST_PRACTICES.is_empty(),
            "BEST_PRACTICES should have at least one entry"
        );
    }
}
