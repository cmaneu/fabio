---
applyTo: "tests/**"
---

# Test Instructions — `tests/**`

This guide defines how tests in fabio should be structured and implemented. It applies to all files under `tests/`.

---

## 1) Test Taxonomy

### Unit tests (fast, isolated)

- Location: inline `#[cfg(test)]` modules within source files
- Scope: parsing, validation, output formatting, error mapping
- No network calls; use `wiremock` for HTTP mocking when needed
- Must pass without any environment variables

### E2E tests (live API)

- Location: `tests/e2e_*.rs`
- Scope: full command execution against a live Fabric tenant
- Require `FABIO_TEST_*` environment variables
- Use `#[ignore]` attribute — run with `cargo test -- --ignored`
- Test harness: `tests/common/mod.rs` provides `TestConfig`

---

## 2) Naming Conventions

- **File names**: `tests/e2e_{command_group}.rs` (e.g., `e2e_lakehouse.rs`, `e2e_workspace.rs`)
- **Test function names**: `test_{subcommand}_{scenario}` (e.g., `test_list_json_output`, `test_create_dry_run`)
- **Modules**: group related tests with `mod` blocks for clarity

---

## 3) E2E Test Patterns

### Standard test structure

```rust
#[tokio::test]
#[ignore]
async fn test_list_json_output() {
    let config = TestConfig::new();
    let output = config
        .command()
        .args(["example", "list", "--workspace", &config.workspace, "-o", "json"])
        .output()
        .await
        .expect("command failed");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["data"].is_array());
    assert!(json["count"].is_number());
}
```

### Dry-run test (offline, no API needed)

```rust
#[tokio::test]
#[ignore]
async fn test_create_dry_run() {
    let config = TestConfig::new();
    let output = config
        .command()
        .args([
            "example", "create",
            "--workspace", &config.workspace,
            "--name", "test-item",
            "--dry-run",
        ])
        .output()
        .await
        .expect("command failed");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "dry_run");
}
```

---

## 4) What Every Command Group Must Test

- **List**: JSON output structure (`data` array + `count`), `--limit` truncation
- **Show**: Single object output, 404 error handling
- **Create**: `--dry-run` (offline), live creation + cleanup (delete after)
- **Update**: At least one field change, validate "no fields" error
- **Delete**: `--dry-run`, verify response format `{"status":"deleted","id":"..."}`
- **Definition ops** (if applicable): `get-definition` + `update-definition` round-trip
- **Error paths**: Invalid workspace ID, missing required flags

---

## 5) Test Environment Variables

```bash
FABIO_TEST_SOURCE_WORKSPACE    # Workspace ID for read operations
FABIO_TEST_SOURCE_LAKEHOUSE    # Lakehouse ID for file/table tests
FABIO_TEST_DEST_WORKSPACE      # Workspace ID for write operations
FABIO_TEST_DEST_LAKEHOUSE      # Lakehouse ID for copy/move targets
FABIO_TEST_NOTEBOOK_ID         # Notebook ID for run tests
FABIO_TEST_CAPACITY_ID         # Capacity ID for capacity tests
```

---

## 6) Rules

- **No flakiness**: Tests must be deterministic; use unique names with timestamps/UUIDs
- **Clean up**: Tests that create items must delete them (even on failure — use scopeguard or finally patterns)
- **No order dependency**: Each test must be independently runnable
- **No secrets in code**: Environment variables for all tenant-specific values
- **Parallel safe**: Tests must not conflict with each other (use unique resource names)
- **Timeout handling**: Long-running operations (notebook run, deploy) need appropriate timeouts

---

## 7) Output Validation

```rust
// Validate JSON envelope for lists
let json: Value = serde_json::from_slice(&output.stdout).unwrap();
assert!(json["data"].is_array());
assert!(json["count"].as_u64().unwrap() > 0);

// Validate error on stderr
let stderr = String::from_utf8_lossy(&output.stderr);
let err: Value = serde_json::from_str(&stderr).unwrap();
assert_eq!(err["error"]["code"], "NOT_FOUND");
```

---

## 8) Security

- Never hardcode tokens, tenant IDs, or PII in test files
- Use environment variables for all workspace/item identifiers
- Test output should not leak sensitive information from API responses
- Cassettes/recordings (if used) must be scrubbed of auth headers and personal data
