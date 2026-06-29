# Fabio CLI - Session Context

## Goal
- Design and implement an agent-native CLI (`fabio`) to manage Microsoft Fabric artifacts and data, inspired by AWS/gcloud/Azure CLI principles, with structured JSON output, composability via stdin/stdout, and machine-readable errors.

## Agent-Native CLI Principles

Fabio must always respect these 10 principles for agent-native CLIs:
https://trevinsays.com/p/10-principles-for-agent-native-clis

1. **Non-interactive by default** — No prompts; all inputs via flags/env/files. Non-TTY must fail fast.
2. **Structured, parseable output** — `--json` on every command; stdout = data, stderr = diagnostics; stable exit codes.
3. **Errors that teach and enumerate** — Errors include valid enum values, corrected command examples, and machine-readable codes with hints.
4. **Safe retries and explicit mutation boundaries** — `--dry-run` for mutations; idempotency-safe; stable returned IDs.
5. **Bounded responses** — `--limit` for list commands; default to concise output; truncation metadata in envelope.
6. **Cross-CLI vocabulary consistency** — Canonical agent verbs: `list`, `show`, `create`, `delete`, `copy`, `move`.
7. **Three-layer introspection** — `fabio context agent` provides machine-readable command schema (flags, types, mutability, examples). `fabio context` provides semantic knowledge (item definition schemas, workflow recipes, output examples, best-practices guidance).
8. **Async-aware execution** — `--wait` for async jobs; local job ledger (`fabio jobs list/get/prune`); status polling.
9. **Persistent identity through profiles** — Named profiles (`fabio profile save/use/list/show/delete`); `--profile` flag.
10. **Two-way I/O** — Feedback channel (`fabio feedback send/list`); artifact delivery via stdout/file.

## Constraints & Preferences
- **Windows-first compatibility** — All code must work on Windows. Use `Path::new().join()` (never hardcoded `/` for filesystem paths), `dirs::home_dir()` (never manual `HOME`/`USERPROFILE`), `.lines()` for text parsing (handles CRLF), no Unix-specific APIs. `.gitattributes` enforces LF line endings.
- **Throttling reduction** — Reduce the likelihood of API throttling by:
  - Use bulk and batch operations when available (e.g., `item bulk-create`, `item bulk-delete`, workspace role batch-assign, domain batch-assign).
  - Prefer list APIs over repeated single-resource requests (e.g., use a single list call + client-side filter rather than N individual show calls).
- CLI designed for AI agents first (structured output, no interactive prompts, explicit params)
- JSON output by default with `--output json|table|plain` flag
- Composable: manage inputs and produce outputs for piping
- Machine-readable error codes in structured JSON envelope
- Rust (edition 2024, rust-version 1.96), uses clap derive, tokio, reqwest, azure_identity, serde, serde_yaml, comfy-table, thiserror/anyhow
- Linting: clippy pedantic+nursery (zero warnings), rustfmt
- CI: GitHub Actions (cargo fmt, clippy, test, build release) on ubuntu/macos/windows
- Installable via `cargo install --git https://github.com/iemejia/fabio.git`
- **Dependency version freshness** — When introducing a new Cargo dependency or a new GitHub Action, always validate that you are using the most recent available and compatible version. Check crates.io for Rust crates and the action's repository releases/tags for GitHub Actions. Do NOT copy outdated versions from examples or memory — verify against the source of truth before adding. Additionally, reject any dependency with an incompatible license (GPL, LGPL, AGPL, SSPL, or any other copyleft license that would impose restrictions on the project). Only permissive licenses (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode-3.0, etc.) are acceptable.
- **GitHub Actions pinning** — ALL GitHub Actions in `.github/workflows/*.yml` MUST be pinned to their full commit SHA with the version in a trailing comment. Format: `uses: owner/action@<40-char-sha> # v<major>` (or `# v<major>.<minor>.<patch>` for non-major tags). NEVER use floating tag references like `@v7` or `@stable`. This prevents supply-chain attacks where a tag is force-pushed to a compromised commit. When updating an action, always verify the new SHA matches the expected release tag from the action's repository.
- **Modern Rust idioms (MANDATORY)** — All code MUST leverage features available in the declared `rust-version` (currently 1.96). Do NOT write code using older patterns when a modern equivalent exists. When the MSRV is bumped, audit and migrate existing code. Key idioms to prefer:
  - `str::floor_char_boundary()` for safe string truncation (never raw `&s[..n]` on user/API text)
  - Let chains (`if let Some(x) = opt && condition { ... }`) instead of nested `if let` + `if`
  - `Option::is_none_or(|v| cond)` instead of `opt.is_none() || opt == Some(x)` or `opt.map_or(true, ...)`
  - `Option::is_some_and(|v| cond)` instead of `matches!(opt, Some(x) if cond)` or `opt.map_or(false, ...)`
  - `Duration::from_mins()` / `from_hours()` instead of `from_secs(N * 60)`
  - `std::io::read_to_string(reader)` instead of `let mut buf = String::new(); reader.read_to_string(&mut buf)`
  - `Vec::extract_if()` when you need both the removed elements and the remainder
  - `Value::from(x)` instead of `Value::String(x.to_string())` for `&str` values (canonical serde_json idiom)
  - `x.to_string()` instead of `format!("{x}")` for single-value Display formatting
  - `eq_ignore_ascii_case()` instead of `a.to_lowercase() == b.to_lowercase()` (allocation-free)
  - `HashSet`/`BTreeSet` for membership tests instead of `Vec::contains` or `.iter().any()` when the collection is checked multiple times
  - `const fn` for pure functions returning static data (enables compile-time evaluation)
  - `#[inline]` on small, hot-path functions called across module boundaries

## Irreversible Operations & Agent Safety (MANDATORY)

Fabio is agent-first. AI agents consume structured output and may automatically retry failed commands. When a command performs an irreversible or destructive operation, you MUST implement safety guardrails so agents are explicitly warned before proceeding.

### Rules for new commands or features:

1. **Identify irreversible operations** — Any operation that deletes data, overwrites definitions without backup, or cannot be undone. Examples: item deletion, `--hard-delete`, `--delete-orphans`, `--force-all` (overwrites all definitions), `updateDefinition` (replaces content permanently).

2. **Use `FabioError::with_hint()` for safety-bypass flags** — When an error or guard blocks execution and the hint suggests a flag that bypasses the safety check (e.g., `--force`, `--hard-delete`, `--allow-delete-types`), always use `with_hint()`. The hint text triggers the agent safety notice automatically when an AI agent is detected (`src/agent.rs`).

3. **Dangerous flags must be in `DANGEROUS_FLAGS`** — If you add a new safety-bypass flag, add it to the `DANGEROUS_FLAGS` array in `src/agent.rs`. This ensures the agent safety notice fires when the flag is suggested in an error hint.

4. **Add `"destructive": true/false` to batch output** — For commands that produce a plan or summary of multiple actions (like `deploy plan/apply`), include a `"destructive"` boolean field in the structured output. Set to `true` when the operation includes deletions, overwrites, or other irreversible actions. Agents use this field to decide whether to ask the human for confirmation.

5. **Protected types require explicit opt-in** — Data-bearing item types (Lakehouse, Warehouse, SQLDatabase, Eventhouse, KQLDatabase) require `--allow-delete-types` for deletion. If you add support for a new data-bearing item type, add it to `PROTECTED_DELETE_TYPES` in `src/commands/deploy/mod.rs`.

6. **Warn on force/override modes** — When `--force-all`, `--force`, or similar override flags are active, emit a warning in the output explaining the irreversibility. This helps agents surface the risk to the human.

7. **Never add interactive prompts** — Fabio is non-interactive (Principle 1). Do NOT add `y/N` prompts or `--auto-approve` flags. Instead, use structured output signals (`"destructive": true`, warnings, `agentNotice`) that agents can programmatically evaluate.

### How agent safety notices work:

When ALL of the following conditions are true, the error output includes an `agentNotice` field:
1. The error has a `hint` field
2. The hint text contains a flag from `DANGEROUS_FLAGS` (e.g., `--force`, `--hard-delete`)
3. An AI agent is detected via environment variables (see `AGENT_ENV_VARS` in `src/agent.rs`)

The notice warns the agent: *"do not retry with the safety-bypass flag suggested above unless the user has explicitly approved it."*

### Example output with agent notice:

```json
{"error":{"code":"INVALID_INPUT","message":"Output directory is not empty: /tmp/export","hint":"Use --overwrite to replace existing content.","agentNotice":"Note for AI agents (Claude Code): do not retry with the safety-bypass flag suggested above unless the user has explicitly approved it. The flag bypasses a safety check and the operation may be irreversible."}}
```

### Example deploy output with destructive field:

```json
{"data":{"status":"dry_run","summary":{"create":1,"delete":3,"skip":2},"destructive":true,"warnings":["--force-all is active: ALL matched items will be overwritten regardless of content changes. This is irreversible."]}}
```

## Command File Structure (MANDATORY)

Any command module that exceeds **1500 lines of code** MUST be refactored into a directory module with one file per subcommand group. Follow the pattern established by `context/`, `deploy/`, and `lakehouse/`:

```
src/commands/<command>/
├── mod.rs          — Subcommand enum, execute() dispatch, shared helpers
├── <subcommand_a>.rs  — Handler for one subcommand (or small cohesive group)
├── <subcommand_b>.rs  — Handler for another subcommand
└── ...
```

**Rules:**
- `mod.rs` contains the `<Command>Command` enum, the `execute()` dispatch function, and any helpers shared across submodules.
- Split by **subcommand**, not by abstract concern. Each file maps directly to one or a small group of related subcommands (e.g., `iceberg.rs` for all iceberg-* subcommands, `sync.rs` for the sync subcommand, `crud.rs` for list/show/create/update/delete).
- Functions called from `execute()` are `pub(super)`. Internal helpers stay private.
- Embedded data files (JSON schemas, templates) go in a `data/` subdirectory within the module.
- When adding new subcommands to an existing directory module, place the handler in the appropriate submodule file — do NOT add it to `mod.rs`.
- When a single-file command grows past 1500 lines, split it proactively rather than waiting for the next feature addition.

**Current directory modules:** `context/` (7 files), `deploy/` (12 files), `lakehouse/` (10 files).

**Scope:** This rule applies only to `src/commands/` source files. E2E test files (`tests/e2e_*.rs`) are NOT subject to the 1500-line limit — a single test file per command group is the preferred structure.

## Pre-Commit Validation (MANDATORY)

Before committing ANY change, you MUST run the following validation steps in order and ensure they all pass with zero errors and zero warnings:

```bash
# 1. Format check (must produce no diffs)
cargo fmt -- --check

# 2. Clippy with all tests and deny warnings (must produce zero warnings)
cargo clippy --tests -- -D warnings

# 3. Run tests (must all pass)
cargo test
```

**Local pre-commit hooks (prek):** The project uses [prek](https://prek.j178.dev) — a fast, Rust-native pre-commit runner configured in `prek.toml`. When installed (`cargo install prek && prek install`), it automatically enforces format and lint checks on every `git commit`. The hooks run: trailing-whitespace fix, EOF fixer, TOML/YAML validation, merge-conflict detection, large-file guard (500KB), gitleaks secret scanning, `cargo fmt -- --check`, and `cargo clippy --tests -- -D warnings`. Tests (`cargo test`) are NOT included in the hook (too slow for interactive commits) — run them manually before pushing.

**Rules:**
- Do NOT commit if any of these steps fail.
- If prek is available, always let the hooks run on commit. If they reject the commit, fix the issues before retrying. Do NOT bypass hooks with `--no-verify`.
- Fix all formatting issues (`cargo fmt` to auto-fix), clippy warnings, and test failures before committing.
- If you add new code, ensure it has no clippy pedantic+nursery warnings.
- If you modify existing tests or add new tests, verify they pass.
- Check for unused imports before committing. Clippy catches these (`unused_imports` lint), but proactively remove any `use` statements you added that are no longer needed after refactoring. Run `cargo clippy --tests -- -D warnings` and fix all `unused import` warnings — do not leave them for a follow-up commit.
- These steps mirror the CI pipeline — if they pass locally, CI will pass. The CI release build is an additional artifact-packaging step, not a correctness gate.

## Pre-Commit Self-Review (MANDATORY)

Before committing, you MUST perform a deep, thoughtful review of ALL changes you are about to commit. This is not a formality — it is a critical quality gate:

1. **Re-read every changed file** — Use `git diff --staged` (or `git diff` if not yet staged) and carefully review each hunk.
2. **Check for issues you may have introduced** — Look for:
   - Logic errors, off-by-one mistakes, or incorrect assumptions
   - Missing error handling or edge cases
   - Copy-paste errors (e.g., wrong variable names, leftover placeholder text)
   - Inconsistencies with existing code patterns and conventions
   - Dead code, unused imports, or debug artifacts left behind
   - Incomplete implementations (TODO comments without corresponding work)
   - Naming inconsistencies (does the new code match the codebase's naming style?)
3. **Verify correctness against the intent** — Does the code actually accomplish what was requested? Are there subtle misunderstandings?
4. **Fix any issues found** — Do NOT commit known problems. Fix them first, then re-run the pre-commit validation steps.

**Rules:**
- Treat this review as if you were reviewing someone else's code — be critical and objective.
- If you find even a minor issue, fix it before committing. Do not leave it for later.
- This step comes AFTER pre-commit validation passes but BEFORE the actual `git commit`.

## Pre-Push Validation (MANDATORY)

Before pushing changes to the remote, you MUST run the cross-compilation check to catch platform-specific issues (Windows/macOS quirks, conditional compilation errors):

```bash
./scripts/cross-check.sh
```

**Rules:**
- Do NOT push if the cross-check script fails.
- Fix any cross-compilation errors (e.g., `cfg(windows)` blocks, platform-specific imports, path handling) before pushing.
- You can target a single platform to iterate faster: `./scripts/cross-check.sh --target windows-x64`
- This catches issues that local clippy/tests miss: Windows-only code paths (`windows-sys`, `windows` crates), macOS Darwin targets, and ARM64 variants.

## Auto-Generated Files (MANDATORY)

The following files are auto-generated from the CLI source of truth. **NEVER edit them manually** — edits will be overwritten on regeneration and drift detection tests will fail in CI.

### Regeneration Commands

After adding, modifying, or removing commands/flags, run ALL of these:

```bash
# 1. Regenerate commands.json (the single source of truth for all agent-facing metadata)
cargo test --bin fabio generate_agent_schema -- --include-ignored

# 2. Verify drift detection passes (these run in cargo test / CI)
cargo test --bin fabio agent_schema_covers
```

### File Inventory

| File | Generated from | Drift test | When to regenerate |
|------|---------------|------------|-------------------|
| `src/commands/context/data/agent/commands.json` | clap metadata | `agent_schema_covers_all_groups`, `agent_schema_covers_all_subcommands` | New command/subcommand/flag added |

### How Drift Detection Works

Each auto-generated file has a corresponding unit test that:
1. Regenerates the expected content in memory (same algorithm as the generator)
2. Reads the committed file from disk
3. Asserts they are identical
4. Fails with a message showing the exact regeneration command

These tests run as part of the standard `cargo test` suite and in CI. If a contributor adds a command but forgets to regenerate, CI will fail with a clear error message.

### One-Liner (Regenerate Everything)

```bash
cargo test --bin fabio generate_agent_schema -- --include-ignored
```

## Documentation Updates (MANDATORY)

When adding new features, commands, or discovering API behaviors, you MUST update the following documentation before committing:

1. **AGENTS.md** — Update these sections as applicable:
   - **Progress > Done**: Add bullet points for new commands/features implemented.
   - **Key Decisions**: Document significant architectural or design choices.
   - **Relevant Files**: Add new source files, test files, or config files.
   - **API Behaviors Discovered**: Append to `.agents/API-BEHAVIORS-DISCOVERED.md` under the appropriate section heading. Do NOT add API behavior documentation to AGENTS.md directly — it was extracted to reduce context size.

2. **`src/commands/context/agent.rs`** — Update the machine-readable command schema so AI agents can discover the new commands (flags, types, mutability, examples).

   **Auto-generation (preferred)**: Run `cargo test generate_agent_schema -- --ignored` to regenerate `commands.json` from clap metadata. This extracts group names, subcommand names, flag names/types/required/descriptions directly from the CLI definition. Semantic annotations (`mutates`, `returns`, `destructive`) are auto-inferred from command naming conventions (e.g., `list*` → read-only + returns list, `delete*` → mutates + destructive + returns void). Only `async` (LRO) and `auth_scope` (per-group) cannot be inferred and must be added manually for new entries that need them.

   **Drift detection**: Two unit tests (`agent_schema_covers_all_groups`, `agent_schema_covers_all_subcommands`) will FAIL if `commands.json` is missing any group or subcommand present in the actual CLI. These tests run as part of `cargo test` and prevent drift from accumulating.

   **NEVER manually edit `commands.json`** — The file at `src/commands/context/data/agent/commands.json` is auto-generated. Manual edits will be overwritten on the next regeneration. All structural data (groups, subcommands, flags, types, descriptions) comes from clap derive annotations in the source code. Only semantic annotations (`mutates`, `returns`, `async`, `destructive`, `auth_scope`) are preserved across regenerations via merge logic.

   **Exact steps when adding a new command or subcommand:**

   ```bash
   # 1. Write the command code with proper clap derive annotations
   #    (doc comments become descriptions, arg types become flag types)

   # 2. Regenerate commands.json from the actual CLI surface
   cargo test generate_agent_schema -- --ignored

   # 3. Add semantic annotations to the NEW entries only.
   #    Open src/commands/context/data/agent/commands.json and find your new
   #    subcommand(s). Add these fields that clap cannot infer:
   #
   #    "mutates": true/false       — does it change state?
   #    "returns": "list|object|void" — what shape is the output?
   #    "async": true               — (optional) is it an LRO?
   #    "destructive": true         — (optional) does it delete data?
   #
   #    For new command GROUPS, also set:
   #    "auth_scope": "fabric|storage|arm|mixed"

   # 4. Verify drift detection passes
   cargo test agent_schema_covers

   # 5. Done — the MCP server, --format mcp, --group, describe, find
   #    all automatically pick up the new commands with zero extra work.
   ```

   **`src/commands/context/`** — If the new feature introduces an item type, add a schema file in `context/data/schemas/`. If it's part of a multi-step workflow, consider adding a workflow recipe in `context/data/workflows/`. If the new feature adds significant query/output patterns, add output examples in `context/data/examples/` so agents understand the response shapes (e.g., new KQL intelligence commands like `list-entities`, `diagnostics`, `deeplink` should have representative output examples).

   **Output examples format** — Each example is a JSON file in `src/commands/context/data/examples/` with the structure: `{"command": "fabio ...", "description": "...", "response": {...}, "notes": "...", "query_examples": [...]}`. After creating the file, it MUST be registered in `src/commands/context/examples.rs` in the `OUTPUT_EXAMPLES` constant using `include_str!()`. Without registration, the example won't be discoverable via `fabio context examples <group> <command>`.

   **Best-practices registration** — Each best-practice is a JSON file in `src/commands/context/data/best_practices/` with required fields: `topic`, `title`, `summary` (for search discoverability), and topic-specific content. **Auto-registered**: the `build.rs` script scans this directory at compile time and generates the registration code. Just drop a `.json` file and rebuild — no manual `include_str!()` wiring needed.

   **Workflow registration** — Each workflow recipe is a JSON file in `src/commands/context/data/workflows/` with required fields: `name`, `description` (for search discoverability), `steps` (array of step objects). **Auto-registered**: same as best-practices — drop a `.json` file and rebuild.

   **Discoverability via `fabio context find`** — Best-practices and workflows are automatically searchable via `fabio context find "<query>"` once registered. The search indexes topic names, descriptions/summaries, and full JSON content. No additional wiring is needed beyond placing the file in the correct directory.

3. **README.md** — Update the user-facing documentation:
   - Add new commands to the command listing/examples.
   - Update feature descriptions if capabilities have expanded.
   - Update installation or usage instructions if relevant.
   - GitHub Actions examples and agent safety documentation live here.

**Rules:**
- Documentation updates are part of the feature — do NOT commit code without corresponding doc updates.
- API behaviors discovered during implementation MUST be captured in `.agents/API-BEHAVIORS-DISCOVERED.md` (this is critical institutional knowledge for future development).
- The `context agent` schema must stay in sync with the actual CLI surface — agents rely on it for discovery.
- The `docs` data files must be updated when new item types or workflows are added — agents rely on them for understanding definition formats and best practices.
- Output examples in `context/data/examples/` SHOULD be added for commands with non-obvious response shapes (e.g., nested objects, aggregated multi-section results, URL outputs) so agents can parse responses correctly.

## Testing Requirements (MANDATORY)

All new features, improvements, and bug fixes MUST have corresponding tests. This is non-negotiable — code without tests is incomplete code. Do NOT submit or consider work done until both unit tests and E2E tests are written, passing, and validated live.

1. **Unit tests** — Add unit tests in the same source file (or a `#[cfg(test)]` module) for:
   - New helper functions, parsers, or data transformations.
   - Edge cases in business logic (error paths, boundary conditions).
   - Output formatting and serialization.

2. **E2E tests** — Add integration tests in `tests/e2e_*.rs` for:
   - New CLI commands (verify structured output, exit codes, `--dry-run` behavior).
   - API interactions (create/read/update/delete lifecycle).
   - Error handling (invalid inputs, permission errors, not-found responses).

3. **Live tenant validation** — You have access to a live Microsoft Fabric tenant for E2E testing:
   - **ALWAYS run your new feature live against the tenant** before considering the work done. Do not skip this step.
   - Use `cargo run -- <command> ...` to execute against the real Fabric APIs and verify the feature works end-to-end.
   - Use the test env vars (`FABIO_TEST_SOURCE_WORKSPACE`, `FABIO_TEST_CAPACITY_ID`, etc.) for workspace/item references.
   - If env vars are not set in your session, use the values from `tests/common/mod.rs` or ask the user.
   - If a feature requires additional Azure resources (VNets, storage accounts, etc.), use `az cli` to create them as part of test setup.
   - Document any API behaviors discovered during testing in the appropriate AGENTS.md section.
   - Clean up any test resources you create (delete items, profiles, etc.) after validation.

**Rules:**
- Do NOT commit new commands or features without corresponding unit AND E2E tests.
- Do NOT consider a feature complete until it has been validated live against the tenant (not just dry-run).
- E2E tests should cover at minimum: `--dry-run` validation, happy-path execution, and error cases (invalid ID, missing permissions).
- Follow existing test patterns in `tests/common/mod.rs` and existing `tests/e2e_*.rs` files.
- Tests must pass locally (`cargo test`) before committing.

## Release Workflow (MANDATORY)

When creating a new release, you MUST complete ALL of the following steps in order before triggering the release. Do NOT skip any step.

### Pre-Release Checklist

Before tagging or publishing, complete these mandatory pre-flight steps:

#### 1. Bump the Version Number

Update `Cargo.toml` with the new version. Development versions use the `-dev` suffix (e.g., `0.25.0-dev`). Release versions MUST NOT contain `-dev` — strip it before tagging:

```bash
# Check current version (should be X.Y.Z-dev during development)
grep '^version' Cargo.toml | head -1

# Update to release version (remove -dev suffix, e.g., 0.25.0-dev → 0.25.0)
sed -i 's/^version = ".*"/version = "0.25.0"/' Cargo.toml
```

Run `cargo check` or `cargo build` to regenerate `Cargo.lock` with the new version.

#### 2. Validate Dependency Freshness

Check that ALL Cargo dependencies are at their latest compatible versions:

```bash
# Show outdated dependencies (requires cargo-outdated or cargo-edit)
cargo outdated --root-deps-only

# Alternative: use `cargo update --dry-run` to see what would be updated
cargo update --dry-run
```

**Rules:**
- Update any dependency that has a newer compatible version available (within semver range).
- For major version bumps, evaluate changelog for breaking changes before updating.
- Verify no dependency has an incompatible license (GPL, LGPL, AGPL, SSPL, or other copyleft). Only permissive licenses (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode-3.0, etc.) are acceptable.
- If updating dependencies, run the full pre-commit validation (`cargo fmt -- --check`, `cargo clippy --tests -- -D warnings`, `cargo test`) after updates.
- Also check GitHub Actions in `.github/workflows/*.yml` — ensure all action versions (e.g., `actions/checkout@v4`) are at their latest release tags.

#### 3. Update Version References in Documentation

Update version-specific strings throughout the repository:

1. **README.md** — Docker image version in usage examples (e.g., `ghcr.io/iemejia/fabio:0.23.0` → `ghcr.io/iemejia/fabio:0.24.0`).
2. **AGENTS.md** — Docker & Devcontainer section version examples.

#### 4. Run Full Validation

```bash
# Format check
cargo fmt -- --check

# Clippy with deny warnings
cargo clippy --tests -- -D warnings

# Run all tests
cargo test

# Cross-compilation check
./scripts/cross-check.sh
```

ALL must pass with zero errors and zero warnings.

#### 5. Commit Cargo.toml AND Cargo.lock Together

Both files MUST be committed in the same commit. `Cargo.lock` is tracked in this repository (binary CLI tool — deterministic builds require a lockfile).

```bash
git add Cargo.toml Cargo.lock README.md AGENTS.md
git status  # verify only intended files are staged
git commit -m "chore: bump version to 0.23.0"
```

**Rules:**
- NEVER tag a release without `Cargo.lock` reflecting the exact dependency tree.
- If you updated dependencies in step 2, `Cargo.lock` will have additional changes — these MUST be committed.
- Verify `git status` is clean (no uncommitted changes) before proceeding to tagging.

#### 6. Generate Release Notes

Use `git-cliff` to produce a grouped commit list, then write curated narrative:

```bash
# Preview unreleased changes (before tagging):
git cliff --unreleased

# For the latest tag (after tagging, most common):
git cliff --latest

# Between two specific tags:
git cliff v0.22.0..v0.23.0
```

The output is grouped by commit type (New Features, Bug Fixes, CI/CD, etc.) with links to commits. This ensures no changes are missed.

**Writing the curated narrative** — Follow the template in `.github/RELEASE_TEMPLATE.md`:

1. **Lead with impact**: Put the most user-visible features first (new item types, major new capabilities)
2. **Group related changes**: Multiple commits that form one feature should be described together
3. **Include examples**: Show command usage for new features
4. **Stats at the end**: Commit count, lines changed, test coverage additions

#### 7. Tag and Trigger the Release

```bash
# Tag the release
git tag v0.23.0

# Push commit and tag
git push
git push origin v0.23.0
```

The CI release workflow (`.github/workflows/release.yml`) is triggered by the tag push and builds 6 binaries + Docker image automatically.

#### 8. Publish Release Notes

```bash
# Wait for the GitHub Release to be created by CI, then update notes:
gh release edit v0.23.0 --notes-file release-notes.md

# Or create a new release with notes (if CI doesn't auto-create):
gh release create v0.23.0 --notes-file release-notes.md --title "v0.23.0"
```

### Post-Release: Bump to Next Dev Version

Immediately after a release is tagged and pushed, bump `Cargo.toml` to the next development version:

```bash
# Bump to next version with -dev suffix (e.g., 0.23.0 → 0.24.0-dev)
sed -i 's/^version = ".*"/version = "0.24.0-dev"/' Cargo.toml
cargo check  # regenerate Cargo.lock
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to 0.24.0-dev"
git push
```

**Rules:**
- The `-dev` suffix signals "unreleased development build" and prevents `fabio upgrade` from overwriting dev builds with older releases.
- Release versions MUST NOT contain `-dev` — the release workflow strips it.
- Development versions MUST contain `-dev` — all commits between releases use this format.
- The version lifecycle is: `0.24.0-dev` (development) → `0.24.0` (release tag) → `0.25.0-dev` (next cycle).

### Automated Release Script

The `scripts/release.sh` script automates ALL 8 steps end-to-end:

```bash
./scripts/release.sh 0.23.0
```

It handles: version bump, dependency freshness check (with optional `cargo update`), doc version updates, full validation (fmt + clippy + test + cross-check), commit (Cargo.toml + Cargo.lock + docs), changelog generation, tagging, pushing, and release note publishing.

The script pauses at two interactive points:
- After showing outdated dependencies — asks whether to run `cargo update`
- After generating raw changelog — waits for you to edit the release notes file

If any validation step fails (fmt, clippy, tests, cross-check), the script aborts immediately with a clear error message.

### Configuration

- `cliff.toml` — git-cliff configuration (commit parsers, grouping, template)
- `.github/RELEASE_TEMPLATE.md` — Narrative structure template
- `scripts/release.sh` — Automated release script (version bump, changelog, tag, push, publish notes)

### Release Notes Rules

- ALWAYS run `git cliff --latest` (or `--unreleased`) first to get the complete raw list — do NOT rely on memory or `git log` alone.
- The curated narrative must cover ALL features/fixes from the raw changelog (nothing should be silently dropped).
- New item types and headline features go FIRST in the release notes.
- CI/CD and documentation-only changes go at the end (lower priority for users).
- Include a `Stats` section with commit count, files changed, and lines added/removed.
- Include the `Full Changelog` comparison link at the bottom.

**Note:** The release workflow automatically publishes the Docker image to `ghcr.io/iemejia/fabio:{version}` and `ghcr.io/iemejia/fabio:{major}.{minor}` as part of the `docker` job in `.github/workflows/release.yml`. No manual Docker build/push is needed.

## Progress
### Done
- **Full Rust implementation** (broad command surface): auth, workspace, item, lakehouse, capacity, catalog, context, notebook, warehouse, data-agent, sql-database, sql-endpoint, ontology, environment, data-pipeline, copy-job, dataflow, report, semantic-model, eventhouse, eventstream, kql-database, kql-queryset, kql-dashboard, mirrored-database, mirrored-catalog, mirrored-databricks-catalog, mirrored-warehouse, reflex, ml-model, ml-experiment, spark, spark-job-definition, graphql-api, cosmos-db-database, snowflake-database, digital-twin-builder, digital-twin-builder-flow, event-schema-set, operations-agent, mounted-data-factory, user-data-function, git, connection, deployment-pipeline, domain, deploy, gateway, job-scheduler, variable-library, map, graph-query-set, graph-model, onelake-security, managed-private-endpoint, warehouse-snapshot, admin, paginated-report, dashboard, datamart, anomaly-detector, apache-airflow-job, app-backend, azure-databricks-storage, data-build-tool-job, org-app, org-app-audience, rti, rest, profile, jobs, feedback, operation, upgrade
- Core output system: JSON envelope (`{"data":..., "count":N}` or `{"error":{"code":...,"message":...}}`), table, plain, CSV, TSV formats
- Structured error system: `ErrorCode` enum (AUTH_REQUIRED, NOT_FOUND, RATE_LIMITED, CAPACITY_INACTIVE, API_ERROR, TIMEOUT, etc.) + `FabioError`
- Global options fully wired: `--output/-o`, `--query/-q` (JMESPath expression — see jmespath.org), `--quiet` (suppresses stdout), `--verbose/-v` (HTTP/LRO/auth diagnostics on stderr), `--profile`, `--dry-run`, `--limit`, `--all`, `--continuation-token`, `--lro-timeout`
- HTTP client: async get/post/put/patch/delete with LRO polling (`Location` + `x-ms-operation-id` + resource follow)
- OneLake operations: DFS upload (create+append+flush with Content-MD5), download, file listing; Blob API copy (server-side async)
- **Parallel file/table operations**: Upload, copy, move support glob patterns with concurrent execution and rate-limit retry
- **Sync command**: `lakehouse sync` copies new/modified files between lakehouses using ETag/MD5 comparison, with rename detection (`--delete` + optional `--checksum`), server-side dedup (copies from existing dest content), rsync-inspired flags (`--include`, `--exclude`, `--size-only`, `--no-overwrite`, `--force`, `--no-recursive`, `--max-delete`, `--existing`, `--remove-source-files`, `--min-size`, `--max-size`, `--itemize`), and `--local` for local-to-remote sync (parallel upload of only new/changed files from a local directory)
- **LRO polling**: 2s default interval (respects `Retry-After` header, capped at 60s), 120s max, handles 200/202, checks `status` field until Succeeded/Failed
- **Transport retry**: Automatic retry on 502/503/504 gateway errors (3 attempts, linear backoff 1-3s)
- **Error code headers**: Extracts `x-ms-public-api-error-code` / `x-ms-error-code` response headers into error messages
- **Server-side file copy/move**: Blob API `PUT` with `x-ms-copy-source`; atomic rename via DFS `x-ms-rename-source` for same-item moves (O(1) metadata op), fallback to copy + delete for cross-item
- **Server-side table copy/move/delete**: Root listing + prefix filter, per-file Blob copy, recursive DFS delete; same-item table moves use atomic directory rename
- **Shortcuts**: Create/get/delete OneLake, ADLS Gen2, S3 shortcuts
- **Lakehouse table maintenance**: optimize-table (V-Order + Z-Order via Jobs API), vacuum-table (retention period formatting), table-schema (Delta log parsing from OneLake DFS)
- **OneLake Iceberg REST Catalog**: iceberg-config, iceberg-namespaces, iceberg-namespace, iceberg-tables, iceberg-table (Apache Iceberg REST Catalog v1 at `https://onelake.table.fabric.microsoft.com` — provides full table metadata: schema, partitions, sort-orders, snapshots, properties; uses storage-scoped auth)
- **OneLake Iceberg extended**: iceberg-table-exists, iceberg-namespace-exists (HEAD checks), iceberg-credentials (vended storage tokens for external tools), iceberg-stats (record/file/size summary from latest snapshot), iceberg-snapshots (full snapshot history with operations and record counts)
- **Enhanced table-schema**: Uses Iceberg REST Catalog as primary backend (more reliable than Delta log parsing), falls back to DFS `_delta_log` parsing when Table API is unavailable
- **Notebook run**: Captures job instance ID from Location header, status/stop via Jobs API
- **Notebook `--wait` flag**: Polls job status every 5s until Completed/Failed/Cancelled, with configurable `--timeout` (default 600s)
- **Item copy/move**: getDefinition LRO + create in dest workspace LRO; move = copy + delete source
- **Item exists/url/inspect**: exists returns `{exists: true/false}` (never errors on 404); url returns Fabric portal URL; inspect aggregates metadata + definition + connections in single response
- **Private link URL routing**: `with_private_link()` builder on FabricClient; `fabric_url()`, `onelake_dfs_url()`, `onelake_blob_url()` helpers transform URLs when `private_link_workspace` is configured via profile
- **Workspace**: 47 subcommands (CRUD + capacity + identity + role assignments + settings + networking + storage format + folders + OneLake + lifecycle policies + url)
- **Warehouse**: list/show/create/update/delete/query/connection-string (endpoint resolved, stdin/file/flag SQL input)
- **Git integration**: status, commit, pull, connect, disconnect, initialize, switch (branch), connection/credentials management, show-tracked
- **Ontology management**: list, show, create, update, delete, get-definition, update-definition (RDF file support, --dir for Fabric definition format, --decode for readable output), import (OWL RDF/XML + JSON-LD → Fabric, compatible with Ontology Playground), export (Fabric → OWL RDF/XML or JSON-LD, full round-trip support)
- **Environment**: list, show, create, update, delete, publish, cancel-publish, get-spark-settings, get-staging-spark-settings, upload-staging-library
- **Data Pipeline**: list, show, create, update, delete, run (triggers Pipeline job), create-schedule, list-schedules, get-schedule, update-schedule, delete-schedule, list-instances, get-instance
- **Eventhouse**: list, show, create, update, delete
- **Eventstream**: list, show, create, update, delete, get-definition, update-definition, get-topology, pause, resume, get/pause/resume-source, get-source-connection, get/pause/resume-destination, get-destination-connection, add-source, add-destination, add-sample-source, add-derived-stream, validate, list-components
- **KQL Database**: list, show, create, update, delete, query, get-definition, update-definition, list-entities, describe, describe-entity, sample, ingest, show-queryplan, diagnostics, deeplink, list-shortcuts, create-shortcut, get-shortcut, delete-shortcut, bulk-create-shortcuts (ReadWrite/ReadOnlyFollowing)
- **KQL Queryset**: list, show, create, update, delete, get-definition, update-definition, run (executes saved query tabs against configured data source)
- **KQL Dashboard**: list, show, create, update, delete, get-definition, update-definition (RealTimeDashboard.json)
- **Mirrored Database**: list, show, create, update, delete, get/update-definition, start, stop, status, table-status
- **Reflex**: list, show, create, update, delete, get-definition, update-definition, create-trigger (auto-generates full entity hierarchy from simple flags: KQL source + email/Teams alerts)
- **ML Model**: list, show, create, update, delete (CRUD only, no definition support)
- **ML Experiment**: list, show, create, update, delete (CRUD only, no definition support)
- **Copy Job**: list, show, create, update, delete, get-definition, update-definition, reset (data movement)
- **Dataflow**: list, show, create, update, delete, get-definition, update-definition, discover-parameters, run, execute-query (with --arrow-version 1|2, LRO-aware) (Power BI transformation)
- **GraphQL API**: list, show, create, update, delete, get-definition, update-definition (schema.graphql)
- **Report**: list, show, create (from definition file), update, delete, get-definition, update-definition
- **Semantic Model**: list, show, create (from model.bim), update, delete, get-definition, update-definition, query, refresh, bind-connection, unbind-connection, takeover
- **Map**: list, show, create, update, delete, get-definition, update-definition (geospatial visualization with Azure Maps)
- **Spark Job Definition**: list, show, create, update, delete, get-definition, update-definition, run
- **Capacity**: list, show, suspend, resume, create, update, delete, list-skus, check-name (Fabric read-only API + ARM API for lifecycle management)
- **Connection**: list, show, create, update, delete, list-supported-types
- **Deployment Pipeline**: list, show, create, update, delete, list-stages, list-stage-items, assign-workspace, unassign-workspace, deploy
- **Domain**: list, show, create, update, delete, list-workspaces, assign-workspaces, unassign-workspaces, assign-by-capacity, assign-by-principal
- **Job Scheduler**: list-instances, get-instance, run-on-demand (with `--wait`/`--timeout`/`--cancel-on-timeout`), cancel-instance, list-schedules, get-schedule, create-schedule, update-schedule, delete-schedule
- **Spark**: get-settings, update-settings, list-pools, get-pool, create-pool, update-pool, delete-pool
- **OneLake Security**: list, show, upsert, delete, create (data access roles for row/column-level security)
- **Managed Private Endpoint**: list, show, create, delete (workspace private networking)
- **Pagination**: `--all` fetches all pages, `--continuation-token` resumes from a specific token, `--limit` truncates client-side
- **Agent-native compliance** (all 10 principles implemented):
  - Principle 1: Non-interactive by default
  - Principle 2: Structured parseable output
  - Principle 3: Errors that teach and enumerate
  - Principle 4: Safe retries (`--dry-run`)
  - Principle 5: Bounded responses (`--limit`, `--continuation-token`, truncation metadata)
  - Principle 6: Consistent vocabulary (list/show/create/delete/copy/move)
  - Principle 7: `fabio context agent` machine-readable schema + `fabio context` semantic knowledge (item schemas, workflows, best practices)
  - Principle 8: Async-aware (`--wait`, jobs ledger)
  - Principle 9: Named profiles (`fabio profile save/use/list/show/delete`)
  - Principle 10: Two-way I/O (`fabio feedback send/list`)
- **SQL Database**: list/show/create/update/delete/query/connection-string/import (TDS + type inference)
- **SQL Database import**: Reads CSV/JSON files, infers column types (Int/BigInt/Float/Bit/Date/NVarChar), generates CREATE TABLE + batched INSERTs via TDS. Supports --drop-if-exists, --no-create-table, --batch-size.
- **SQL Endpoint**: list/show/connection-string/query/refresh-metadata/get-audit-settings/update-audit-settings/set-audit-actions (read-only companion to lakehouses)
- **Data Agent**: list/show/create/update/delete/query/get-definition/update-definition/publish/reset (28 subcommands via public staging management API), get-config/update-config (`--stage staging|published`), add/remove/list/show/update-datasource (auto-type detection, `--stage`), select-tables (toggle table selection), list-elements/describe-element/delete-element (`--stage`), list/show/add/update/remove/clear-fewshots/upload-fewshots (`--stage`, JSON + CSV/TSV bulk upload), query `--stage sandbox|production` + `--timeout` + `--show-steps`
- **Variable Library**: list/show/create/update/delete/get-definition/update-definition (variables.json + settings.json)
- **Event Schema Set**: list/show/create/update/delete/get-definition/update-definition (EventSchemaSetDefinition.json)
- **User Data Function**: list/show/create/update/delete/get-definition/update-definition (Python runtime)
- **Operations Agent**: list/show/create/update/delete/get-definition/update-definition (Configurations.json, goals/instructions/dataSources/actions)
- **Digital Twin Builder**: list/show/create/update/delete/get-definition/update-definition (links to lakehouse)
- **Digital Twin Builder Flow**: list/show/create/update/delete/get-definition/update-definition (requires parent DTB)
- **Cosmos DB Database**: list/show/create/update/delete/get-definition/update-definition (empty shell creation supported)
- **Snowflake Database**: list/show/create/update/delete/get-definition/update-definition (requires connection payload)
- **Anomaly Detector**: list/show/create/update/delete/get-definition/update-definition (Configurations.json)
- **Deploy**: plan/apply/export/init-params/validate (CI/CD deployment engine: content-hash diffing, parameter substitution, rename detection, creationPayload, post-deploy hooks, logical ID resolution, workspace folder management, git-diff selective deploy, deploy config file JSON+YAML, selective filtering, workspace ID regex replacement, protected type deletion guards, full fabric-cicd compatibility)
- **Gateway**: list/show/create/update/delete, list-members/update-member/delete-member, list/add/show/update/delete-role-assignments, check-status/check-member-status/restart/shutdown (VNet gateways)
- **Admin**: 49 subcommands (tenant settings, tags, workloads, workspaces, items, users, domains, labels, sharing links, external data shares, network policies)
- **Apache Airflow Job**: list/show/create/update/delete/get-definition/update-definition, start-environment/stop-environment/get-environment, list-files/get-file/upload-file/delete-file, get-compute/get-workspace-settings/deploy-requirements
- **App Backend**: list/show/create/update/delete (`--hard-delete` support, create uses LRO, update requires `--name` and/or `--description`)
- **Azure Databricks Storage**: list/show/create/update/delete/get-definition/update-definition (Fabric integration with Azure Databricks, definition format `AzureDatabricksStorageV1`, part path `definition.json`)
- **Data Build Tool Job**: list/show/create/update/delete/get-definition/update-definition/run (with --wait/--timeout/--cancel-on-timeout) [preview]
- **OrgApp**: list/show/create/update/delete/get-definition/update-definition (Organizational App)
- **OrgAppAudience**: list/show/create/update/delete/get-definition/update-definition (Org App Audience)
- **Mirrored Catalog**: list/show/create/update/delete/get-definition/update-definition, refresh-metadata/mirroring-status/tables-status (requires tenant feature flag)
- **Mirrored Databricks Catalog**: list/show/create/update/delete/get-definition/update-definition, discover-catalogs/refresh-metadata/mirroring-status
- **Mirrored Warehouse**: list (requires tenant feature flag for mutations)
- **Warehouse Snapshot**: list/show/create/update/delete (requires --warehouse-id on create)
- **Graph Model**: list/show/create/update/delete/get-definition/update-definition, refresh-graph/execute-query/get-queryable-graph-type (portal initialization required for refresh)
- **Graph Query Set**: list/show/create/update/delete/get-definition/update-definition (definition is read-only export)
- **Catalog**: search (tenant-level full-text search across workspaces)
- **Context**: tenant (workspace graph extraction — builds a relationship graph of items with nodes/edges/summary for agent memory; three-layer discovery: properties, definitions via `--deep`, connections via `--include-connections`; parallel execution; supports multi-workspace, `--item-types` filter, `--concurrency`; incremental building via `--output-file` + `--merge`; fast inventory via `--no-properties`; 5 output formats: `graph` native, `jsonld` RDF instances, `owl` OWL JSON-LD schema, `rdf` OWL RDF/XML schema, `full` combined schema+instances RDF/XML)
- **Context (agent knowledge)**: `fabio context` provides structured knowledge for AI agents. Subcommands:
  - `fabio context agent` — Compact index of all command groups + subcommand names (default). Flags:
    - `--group <G>` — Full flags/types/descriptions for a single group
    - `--full` — Complete 14K-line schema dump (all metadata)
    - `--format mcp` — MCP tool definitions (JSON Schema `inputSchema`, `annotations`)
    - `--format openai` — OpenAI function-calling format
  - `fabio context describe <GROUP> <CMD>` — Deep-dive on one command: all flags, output example, auth scope, notes
  - `fabio context find "<query>"` — Keyword search across commands (returns top-10 ranked results)
  - `fabio context list` — Discover all available topics (workflows, best-practices, schemas, examples)
  - `fabio context workflow <NAME>` — Multi-step workflow recipes with exact command syntax:
    - `rti-pipeline` — Eventhouse + KQL DB + EventStream end-to-end
    - `direct-lake-report` — Semantic model (model.bim or TMDL) + report creation with template
    - `cicd-deploy` — Export + plan + apply with content-hash convergence
    - `lakehouse-etl` — Lakehouse + notebook + load-table + schedule
    - `data-agent-setup` — Create + datasource + schema discovery retry + instructions + few-shots + publish
  - `fabio context best-practices <TOPIC>` — Operational guidance:
    - `shortcuts` — ADLS Gen2 connection + shortcut two-step pattern, known list-files limitation
    - `lro` — Automatic LRO polling, --wait for jobs, timeout handling
    - `throttling` — Automatic 429 retry, bounded parallelism
    - `pagination` — --all, --limit, --continuation-token
    - `admin-apis` — When to use admin vs workspace-scoped commands
  - `fabio context schema <TYPE>` — Item definition schemas (22 types: notebook, lakehouse, semantic_model, etc.)
  - `fabio context examples <GROUP> <CMD>` — Output shape examples for command responses
- **MCP Server**: `fabio mcp serve` starts a JSON-RPC 2.0 server over stdio implementing the Model Context Protocol. Exposes all 807 subcommands as MCP tools with `inputSchema` (JSON Schema), `annotations` (readOnlyHint, destructiveHint, auth_scope). Handles `initialize`, `tools/list`, `tools/call`, `ping`. Agent frameworks (Claude Desktop, VS Code Copilot, custom agents) can integrate fabio as a native tool server without shell-exec.
- **Dashboard**: list (read-only, portal-created)
- **Datamart**: list (read-only, portal-created)
- **Paginated Report**: list/show/create/update/delete/get-definition/update-definition (previously only list+update)
- **RTI (Real-Time Intelligence)**: nl-to-kql (natural language to KQL translation via POST /realTimeIntelligence/nltokql?beta=true)
- **Lakehouse query**: Resolves SQL analytics endpoint from lakehouse properties, executes T-SQL via shared TDS utilities
- **Rest**: Raw REST passthrough command (`fabio rest call`); supports GET/POST/PUT/PATCH/DELETE; `--body` accepts inline JSON, `@file`, `@-` (stdin); `--query-params` for URL params; `--poll` for LRO; dry-run for mutating methods; `--api powerbi` targets Power BI REST API
- **Power BI API pass-through**: `fabio rest call --api powerbi` routes to `https://api.powerbi.com/v1.0/myorg`; reuses Fabric token (no separate scope needed); supports all HTTP methods; dry-run shows `"api": "powerbi"` in output
- **Semantic Model Power BI commands** (12 subcommands via Power BI REST API): `list-parameters`, `update-parameters`, `list-datasources`, `update-datasources`, `list-users`, `add-user`, `delete-user`, `refresh-status`, `list-upstream`, `clone`, `export-pbix`, `import-pbix`
- **Semantic Model clone**: POST `/groups/{ws}/datasets/{id}/Default.Clone` with `--name`, `--target-workspace` (optional cross-workspace clone)
- **Semantic Model export-pbix**: POST `.../Default.Export` → binary download to `--file` path; creates parent dirs; reports `size_bytes` in output
- **Semantic Model import-pbix**: POST `/groups/{ws}/imports` multipart/form-data; `--name`, `--file`, `--name-conflict` (Abort|Overwrite|CreateOrOverwrite|GenerateUniqueName); validates file exists client-side
- **Item bulk-create/bulk-delete**: Client-side parallel operations using `execute_parallel` with bounded concurrency and rate-limit retry; per-item success/failure reporting
- **Item move-to-folder**: `POST /workspaces/{ws}/items/{id}/move` with `targetFolderId`; omit folder-id to move to workspace root
- **Item create-external-data-share**: Polymorphic recipients (`--recipient-type User|ServicePrincipal`, `--recipient-id`)
- **`--hard-delete` on all item deletes**: 38 item type delete commands support `--hard-delete` flag to permanently delete (skip recycle bin); appends `?hardDelete=true` to URL
- **Semantic Model unbind-connection**: Sends `{"connectionId": null}` to the bind endpoint to unbind
- **Dataflow discover-parameters**: `GET /workspaces/{ws}/dataflows/{id}/parameters` with pagination support
- **Warehouse connection-string extended**: `--guest-tenant-id` and `--private-link-type` optional query params for cross-tenant and private link scenarios
- **Error `isRetriable` field**: Parsed from API response `error.isRetriable` into `FabioError.retriable: Option<bool>`; emitted in structured error output when present
- **Notebook --strip-output**: `get-definition --strip-output` clears `outputs`/`execution_count` from ipynb cells; gracefully passes through `.py` format
- **CSV/TSV output**: Global `--output csv|tsv` on all commands; RFC 4180 quoting via `format_csv_value()`
- **Deploy validate**: Local-only pre-flight checks on source directory (validates .platform files, item types, definition structure, logical ID references); no API calls required
- **Deploy fabric-cicd full compatibility**: Parses .children/ KQL database discovery, .pbi/ directory exclusion, creationPayload from .platform metadata, SparkJobDefinitionV2 format auto-detection, Report byPath→byConnection transform, notebook part ordering (.py before .json), ItemDisplayNameNotAvailableYet retry (up to 5 min), binary file skip, .platform included as definition part but excluded from content hash for idempotency
- **1672 Rust tests** (927 unit + 745 offline/E2E integration), zero clippy warnings, rustfmt clean
- **CI/CD**: GitHub Actions (6-target matrix: x64+arm64 for linux/macos/windows), Dependabot auto-merge, CodeQL, Secret Scanning
- **Release workflow**: Triggered on tags, builds 6 binaries, publishes GitHub Release with SHA256 checksums
- Release binary: ~16 MB, stripped, full LTO, panic=abort

### Blocked
- (none)

## Key Decisions
- JSON envelope always wraps output: lists get `{"data":[...],"count":N}`, objects get `{"data":{...}}`
- Errors on stderr as `{"error":{"code":"...","message":"..."}}` with non-zero exit
- `--query` supports full JMESPath expressions (see jmespath.org) — filter, project, slice, multiselect, pipe, functions (length, sort_by, etc.)
- `--quiet` suppresses all stdout; errors still go to stderr
- OneLake upload uses DFS create+append+flush 3-step pattern with `x-ms-content-md5` on flush (computes MD5 client-side, stores as file property for content-based matching)
- Notebook creation builds minimal .ipynb JSON, base64-encodes for Fabric API; `source` must be list of strings
- Item copy fetches definition from source via LRO, posts to destination workspace via LRO
- LRO polling: 2s default interval (respects `Retry-After` header, capped at 60s), 120s max wait, handles `Location`/`x-ms-operation-id` headers
- `post()` accepts `poll: bool` for LRO-aware operations
- Load-table requires PascalCase values (`"Overwrite"`, `"Csv"`) and `format` inside `formatOptions`
- **Load-table only supports Csv and Parquet**: The Fabric REST API `formatOptions` discriminated union only has `Csv` (with `header`/`delimiter`) and `Parquet` (format only). JSON is NOT supported — must convert to CSV/Parquet first. Sending CSV-specific fields (header, delimiter) with Parquet format causes API rejection.
- **SQL Database import**: Uses type inference with `Unknown` initial state → first non-empty observation sets the type, subsequent observations widen (Int→BigInt→Float→NVarChar, never narrows)
- **Server-side copy**: OneLake Blob API supports `PUT` with `x-ms-copy-source`; returns 202 with pending status. Poll via HEAD.
- **Atomic rename for same-item moves**: DFS `x-ms-rename-source` works within the same OneLake item (workspace + lakehouse). Works for both files and directories. Returns 201. Fails with 403 for cross-item/cross-workspace. Fallback: copy + delete.
- **Table file listing**: Must list from root (no `directory` param) to get real paths prefixed with item ID.
- **Recursive delete**: DFS `DELETE /{ws}/{lh}/Tables/{name}?recursive=true` works for directories.
- All destructive actions use consistent verb `delete` (not `remove`)
- Cross-workspace ops use `--source-workspace`/`--dest-workspace` with `visible_alias` short forms
- Auth relies on a multi-source credential chain: fabio cache (device code, browser PKCE, or service principal), environment variables, managed identity, Azure CLI, Azure Developer CLI
- `azure_identity`/`azure_core` with `default-features = false` (no OpenSSL dependency on Linux/macOS; OpenSSL for certificate auth via `client_certificate` feature)
- **Windows-first compatibility** — Token cache encrypted with DPAPI (`CryptProtectData`, user scope); WAM broker SSO via `--wam` flag
- `unsafe_code = "forbid"` in lints
- **KQL Queryset definition format**: Uses `RealTimeQueryset.json` (NOT `RawQueryset.kql`). JSON structure: `{"queryset":{"version":"1.0.0","dataSources":[{"id","clusterUri","type","databaseName"}],"tabs":[{"id","content","title","dataSourceId"}]}}`. The `content` field holds the KQL query text with `\n` for newlines.
- **KQL Queryset run**: Fetches definition via LRO, decodes `RealTimeQueryset.json`, selects tab by name or index, resolves data source (clusterUri + databaseName), executes via Kusto REST API. Tab selection is case-insensitive by title.
- **Deploy diff strategy**: Content hash vs live workspace (not git diff) — detects portal edits, works without git, idempotent convergence
- **Deploy parallelism**: Semaphore-bounded `tokio::spawn` per-item within type batch (default 8); sequential for DataPipeline; deletes always sequential
- **Deploy parameter format**: JSON (not YAML) — no extra crate dependency, agent-native consistency
- **Deploy plan staleness**: Workspace fingerprint = SHA256 of sorted `(id, type, name)` tuples; mismatch → error unless `--force`
- **Deploy logical ID resolution**: String replacement in base64 payloads; resolves items created earlier in same session
- **Deploy rename detection**: Two-pass matching — first by (type, name), then unmatched source items with logical IDs get candidates checked via `fetch_deployed_logical_id()` which reads `.platform` part from deployed item definition
- **Deploy creationPayload**: Separate `creationPayload.json` file in item directory; merged into creation body as `creationPayload` field; parameter substitution applied
- **Deploy post-hooks**: Opt-out via `--no-post-hooks`; hooks never fire during `--dry-run`; failures are non-fatal (reported in output, don't fail the deploy). SemanticModel → `POST /refreshes`, Environment → `POST /staging/publish`
- **Deploy empty definitions**: Items with no parts (Lakehouse, MLModel) omit `definition` field on create; skip `updateDefinition` on update
- **Deploy ordering**: 45 item types in `DEPLOY_ORDER`; deployed in dependency order (storage → compute → code → models → reactive → APIs → ML → graph → viz)
- **Deploy no state file**: Stateless — always queries live workspace. No `.tfstate` equivalent.
- **Deploy .platform in parts but excluded from hash**: `.platform` IS sent as a definition part (enables `?updateMetadata=true` for metadata propagation), but EXCLUDED from content hash (API rewrites `logicalId` in `.platform`, which would break idempotent skip detection)
- **Deploy workspace ID regex replacement**: Uses regex matching on `workspaceId`, `default_lakehouse_workspace_id`, `workspace` keys — not blanket string replacement. Skips shortcuts (handled separately with lakehouse GUID). Opt-out: `--no-workspace-id-replace`
- **Deploy config file (JSON + YAML)**: `--config <file> --env <name>` loads per-environment workspace/source/parameters; `serde_yaml` crate for YAML; CLI flags override config values
- **Deploy protected type deletion**: Lakehouse, Warehouse, SQLDatabase, Eventhouse, KQLDatabase require `--allow-delete-types` to be deleted by `--delete-orphans`
- **Deploy fabric-cicd full compatibility**: Source directory format, .platform file schema, definition parts, logical ID resolution, workspace ID replacement, creationPayload, .children/ discovery, .pbi/ exclusion, notebook ordering, Report byPath transform — all aligned with Microsoft's fabric-cicd Python library
- **Upgrade**: `fabio upgrade` downloads latest release from GitHub, verifies SHA256 checksum, extracts platform-appropriate archive (tar.gz on Unix, zip on Windows), atomically replaces running binary; supports `--check` (version query only), `--target-version` (pin specific version), `--force` (reinstall even if current), `--dry-run`

## Critical Context
- User's tenant: set locally via secure environment configuration (redacted)
- Active capacity: set locally via secure environment configuration (redacted)
- Inactive capacity: set locally via secure environment configuration (redacted)
- Source workspace/lakehouse: set locally via secure environment configuration (redacted)
- Destination workspace/lakehouse: set locally via secure environment configuration (redacted)
- Notebook ID: set locally via secure environment configuration (redacted)
- Fabric REST base URL: `https://api.fabric.microsoft.com/v1`
- OneLake DFS base URL: `https://onelake.dfs.fabric.microsoft.com`
- OneLake Blob base URL: `https://onelake.blob.fabric.microsoft.com`
- Fabric scope: `https://api.fabric.microsoft.com/.default`
- Storage scope: `https://storage.azure.com/.default`
- Spark rate limit on small capacity: LRO reports 430 `TooManyRequestsForCapacity` (non-standard code)
- Test env vars: `FABIO_TEST_SOURCE_WORKSPACE`, `FABIO_TEST_SOURCE_LAKEHOUSE`, `FABIO_TEST_DEST_WORKSPACE`, `FABIO_TEST_DEST_LAKEHOUSE`, `FABIO_TEST_NOTEBOOK_ID`, `FABIO_TEST_CAPACITY_ID`
- Fabric REST API specs (OpenAPI): `https://github.com/Azure/azure-rest-api-specs/` (look under `specification/fabric/`)

## Relevant Files
- `Cargo.toml`: Project config, dependencies, clippy/lints config, release profile (LTO+strip)
- `rust-toolchain.toml`: stable channel, rustfmt+clippy components
- `prek.toml`: Pre-commit hook configuration (prek — Rust-native pre-commit runner)
- `.agents/skills/fabio/SKILL.md`: Agent skill bootstrapping document (330 lines, loaded by agent frameworks on activation)
- `.agents/skills/fabio/references/API-BEHAVIORS.md`: Critical API gotchas that cause silent failures
- `.agents/skills/fabio/scripts/install.sh`: Cross-platform binary installer for the skill
- `tests/eval/promptfooconfig.yaml`: 77-case promptfoo eval testing skill instruction quality (run with `GITHUB_TOKEN=$(gh auth token) promptfoo eval -c tests/eval/promptfooconfig.yaml`)
- `src/main.rs`: Entry point, `#![recursion_limit = "256"]`, tokio async main, error handling dispatch
- `src/cli.rs`: Clap derive CLI definition, OutputFormat enum, Command enum with 74 subcommand groups
- `src/errors.rs`: ErrorCode enum (with stable exit codes) + FabioError struct with thiserror
- `src/output.rs`: render_list_with_token, render_object, render_error (respects --quiet/--query/--wrap-untrusted), apply_query, dry_run_guard, unit tests
- `src/parallel.rs`: Parallel execution framework for concurrent file/table operations with rate-limit retry
- `src/verbose.rs`: Lightweight `--verbose` diagnostics module (global AtomicBool flag, HTTP/LRO/auth tracing to stderr)
- `src/client.rs`: FabricClient with async HTTP (get/post/put/patch/delete), LRO polling, OneLake DFS/Blob ops, ARM API methods (arm_get/post/put/patch/delete with ARM LRO polling), Power BI API methods (get/post/put/patch/delete/bytes/multipart_powerbi), run_notebook, trigger_item_job
- `src/commands/mod.rs`: Command dispatch
- `src/commands/auth.rs`: login (device code + browser PKCE + service principal: secret/certificate/federated token), logout, status
- `src/commands/workspace.rs`: 47 subcommands (CRUD + capacity + identity + role assignments + settings + networking + storage format + folders + OneLake + lifecycle policies + url)
- `src/commands/item.rs`: 18 subcommands (CRUD + copy/move + definitions + list-connections + exists/url/inspect + bulk-create/bulk-delete + move-to-folder + create-external-data-share)
- `src/commands/lakehouse.rs`: 34 subcommands (CRUD + tables, files, upload, download, load-table, copy-file, delete-file, move-file, create-directory, delete-table, copy-table, move-table, sync, create-shortcut, get-shortcut, delete-shortcut, optimize-table, vacuum-table, table-schema, iceberg-config, iceberg-namespaces, iceberg-namespace, iceberg-tables, iceberg-table, iceberg-table-exists, iceberg-namespace-exists, iceberg-credentials, iceberg-stats, iceberg-snapshots, query)
- `src/commands/notebook.rs`: create/get-definition (with --strip-output)/run (with --wait/--timeout/--parameters/--compute-type/--execution-data)/status/stop/delete
- `src/commands/warehouse.rs`: list/show/create/update/delete/query/connection-string (endpoint resolved, stdin/file/flag SQL input)
- `src/commands/sql_database.rs`: list/show/create/update/delete/query/connection-string/import (TDS + type inference)
- `src/commands/tds_utils.rs`: Shared TDS utilities (resolve_sql_input, parse_connection_string, execute_and_render_sql, column_value_to_json)
- `src/commands/dataagent.rs`: list/show/create/update/delete/query/get-definition/update-definition/publish + get-config/update-config, add/remove/list/show-datasource, select-tables, list-elements/describe-element, add/remove/list-fewshots/upload-fewshots
- `src/commands/git.rs`: status/commit/pull/connect/disconnect/initialize/switch/connection/credentials/show-tracked
- `src/commands/ontology.rs`: list/show/create/update/delete/get-definition/update-definition/import/export
- `src/commands/ontology_import.rs`: OWL RDF/XML + JSON-LD parser, Fabric format generator, RDF serializer (import + export)
- `src/commands/environment.rs`: list/show/create/update/delete/publish/cancel-publish/get-spark-settings/get-staging-spark-settings/upload-staging-library
- `src/commands/data_pipeline.rs`: list/show/create/update/delete/run, create-schedule, list-schedules/get-schedule/update-schedule/delete-schedule, list-instances/get-instance
- `src/commands/report.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/semantic_model.rs`: list/show/create/update/delete/get-definition/update-definition + query/refresh/bind-connection/unbind-connection/takeover + list-parameters/update-parameters/list-datasources/update-datasources/list-users/add-user/delete-user/refresh-status/list-upstream/clone/export-pbix/import-pbix
- `src/commands/eventhouse.rs`: list/show/create/update/delete
- `src/commands/eventstream/mod.rs`: list/show/create/update/delete/get-definition/update-definition/get-topology/pause/resume/sources/destinations
- `src/commands/eventstream/builder.rs`: add-source/add-destination/add-sample-source/add-derived-stream/validate/list-components
- `src/commands/kql_database/mod.rs`: list/show/create/update/delete/get-definition/update-definition/shortcuts
- `src/commands/kql_database/intelligence.rs`: query/list-entities/describe/describe-entity/sample/ingest/show-queryplan/diagnostics/deeplink
- `src/commands/kql_utils.rs`: Shared KQL utilities (resolve_kql_input, resolve_query_uri, execute_kql, parse v1/v2 responses, render results)
- `src/commands/kql_queryset.rs`: CRUD + get-definition/update-definition + run (fetch definition, select tab, execute against Kusto REST API)
- `src/commands/kql_dashboard.rs`: list/show/create/update/delete/get-definition/update-definition (RealTimeDashboard.json)
- `src/commands/mirrored_database.rs`: list/show/create/update/delete/get-definition/update-definition/start/stop/status/table-status
- `src/commands/reflex.rs`: list/show/create/update/delete/get-definition/update-definition/create-trigger (Data Activator)
- `src/commands/ml_model.rs`: list/show/create/update/delete (CRUD only)
- `src/commands/ml_experiment.rs`: list/show/create/update/delete (CRUD only)
- `src/commands/copy_job.rs`: list/show/create/update/delete/get-definition/update-definition/reset
- `src/commands/dataflow.rs`: list/show/create/update/delete/get-definition/update-definition/discover-parameters/run/execute-query
- `src/commands/graphql_api.rs`: list/show/create/update/delete/get-definition/update-definition (schema.graphql)
- `src/commands/spark.rs`: get-settings/update-settings/list-pools/get-pool/create-pool/update-pool/delete-pool
- `src/commands/spark_job_definition.rs`: list/show/create/update/delete/get-definition/update-definition/run
- `src/commands/map.rs`: list/show/create/update/delete/get-definition/update-definition (geospatial Azure Maps)
- `src/commands/capacity.rs`: list/show (Fabric API) + suspend/resume/create/update/delete/list-skus/check-name (ARM API)
- `src/commands/connection.rs`: list/show/create/update/delete/list-supported-types
- `src/commands/deployment_pipeline.rs`: list/show/create/update/delete/list-stages/list-stage-items/assign-workspace/unassign-workspace/deploy
- `src/commands/domain.rs`: list/show/create/update/delete/list-workspaces/assign-workspaces/unassign-workspaces/assign-by-capacity/assign-by-principal
- `src/commands/job_scheduler.rs`: list-instances/get-instance/run-on-demand (with `--wait`/`--timeout`/`--cancel-on-timeout`), cancel-instance/list-schedules/get-schedule/create-schedule/update-schedule/delete-schedule
- `src/commands/onelake_security.rs`: list/show/upsert/delete/create (data access roles)
- `src/commands/managed_private_endpoint.rs`: list/show/create/delete
- `src/commands/variable_library.rs`: list/show/create/update/delete/get-definition/update-definition (variables.json + settings.json)
- `src/commands/event_schema_set.rs`: list/show/create/update/delete/get-definition/update-definition (EventSchemaSetDefinition.json)
- `src/commands/user_data_function.rs`: list/show/create/update/delete/get-definition/update-definition (definition.json, Python runtime)
- `src/commands/operations_agent.rs`: list/show/create/update/delete/get-definition/update-definition (Configurations.json)
- `src/commands/digital_twin_builder.rs`: list/show/create/update/delete/get-definition/update-definition (definition.json, links to lakehouse)
- `src/commands/digital_twin_builder_flow.rs`: list/show/create/update/delete/get-definition/update-definition (requires parent DTB)
- `src/commands/cosmos_db_database.rs`: list/show/create/update/delete/get-definition/update-definition (definition.json)
- `src/commands/snowflake_database.rs`: list/show/create/update/delete/get-definition/update-definition (requires connection payload)
- `src/commands/sql_endpoint.rs`: list/show/connection-string/query/refresh-metadata/get-audit-settings/update-audit-settings/set-audit-actions
- `src/commands/anomaly_detector.rs`: list/show/create/update/delete/get-definition/update-definition (Configurations.json)
- `src/commands/deploy/mod.rs`: DeployCommand enum (plan/apply/export/init-params/validate); execute dispatch; workspace name resolution
- `src/commands/deploy/apply.rs`: execute_changeset, execute_post_hooks, Rename handling (PATCH + updateDefinition), build_resolution_map, resolve_logical_ids_in_payload
- `src/commands/deploy/plan.rs`: build_changeset (two-pass with rename), validate_references, fetch_deployed_logical_id, compute_workspace_fingerprint
- `src/commands/deploy/params.rs`: Parameter substitution: find_replace, key_value_replace, spark_pool, semantic_model_binding
- `src/commands/deploy/init_params.rs`: scan_for_candidates, diff_for_parameters (GUID discovery, cross-environment diffing)
- `src/commands/deploy/changeset.rs`: Change, ChangeAction (Create/Update/Rename/Delete/Skip), Changeset (with warnings/errors), DeployResult
- `src/commands/deploy/ordering.rs`: DEPLOY_ORDER (45 types), deploy_priority, delete_priority, topological_sort
- `src/commands/deploy/platform.rs`: parse_source_directory (creationPayload.json parsing), SourceItem, SourceWorkspace, PlatformMetadata
- `src/commands/deploy/export.rs`: export_workspace (getDefinition LRO per item, write .platform + parts)
- `src/commands/deploy/config.rs`: DeployConfig struct (JSON+YAML parsing via serde_yaml), per-environment workspace/source/parameters resolution, FilterConfig, OptionsConfig
- `src/commands/deploy/folders.rs`: Workspace folder management (discover from source directory, create/move/delete folders), SourceFolder, DeployedFolder, FolderPlan
- `src/commands/deploy/git_diff.rs`: Git diff-based selective deployment (get_changed_items via `git diff --name-status`, GitDiffResult with changed/deleted sets)
- `src/commands/gateway.rs`: list/show/create/update/delete, members, role assignments, check-status/check-member-status/restart/shutdown (VNet gateways)
- `src/commands/admin.rs`: 49 subcommands for tenant administration
- `src/commands/apache_airflow_job.rs`: CRUD + environment lifecycle + file ops + compute settings
- `src/commands/azure_databricks_storage.rs`: list/show/create/update/delete/get-definition/update-definition (definition.json, AzureDatabricksStorageV1 format)
- `src/commands/mirrored_catalog.rs`: CRUD + definition + mirroring operations
- `src/commands/mirrored_databricks_catalog.rs`: CRUD + definition + discover/refresh/status
- `src/commands/mirrored_warehouse.rs`: list only (tenant feature flag blocks mutations)
- `src/commands/warehouse_snapshot.rs`: list/show/create/update/delete
- `src/commands/graph_model.rs`: CRUD + definition + refresh-graph/execute-query/get-queryable-graph-type
- `src/commands/graph_query_set.rs`: CRUD + get-definition/update-definition (read-only export)
- `src/commands/catalog.rs`: search (tenant-level)
- `src/commands/context.rs`: extract (workspace graph extraction — nodes/edges/summary, three-layer relationship discovery, parallel execution, incremental building)
- `src/commands/dashboard.rs`: list (read-only)
- `src/commands/datamart.rs`: list (read-only)
- `src/commands/paginated_report.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/profile.rs`: save/use/list/show/delete (named profiles with defaults)
- `src/commands/jobs.rs`: list/get/prune (local async job ledger)
- `src/commands/feedback.rs`: send/list (two-way I/O for CLI friction reporting)
- `src/commands/context/agent.rs`: Machine-readable command schema for AI agents (hierarchical access, MCP/OpenAI format emission, drift detection, auto-generation)
- `src/commands/context/schemas.rs`: Item definition schemas (22 types)
- `src/commands/context/workflows.rs`: Multi-step workflow recipes (5 recipes)
- `src/commands/context/best_practices.rs`: Best-practices guidance (5 topics)
- `src/commands/context/examples.rs`: Output shape examples (34 commands)
- `src/commands/context/tenant.rs`: Live workspace graph extraction
- `src/commands/mcp/mod.rs`: MCP command group (serve subcommand)
- `src/commands/mcp/serve.rs`: MCP JSON-RPC 2.0 server (initialize, tools/list, tools/call over stdio)
- `src/commands/rest.rs`: Raw REST passthrough (method/path/body/query-params/poll); `resolve_body()` for @file/@- support; `--api powerbi` targets Power BI REST API
- `src/commands/rti.rs`: nl-to-kql (natural language to KQL translation)
- `src/commands/data_build_tool_job.rs`: list/show/create/update/delete/get-definition/update-definition/run (with --wait/--timeout/--cancel-on-timeout) [preview]
- `src/commands/org_app.rs`: list/show/create/update/delete/get-definition/update-definition (Organizational App)
- `src/commands/org_app_audience.rs`: list/show/create/update/delete/get-definition/update-definition (Org App Audience)
- `src/commands/upgrade.rs`: upgrade (check/download/verify/replace binary from GitHub Releases)
- `tests/common/mod.rs`: Shared E2E test harness (TestConfig, helpers)
- `tests/e2e_auth.rs`: Auth integration tests (device code, service principal secret/certificate/federated, WAM, input validation)
- `tests/e2e_workspace.rs`: Workspace CRUD + assign-capacity + networking + OneLake settings + folders + storage format + roles filter + CMK encryption tests
- `tests/e2e_global_options.rs`: --query, --quiet, --output format tests
- `tests/e2e_item.rs`: Item list/show/create/delete/copy/move/bulk-create/bulk-delete tests
- `tests/e2e_lakehouse.rs`: Tables/files/upload/download/query tests
- `tests/e2e_lakehouse_files.rs`: File copy/move/delete tests
- `tests/e2e_lakehouse_tables.rs`: Table load/copy/move/delete tests
- `tests/e2e_lakehouse_shortcuts.rs`: Shortcut create/get/delete tests
- `tests/e2e_lakehouse_iceberg.rs`: Iceberg REST Catalog tests (config, namespaces, tables, schema)
- `tests/e2e_notebook.rs`: Notebook create/get-definition/run/run --wait/status/stop/delete/strip-output tests
- `tests/e2e_warehouse.rs`: Warehouse list/show/query/query-stdin tests
- `tests/e2e_sql_database.rs`: SQL Database CRUD + query + import + revalidate-cmk dry-run tests
- `tests/e2e_dataagent.rs`: Data agent tests (34 tests: CRUD, query, definition, publish, datasource lifecycle, fewshot lifecycle, elements lifecycle, config, CSV upload, dry-run validations)
- `tests/e2e_git.rs`: Git command group tests
- `tests/e2e_ontology.rs`: Ontology CRUD + definition tests
- `tests/e2e_agent_native.rs`: Agent-native compliance tests (principles 1-10)
- `tests/e2e_verbose.rs`: Verbose flag tests (16 tests: offline flag acceptance, HTTP/auth/LRO tracing, --quiet suppression, --dry-run interaction)
- `tests/e2e_sync.rs`: Lakehouse sync tests (24 tests: basic copy, skip unchanged, delete, checksum, parallel, rename detection, dedup, include/exclude, size-only, no-overwrite, force, max-delete, existing, remove-source-files, local-to-remote sync)
- `tests/e2e_connection.rs`: Connection CRUD + list-supported-types tests
- `tests/e2e_environment.rs`: Environment CRUD tests
- `tests/e2e_data_pipeline.rs`: Data pipeline CRUD + run + schedule/instance tests
- `tests/e2e_eventhouse.rs`: Eventhouse CRUD tests
- `tests/e2e_eventstream.rs`: Eventstream CRUD tests
- `tests/e2e_kql_database.rs`: KQL database tests
- `tests/e2e_kql_queryset.rs`: KQL queryset tests
- `tests/e2e_kql_dashboard.rs`: KQL dashboard tests
- `tests/e2e_mirrored_database.rs`: Mirrored database tests
- `tests/e2e_reflex.rs`: Reflex CRUD + definition (get/update with simulator pipeline) tests
- `tests/e2e_graphql_api.rs`: GraphQL API CRUD tests
- `tests/e2e_ml_model.rs`: ML model CRUD tests
- `tests/e2e_ml_experiment.rs`: ML experiment CRUD tests
- `tests/e2e_copy_job.rs`: Copy job CRUD + reset tests
- `tests/e2e_dataflow.rs`: Dataflow CRUD + run + execute-query tests
- `tests/e2e_report.rs`: Report CRUD tests
- `tests/e2e_semantic_model.rs`: Semantic model CRUD tests
- `tests/e2e_map.rs`: Map CRUD + definition tests
- `tests/e2e_spark_job_definition.rs`: Spark job definition tests
- `tests/e2e_deployment_pipeline.rs`: Deployment pipeline tests
- `tests/e2e_domain.rs`: Domain management tests
- `tests/e2e_job_scheduler.rs`: Job scheduler tests (11 tests: list, dry-run, fire-and-forget, --wait with polling)
- `tests/e2e_spark.rs`: Spark settings and pool tests
- `tests/e2e_capacity.rs`: Capacity list/show tests + ARM dry-run tests (suspend/resume/create/update/delete)
- `tests/e2e_onelake_security.rs`: OneLake security tests
- `tests/e2e_managed_private_endpoint.rs`: Managed private endpoint tests
- `tests/e2e_admin.rs`: Admin API tests (63 tests: listing, tag lifecycle, domain lifecycle, dry-run validations, sharing links, labels, external data shares)
- `tests/e2e_deploy.rs`: Deploy plan/apply/export/validate tests (42 tests: create, update, rename, creationPayload, parameters, staleness, logical ID resolution, post-hooks, init-params, validate)
- `tests/e2e_fabric_cicd_compat.rs`: fabric-cicd compatibility tests (11 tests: validate source directory, nested folders, workspace ID replacement, parameter substitution, selective filtering, config file YAML, init-params scan)
- `tests/e2e_gateway.rs`: Gateway CRUD + role assignment + lifecycle tests
- `tests/e2e_apache_airflow_job.rs`: Apache Airflow job CRUD + environment + file ops tests
- `tests/e2e_azure_databricks_storage.rs`: Azure Databricks storage CRUD + definition + lifecycle tests
- `tests/e2e_mirrored_catalog.rs`: Mirrored catalog tests
- `tests/e2e_mirrored_databricks_catalog.rs`: Mirrored Databricks catalog tests
- `tests/e2e_mirrored_warehouse.rs`: Mirrored warehouse tests
- `tests/e2e_warehouse_snapshot.rs`: Warehouse snapshot tests
- `tests/e2e_graph_model.rs`: Graph model CRUD + refresh + query tests
- `tests/e2e_graph_query_set.rs`: Graph query set tests
- `tests/e2e_catalog.rs`: Catalog search tests
- `tests/e2e_context.rs`: Context tenant tests (10 offline dry-run + 10 live graph extraction)
- `tests/e2e_dashboard.rs`: Dashboard list tests
- `tests/e2e_datamart.rs`: Datamart list tests
- `tests/e2e_paginated_report.rs`: Paginated report tests
- `tests/e2e_anomaly_detector.rs`: Anomaly detector CRUD + definition tests
- `tests/e2e_cosmos_db_database.rs`: Cosmos DB database CRUD tests
- `tests/e2e_snowflake_database.rs`: Snowflake database tests
- `tests/e2e_digital_twin_builder.rs`: Digital Twin Builder CRUD tests
- `tests/e2e_digital_twin_builder_flow.rs`: Digital Twin Builder Flow tests
- `tests/e2e_event_schema_set.rs`: Event Schema Set CRUD tests
- `tests/e2e_operations_agent.rs`: Operations Agent CRUD + definition tests
- `tests/e2e_mounted_data_factory.rs`: Mounted Data Factory tests
- `tests/e2e_user_data_function.rs`: User Data Function CRUD tests
- `tests/e2e_variable_library.rs`: Variable Library CRUD + definition tests
- `tests/e2e_sql_endpoint.rs`: SQL Endpoint tests
- `tests/e2e_profile.rs`: Profile save/use/list/show/delete tests
- `tests/e2e_jobs.rs`: Jobs ledger tests
- `tests/e2e_feedback.rs`: Feedback send/list tests
- `tests/e2e_context_agent.rs`: Agent context schema tests
- `tests/e2e_rest.rs`: REST passthrough tests (dry-run, body resolution, live calls)
- `tests/e2e_rti.rs`: RTI nl-to-kql tests (dry-run + live failure)
- `tests/e2e_data_build_tool_job.rs`: DataBuildToolJob CRUD + definition + run tests
- `tests/e2e_org_app.rs`: OrgApp CRUD + definition tests
- `tests/e2e_org_app_audience.rs`: OrgAppAudience CRUD + definition tests
- `tests/e2e_upgrade.rs`: Upgrade tests (dry-run, check, version targeting, JSON output)
- `.github/workflows/ci.yml`: Rust CI (fmt, clippy, test, build) on 6 targets (x64+arm64 x linux/macos/windows)
- `.github/workflows/release.yml`: Release workflow (tag-triggered, 6 binaries, SHA256 checksums, GitHub Release)
- `.github/workflows/dependabot-auto-merge.yml`: Auto-merge Dependabot PRs on CI pass
- `.github/dependabot.yml`: Cargo + GitHub Actions dependency updates
- `cliff.toml`: git-cliff configuration (commit parsers, grouping, template)
- `.github/RELEASE_TEMPLATE.md`: Release notes narrative structure template

## Docker & Devcontainer

### Production Docker Image

Published to GHCR on every push to `main` and on version tags:

```
ghcr.io/iemejia/fabio:latest       # latest stable release
ghcr.io/iemejia/fabio:0.30.0       # release version
ghcr.io/iemejia/fabio:0.23         # major.minor
```

Multi-arch manifest: `linux/amd64` + `linux/arm64`.

**Dockerfile** (root): Multi-stage build — compiles release binary in Ubuntu builder stage, copies to minimal runtime image (~52MB) with only `ca-certificates`.

### Devcontainer

Located in `.devcontainer/` for VS Code and GitHub Codespaces. Provides the full development environment:

**System packages** (in Dockerfile): `build-essential`, `pkg-config`, `libssl-dev`, `lld`, `clang`, `zig 0.16.0`

**Devcontainer features**: Rust (with cross targets), Git, GitHub CLI, Azure CLI

**Cargo tools** (installed via `postCreateCommand`): `git-cliff`, `cargo-zigbuild`, `cargo-xwin`, `cargo-audit`

**Cross-compilation targets** (for `./scripts/cross-check.sh`): `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`

**VS Code extensions**: rust-analyzer, Even Better TOML, CodeLLDB debugger, Dependi (crate version checker)

### Docker CI Workflow (`.github/workflows/docker.yml`)

| Trigger | Build | Push to GHCR |
|---------|-------|--------------|
| Pull request | amd64 + arm64 | No (validation only) |
| Push to `main` | amd64 + arm64 | No (validation only) |

Uses GitHub Actions cache (`type=gha`) for Docker layer caching. QEMU for arm64 cross-build. `GITHUB_TOKEN` for GHCR auth (no extra secrets).

The release workflow (`.github/workflows/release.yml`) handles tagged version images (`:latest`, `:X.Y.Z`, `:X.Y`) as a separate `docker` job that runs after binaries are published.

### Relevant Docker Files

- `Dockerfile`: Production multi-stage image (builder + minimal runtime)
- `.devcontainer/Dockerfile`: Dev environment base image (Ubuntu + system deps + zig)
- `.devcontainer/devcontainer.json`: Features, extensions, cargo tools, cross targets
- `.github/workflows/docker.yml`: Build validation + GHCR publish workflow

## API Behaviors Discovered

Runtime behaviors, quirks, and undocumented API details are documented in a separate file to reduce context size:

**File:** `.agents/API-BEHAVIORS-DISCOVERED.md` (2019 lines)

Reference this file when working on specific command groups. Do NOT load the entire file into context — search for the relevant section by command group name (e.g., "Lakehouse API Behaviors Discovered", "Deploy Command Design & Behaviors").

When discovering new API behaviors during implementation, append them to `.agents/API-BEHAVIORS-DISCOVERED.md` under the appropriate section heading.
