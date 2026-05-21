# Copilot Instructions for fabio

## Overview

Fabio is an agent-first CLI for Microsoft Fabric, written in Rust (edition 2024, MSRV 1.85). It manages 37 command groups with 265+ subcommands. All output is structured JSON by default.

## Language & Framework Conventions

### Rust

- **Edition 2024**, `rust-version = "1.85"`
- Clippy: `pedantic` + `nursery` lints enabled, zero warnings required
- `unsafe_code = "forbid"` — no unsafe code allowed anywhere
- Use `thiserror` for error types, `anyhow` for propagation in command handlers
- Use `clap` derive macros for CLI argument parsing
- Use `tokio` for async runtime (full features)
- Use `serde` derive for serialization — all API response structs need `#[derive(Deserialize, Serialize)]`
- Use `reqwest` with `rustls` (no OpenSSL dependency)
- Prefer `.lines()` over splitting on `\n` (handles CRLF)

### Windows Compatibility (Critical)

All code must work on Windows:
- Use `Path::new().join()` — never hardcode `/` in filesystem paths
- Use `dirs::home_dir()` — never read `HOME`/`USERPROFILE` manually
- Use `.lines()` for text parsing — handles CRLF transparently
- No Unix-specific APIs (`std::os::unix`, signals, etc.)
- `.gitattributes` enforces LF line endings in the repo

## Architecture Patterns

### Command Structure

Every command module follows this pattern:
1. Define a `Command` enum with clap `Subcommand` derive in the module
2. Implement an `execute(cli: &Cli, cmd: &Command)` async function
3. Register the module in `src/commands/mod.rs` (add `pub mod` + match arm)
4. Add the variant to `src/cli.rs` `Command` enum

### Output Envelope

- Lists: `{"data":[...],"count":N}` with optional `"continuationToken"`
- Objects: `{"data":{...}}`
- Errors (stderr): `{"error":{"code":"...","message":"...","hint":"..."}}`
- Use `render_list()` / `render_list_with_token()` / `render_object()` from `src/output.rs`
- Never print raw text to stdout — always use the output helpers

### Error Handling

- Use `ErrorCode` enum from `src/errors.rs` for machine-readable codes
- Include `hint` field with valid values or corrected command examples
- Map HTTP status codes: 401→`AuthRequired`, 403→`Forbidden`, 404→`NotFound`, 409→`Conflict`, 429→`RateLimited`

### HTTP Client

- All API calls go through `FabricClient` in `src/client.rs`
- GET/POST/PUT/PATCH/DELETE helpers handle auth token injection
- LRO polling: `post(..., poll: true)` follows `Location`/`x-ms-operation-id` headers
- OneLake DFS: 3-step upload (create → append → flush)
- OneLake Blob: server-side copy via `x-ms-copy-source` header

### Global Flags

All commands must respect these (handled by output helpers):
- `--output json|table|plain` — format selection
- `--query` — dot-notation field projection
- `--quiet` — suppress stdout
- `--dry-run` — preview mutations via `dry_run_guard()`
- `--limit` — client-side truncation
- `--all` — auto-paginate all pages
- `--continuation-token` — resume from token

## Test Conventions

- Unit tests: inline `#[cfg(test)]` modules using `wiremock` for HTTP mocking
- E2E tests: `tests/e2e_*.rs` files, require live Fabric tenant via `FABIO_TEST_*` env vars
- E2E tests use `#[ignore]` attribute (run with `cargo test -- --ignored`)
- Test harness: `tests/common/mod.rs` provides `TestConfig` with workspace/lakehouse IDs
- Use `assert_cmd` + `predicates` for CLI binary assertions
- Test names follow pattern: `test_{command}_{subcommand}_{scenario}`

## Code Style

- No comments on obvious code — only clarify non-obvious logic
- Prefer exhaustive `match` over `if let` chains
- Use `visible_alias` for common flag short forms (e.g., `--sw` for `--source-workspace`)
- Keep modules focused: one file per command group
- Imports: group by std → external crates → internal crates, separated by blank lines

## Commit Conventions

- Use `Assisted-by: Copilot:claude-opus-4.6` trailer
- Prefix with type: `feat`, `fix`, `chore`, `docs`, `test`, `refactor`
- Example: `feat(lakehouse): add sync command for file delta transfer`

## API Reference

- Fabric REST: `https://api.fabric.microsoft.com/v1`
- OneLake DFS: `https://onelake.dfs.fabric.microsoft.com`
- OneLake Blob: `https://onelake.blob.fabric.microsoft.com`
- Auth scopes: `https://analysis.windows.net/powerbi/api/.default` (Fabric), `https://storage.azure.com/.default` (Storage)

## Maintenance Matrix

When you change these files, update the corresponding dependents:

| Changed | Must Also Update |
|---------|-----------------|
| `src/cli.rs` (add Command variant) | `src/commands/mod.rs` (match arm), new `src/commands/{name}.rs` |
| `src/commands/mod.rs` (add pub mod) | `src/cli.rs` (import + Command variant) |
| `src/errors.rs` (add ErrorCode) | `ErrorCode::Display` impl, update AGENTS.md error codes list |
| `src/output.rs` (change envelope) | All E2E tests that parse JSON output |
| `src/client.rs` (change auth/HTTP) | All command modules that call client methods |
| `Cargo.toml` (add dependency) | `Cargo.lock` (auto), verify `default-features = false` if Azure crate |
| `.github/workflows/ci.yml` | Verify all matrix targets still pass |
| `src/parallel.rs` | `src/commands/lakehouse.rs` (upload/copy/move/sync) |
| Any command module | Corresponding `tests/e2e_{name}.rs`, README Commands section |
