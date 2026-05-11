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


@workspace.command(name="show")
@click.option("--name", "-n", required=True, help="Name of the workspace to show.")
def show_workspace(name: str) -> None:
    """Show Fabric artifacts in a workspace."""
    # Resolve workspace name to ID.
    data = client.get("/workspaces")
    workspaces = data.get("value", [])

    match = next((ws for ws in workspaces if ws.get("displayName") == name), None)
    if match is None:
        console.print(f"[red]Workspace not found:[/red] {name}")
        raise SystemExit(1)

    workspace_id = match["id"]

    # Fetch items in the workspace.
    items_data = client.get(f"/workspaces/{workspace_id}/items")
    items = items_data.get("value", [])

    if not items:
        console.print(f"[yellow]No artifacts found in workspace '{name}'.[/yellow]")
        return

    table = Table(title=f"Artifacts in '{name}'")
    table.add_column("Name", style="bold")
    table.add_column("Type")
    table.add_column("ID", style="dim")

    for item in items:
        table.add_row(
            item.get("displayName", ""),
            item.get("type", ""),
            item.get("id", ""),
        )

    console.print(table)
