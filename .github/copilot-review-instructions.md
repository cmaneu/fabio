# Copilot Code Review Instructions for fabio

## Review Focus Areas

When reviewing pull requests to this repository, prioritize the following:

### 1. Agent-Native CLI Compliance

Every command must follow the 10 agent-native CLI principles:
- Non-interactive: no prompts, all inputs via flags/env/files
- Structured output: `--json` on every command, stdout = data, stderr = diagnostics
- Errors that teach: include valid enum values, corrected examples, machine-readable codes
- Safe retries: `--dry-run` for mutations, idempotency-safe
- Bounded responses: `--limit` for lists, concise defaults
- Consistent vocabulary: `list`, `show`, `create`, `delete`, `copy`, `move`
- Introspection: commands must be representable in `agent-context`
- Async-aware: `--wait` for long-running operations
- Profile support: `--profile` flag respected
- Two-way I/O: composable stdin/stdout

### 2. Rust Quality Standards

- **Zero clippy warnings** under `pedantic` + `nursery` lint groups
- **`unsafe_code = "forbid"`** — reject any unsafe code
- **Error handling**: `FabioError` with `ErrorCode` enum, never panic in command handlers
- **Output**: always use `render_list()`/`render_object()` helpers, never raw `println!`
- **Exhaustive matches**: prefer `match` over `if let` chains for enums

### 3. Windows Compatibility (Critical)

Flag any code that would break on Windows:
- Hardcoded `/` in filesystem paths (must use `Path::new().join()`)
- Manual `HOME`/`USERPROFILE` reads (must use `dirs::home_dir()`)
- Splitting on `\n` without handling `\r\n` (must use `.lines()`)
- Unix-specific APIs (`std::os::unix`, signals, etc.)

### 4. API Correctness

- Verify HTTP method matches the operation (GET for reads, POST for creates, PATCH for updates)
- LRO operations must use `poll: true` where the API returns 202
- Check that `--dry-run` is implemented for all mutations
- Verify error enrichment: 403 should hint required role, 404 should suggest `list` command
- Pagination: list commands must support `--all`, `--limit`, `--continuation-token`

### 5. Test Coverage

- New commands require corresponding `tests/e2e_{name}.rs`
- Unit tests for parsing logic, validators, and output formatting
- Tests must use `#[ignore]` for live API tests
- Dry-run tests (offline) should be present for all mutations

### 6. Security

- Never log or expose auth tokens, secrets, or PII
- Validate all user inputs (GUIDs, paths, JSON) before sending to API
- Token cache must use platform-appropriate encryption (DPAPI on Windows)

## Review Anti-Patterns (Flag These)

- `println!` or `eprintln!` instead of output helpers
- Missing `--dry-run` support on a mutation command
- Hardcoded URLs (should use `fabric_url()`, `onelake_dfs_url()`, etc.)
- `unwrap()` or `expect()` in non-test code without justification
- New dependencies without `default-features = false` evaluation
- Inconsistent command vocabulary (e.g., `remove` instead of `delete`)
- Missing `enrich_forbidden()` on API calls that can return 403
- Tests that depend on execution order or shared mutable state

## Commit Message Format

Verify PR titles and commits follow conventional commits:
```
type(scope): description
```
Valid types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `build`, `ci`, `revert`
