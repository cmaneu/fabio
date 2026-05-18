"""Structured output system for fabio CLI.

Every command produces a consistent envelope:

    Success (list):  {"data": [...], "count": N}
    Success (item):  {"data": {...}}
    Error:           {"error": {"code": "ERROR_CODE", "message": "..."}}

Output formats:
    json   - Raw JSON to stdout (default, for agents/piping)
    table  - Human-readable table on stderr, JSON on stdout
    plain  - Minimal text output for simple scripting
"""

from __future__ import annotations

import json
import sys
from enum import Enum
from typing import Any

import click


class OutputFormat(str, Enum):
    """Supported output formats."""

    JSON = "json"
    TABLE = "table"
    PLAIN = "plain"


def _serialize(obj: Any) -> str:
    """Serialize to compact JSON (no trailing newline for piping)."""
    return json.dumps(obj, separators=(",", ":"), ensure_ascii=False)


def _serialize_pretty(obj: Any) -> str:
    """Serialize to indented JSON for readability."""
    return json.dumps(obj, indent=2, ensure_ascii=False)


def render_json(data: Any, *, count: int | None = None) -> str:
    """Build the standard JSON envelope string.

    Parameters
    ----------
    data:
        The payload (list or dict).
    count:
        If data is a list, include count in envelope.
    """
    if isinstance(data, list):
        count_val = count if count is not None else len(data)
        envelope: dict[str, Any] = {"data": data, "count": count_val}
    else:
        envelope = {"data": data}
    return _serialize(envelope)


def render_table(
    data: list[dict[str, Any]],
    columns: list[str],
    *,
    headers: list[str] | None = None,
) -> str:
    """Render a list of dicts as an aligned text table.

    Parameters
    ----------
    data:
        List of row dicts.
    columns:
        Keys to extract from each row.
    headers:
        Display headers (defaults to column names).
    """
    display_headers = headers or columns

    # Calculate column widths
    widths = [len(h) for h in display_headers]
    rows: list[list[str]] = []
    for item in data:
        row = [str(item.get(col, "")) for col in columns]
        rows.append(row)
        for i, cell in enumerate(row):
            widths[i] = max(widths[i], len(cell))

    # Build output
    lines: list[str] = []
    header_line = "  ".join(h.ljust(w) for h, w in zip(display_headers, widths, strict=True))
    lines.append(header_line)
    lines.append("  ".join("-" * w for w in widths))
    for row in rows:
        lines.append("  ".join(cell.ljust(w) for cell, w in zip(row, widths, strict=True)))

    return "\n".join(lines)


def render_plain(data: Any, key: str = "id") -> str:
    """Render a minimal plain-text output (one value per line).

    For lists: outputs one `key` value per line.
    For dicts: outputs key=value pairs.
    """
    if isinstance(data, list):
        return "\n".join(str(item.get(key, "")) for item in data)
    if isinstance(data, dict):
        return "\n".join(f"{k}={v}" for k, v in data.items())
    return str(data)


def output(
    ctx: click.Context,
    data: Any,
    *,
    columns: list[str] | None = None,
    headers: list[str] | None = None,
    plain_key: str = "id",
) -> None:
    """Write structured output according to the current format setting.

    This is the primary function commands should call to produce output.

    Parameters
    ----------
    ctx:
        Click context (carries format setting from global options).
    data:
        Payload to output.
    columns:
        For table format - which keys to display.
    headers:
        For table format - display headers.
    plain_key:
        For plain format - which key to output per line.
    """
    fmt = ctx.obj.get("format", OutputFormat.JSON) if ctx.obj else OutputFormat.JSON
    query = ctx.obj.get("query") if ctx.obj else None

    # Apply query filter if specified
    if query:
        data = _apply_query(data, query)

    if fmt == OutputFormat.JSON:
        click.echo(render_json(data))
    elif fmt == OutputFormat.TABLE:
        # JSON always goes to stdout for piping; table goes to stderr for humans
        click.echo(render_json(data))
        if isinstance(data, list) and columns:
            table_str = render_table(data, columns, headers=headers)
            click.echo(table_str, err=True)
        elif isinstance(data, dict):
            pretty = _serialize_pretty(data)
            click.echo(pretty, err=True)
    elif fmt == OutputFormat.PLAIN:
        click.echo(render_plain(data, key=plain_key))


def output_error(ctx: click.Context, code: str, message: str, *, exit_code: int = 1) -> None:
    """Write a structured error and exit.

    Parameters
    ----------
    ctx:
        Click context.
    code:
        Machine-readable error code (e.g. "AUTH_REQUIRED").
    message:
        Human-readable message.
    exit_code:
        Process exit code.
    """
    envelope = {"error": {"code": code, "message": message}}
    click.echo(_serialize(envelope), err=True)
    ctx.exit(exit_code)


def _apply_query(data: Any, query: str) -> Any:
    """Apply a simple dot-notation query to extract nested fields.

    Supports:
        "field"         -> data[field] (for dicts)
        "[].field"      -> [item[field] for item in data] (for lists)
        "field1,field2" -> project only those fields

    This is intentionally simple. For full JMESPath, users can pipe to `jq`.
    """
    if not query:
        return data

    # List projection: [].field or [].field1,field2
    if query.startswith("[]."):
        fields = query[3:].split(",")
        if isinstance(data, list):
            return [{f: item.get(f) for f in fields if f in item} for item in data]
        return data

    # Multi-field projection for dicts
    if "," in query:
        fields = query.split(",")
        if isinstance(data, dict):
            return {f: data[f] for f in fields if f in data}
        if isinstance(data, list):
            return [{f: item.get(f) for f in fields if f in item} for item in data]
        return data

    # Single field access
    if isinstance(data, dict):
        return data.get(query, data)
    if isinstance(data, list):
        return [item.get(query) for item in data if isinstance(item, dict)]

    return data


def read_stdin_json() -> Any | None:
    """Read JSON from stdin if data is being piped in.

    Returns None if stdin is a TTY (interactive).
    """
    if sys.stdin.isatty():
        return None
    try:
        raw = sys.stdin.read()
        if not raw.strip():
            return None
        parsed = json.loads(raw)
        # Unwrap our envelope format
        if isinstance(parsed, dict) and "data" in parsed:
            return parsed["data"]
        return parsed
    except (json.JSONDecodeError, OSError):
        return None
