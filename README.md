# fabio

Agent-first CLI for managing Microsoft Fabric artifacts and data.

Designed for composability: structured JSON output by default, consistent error codes, and stdin/stdout piping between commands. Follows the [10 principles for agent-native CLIs](https://trevinsays.com/p/10-principles-for-agent-native-clis).

## Installation

From source (requires Rust 1.85+):

```bash
cargo install --git https://github.com/iemejia/fabio.git
```

Or download pre-built binaries from the [releases page](https://github.com/iemejia/fabio/releases).

## Design Principles

- **JSON by default** -- All commands output structured JSON for machine consumption
- **Composable** -- Pipe output between commands via stdout/stdin
- **Structured errors** -- Machine-readable error codes with hints and valid enum values
- **Non-interactive** -- No prompts; all parameters via flags/env/files
- **Safe mutations** -- `--dry-run` for destructive operations; idempotent where possible
- **Bounded responses** -- `--limit` for list commands; concise default output
- **Async-aware** -- `--wait` for long-running operations; local job ledger
- **Discoverable** -- `fabio agent-context` provides machine-readable command schema

## Quick Start

```bash
# Authenticate (uses az login credentials)
fabio auth status

# List workspaces (JSON output)
fabio workspace list

# Table format for humans
fabio workspace list -o table

# Field projection
fabio workspace list --query displayName

# List items in a workspace
fabio item list --workspace <workspace-id>

# List lakehouse tables
fabio lakehouse tables --workspace <id> --id <lakehouse-id>

# Upload files with glob patterns (parallel)
fabio lakehouse upload --workspace <id> --id <lakehouse-id> --source "data/*.csv" --dest Files/

# Sync files between lakehouses (copies new/modified only)
fabio lakehouse sync --workspace <src-ws> --id <src-lh> --dest-workspace <dst-ws> --dest-id <dst-lh>

# Run a notebook and wait for completion
fabio notebook run --workspace <id> --id <notebook-id> --wait

# Query a data agent
fabio data-agent query --workspace <id> --id <agent-id> --prompt "What were total sales last quarter?"

# Check Git status for a workspace
fabio git status --workspace <id>

# Machine-readable command schema for AI agents
fabio agent-context
```

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
{"error":{"code":"AUTH_REQUIRED","message":"Not authenticated. Run 'az login' first."}}
```

Error codes: `AUTH_REQUIRED`, `NOT_FOUND`, `FORBIDDEN`, `RATE_LIMITED`, `CAPACITY_INACTIVE`, `API_ERROR`, `NETWORK_ERROR`, `TIMEOUT`

## Global Options

| Flag | Description |
|------|-------------|
| `-o`, `--output` | Output format: `json` (default), `table`, `plain` |
| `-q`, `--query` | Field projection (dot-notation extraction) |
| `--quiet` | Suppress all stdout output |
| `--profile` | Use a named profile for default settings |
| `--dry-run` | Preview mutations without executing |
| `--limit` | Limit number of results for list commands |

## Commands

```
fabio auth status            Show authentication status
fabio auth login             Sign in to Microsoft Fabric
fabio auth logout            Sign out and clear credentials

fabio workspace list         List all accessible workspaces
fabio workspace show         Show workspace details
fabio workspace create       Create a new workspace
fabio workspace delete       Delete a workspace
fabio workspace assign-capacity  Assign a capacity to a workspace

fabio item list              List items in a workspace
fabio item show              Show item details
fabio item create            Create a new item
fabio item delete            Delete an item
fabio item copy              Copy an item between workspaces
fabio item move              Move an item between workspaces

fabio lakehouse tables       List tables in a lakehouse
fabio lakehouse files        List files in a lakehouse
fabio lakehouse upload       Upload files (supports glob patterns, parallel)
fabio lakehouse download     Download a file from a lakehouse
fabio lakehouse load-table   Load a file into a Delta table
fabio lakehouse copy-file    Copy files (supports glob patterns, parallel)
fabio lakehouse move-file    Move files (supports glob patterns, parallel)
fabio lakehouse delete-file  Delete a file
fabio lakehouse copy-table   Copy a table between lakehouses
fabio lakehouse move-table   Move a table (copy + delete source)
fabio lakehouse delete-table Delete a table
fabio lakehouse sync         Sync files between lakehouses (ETag/MD5)
fabio lakehouse create-shortcut  Create a shortcut
fabio lakehouse get-shortcut     Get shortcut details
fabio lakehouse delete-shortcut  Delete a shortcut

fabio notebook create        Create a new notebook
fabio notebook get-definition  Get notebook source code
fabio notebook run           Run a notebook (--wait to block until done)
fabio notebook status        Check run status
fabio notebook stop          Stop a running notebook
fabio notebook delete        Delete a notebook

fabio warehouse list         List warehouses in a workspace
fabio warehouse show         Show warehouse details
fabio warehouse query        Execute SQL (--sql, @file, or stdin)

fabio data-agent list        List data agents in a workspace
fabio data-agent show        Show data agent details
fabio data-agent create      Create a new data agent
fabio data-agent update      Update name/description
fabio data-agent delete      Delete a data agent
fabio data-agent query       Chat with a published data agent

fabio git status             Show workspace Git status
fabio git commit             Commit workspace changes to remote
fabio git pull               Pull remote changes into workspace
fabio git connect            Connect a workspace to a Git repo
fabio git disconnect         Disconnect a workspace from Git
fabio git initialize         Initialize Git connection after connect
fabio git switch             Switch to a different branch
fabio git connection show    Show Git connection details
fabio git credentials show   Show Git credentials configuration
fabio git credentials update Update Git credentials

fabio ontology list          List ontologies in a workspace
fabio ontology show          Show ontology details
fabio ontology create        Create an ontology
fabio ontology update        Update ontology properties
fabio ontology delete        Delete an ontology
fabio ontology get-definition   Get ontology definition
fabio ontology update-definition Update ontology definition

fabio profile save           Save a named profile
fabio profile use            Set the active profile
fabio profile list           List saved profiles
fabio profile show           Show profile details
fabio profile delete         Delete a profile

fabio jobs list              List recent jobs from local ledger
fabio jobs get               Get details of a specific job
fabio jobs prune             Remove completed/failed jobs

fabio feedback send          Record feedback about CLI friction
fabio feedback list          List recorded feedback entries

fabio agent-context          Machine-readable command schema for AI agents
```

## Authentication

Fabio uses the Azure credential chain (`az login` or environment credentials). No built-in token storage -- it delegates to the Azure CLI credential cache.

```bash
# Login via Azure CLI first
az login

# Verify fabio can authenticate
fabio auth status
```

## Development

```bash
git clone https://github.com/iemejia/fabio.git && cd fabio

# Build
cargo build

# Run tests (unit tests)
cargo test

# Run E2E tests (requires live Fabric tenant)
cargo test -- --ignored

# Lint
cargo clippy --tests -- -D warnings

# Format
cargo fmt
```

## License

MIT
