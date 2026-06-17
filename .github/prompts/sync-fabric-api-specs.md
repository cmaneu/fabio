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
6. Before committing, run the mandatory pre-commit validation defined in AGENTS.md (section "Pre-Commit Validation (MANDATORY)"). All steps must pass with zero errors and zero warnings.
7. Write a file called /tmp/pr-body.md describing what was changed and why, referencing the spec commits.

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

## Tool Usage Rules

- You have read-only access to git (status, diff, log, show, rev-parse, ls-files, blame, branch).
- **Under NO circumstance may you run `git add`, `git commit`, or `git push`.** The CI workflow that invokes you handles all staging, committing, branch creation, and PR submission. Your job is to edit files and write /tmp/pr-body.md — nothing more.
- You may run `cargo check`, `cargo clippy`, and `cargo test` to validate your changes.
- Use `read`, `write`, and `edit` tools for file modifications.

Focus on high-impact changes: new endpoints that map to fabio command groups, breaking schema changes, and new required fields.

Follow all constraints and preferences defined in AGENTS.md, in particular the pre-commit validation rules and Windows-first compatibility requirements.
