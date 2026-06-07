# fabio

Agent-first CLI for managing Microsoft Fabric artifacts and data.

Designed for composability: structured JSON output by default, consistent error codes, and stdin/stdout piping between commands. Follows the [10 principles for agent-native CLIs](https://trevinsays.com/p/10-principles-for-agent-native-clis).

## Installation

From source (requires Rust 1.85+):

```bash
cargo install --git https://github.com/iemejia/fabio.git
```

Or download pre-built binaries from the [releases page](https://github.com/iemejia/fabio/releases) (Linux, macOS, Windows — x64 and arm64).

## Design Principles

- **JSON by default** -- All commands output structured JSON for machine consumption
- **Composable** -- Pipe output between commands via stdout/stdin
- **Structured errors** -- Machine-readable error codes with hints and valid enum values
- **Non-interactive** -- No prompts; all parameters via flags/env/files
- **Safe mutations** -- `--dry-run` for destructive operations; idempotent where possible
- **Bounded responses** -- `--limit` for list commands; concise default output
- **Async-aware** -- `--wait` for long-running operations; local job ledger
- **Discoverable** -- `fabio agent-context` provides machine-readable command schema
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

## Commands

See [COMMANDS.md](COMMANDS.md) for the full list of 70 command groups and 771 subcommands.

If you are an AI agent, run `fabio agent-context` to get a machine-readable command schema with flags, types, mutability, and examples.

## Authentication

Fabio authenticates with its own dedicated Entra ID application ("Fabio CLI"). It also supports service principal and the Azure credential chain as a fallback.

```bash
# Login with Fabio CLI identity (device code flow, works on any machine)
fabio auth login

# Service principal with client secret
fabio auth login --service-principal --tenant <TENANT_ID> --client-id <CLIENT_ID> --client-secret <SECRET>

# Service principal with certificate (PEM or PFX)
fabio auth login --service-principal --tenant <TENANT_ID> --client-id <CLIENT_ID> --certificate ./cert.pem

# Service principal with federated token (OIDC, for CI/CD)
fabio auth login --service-principal --tenant <TENANT_ID> --client-id <CLIENT_ID> --federated-token-file $ACTIONS_ID_TOKEN_REQUEST_TOKEN

# Verify authentication
fabio auth status
```

Supported credential sources (in priority order):
1. Fabio CLI identity (`fabio auth login` -- recommended for interactive use)
2. Environment variables (`AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_CLIENT_SECRET`)
3. Managed Identity (when running on Azure)
4. Azure CLI (`az login`)
5. Azure Developer CLI (`azd auth login`)

## Shell Completions

Generate tab-completion scripts for your shell. Completions cover all 70 command groups, 771 subcommands, and their flags.

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

See [EXAMPLES.md](EXAMPLES.md) for usage examples covering all major workflows (lakehouse, notebooks, warehouses, real-time intelligence, semantic models, CI/CD, GitHub Actions, and more).

## Development

```bash
git clone https://github.com/iemejia/fabio.git && cd fabio

# Build
cargo build

# Run tests (unit + offline integration -- 634 tests)
cargo test

# Run E2E tests (requires live Fabric tenant -- 667 tests)
cargo test -- --ignored

# Lint (pedantic + nursery, zero warnings required)
cargo clippy --tests -- -D warnings

# Format
cargo fmt
```

### CI/CD

- GitHub Actions CI runs on 6 targets: x64 + arm64 for Linux, macOS, and Windows
- Release workflow: tag-triggered, builds 5 binaries with SHA256 checksums (Linux x64/arm64, macOS arm64, Windows x64/arm64)
- `cargo-deny` checks for security advisories and license compliance (permissive-only policy)
- Dependabot auto-merge for passing dependency updates
- CodeQL and Secret Scanning enabled

### Project Stats

- **70 command groups** with **771 subcommands**
- **1301 tests** (496 unit + 138 offline integration + 667 E2E requiring live tenant)
- **~16 MB** release binary (stripped, full LTO, panic=abort)
- Zero clippy warnings, zero unsafe code

## License

MIT
