//! Generator for intent-scoped sub-skills (Layer 2 of the information architecture).
//!
//! Each sub-skill combines **authored judgment** (a JSON file in `data/skills/`,
//! auto-registered by `build.rs`) with a **generated command index** pulled from
//! `commands.json` (the source of truth). This realizes the division of labor:
//! prose carries judgment (when to use, gotchas, safety, routing); the command
//! table is mechanically derived and therefore drift-free.
//!
//! The generated Markdown lives at `.agents/skills/fabio-<family>/SKILL.md`.
//! Regenerate with `cargo test generate_subskills -- --ignored`; a drift test
//! (`subskills_match_generated`) fails in CI if the committed files are stale.

use serde_json::Value;
use std::fmt::Write as _;

include!(concat!(env!("OUT_DIR"), "/skills.rs"));

/// Directory name for a sub-skill family (e.g. `lakehouse` -> `fabio-lakehouse`).
fn subskill_dir_name(family: &str) -> String {
    format!("fabio-{family}")
}

/// Render a bullet list from a JSON array field, or an empty string if absent.
fn render_bullets(value: Option<&Value>) -> String {
    let Some(arr) = value.and_then(Value::as_array) else {
        return String::new();
    };
    arr.iter()
        .filter_map(Value::as_str)
        .fold(String::new(), |mut out, s| {
            let _ = writeln!(out, "- {s}");
            out
        })
}

/// Escape a Markdown table cell (pipes would break the column layout).
fn escape_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// Build the generated command-index section for a sub-skill from `commands.json`.
fn render_command_index(command_groups: &[&str], commands: &Value) -> String {
    let mut out = String::from(
        "## Command index\n\nGenerated from fabio's command schema. For full flag details use `fabio context agent --group <group>` or `fabio context describe <group> <command>`.\n\n",
    );
    for group in command_groups {
        let Some(group_val) = commands.get(*group) else {
            continue;
        };
        let group_desc = group_val
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let _ = writeln!(out, "### fabio {group}");
        if !group_desc.is_empty() {
            let _ = writeln!(out, "{group_desc}\n");
        }
        let Some(subcommands) = group_val.get("subcommands").and_then(Value::as_object) else {
            out.push('\n');
            continue;
        };
        out.push_str("| Command | Mutates | Description |\n|---|---|---|\n");
        let mut names: Vec<&String> = subcommands.keys().collect();
        names.sort();
        for name in names {
            let sub = &subcommands[name];
            let desc = sub.get("description").and_then(Value::as_str).unwrap_or("");
            let mutates = sub.get("mutates").and_then(Value::as_bool).unwrap_or(false);
            let _ = writeln!(
                out,
                "| `fabio {group} {name}` | {} | {} |",
                if mutates { "yes" } else { "no" },
                escape_cell(desc)
            );
        }
        out.push('\n');
    }
    out
}

/// Generate the full SKILL.md Markdown for one sub-skill family.
pub(super) fn generate_markdown(family_value: &Value, commands: &Value) -> String {
    let family = family_value
        .get("family")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let title = family_value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or(family);
    let description = family_value
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let command_groups: Vec<&str> = family_value
        .get("command_groups")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let name = subskill_dir_name(family);

    let mut md = String::new();
    // Frontmatter — folded block scalar avoids quote-escaping issues.
    md.push_str("---\n");
    let _ = writeln!(md, "name: {name}");
    md.push_str("description: >-\n");
    let _ = writeln!(md, "  {description}");
    md.push_str("license: MIT\n");
    md.push_str("---\n\n");

    let _ = writeln!(md, "# {name} — {title}\n");
    md.push_str(
        "> **Generated file — do not edit by hand.** This intent-scoped sub-skill of the `fabio` \
         skill is generated from fabio's command schema plus authored judgment. Regenerate with \
         `cargo test generate_subskills -- --ignored`. For install, auth, output envelope, global \
         flags, and agent-safety rules, see the root `fabio` skill.\n\n",
    );
    md.push_str(
        "> **Prefer runtime introspection.** This index is a snapshot; the installed binary is \
         always authoritative. Use `fabio context agent --group <group>` and \
         `fabio context describe <group> <command>` for exact flags and output shapes.\n\n",
    );

    let when = render_bullets(family_value.get("when_to_use"));
    if !when.is_empty() {
        md.push_str("## When to use\n");
        md.push_str(&when);
        md.push('\n');
    }

    let when_not = render_bullets(family_value.get("when_not_to_use"));
    if !when_not.is_empty() {
        md.push_str("## When NOT to use (route elsewhere)\n");
        md.push_str(&when_not);
        md.push('\n');
    }

    md.push_str(&render_command_index(&command_groups, commands));

    let gotchas = render_bullets(family_value.get("key_gotchas"));
    if !gotchas.is_empty() {
        md.push_str("## Key gotchas\n");
        md.push_str(&gotchas);
        md.push('\n');
    }

    let safety = render_bullets(family_value.get("safety"));
    if !safety.is_empty() {
        md.push_str("## Safety\n");
        md.push_str(&safety);
        md.push('\n');
    }

    let see_also = render_bullets(family_value.get("see_also"));
    if !see_also.is_empty() {
        md.push_str("## See also\n");
        md.push_str(&see_also);
    }

    // Normalize to a single trailing newline.
    format!("{}\n", md.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands() -> Value {
        super::super::agent_commands_schema()
    }

    #[test]
    fn all_skill_families_are_valid_json_with_required_fields() {
        for (name, content) in SKILLS {
            let val: Value = serde_json::from_str(content)
                .unwrap_or_else(|e| panic!("skill family '{name}' invalid JSON: {e}"));
            assert!(
                val.get("family").is_some(),
                "skill family '{name}' must have a 'family' field"
            );
            assert!(
                val.get("description").is_some(),
                "skill family '{name}' must have a 'description' field"
            );
            assert!(
                val.get("command_groups")
                    .and_then(Value::as_array)
                    .is_some(),
                "skill family '{name}' must have a 'command_groups' array"
            );
        }
    }

    #[test]
    fn skill_family_command_groups_exist_in_schema() {
        let cmds = commands();
        for (name, content) in SKILLS {
            let val: Value = serde_json::from_str(content).unwrap();
            for group in val.get("command_groups").and_then(Value::as_array).unwrap() {
                let group = group.as_str().unwrap();
                assert!(
                    cmds.get(group).is_some(),
                    "skill family '{name}' references unknown command group '{group}'"
                );
            }
        }
    }

    #[test]
    fn generated_markdown_has_frontmatter_and_index() {
        let cmds = commands();
        let (_, content) = SKILLS
            .iter()
            .find(|(n, _)| *n == "lakehouse")
            .expect("lakehouse family exists");
        let val: Value = serde_json::from_str(content).unwrap();
        let md = generate_markdown(&val, &cmds);
        assert!(md.starts_with("---\nname: fabio-lakehouse\n"));
        assert!(md.contains("## Command index"));
        assert!(md.contains("`fabio lakehouse create`"));
        assert!(md.contains("## When NOT to use"));
    }

    /// Drift detection: committed sub-skill files must match generator output.
    #[test]
    fn subskills_match_generated() {
        let cmds = commands();
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".agents/skills");
        let mut stale = Vec::new();
        for (name, content) in SKILLS {
            let val: Value = serde_json::from_str(content).unwrap();
            let family = val.get("family").and_then(Value::as_str).unwrap_or(name);
            let expected = generate_markdown(&val, &cmds);
            let path = base.join(subskill_dir_name(family)).join("SKILL.md");
            match std::fs::read_to_string(&path) {
                Ok(actual) if actual == expected => {}
                Ok(_) => stale.push(format!("{} (out of date)", path.display())),
                Err(_) => stale.push(format!("{} (missing)", path.display())),
            }
        }
        assert!(
            stale.is_empty(),
            "Generated sub-skills are stale or missing:\n  {}\n\
             Run `cargo test generate_subskills -- --ignored` to regenerate.",
            stale.join("\n  ")
        );
    }

    /// Regenerate the sub-skill Markdown files. Run with:
    /// `cargo test generate_subskills -- --ignored`
    #[test]
    #[ignore = "writes sub-skill SKILL.md files to disk — run manually after changing commands or skill families"]
    fn generate_subskills() {
        let cmds = commands();
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".agents/skills");
        for (name, content) in SKILLS {
            let val: Value = serde_json::from_str(content).unwrap();
            let family = val.get("family").and_then(Value::as_str).unwrap_or(name);
            let md = generate_markdown(&val, &cmds);
            let dir = base.join(subskill_dir_name(family));
            std::fs::create_dir_all(&dir).expect("create sub-skill dir");
            let path = dir.join("SKILL.md");
            std::fs::write(&path, md).expect("write sub-skill SKILL.md");
            println!("Wrote {}", path.display());
        }
    }
}
