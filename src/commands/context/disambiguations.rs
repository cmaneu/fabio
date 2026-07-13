//! Terminology disambiguation tables for overloaded Fabric terms.
//!
//! Many Fabric terms mean different things in different workloads (e.g.
//! "materialized view" in Spark vs KQL vs Warehouse). These tables resolve a
//! term to the concrete artifact + the fabio command group that handles it, so
//! agents route to the right place. Authored as JSON data, auto-registered by
//! `build.rs`.

use serde_json::{Value, json};

use crate::cli::Cli;
use crate::output;

use super::find_entry;

pub(super) fn execute(cli: &Cli, term: &str) {
    let normalized = term.to_lowercase().replace(['-', '_', ' '], "");
    if let Some(content) = find_entry(DISAMBIGUATIONS, &normalized) {
        let val: Value =
            serde_json::from_str(content).unwrap_or_else(|_| json!({"content": content}));
        output::render_object(cli, &val, "term");
    } else {
        let available: Vec<&str> = DISAMBIGUATIONS.iter().map(|(name, _)| *name).collect();
        let result = json!({
            "error": format!("No disambiguation table found for '{term}'"),
            "available_terms": available,
            "hint": "Use 'fabio context list' to see all disambiguation terms"
        });
        output::render_object(cli, &result, "error");
    }
}

pub(super) fn list_names() -> Vec<&'static str> {
    DISAMBIGUATIONS.iter().map(|(name, _)| *name).collect()
}

pub(super) const fn entries() -> &'static [(&'static str, &'static str)] {
    DISAMBIGUATIONS
}

include!(concat!(env!("OUT_DIR"), "/disambiguations.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_disambiguation_entries_are_valid_json() {
        for (name, content) in DISAMBIGUATIONS {
            let val: Result<serde_json::Value, _> = serde_json::from_str(content);
            assert!(
                val.is_ok(),
                "Disambiguation '{name}' contains invalid JSON: {}",
                val.unwrap_err()
            );
        }
    }

    #[test]
    fn all_disambiguation_entries_have_required_fields() {
        for (name, content) in DISAMBIGUATIONS {
            let val: serde_json::Value = serde_json::from_str(content).unwrap();
            assert!(
                val.get("term").is_some(),
                "Disambiguation '{name}' must have a 'term' field"
            );
            assert!(
                val.get("summary").is_some(),
                "Disambiguation '{name}' must have a 'summary' field for discoverability"
            );
            assert!(
                val.get("meanings").is_some(),
                "Disambiguation '{name}' must have a 'meanings' field"
            );
        }
    }

    #[test]
    fn disambiguations_is_non_empty() {
        assert!(
            !DISAMBIGUATIONS.is_empty(),
            "DISAMBIGUATIONS should have at least one entry"
        );
    }
}
