# fabio

Agent-first CLI for managing Microsoft Fabric artifacts and data.

Designed for composability: structured JSON output by default, consistent error codes, and stdin/stdout piping between commands.

## Installation

With [uv](https://docs.astral.sh/uv/) (recommended):

```bash
# Install globally as a CLI tool
uv tool install git+https://github.com/iemejia/fabio.git

# Or run without installing (ephemeral)
uvx --from git+https://github.com/iemejia/fabio.git fabio workspace list
```

With pip:

```bash
pip install git+https://github.com/iemejia/fabio.git
```

## Design Principles

- **JSON by default** - All commands output structured JSON for machine consumption
- **Composable** - Pipe output between commands via stdout/stdin
- **Structured errors** - Machine-readable error codes (`AUTH_REQUIRED`, `NOT_FOUND`, etc.)
- **Explicit** - No interactive prompts; all parameters are flags
- **Deterministic** - Same inputs produce same outputs
- **Discoverable** - `--help` on every command, consistent `--query` filtering

## Quick Start

```bash
# Authenticate
fabio auth login

# List workspaces (JSON output)
fabio workspace list

# Get just IDs (plain output for scripting)
fabio workspace list -o plain

# Filter fields
fabio workspace list --query '[].id,displayName'

# List items in a workspace
fabio item list --workspace <workspace-id>

# Filter by type
fabio item list --workspace <id> --type Lakehouse

# Show a specific item
fabio item show --workspace <id> --name "MyLakehouse"

# List lakehouse tables
fabio lakehouse tables --workspace <id> --id <lakehouse-id>

# List lakehouse files recursively
fabio lakehouse files --workspace <id> --id <lakehouse-id> --path Files/raw -r
```

## Output Formats

```bash
# JSON (default) - structured envelope for agents
fabio workspace list
# {"data":[...],"count":2}

# Plain - one value per line for shell scripting
fabio workspace list -o plain
# ws-001
# ws-002

# Table - human-readable table on stderr, JSON on stdout
fabio workspace list -o table
```

## Composability

Commands produce and consume structured JSON, enabling pipelines:

```bash
# Get workspace ID, then list its items
WS_ID=$(fabio workspace show --name "Analytics" -o plain)
fabio item list --workspace "$WS_ID" --type Lakehouse
```

## Error Handling

All errors are structured JSON on stderr with machine-readable codes:

```json
{"error":{"code":"AUTH_REQUIRED","message":"Not authenticated. Run 'fabio auth login' first."}}
```

Error codes: `AUTH_REQUIRED`, `AUTH_EXPIRED`, `AUTH_FAILED`, `NOT_FOUND`, `FORBIDDEN`, `RATE_LIMITED`, `SERVER_ERROR`, `API_ERROR`, `INVALID_INPUT`, `MISSING_PARAM`, `CONFLICT`, `TIMEOUT`

## Global Options

| Flag | Env Var | Description |
|------|---------|-------------|
| `-o`, `--output` | `FABIO_OUTPUT` | Output format: `json`, `table`, `plain` |
| `-q`, `--query` | - | Filter output fields (e.g. `[].id,displayName`) |
| `--quiet` | `FABIO_QUIET` | Suppress non-essential output |

## Commands

```
fabio auth login       Sign in to Microsoft Fabric
fabio auth logout      Sign out and clear credentials
fabio auth status      Show authentication status

fabio workspace list   List all accessible workspaces
fabio workspace show   Show details for a workspace (--id or --name)

fabio item list        List items in a workspace (--workspace, --type)
fabio item show        Show item details (--workspace, --id or --name)
fabio item create      Create a new item (--workspace, --name, --type)
fabio item delete      Delete an item (--workspace, --id)

fabio lakehouse tables List tables (--workspace, --id)
fabio lakehouse files  List files (--workspace, --id, --path, -r)
```

## Development

```bash
# Clone & install in editable mode with dev dependencies
git clone <repo-url> && cd fabio
uv venv .venv && source .venv/bin/activate
uv pip install -e ".[dev]"

# Run tests
pytest

# Lint & format
ruff check src tests
ruff format src tests

# Type-check
mypy src
```

## License

MIT
