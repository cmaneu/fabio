"""Main CLI entry point for fabio.

Designed for AI agents first:
- JSON output by default (machine-parseable)
- Structured errors with codes
- Composable via stdin/stdout piping
- All params explicit (no interactive prompts)
"""

from __future__ import annotations

import json

import click

from fabio import __version__
from fabio.errors import FabioError
from fabio.output import OutputFormat


class FabioGroup(click.Group):
    """Custom group that catches FabioError and renders structured output."""

    def invoke(self, ctx: click.Context) -> None:
        try:
            super().invoke(ctx)
        except FabioError as e:
            click.echo(json.dumps(e.to_dict(), separators=(",", ":")), err=True)
            ctx.exit(1)


@click.group(cls=FabioGroup)
@click.version_option(version=__version__, prog_name="fabio")
@click.option(
    "--output",
    "-o",
    type=click.Choice(["json", "table", "plain"]),
    default="json",
    envvar="FABIO_OUTPUT",
    help="Output format (default: json).",
)
@click.option(
    "--query",
    "-q",
    default=None,
    help="Filter output fields (e.g. '[].id,displayName' or 'id').",
)
@click.option(
    "--quiet",
    is_flag=True,
    default=False,
    envvar="FABIO_QUIET",
    help="Suppress non-essential output.",
)
@click.pass_context
def main(ctx: click.Context, output: str, query: str | None, quiet: bool) -> None:
    """fabio - Manage Microsoft Fabric artifacts and data.

    Agent-first CLI: outputs structured JSON by default.
    Pipe commands together for composability.

    \b
    Examples:
        fabio workspace list
        fabio workspace list --query '[].id,displayName'
        fabio item list --workspace <id> | fabio item show --stdin
        fabio lakehouse tables --workspace <id> --item <id>
    """
    ctx.ensure_object(dict)
    ctx.obj["format"] = OutputFormat(output)
    ctx.obj["query"] = query
    ctx.obj["quiet"] = quiet


# Import and register command groups
from fabio.commands.auth import auth  # noqa: E402
from fabio.commands.item import item  # noqa: E402
from fabio.commands.lakehouse import lakehouse  # noqa: E402
from fabio.commands.notebook import notebook  # noqa: E402
from fabio.commands.warehouse import warehouse  # noqa: E402
from fabio.commands.workspace import workspace  # noqa: E402

main.add_command(auth)
main.add_command(workspace)
main.add_command(item)
main.add_command(lakehouse)
main.add_command(notebook)
main.add_command(warehouse)
