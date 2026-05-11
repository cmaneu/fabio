"""``fabio workspace`` command group."""

from __future__ import annotations

import click
from rich.console import Console
from rich.table import Table

from fabio import client

console = Console()


@click.group()
def workspace() -> None:
    """Manage Microsoft Fabric workspaces."""


@workspace.command(name="list")
def list_workspaces() -> None:
    """List all accessible workspaces."""
    data = client.get("/workspaces")
    workspaces = data.get("value", [])

    if not workspaces:
        console.print("[yellow]No workspaces found.[/yellow]")
        return

    table = Table(title="Workspaces")
    table.add_column("Name", style="bold")
    table.add_column("ID", style="dim")
    table.add_column("Type")
    table.add_column("Capacity ID", style="dim")

    for ws in workspaces:
        table.add_row(
            ws.get("displayName", ""),
            ws.get("id", ""),
            ws.get("type", ""),
            ws.get("capacityId", ""),
        )

    console.print(table)
