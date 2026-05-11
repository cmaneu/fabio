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
@click.option("--item", "-i", default=None, help="Name of a specific item to inspect.")
def show_workspace(name: str, item: str | None) -> None:
    """Show Fabric artifacts in a workspace, or details of a specific item."""
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

    if item is None:
        # List all artifacts.
        if not items:
            console.print(f"[yellow]No artifacts found in workspace '{name}'.[/yellow]")
            return

        table = Table(title=f"Artifacts in '{name}'")
        table.add_column("Name", style="bold")
        table.add_column("Type")
        table.add_column("ID", style="dim")

        for entry in items:
            table.add_row(
                entry.get("displayName", ""),
                entry.get("type", ""),
                entry.get("id", ""),
            )

        console.print(table)
        return

    # Find the specific item by name.
    item_match = next((i for i in items if i.get("displayName") == item), None)
    if item_match is None:
        console.print(f"[red]Item not found:[/red] '{item}' in workspace '{name}'")
        raise SystemExit(1)

    item_id = item_match["id"]

    # Fetch item details.
    item_detail = client.get(f"/workspaces/{workspace_id}/items/{item_id}")

    console.print(f"[bold]{item_detail.get('displayName', '')}[/bold]")
    console.print(f"  Type:        {item_detail.get('type', '')}")
    console.print(f"  ID:          {item_detail.get('id', '')}")
    console.print(f"  Workspace:   {name} ({workspace_id})")
    if item_detail.get("description"):
        console.print(f"  Description: {item_detail['description']}")
