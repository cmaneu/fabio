# fabio

Agent-native CLI for managing Microsoft Fabric artifacts and data.

Composable by design: commands just work together with JSON output by default, errors that explain how to fix things and what to try next, and results that pipe straight into the next command. Follows the [10 principles for agent-native CLIs](https://trevinsays.com/p/10-principles-for-agent-native-clis).

Everything fabio does is achievable via the public Fabric REST APIs — `fabio` encodes the operational knowledge so coding agents can skip the trial and error, saving time, tokens, and money.

## Why fabio?

Microsoft Fabric has two official tools: [Fabric CLI](https://github.com/microsoft/fabric-cli) (`fab` — interactive Python CLI) and [fabric-cicd](https://github.com/microsoft/fabric-cicd) (Python CI/CD library). fabio is designed to be a strict superset — everything they do, fabio does too, plus capabilities neither offers.

### vs. Fabric CLI (`fab` — interactive Python CLI)

| Capability | Fabric CLI | fabio |
|---|---|---|
| Design philosophy | Interactive-first (shell with `cd`/`ls`) | Agent-native (non-interactive, structured output) |
| CI/CD deployment | Wraps fabric-cicd library | Native engine with content-hash diffing |
| Default output | Human text | JSON (machine-parseable by default) |
| Item type coverage | ~20 commands | 76 command groups |
| SQL query execution | No | Warehouse, SQL Database, Lakehouse (T-SQL via TDS) |
| KQL query execution | No | KQL Database and Eventhouse queries |
| Data Agent interaction | No | Create, configure, publish, and query Data Agents (28 subcommands: staging management API, datasource/fewshot/config CRUD, `--stage` for draft vs published, `--show-steps`) |
| Git integration | No | Full lifecycle: connect, status, commit, pull, switch branch |
| Notebook run | `job run` with `--wait`/`--timeout` | Run with `--wait`, `--timeout`, `--parameters`, cancel |
| Bulk operations | No | `item bulk-create`, `item bulk-delete` (parallel) |
| OneLake sync | No | rsync-like sync with rename detection and dedup |
| Parallel data operations | No | Upload, copy, move, sync with bounded concurrency |
| Error handling | Generic messages | Machine-readable codes with hints and valid enum values |
| Runtime | Python 3.10+ (pip install) | Single Rust binary, no runtime |

### vs. fabric-cicd (Python CI/CD library)

| Capability | fabric-cicd | fabio |
|---|---|---|
| Source directory format | `.platform` directories | Same (100% compatible) |
| Plan before apply | No (always pushes everything) | `plan` → review → `apply` |
| Skip unchanged items | No (re-uploads every definition) | SHA-256 content hash (terraform-like) |
| Rename detection | No (delete + create) | Logical ID matching (preserves item GUID) |
| Dry-run mode | No | `--dry-run` on all mutations |
| Export from workspace | No | `deploy export` |
| Local validation | No | `deploy validate` (no API calls needed) |
| Parameter scaffolding | No | `deploy init-params` (scans/diffs GUIDs automatically) |
| Output format | Python logs | JSON envelope (stdout/stderr separation) |
| Item types supported | 27 (deploy only) | 46 (deploy) + 76 command groups (full CRUD, query, run) |
| Selective filtering | Feature-flagged, limited | `--exclude-regex`, `--include-items`, `--include-folders` |
| Runtime | Python 3.9+ (pip install) | Single Rust binary, no runtime |

### What fabio adds beyond both

- **AI-native interactions** — create, configure, and query Data Agents (28 subcommands via public staging management API: datasource CRUD, few-shot management, table selection, element descriptions, `--stage` for draft vs published, reset/publish lifecycle); execute KQL for real-time intelligence, NL-to-KQL translation
- **Self-correcting error hints** — every error includes a `hint` field with the exact corrected command, valid enum values, or the logical next step so agents can retry without consulting docs
- **Self-improving** — when new Fabric REST APIs are detected, fabio auto-implements support for new commands and item types
- **Terraform-like convergence** — re-running `deploy apply` on a synced workspace produces zero API calls
- **Saved deployment plans** — `--out plan.json` then `apply --plan plan.json` (with staleness detection)
- **Workspace folder management** — infers folder hierarchy, creates/moves/deletes automatically
- **Protected type deletion guards** — Lakehouse/Warehouse/Eventhouse safe from accidental deletion
- **Post-deploy automation** — Semantic Model refresh, Environment publish, SQL endpoint polling
- **Parallel execution** — bounded-concurrency async deployment (default 8 parallel ops per type)
- **Profile management** — named profiles store default workspace, capacity, output format, and private link settings; switch contexts with `fabio profile use`

## Installation

**Linux / macOS** (installs to `~/.local/bin`):

```bash
curl -fsSL https://raw.githubusercontent.com/iemejia/fabio/main/install.sh | bash
```

**Windows** (PowerShell, installs to `%LOCALAPPDATA%\fabio`):

```powershell
irm https://raw.githubusercontent.com/iemejia/fabio/main/install.ps1 | iex
```

Or download pre-built binaries from the [releases page](https://github.com/iemejia/fabio/releases) (Linux, macOS, Windows — x64 and arm64).

**Via agent skill** (for AI agents using [GitHub Copilot CLI](https://docs.github.com/en/copilot/github-copilot-in-the-cli) or compatible platforms):

The fabio agent skill ships in-repo at `.agents/skills/fabio/`. Install it directly from the main repository:

```bash
npx skills add https://github.com/iemejia/fabio
```

Or for agents that read a local skills directory:

```bash
git clone https://github.com/iemejia/fabio ~/.agents/skills/fabio-repo
# Skills are at ~/.agents/skills/fabio-repo/.agents/skills/fabio/
```

**Docker** (multi-arch: amd64 + arm64):

```bash
docker pull ghcr.io/iemejia/fabio:latest
docker run --rm ghcr.io/iemejia/fabio --help
```

Use a specific version:

```bash
docker run --rm ghcr.io/iemejia/fabio:0.33.0 workspace list
```

Pass credentials via environment variables for non-interactive use:

```bash
docker run --rm \
  -e AZURE_TENANT_ID=<tenant-id> \
  -e AZURE_CLIENT_ID=<client-id> \
  -e AZURE_CLIENT_SECRET=<client-secret> \
  ghcr.io/iemejia/fabio workspace list --output json
```

The Docker image uses a minimal [distroless](https://github.com/GoogleContainerTools/distroless)
base (~20MB) with no shell or package manager. Authentication options inside the container:

| Method | How |
|--------|-----|
| Service principal | `-e AZURE_TENANT_ID` + `-e AZURE_CLIENT_ID` + `-e AZURE_CLIENT_SECRET` |
| Workload identity (AKS/OIDC) | `-e AZURE_TENANT_ID` + `-e AZURE_CLIENT_ID` + `-e AZURE_FEDERATED_TOKEN_FILE` + volume mount |
| Managed identity | Automatic in Azure compute (Container Apps, ACI, AKS) |

> **Note:** Azure CLI (`az login`) credentials do not work inside the container because
> `az` is not installed. Use service principal or managed identity for containerized workloads.

## Design Principles

- **JSON by default** -- All commands output structured JSON for machine consumption
- **Composable** -- Pipe output between commands via stdout/stdin
- **Structured errors** -- Machine-readable error codes with hints and valid enum values
- **Non-interactive** -- No prompts; all parameters via flags/env/files
- **Safe mutations** -- `--dry-run` for destructive operations; idempotent where possible
- **Bounded responses** -- `--limit` for list commands; concise default output
- **Async-aware** -- `--wait` for long-running operations; local job ledger
- **Discoverable** -- `fabio context agent` provides machine-readable command schema; `fabio context` provides item schemas, workflows, and best practices
- **Context-aware** -- `fabio context tenant` builds a workspace relationship graph for agent memory; exports as OWL RDF/XML or JSON-LD for Fabric Ontology import
- **Ontology round-trip** -- `fabio ontology import/export` converts between OWL (RDF/XML, JSON-LD) and Fabric format; compatible with [Ontology Playground](https://github.com/microsoft/Ontology-Playground)
- **Throttling-aware** -- Bulk/batch APIs preferred; parallel execution with rate-limit retry

## Quick Start

```bash
# 1. Sign in
fabio auth login

# 2. Create a workspace and assign compute capacity
fabio workspace create --name "sales-analytics" -o table
fabio workspace assign-capacity --id <workspace-id> --capacity <capacity-id>

# 3. Create a lakehouse for your data
fabio lakehouse create --workspace <workspace-id> --name "SalesLakehouse" -o table

# 4. Upload local CSV files (glob patterns, parallel upload)
fabio lakehouse upload --workspace <ws> --id <lh> --source "data/*.csv" --dest Files/raw/

# 5. Load a CSV into a managed Delta table
fabio lakehouse load-table --workspace <ws> --id <lh> \
  --path Files/raw/orders.csv --table orders --mode Overwrite --format Csv

# 6. Check your tables
fabio lakehouse list-tables --workspace <ws> --id <lh> -o table

# 7. Query the data via SQL
fabio warehouse query --workspace <ws> --id <warehouse-id> \
  --sql "SELECT country, SUM(revenue) as total FROM dbo.orders GROUP BY country"
```

That's it -- from sign-in to queryable Delta tables in 7 commands.

## Output Formats

```bash
# JSON (default) - structured envelope for agents
fabio workspace list
# {"data":[...],"count":2}

# Table - human-readable columns
fabio workspace list -o table

# Plain - one value per line for shell scripting
fabio workspace list -o plain
```

## Error Handling

All errors are structured JSON on stderr with machine-readable codes:

```json
{"error":{"code":"AUTH_REQUIRED","message":"Not authenticated. Run 'az login' first.","hint":"Run: az login"}}
```

Error codes: `AUTH_REQUIRED`, `FORBIDDEN`, `NOT_FOUND`, `CONFLICT`, `RATE_LIMITED`, `CAPACITY_INACTIVE`, `INVALID_INPUT`, `API_ERROR`, `TIMEOUT`, `NETWORK_ERROR`

## Global Options

| Flag | Description |
|------|-------------|
| `-o`, `--output` | Output format: `json` (default), `table`, `plain`, `csv`, `tsv` |
| `--json` | Shorthand for `--output json` |
| `-q`, `--query` | JMESPath query expression (see [jmespath.org](https://jmespath.org/)) |
| `--quiet` | Suppress all stdout output |
| `-v`, `--verbose` | Enable HTTP/LRO/auth diagnostic tracing on stderr (for debugging only) |
| `--force` | Skip confirmation prompts for destructive operations |
| `--dry-run` | Preview mutations without executing |
| `--limit` | Limit number of results for list commands |
| `--all` | Fetch all pages (auto-paginate) |
| `--continuation-token` | Resume pagination from a previous token |
| `--profile` | Use a named profile for default settings |
| `--lro-timeout` | Override default LRO polling timeout (seconds) |
| `--hard-delete` | Permanently delete (skip recycle bin) -- on item deletes |
| `--readonly` | Block all mutations at HTTP layer (env: `FABIO_READONLY`) |
| `--enable-commands` | Allowlist command groups, comma-separated (env: `FABIO_ENABLE_COMMANDS`) |
| `--disable-commands` | Denylist command groups, comma-separated (env: `FABIO_DISABLE_COMMANDS`) |
| `--wrap-untrusted` | Wrap user-authored fields with `<<<UNTRUSTED>>>` markers (env: `FABIO_WRAP_UNTRUSTED`) |

## Commands

76 command groups with 807+ subcommands covering the full Fabric REST API surface. Use the built-in introspection to discover commands:

```bash
# Compact index of all groups + subcommand names
fabio context agent

# Full details for a specific group (all flags, types, examples)
fabio context agent --group lakehouse

# Token-budget-aware: richest subset that fits within N tokens
fabio context agent --budget 4000

# Deep-dive on one command
fabio context describe lakehouse sync

# Search by keyword
fabio context find "upload"

# Multi-step workflow recipes
fabio context workflow cicd-deploy

# Best practices
fabio context best-practices throttling
```

## Authentication

Fabio authenticates with its own dedicated Entra ID application ("Fabio CLI"). It supports multiple authentication methods for both interactive and non-interactive scenarios.

```bash
# Device code flow (interactive, any platform — works in headless/SSH)
fabio auth login

# Browser-based PKCE (opens system browser; SSO on macOS with Enterprise SSO Extension)
fabio auth login --browser

# Service principal with client secret (CI/CD, automation)
fabio auth login --service-principal --tenant <TENANT_ID> --client-id <CLIENT_ID> --client-secret <SECRET>

# Service principal with certificate (PEM or PFX)
fabio auth login --service-principal --tenant <TENANT_ID> --client-id <CLIENT_ID> --certificate ./cert.pem
fabio auth login --service-principal --tenant <TENANT_ID> --client-id <CLIENT_ID> --certificate ./cert.pfx --certificate-password <PW>

# Service principal with federated token (OIDC, for GitHub Actions / Azure Pipelines)
fabio auth login --service-principal --tenant <TENANT_ID> --client-id <CLIENT_ID> --federated-token <JWT>
fabio auth login --service-principal --tenant <TENANT_ID> --client-id <CLIENT_ID> --federated-token-file <PATH>

# Windows WAM broker SSO (Windows only — uses OS-level sign-in)
fabio auth login --wam

# Verify authentication
fabio auth status
```

Supported credential sources (in priority order):
1. Fabio CLI identity (`fabio auth login` -- recommended for interactive use)
2. Environment variables (`AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_CLIENT_SECRET`)
3. Managed Identity (when running on Azure)
4. Azure CLI (`az login`)
5. Azure Developer CLI (`azd auth login`)

**Windows-specific features:**
- Token cache encrypted with DPAPI (`CryptProtectData`, user scope) — matches Azure CLI behavior
- WAM broker SSO via `--wam` flag — uses Windows OS-level sign-in, no browser needed

## Shell Completions

Generate tab-completion scripts for your shell. Completions cover all 76 command groups, 830+ subcommands, and their flags.

### Bash

```bash
# Add to ~/.bashrc
eval "$(fabio completions bash)"
```

### Zsh

```bash
# Add to ~/.zshrc
eval "$(fabio completions zsh)"
```

Or, for faster shell startup (generates a static file):

```bash
fabio completions zsh > ~/.zfunc/_fabio
# Ensure ~/.zfunc is in your fpath (add to ~/.zshrc before compinit):
# fpath=(~/.zfunc $fpath)
```

### Fish

```bash
fabio completions fish > ~/.config/fish/completions/fabio.fish
```

### PowerShell

```powershell
# Add to your $PROFILE
fabio completions powershell | Out-String | Invoke-Expression
```

Or, for persistent completions:

```powershell
# Generate and save to profile directory
fabio completions powershell > "$HOME\Documents\PowerShell\Completions\fabio.ps1"
# Source in $PROFILE:
# . "$HOME\Documents\PowerShell\Completions\fabio.ps1"
```

### Elvish

```bash
fabio completions elvish >> ~/.config/elvish/rc.elv
```

After setting up completions, restart your shell or source the configuration file. Then use `Tab` to complete commands, subcommands, and flags:

```
fabio lak<Tab>         → fabio lakehouse
fabio lakehouse <Tab>  → list  show  create  upload  ...
fabio lakehouse list --out<Tab> → --output
```

## Examples

Use `fabio context describe <group> <command>` for examples of any specific command. Use `fabio context workflow <name>` for multi-step recipes. Available workflows:

- `lakehouse-etl` -- Lakehouse + notebook + load-table + schedule
- `rti-pipeline` -- Eventhouse + KQL DB + EventStream end-to-end
- `direct-lake-report` -- Semantic model (TMDL) + report creation
- `cicd-deploy` -- Export + plan + apply with content-hash convergence
- `data-agent-setup` -- Create + datasource + few-shots + publish

## GitHub Actions

Use fabio in CI/CD workflows to deploy Fabric artifacts automatically.

### OIDC Federated Credentials (Recommended)

No long-lived secrets. Uses GitHub's OIDC token exchange:

```yaml
name: Fabric Deploy
on: [push]

permissions:
  id-token: write
  contents: read

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Azure Login (OIDC)
        uses: azure/login@v3
        with:
          client-id: ${{ secrets.AZURE_CLIENT_ID }}
          tenant-id: ${{ secrets.AZURE_TENANT_ID }}
          allow-no-subscriptions: true

      - name: Install fabio
        run: |
          ARCH=$(uname -m | sed 's/x86_64/x64/;s/aarch64/arm64/')
          curl -fsSL "https://github.com/iemejia/fabio/releases/latest/download/fabio-linux-${ARCH}.tar.gz" \
            | tar -xz -C /usr/local/bin

      - name: Deploy to Fabric
        run: |
          fabio deploy plan --source ./fabric-items/ --workspace "Production"
          fabio deploy apply --source ./fabric-items/ --workspace "Production"
```

### Service Principal with Client Secret

Simplest setup -- just set environment variables:

```yaml
name: Fabric Deploy
on: [push]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Install fabio
        run: |
          ARCH=$(uname -m | sed 's/x86_64/x64/;s/aarch64/arm64/')
          curl -fsSL "https://github.com/iemejia/fabio/releases/latest/download/fabio-linux-${ARCH}.tar.gz" \
            | tar -xz -C /usr/local/bin

      - name: Deploy to Fabric
        env:
          AZURE_CLIENT_ID: ${{ secrets.AZURE_CLIENT_ID }}
          AZURE_TENANT_ID: ${{ secrets.AZURE_TENANT_ID }}
          AZURE_CLIENT_SECRET: ${{ secrets.AZURE_CLIENT_SECRET }}
        run: |
          fabio deploy plan --source ./fabric-items/ --workspace "Production"
          fabio deploy apply --source ./fabric-items/ --workspace "Production"
```

### Agent Safety

When giving agents access to fabio, use the safety flags:

```bash
# Read-only mode: blocks all mutations at the HTTP layer
FABIO_READONLY=1 fabio lakehouse list --workspace $WS

# Command allowlist: only these groups are available
FABIO_ENABLE_COMMANDS=workspace,lakehouse,context fabio workspace list

# Combined: agent can only read, only from allowed groups
FABIO_READONLY=1 FABIO_ENABLE_COMMANDS=workspace,lakehouse fabio lakehouse list-tables --workspace $WS --id $LH

# Wrap user-authored fields to prevent prompt injection
FABIO_WRAP_UNTRUSTED=1 fabio item list --workspace $WS
# Output: {"displayName": "<<<UNTRUSTED>>>My Item<<<END_UNTRUSTED>>>", ...}
```

### MCP Server Safety

The MCP server is **read-only by default** — mutation tools are hidden unless opted in:

```bash
# Default: only read-only tools exposed (366 tools)
fabio mcp serve

# Expose mutations for specific groups only
fabio mcp serve --allow-write --allow-tool "workspace,lakehouse"

# Inspect what tools would be exposed (without starting the server)
fabio mcp serve --list-tools
fabio mcp serve --allow-write --list-tools
```

### Stable Exit Codes

Agents can branch on `$?` without parsing JSON:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Generic error (API_ERROR, INVALID_INPUT) |
| 2 | Usage error (bad syntax) |
| 3 | AUTH_REQUIRED |
| 4 | FORBIDDEN / READONLY_MODE |
| 5 | NOT_FOUND |
| 6 | CONFLICT |
| 7 | RATE_LIMITED / CAPACITY_INACTIVE |
| 8 | TIMEOUT |
| 9 | NETWORK_ERROR |

## Updating

Fabio can update itself to the latest release:

```bash
# Check if a newer version is available
fabio upgrade --check

# Update to the latest release (downloads, verifies SHA256, replaces binary)
fabio upgrade

# Install a specific version
fabio upgrade --target-version 0.23.0
```

## Development

```bash
git clone https://github.com/iemejia/fabio.git && cd fabio

# Install from source (requires Rust 1.96+)
cargo install --path .

# Build
cargo build

# Run tests (unit + offline integration -- 841 tests)
cargo test

# Run E2E tests (requires live Fabric tenant -- 721 tests)
cargo test -- --ignored

# Lint (pedantic + nursery, zero warnings required)
cargo clippy --tests -- -D warnings

# Format
cargo fmt
```

### Pre-commit hooks (prek)

The project uses [prek](https://prek.j178.dev) (a fast, Rust-native pre-commit runner) for automated local validation before each commit. Install it and set up the hooks:

```bash
# Install prek (Rust-native, no Python needed)
cargo install prek

# Install the git hooks (reads prek.toml)
prek install
```

Once installed, every `git commit` automatically runs:
1. File hygiene (trailing whitespace, EOF fixer, TOML/YAML validation, merge conflict detection, large file guard)
2. Secret detection via [gitleaks](https://github.com/gitleaks/gitleaks)
3. `cargo fmt -- --check` (format verification)
4. `cargo clippy --tests -- -D warnings` (lint with zero warnings)

Configuration: [`prek.toml`](prek.toml)

### CI/CD

- GitHub Actions CI runs on 6 targets: x64 + arm64 for Linux, macOS, and Windows
- Release workflow: tag-triggered, builds 5 binaries with SHA256 checksums (Linux x64/arm64, macOS arm64, Windows x64/arm64)
- `cargo-deny` checks for security advisories and license compliance (permissive-only policy)
- Dependabot auto-merge for passing dependency updates
- CodeQL and Secret Scanning enabled

### Project Stats

- **76 command groups** with **830+ subcommands**
- **1562 tests** (841 unit + 721 offline/E2E integration)
- Zero clippy warnings, zero unsafe code

### Supported Platforms

| OS | Architectures |
|---|---|
| Linux | x86_64, aarch64 |
| macOS | x86_64, arm64 (Apple Silicon) |
| Windows | x86_64, aarch64 |

All platforms are built and tested in CI. Pre-built binaries and Docker images (multi-arch: amd64 + arm64) are published with every release.

## License

MIT
