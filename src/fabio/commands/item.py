"""``fabio item`` command group.

Commands:
    list   - List items in a workspace
    show   - Get details of a specific item
    create - Create a new item
    delete - Delete an item
"""

from __future__ import annotations

import click

from fabio import client
from fabio.errors import ErrorCode, FabioError
from fabio.output import output


@click.group()
def item() -> None:
    """Manage Fabric items (artifacts) within workspaces."""


@item.command(name="list")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option(
    "--type",
    "-t",
    "item_type",
    default=None,
    help="Filter by item type (e.g. Lakehouse, Notebook).",
)
@click.pass_context
def list_items(ctx: click.Context, workspace: str, item_type: str | None) -> None:
    """List items in a workspace.

    \b
    Output fields: id, displayName, type, description
    Examples:
        fabio item list --workspace <id>
        fabio item list --workspace <id> --type Lakehouse
        fabio workspace list -o plain | xargs -I{} fabio item list --workspace {}
    """
    params: dict[str, str] | None = {"type": item_type} if item_type else None
    data = client.get(f"/workspaces/{workspace}/items", params=params)
    items = data.get("value", [])

    output(
        ctx,
        items,
        columns=["displayName", "id", "type"],
        headers=["NAME", "ID", "TYPE"],
        plain_key="id",
    )


@item.command(name="show")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "item_id", default=None, help="Item ID.")
@click.option("--name", "-n", default=None, help="Item name (resolved within workspace).")
@click.option(
    "--type",
    "-t",
    "item_type",
    default=None,
    help="Item type (helps disambiguate names).",
)
@click.pass_context
def show_item(
    ctx: click.Context,
    workspace: str,
    item_id: str | None,
    name: str | None,
    item_type: str | None,
) -> None:
    """Show details of a specific item.

    \b
    Provide --id or --name. If name is ambiguous, also provide --type.
    Example: fabio item show --workspace <id> --name "SalesReport" --type Report
    """
    if item_id is None and name is None:
        raise FabioError(ErrorCode.MISSING_PARAM, "Provide --id or --name.")

    if item_id is None:
        item_id = _resolve_item_name(workspace, name, item_type)  # type: ignore[arg-type]

    data = client.get(f"/workspaces/{workspace}/items/{item_id}")
    output(ctx, data, plain_key="id")


@item.command(name="create")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--name", "-n", required=True, help="Display name for the new item.")
@click.option(
    "--type",
    "-t",
    "item_type",
    required=True,
    help="Item type (e.g. Lakehouse, Notebook).",
)
@click.option("--description", "-d", default=None, help="Optional description.")
@click.pass_context
def create_item(
    ctx: click.Context,
    workspace: str,
    name: str,
    item_type: str,
    description: str | None,
) -> None:
    """Create a new item in a workspace.

    \b
    Example: fabio item create --workspace <id> --name "MyLakehouse" --type Lakehouse
    """
    body: dict[str, object] = {"displayName": name, "type": item_type}
    if description:
        body["description"] = description

    data = client.post(f"/workspaces/{workspace}/items", body=body)
    output(ctx, data, plain_key="id")


@item.command(name="delete")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "item_id", required=True, help="Item ID to delete.")
@click.pass_context
def delete_item(ctx: click.Context, workspace: str, item_id: str) -> None:
    """Delete an item from a workspace.

    \b
    Example: fabio item delete --workspace <id> --id <item-id>
    """
    client.delete(f"/workspaces/{workspace}/items/{item_id}")
    output(ctx, {"id": item_id, "status": "deleted"}, plain_key="id")


def _resolve_item_name(workspace_id: str, name: str, item_type: str | None) -> str:
    """Resolve an item display name to its ID within a workspace."""
    params: dict[str, str] | None = {"type": item_type} if item_type else None
    data = client.get(f"/workspaces/{workspace_id}/items", params=params)
    items = data.get("value", [])

    matches = [i for i in items if i.get("displayName") == name]

    if not matches:
        raise FabioError(
            ErrorCode.NOT_FOUND,
            f"Item not found: '{name}' in workspace {workspace_id}",
        )
    if len(matches) > 1:
        types = [m.get("type") for m in matches]
        raise FabioError(
            ErrorCode.CONFLICT,
            f"Multiple items named '{name}' (types: {types}). Use --id or add --type.",
        )

    return matches[0]["id"]
