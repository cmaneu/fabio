---
title: How Fabio stays always current
description: The automated pipeline that tracks the Fabric REST API daily and keeps every dependency fresh.
---

Fabric is a fast-moving platform. New item types, API endpoints, and flag options appear frequently. Most community tools — including [fab cli](https://github.com/mrjsj/fab-cli) and [fabric-cicd](https://github.com/microsoft/fabric-cicd) — rely on manual releases to pick up changes.

Fabio runs an automated pipeline that makes staying current the default, not an afterthought.

## Three layers of freshness

### 1. Daily Fabric API spec sync

Microsoft publishes the Fabric REST API surface in the public [`microsoft/fabric-rest-api-specs`](https://github.com/microsoft/fabric-rest-api-specs) repository. A scheduled GitHub Actions workflow runs every day at 23:00 UTC — one hour after the typical upstream spec update window — and performs the following steps:

1. **Detect changes** — A cursor file (`.github/fabric-api-specs-cursor`) records the last processed commit SHA. If the upstream HEAD has moved, the workflow computes the diff.
2. **Implement with Copilot** — The diff and a structured prompt (`sync-fabric-api-specs.md`) are fed to the [GitHub Copilot CLI](https://github.com/github/copilot). Copilot reads the spec changes and writes the corresponding Rust command code, serialisation structs, and help text.
3. **Verify the build** — `cargo check` and `cargo clippy -- -D warnings` are run. If they fail, Copilot is asked to fix the errors automatically.
4. **Open a pull request** — If any code changed, a labelled PR is opened for human review before merging.

This means new Fabric item types and API operations typically land in fabio within 24 hours of the spec being published.

### 2. Weekly dependency updates via Dependabot

Three ecosystems are watched on a weekly Monday schedule:

| Ecosystem | Scope | PR limit |
|-----------|-------|----------|
| Cargo | `/` (Rust crates) | 10 per week |
| GitHub Actions | `/` (workflow steps) | 5 per week |
| npm | `/docs` (website) | 5 per week |

When a Dependabot PR is opened and CI passes, a second workflow (`dependabot-auto-merge.yml`) automatically enables rebase-merge, so compatible patch and minor updates land without any manual interaction.

GitHub Actions references are always pinned to a full 40-character commit SHA — never to a floating tag — so even an auto-merged Actions update is supply-chain safe.

### 3. Per-PR dependency review

Every pull request targeting `main` runs `actions/dependency-review-action`. It compares the dependency graph before and after the change and blocks merge on any introduction with a **high** or **critical** severity CVE. A summary comment is posted to the PR so reviewers have full context.

[CodeQL](https://github.com/features/security) also runs weekly across the Rust source to catch security issues in the codebase itself.

## Why this matters

| | fabio | fab cli | fabric-cicd |
|--|-------|---------|-------------|
| New API endpoints | Within ~24 h (automated) | Manual release | Manual release |
| Dependency updates | Weekly + auto-merge | Manual | Manual |
| Vulnerable dep guard | Per-PR review gate | — | — |
| Build verified before merge | Yes (Copilot fixes too) | — | — |

The result is that fabio's dependency tree is almost always on the latest compatible versions, and its API surface tracks Fabric as closely as the spec repo allows.

## Verifying freshness yourself

```bash
# See when fabio was last built and its version
fabio --version

# Check the Fabric spec cursor (the last synced commit SHA)
cat .github/fabric-api-specs-cursor
```

You can also look at the [open and recently merged sync PRs](https://github.com/iemejia/fabio/pulls?q=is%3Apr+label%3Adependencies+sort%3Aupdated-desc) on GitHub to see exactly what changed and when.
