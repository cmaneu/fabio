# fabio

Agent-first CLI for managing Microsoft Fabric artifacts and data.

Designed for composability: structured JSON output by default, consistent error codes, and stdin/stdout piping between commands.

## Installation

From source (requires Rust 1.85+):

```bash
cargo install --git https://github.com/iemejia/fabio.git
```

Or download pre-built binaries from the [releases page](https://github.com/iemejia/fabio/releases).

## Design Principles

- **JSON by default** -- All commands output structured JSON for machine consumption
- **Composable** -- Pipe output between commands via stdout/stdin
- **Structured errors** -- Machine-readable error codes (`AUTH_REQUIRED`, `NOT_FOUND`, etc.)
- **Explicit** -- No interactive prompts; all parameters are flags
- **Deterministic** -- Same inputs produce same outputs
- **Discoverable** -- `--help` on every command, consistent `--query` filtering

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

# Upload a file
fabio lakehouse upload --workspace <id> --id <lakehouse-id> --source data.csv --dest Files/data.csv

# Run a notebook and wait for completion
fabio notebook run --workspace <id> --id <notebook-id> --wait

# Query a data agent
fabio data-agent query --workspace <id> --id <agent-id> --prompt "What were total sales last quarter?"
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
fabio lakehouse upload       Upload a file to a lakehouse
fabio lakehouse download     Download a file from a lakehouse
fabio lakehouse load-table   Load a file into a Delta table
fabio lakehouse copy-file    Copy a file (server-side)
fabio lakehouse move-file    Move a file (copy + delete source)
fabio lakehouse delete-file  Delete a file
fabio lakehouse copy-table   Copy a table between lakehouses
fabio lakehouse move-table   Move a table (copy + delete source)
fabio lakehouse delete-table Delete a table
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
cargo clippy

# Format
cargo fmt
```

## License

MIT
