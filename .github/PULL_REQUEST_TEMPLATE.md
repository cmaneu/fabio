## Description

Brief description of what this PR does.

## Changes

-

## How to Test

```bash
# Build and run affected commands
cargo build
cargo test
```

## Checklist

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy --tests -- -D warnings` passes (zero warnings)
- [ ] `cargo test` passes (unit tests)
- [ ] New commands registered in `src/cli.rs` and `src/commands/mod.rs`
- [ ] Output uses `render_list()` / `render_object()` helpers (not raw println)
- [ ] `--dry-run` support added for mutations
- [ ] Windows-compatible paths (`Path::new().join()`, no hardcoded `/`)
- [ ] README Commands section updated (if new command)
- [ ] E2E test added in `tests/e2e_*.rs` (if testable)
