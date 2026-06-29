---
name: dev-release
description: "Step-by-step release workflow for fabio. Invoke when cutting a new release version. Covers: version bump, dependency freshness, documentation updates, validation, changelog, tagging, and post-release dev version bump."
---

# Release Workflow for Fabio

Complete ALL steps in order. Do NOT skip any step.

## Step 1: Bump the Version Number

```bash
# Check current version (should be X.Y.Z-dev during development)
grep '^version' Cargo.toml | head -1

# Update to release version (remove -dev suffix)
sed -i 's/^version = ".*"/version = "0.25.0"/' Cargo.toml
```

Run `cargo check` or `cargo build` to regenerate `Cargo.lock`.

## Step 2: Validate Dependency Freshness

```bash
cargo outdated --root-deps-only
# or: cargo update --dry-run
```

**Rules:**
- Update any dependency with a newer compatible version (within semver range).
- For major bumps, check changelog for breaking changes.
- Reject copyleft licenses (GPL, LGPL, AGPL, SSPL). Only permissive (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode-3.0).
- Run full pre-commit validation after updating dependencies.
- Check GitHub Actions versions in `.github/workflows/*.yml`.

## Step 3: Update Version References in Documentation

1. **README.md** — Docker image version in usage examples.
2. **AGENTS.md** — Docker & Devcontainer section version examples.

## Step 4: Run Full Validation

```bash
cargo fmt -- --check
cargo clippy --tests -- -D warnings
cargo test
./scripts/cross-check.sh
```

ALL must pass with zero errors and zero warnings.

## Step 5: Commit Cargo.toml AND Cargo.lock Together

```bash
git add Cargo.toml Cargo.lock README.md AGENTS.md
git status  # verify only intended files
git commit -m "chore: bump version to 0.25.0"
```

**Rules:**
- NEVER tag without `Cargo.lock` reflecting the exact dependency tree.
- `git status` must be clean before tagging.

## Step 6: Generate Release Notes

```bash
# Preview unreleased changes (before tagging):
git cliff --unreleased

# For the latest tag (after tagging):
git cliff --latest

# Between two specific tags:
git cliff v0.24.0..v0.25.0
```

Follow the template in `.github/RELEASE_TEMPLATE.md`:
1. Lead with impact (most user-visible features first)
2. Group related commits into single feature descriptions
3. Include command usage examples for new features
4. Stats at the end (commit count, lines changed, test coverage)

**Rules:**
- ALWAYS run `git cliff` first — do NOT rely on memory.
- Cover ALL features/fixes from the raw changelog.
- New item types and headline features go FIRST.
- CI/CD-only changes go at the end.
- Include `Full Changelog` comparison link.

## Step 7: Tag and Trigger the Release

```bash
git tag v0.25.0
git push
git push origin v0.25.0
```

CI builds 6 binaries + Docker image automatically.

## Step 8: Publish Release Notes

```bash
gh release edit v0.25.0 --notes-file release-notes.md
# or: gh release create v0.25.0 --notes-file release-notes.md --title "v0.25.0"
```

## Step 9: Post-Release — Bump to Next Dev Version

```bash
sed -i 's/^version = ".*"/version = "0.26.0-dev"/' Cargo.toml
cargo check
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to 0.26.0-dev"
git push
```

**Version lifecycle:** `0.25.0-dev` (dev) → `0.25.0` (release tag) → `0.26.0-dev` (next cycle).

## Automated Release Script

```bash
./scripts/release.sh 0.25.0
```

Automates ALL steps. Pauses for:
- Dependency update decision
- Release notes editing

Aborts on any validation failure.

## Configuration

- `cliff.toml` — git-cliff config
- `.github/RELEASE_TEMPLATE.md` — Narrative template
- `scripts/release.sh` — Automated release script
