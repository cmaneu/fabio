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
5. Enrich the runtime agent knowledge (see "Agent Knowledge Enrichment" below).
6. Before committing, run the mandatory pre-commit validation defined in AGENTS.md (section "Pre-Commit Validation (MANDATORY)"). All steps must pass with zero errors and zero warnings.
7. After all code changes, regenerate `commands.json`:
   ```bash
   cargo test --bin fabio generate_agent_schema -- --include-ignored
   ```
8. Write a file called /tmp/pr-body.md describing what was changed and why, referencing the spec commits.

## Agent Knowledge Enrichment

fabio's agent discoverability is entirely runtime-based. There are NO separate documentation files to maintain (no COMMANDS.md, no EXAMPLES.md). All agent-facing knowledge lives in:

- `src/commands/context/data/agent/commands.json` — Auto-generated command schema with manually-preserved `examples` arrays
- `src/commands/context/data/examples/*.json` — Output shape examples (registered in `examples.rs`)
- `src/commands/context/data/schemas/*.json` — Item definition schemas (registered in `schemas.rs`)
- `src/commands/context/data/workflows/*.json` — Multi-step workflow recipes (registered in `workflows.rs`)
- `src/commands/context/data/best_practices/*.json` — Operational guidance (registered in `best_practices.rs`)

### commands.json — Command Examples (MANDATORY for new commands)

After adding a new subcommand, add practical CLI examples to its `examples` field in `commands.json`. These are preserved across regeneration. Add 1-3 examples per subcommand showing non-obvious flag usage:

```json
"my-new-command": {
  "description": "...",
  "flags": {...},
  "mutates": true,
  "returns": "object",
  "examples": [
    "fabio <group> my-new-command --workspace $WS --id $ID --some-flag Value"
  ]
}
```

After adding examples, regenerate to pick up structural changes:
```bash
cargo test --bin fabio generate_agent_schema -- --include-ignored
```

### Output Shape Examples (MANDATORY for non-obvious responses)

If the new command returns non-trivial JSON (nested objects, aggregated results, non-standard envelope), add a file in `data/examples/` and register it in `examples.rs`:

```json
{
  "command": "fabio <group> <cmd> --workspace $WS ...",
  "description": "What this shows",
  "response": {"data": {...}},
  "notes": "Important agent-relevant notes",
  "query_examples": [
    {"query": "data.id", "description": "Extract the ID"}
  ]
}
```

Standard CRUD responses (create returns object with id/displayName, list returns `{"data":[...],"count":N}`, delete returns `{"status":"deleted","id":"..."}`) do NOT need output examples — agents already know these patterns.

### Item Definition Schemas (for new item types)

If the spec introduces a new item type, add `data/schemas/<type>.json` and register in `schemas.rs`. Structure: `type`, `description`, `create_command`, `definition_format`, `definition_parts`, `creation_body_template`, `flags`, `notes`, `related_commands`.

### Workflow Recipes (for new multi-step flows)

If spec changes reveal a new multi-step workflow (create-configure-publish sequence, new integration between item types), add `data/workflows/<name>.json` and register in `workflows.rs`. Structure: `name`, `description`, `prerequisites`, `steps` (numbered with `command` + `description`), `tips`.

### Best Practices (for new operational patterns)

If the spec reveals new gotchas (required field ordering, beta/preview flags, new throttling behavior, PascalCase requirements, non-standard response keys), update `data/best_practices/<topic>.json`.

## Tests

### Unit tests
When the spec provides example request/response JSON, add unit tests in the relevant `src/commands/*.rs` `#[cfg(test)]` module that verify serialization/deserialization against those examples. Cover edge cases (optional fields, enum variants, default values).

### E2E tests
Add or update integration tests in `tests/e2e_*.rs` that exercise new or changed endpoints. Include `--dry-run` tests verifying the constructed request body matches the spec format.

## AGENTS.md API Behaviors

Document non-obvious API behaviors discovered from the spec in the appropriate "API Behaviors Discovered" section in AGENTS.md:
- Required vs optional fields that differ from intuition
- Non-standard response keys (not `"value"`)
- PascalCase vs camelCase requirements
- Query parameter requirements (`?beta=true`, `?preview=true`)
- Discriminated union patterns in request bodies
- Fields the server auto-adds or strips
- Error codes and their meaning

## README.md

Update only if the spec reveals major new capabilities that change the project's feature description (new item type categories, new authentication methods, new deployment capabilities).

## Decision Criteria

| Spec change type | Action required |
|-----------------|----------------|
| New endpoint for existing item type | Add subcommand + examples in commands.json |
| New item type | Add full command module + schema + examples + update AGENTS.md |
| Modified request schema (new required field) | Update command handler + tests + AGENTS.md behavior |
| Modified response schema | Update output handling + add/update output example |
| New enum values | Update clap `possible_values` + commands.json |
| New LRO/async pattern | Update handler + best-practices if novel |
| New beta/preview flag requirement | Add query param + document in AGENTS.md |
| Deprecated field | Remove from request body + add note to AGENTS.md |

## Tool Usage Rules

- You have read-only access to git (status, diff, log, show, rev-parse, ls-files, blame, branch).
- **Under NO circumstance may you run `git add`, `git commit`, or `git push`.** The CI workflow that invokes you handles all staging, committing, branch creation, and PR submission. Your job is to edit files and write /tmp/pr-body.md — nothing more.
- You may run `cargo fmt`, `cargo check`, `cargo clippy`, `cargo build`, and `cargo test` to validate your changes.
- Use `read`, `write`, and `edit` tools for file modifications.

Focus on high-impact changes: new endpoints that map to fabio command groups, breaking schema changes, and new required fields.

Follow all constraints and preferences defined in AGENTS.md, in particular the pre-commit validation rules and Windows-first compatibility requirements.
