You are working on fabio, a Rust CLI for Microsoft Fabric.

The Microsoft Fabric REST API specs (github.com/microsoft/fabric-rest-api-specs) have been updated.

Here are the commits since our last sync:

${SPEC_COMMITS}

Here is a detailed summary of file changes:

${SPEC_CHANGES_SUMMARY}

The full diff is available at /tmp/spec-changes.diff and the full spec repo is at ./fabric-rest-api-specs/.

Your task:
1. Analyze the spec changes to identify new endpoints, modified request/response schemas, new API versions, or deprecated fields.
2. Compare against the current fabio implementation in src/commands/ and src/client.rs.
3. Make targeted improvements to fabio: add new subcommands, update existing request/response handling, fix field names, or add support for new parameters.
4. Update relevant tests in tests/ if you add or modify commands.
5. Harvest examples and behavioral details from the spec changes (see "Examples & Documentation Enrichment" below).
6. Update `fabio context` data files when spec changes introduce new item types, workflows, or best-practice patterns (see "Context Knowledge Updates" below).
7. Before committing, run the mandatory pre-commit validation defined in AGENTS.md (section "Pre-Commit Validation (MANDATORY)"). All steps must pass with zero errors and zero warnings.
8. Write a file called /tmp/pr-body.md describing what was changed and why, referencing the spec commits.

## Examples & Documentation Enrichment

The Fabric API specs contain `x-ms-examples`, example request/response bodies, enum value lists, and behavioral annotations (required fields, default values, validation rules, error codes). These are high-value artifacts — extract and propagate them:

### Tests
- **Unit tests**: When the spec provides example request/response JSON, add unit tests in the relevant `src/commands/*.rs` `#[cfg(test)]` module that verify serialization/deserialization against those examples. Cover edge cases revealed by the spec (optional fields, enum variants, default values).
- **E2E tests**: Add or update integration tests in `tests/e2e_*.rs` that exercise the new or changed endpoints. Use spec examples as reference for expected request bodies and response shapes. Include `--dry-run` tests that verify the constructed request body matches the spec's example format.

### Documentation
- **EXAMPLES.md**: Add practical usage examples for new or changed commands, using the spec's example payloads as realistic `--content` or `--file` values. Show both the fabio command and the expected output shape.
- **README.md**: Update command listings and feature descriptions if the spec reveals new capabilities.
- **COMMANDS.md**: Update flag/option signatures for modified commands.

### AGENTS.md API Behaviors
- **Document discovered behaviors**: When the spec reveals non-obvious API behaviors — required field ordering, enum values, default values, error codes, LRO patterns, pagination keys, response envelope differences, or undocumented constraints — add them to the appropriate "API Behaviors Discovered" section in AGENTS.md. This is critical institutional knowledge that prevents future regressions.
- **Look for**: required vs optional fields that differ from intuition, non-standard response keys (not `"value"`), PascalCase vs camelCase requirements, query parameter requirements (`?beta=true`, `?preview=true`), discriminated union patterns in request bodies, and fields the server auto-adds or strips.

## Context Knowledge Updates

The `fabio context` system provides structured knowledge for AI agents consuming the CLI. When spec changes introduce new capabilities, update the corresponding context data files so agents can discover and use them correctly.

### `src/commands/context/agent.rs` — Command Schema

If you add a new subcommand or modify flags/options on an existing command, update the machine-readable schema in `agent.rs`. Each command group entry lists subcommands with their flags, types, mutability, and descriptions. Agents rely on this for command discovery.

### `src/commands/context/data/schemas/` — Item Definition Schemas

If the spec introduces a **new item type** or changes the definition format (part paths, creation body, required fields) of an existing item type, add or update the corresponding JSON file in `data/schemas/`. Each schema file describes: `type`, `description`, `create_command`, `definition_format`, `definition_parts`, `creation_body_template`, `flags`, `notes`, and `related_commands`. See `data/schemas/lakehouse.json` for the canonical structure.

### `src/commands/context/data/examples/` — Output Examples

If you add a new command with a non-obvious response shape (nested objects, aggregated results, URL outputs), add a JSON example file in `data/examples/` and register it in `src/commands/context/examples.rs` in the `OUTPUT_EXAMPLES` constant via `include_str!()`. Each example has: `command`, `description`, `response` (representative JSON output), `notes`, and optional `query_examples` (JMESPath snippets for common extractions).

### `src/commands/context/data/workflows/` — Workflow Recipes

If the spec changes reveal a **new multi-step workflow** (e.g., a new item type requiring a create-configure-publish sequence, or a new integration between two item types), add a workflow recipe JSON. Structure: `name`, `description`, `prerequisites`, `steps` (numbered with `command` and `description`), and `tips`. Agents use these to orchestrate complex operations.

### `src/commands/context/data/best_practices/` — Best Practices

If the spec reveals new operational patterns (new pagination behavior, new LRO quirk, new beta/preview flag requirement, new throttling guidance, new required query parameters), add or update the relevant best-practice JSON file. Structure: `topic`, `title`, `summary`, plus domain-specific guidance sections.

### Decision Criteria

Update context files when ANY of these apply:
- A new item type is implemented → add `data/schemas/{type}.json` + update `schemas.rs`
- A new command has non-trivial output → add `data/examples/{cmd}.json` + update `examples.rs`
- A new multi-step creation/configuration flow is needed → add `data/workflows/{flow}.json` + update `workflows.rs`
- A spec change introduces a gotcha (required field ordering, beta flag, enum constraint) → update relevant `data/best_practices/{topic}.json`
- Any new subcommand or flag is added → update `agent.rs` command schema
- An API behavioral change affects how a command/subcommand works (new required fields, changed response shape, modified LRO pattern, new error codes, renamed parameters) → update the relevant schema, example, or best-practice file so agents use the updated behavior correctly

## Tool Usage Rules

- You have read-only access to git (status, diff, log, show, rev-parse, ls-files, blame, branch).
- **Under NO circumstance may you run `git add`, `git commit`, or `git push`.** The CI workflow that invokes you handles all staging, committing, branch creation, and PR submission. Your job is to edit files and write /tmp/pr-body.md — nothing more.
- You may run `cargo fmt`, `cargo check`, `cargo clippy`, `cargo build`, and `cargo test` to validate your changes.
- Use `read`, `write`, and `edit` tools for file modifications.

Focus on high-impact changes: new endpoints that map to fabio command groups, breaking schema changes, and new required fields.

Follow all constraints and preferences defined in AGENTS.md, in particular the pre-commit validation rules and Windows-first compatibility requirements.
