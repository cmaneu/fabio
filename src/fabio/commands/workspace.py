"""``fabio workspace`` command group."""

from __future__ import annotations

import click
from rich.console import Console
from rich.table import Table

from fabio import client

console = Console()


def _show_lakehouse_contents(workspace_id: str, lakehouse_id: str, title: str) -> None:
    """Fetch and display tables and files for a lakehouse."""
    # Tables
    tables_data = client.get(
        f"/workspaces/{workspace_id}/lakehouses/{lakehouse_id}/tables"
    )
    tables = tables_data.get("data", [])

    table = Table(title=f"Tables in '{title}'")
    table.add_column("Name", style="bold")
    table.add_column("Type")
    table.add_column("Format")
    table.add_column("Location", style="dim")

    if tables:
        for t in tables:
            table.add_row(
                t.get("name", ""),
                t.get("type", ""),
                t.get("format", ""),
                t.get("location", ""),
            )
    else:
        table.add_row("(empty)", "", "", "")

    console.print(table)
    console.print()

    # Files (via OneLake DFS API)
    files = client.list_onelake_files(workspace_id, lakehouse_id)

    ftable = Table(title=f"Files in '{title}'")
    ftable.add_column("Name", style="bold")
    ftable.add_column("Size")
    ftable.add_column("Last Modified")
    ftable.add_column("Directory", style="dim")

    if files:
        for f in files:
            name = f.get("name", "").rsplit("/", 1)[-1]
            is_dir = f.get("isDirectory", "false")
            size = str(f.get("contentLength", "")) if is_dir != "true" else "-"
            ftable.add_row(
                name,
                size,
                f.get("lastModified", ""),
                "yes" if is_dir == "true" else "",
            )
    else:
        ftable.add_row("(empty)", "", "", "")

    console.print(ftable)


def _show_directory(workspace_id: str, lakehouse_id: str, directory: str) -> None:
    """List all contents of a directory in a lakehouse (recursively)."""
    entries = client.list_onelake_files(
        workspace_id, lakehouse_id, directory=directory, recursive=True
    )

    if not entries:
        console.print(f"[yellow]No files found in '{directory}'.[/yellow]")
        return

    # Strip the directory prefix for cleaner display.
    prefix = f"{directory}/" if not directory.endswith("/") else directory

    table = Table(title=f"Contents of '{directory}'")
    table.add_column("Name", style="bold")
    table.add_column("Size")
    table.add_column("Last Modified")
    table.add_column("Type", style="dim")

    for entry in entries:
        full_name = entry.get("name", "")
        # Show path relative to the listed directory.
        rel_name = full_name[len(prefix):] if full_name.startswith(prefix) else full_name
        if not rel_name:
            continue
        is_dir = entry.get("isDirectory", "false")
        size = str(entry.get("contentLength", "")) if is_dir != "true" else "-"
        table.add_row(
            rel_name,
            size,
            entry.get("lastModified", ""),
            "dir" if is_dir == "true" else "file",
        )

    console.print(table)


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
@click.option("--type", "-t", "item_type", default=None, help="Filter artifacts by type.")
@click.option("--dir", "-d", "dir_path", default=None, help="Directory path to list within a lakehouse.")
def show_workspace(name: str, item: str | None, item_type: str | None, dir_path: str | None) -> None:
    """Show Fabric artifacts in a workspace, or details of a specific item."""
    if dir_path is not None and item is None:
        console.print("[red]--dir requires --item to specify a lakehouse.[/red]")
        raise SystemExit(1)

    # Resolve workspace name to ID.
    data = client.get("/workspaces")
    workspaces = data.get("value", [])

    match = next((ws for ws in workspaces if ws.get("displayName") == name), None)
    if match is None:
        console.print(f"[red]Workspace not found:[/red] {name}")
        raise SystemExit(1)

    workspace_id = match["id"]

    # Fetch items in the workspace.
    params: dict[str, str] | None = {"type": item_type} if item_type else None
    items_data = client.get(f"/workspaces/{workspace_id}/items", params=params)
    items = items_data.get("value", [])

    if item is None:
        # List all artifacts.
        if not items:
            console.print(f"[yellow]No artifacts found in workspace '{name}'.[/yellow]")
            return

        # If type is Lakehouse, show tables and files for each lakehouse.
        if item_type and item_type.lower() == "lakehouse":
            for lakehouse in items:
                lh_name = lakehouse.get("displayName", "")
                lh_id = lakehouse.get("id", "")
                _show_lakehouse_contents(workspace_id, lh_id, lh_name)
                console.print()
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

    # If the item is a Lakehouse, also list its tables and files.
    if item_detail.get("type", "").lower() == "lakehouse":
        if dir_path is not None:
            # List contents of a specific directory.
            directory = f"Files/{dir_path}" if dir_path else "Files"
            _show_directory(workspace_id, item_id, directory)
        else:
            console.print()
            _show_lakehouse_contents(workspace_id, item_id, item_detail.get("displayName", ""))
