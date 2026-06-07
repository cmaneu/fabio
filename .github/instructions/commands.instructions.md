---
applyTo: "src/commands/**"
---

# Command Module Instructions — `src/commands/**`

This guide defines how command modules in fabio should be structured. It applies to all files under `src/commands/`.

---

## 1) Command Anatomy

**Rules:**

1. **One file per command group** (e.g., `lakehouse.rs`, `warehouse.rs`).
2. **Consistent verbs**: `list`, `show`, `create`, `update`, `delete`, `get-definition`, `update-definition`, `run`, `copy`, `move`.
3. **Clap derive pattern**: Define `{Name}Command` enum with `#[derive(Subcommand)]`.
4. **Async execution**: `pub async fn execute(cli: &Cli, cmd: &{Name}Command) -> Result<()>`.
5. **Output via helpers**: Always use `render_list()`, `render_list_with_token()`, or `render_object()`.

---

## 2) Required Structure

```rust
use crate::cli::Cli;
use crate::output::{render_list_with_token, render_object};
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ExampleCommand {
    /// List all examples
    List { /* flags */ },
    /// Show a single example
    Show { /* flags */ },
    /// Create an example
    Create { /* flags */ },
    // ...
}

pub async fn execute(cli: &Cli, cmd: &ExampleCommand) -> Result<()> {
    match cmd {
        ExampleCommand::List { .. } => { /* implementation */ }
        ExampleCommand::Show { .. } => { /* implementation */ }
        ExampleCommand::Create { .. } => { /* implementation */ }
    }
}
```

---

## 3) Flags & Arguments

- `--workspace` / `-w`: Required for workspace-scoped resources
- `--id`: Item identifier (GUID)
- `--name` / `-n`: Display name (for create/update)
- `--description` / `-d`: Optional description
- `--file` / `-f`: File input path
- `--content`: Inline JSON content (alternative to `--file`)
- Use `visible_alias` for short forms (e.g., `--sw` for `--source-workspace`)

---

## 4) Mutation Commands Must

- Implement `dry_run_guard()` — return planned action without executing
- Use `enrich_forbidden()` on API calls that can return 403
- Include `hint` in error messages with valid enum values
- Support `--hard-delete` for item deletion commands (appends `?hardDelete=true`)
- Return `{"status": "deleted", "id": "<id>"}` for delete operations

---

## 5) List Commands Must

- Support pagination: `--all`, `--limit`, `--continuation-token`
- Use `get_list()` with the correct response array key (`"value"` for most APIs, `"data"` for lakehouse tables)
- Apply `render_list_with_token()` for paginated output

---

## 6) LRO Operations

- Use `post(url, body, poll: true)` for operations that return 202
- Default polling: 2s interval, 120s max, respects `Retry-After` header
- Terminal states: `Succeeded`, `Failed`
- Report failure reason from `failureReason.message` in job response

---

## 7) Definition Operations

- `get-definition`: POST with empty body `{}`, LRO poll, decode base64 parts
- `update-definition`: POST with `{"definition":{"parts":[...]}}`, LRO poll
- Support `--decode` flag to show decoded payload alongside raw
- Support `--dir` flag for multi-file definition formats

---

## 8) Error Patterns

```rust
// 403 enrichment
let response = client.get(&url).await.map_err(|e| enrich_forbidden(e, "Contributor"))?;

// Not-found hint
if status == 404 {
    bail!(FabioError::new(ErrorCode::NotFound, "Item not found")
        .with_hint(format!("Run: fabio {group} list --workspace {ws}")));
}

// Validate at least one update field
if name.is_none() && description.is_none() {
    bail!(FabioError::new(ErrorCode::InvalidInput, "No fields to update")
        .with_hint("Provide --name and/or --description"));
}
```

---

## 9) Windows Compatibility

- File paths: `Path::new(base).join(segment)` — never string concatenation with `/`
- Text parsing: `.lines()` — never `.split('\n')`
- Home dir: `dirs::home_dir()` — never `std::env::var("HOME")`
- Line endings: All files use LF (enforced by `.gitattributes`)
