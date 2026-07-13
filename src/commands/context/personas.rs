//! Persona (orchestrator) routing guides for AI agents.
//!
//! Personas are thin, high-level routers that map a request type to the fabio
//! command groups, workflows, and best-practices an agent should use. They hold
//! no implementation depth — they delegate to Layer 3 mechanics (`context agent`,
//! `context workflow`, `context best-practices`). Authored once as JSON data and
//! auto-registered by `build.rs` (like workflows and best-practices).

use serde_json::{Value, json};

use crate::cli::Cli;
use crate::output;

use super::find_entry;

pub(super) fn execute(cli: &Cli, name: &str) {
    let normalized = name.to_lowercase().replace(['-', '_'], "");
    if let Some(content) = find_entry(PERSONAS, &normalized) {
        let val: Value =
            serde_json::from_str(content).unwrap_or_else(|_| json!({"content": content}));
        output::render_object(cli, &val, "name");
    } else {
        let available: Vec<&str> = PERSONAS.iter().map(|(name, _)| *name).collect();
        let result = json!({
            "error": format!("No persona found for '{name}'"),
            "available_personas": available,
            "hint": "Use 'fabio context list' to see all available personas"
        });
        output::render_object(cli, &result, "error");
    }
}

pub(super) fn list_names() -> Vec<&'static str> {
    PERSONAS.iter().map(|(name, _)| *name).collect()
}

pub(super) const fn entries() -> &'static [(&'static str, &'static str)] {
    PERSONAS
}

include!(concat!(env!("OUT_DIR"), "/personas.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_persona_entries_are_valid_json() {
        for (name, content) in PERSONAS {
            let val: Result<serde_json::Value, _> = serde_json::from_str(content);
            assert!(
                val.is_ok(),
                "Persona '{name}' contains invalid JSON: {}",
                val.unwrap_err()
            );
        }
    }

    #[test]
    fn all_persona_entries_have_required_fields() {
        for (name, content) in PERSONAS {
            let val: serde_json::Value = serde_json::from_str(content).unwrap();
            assert!(
                val.get("name").is_some(),
                "Persona '{name}' must have a 'name' field"
            );
            assert!(
                val.get("description").is_some(),
                "Persona '{name}' must have a 'description' field for discoverability"
            );
            assert!(
                val.get("delegates_to").is_some(),
                "Persona '{name}' must have a 'delegates_to' field (the routing table)"
            );
        }
    }

    #[test]
    fn personas_is_non_empty() {
        assert!(
            !PERSONAS.is_empty(),
            "PERSONAS should have at least one entry"
        );
    }
}
