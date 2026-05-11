"""Main CLI entry point for fabio."""

from __future__ import annotations

import click

from fabio import __version__
from fabio.commands.auth import auth


@click.group()
@click.version_option(version=__version__, prog_name="fabio")
def main() -> None:
    """Fabio - manage Microsoft Fabric artifacts and data from the command line."""


main.add_command(auth)
