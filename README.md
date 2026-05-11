# fabio

CLI tool for managing Microsoft Fabric artifacts and data.

## Installation

```bash
pip install fabio-cli
```

Or with [uv](https://docs.astral.sh/uv/):

```bash
uv tool install fabio-cli
```

## Quick start

```bash
# Sign in (opens browser)
fabio auth login

# Check who you're signed in as
fabio auth status

# Sign out
fabio auth logout
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
