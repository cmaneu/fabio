"""``fabio workspace`` command group.

Commands:
    list              - List all accessible workspaces
    show              - Get details for a specific workspace
    create            - Create a new workspace
    delete            - Delete a workspace
    assign-capacity   - Assign a workspace to a different capacity
"""

from __future__ import annotations

import click

from fabio import client
from fabio.errors import ErrorCode, FabioError
from fabio.output import output


@click.group()
def workspace() -> None:
    """Manage Fabric workspaces."""


@workspace.command(name="list")
@click.pass_context
def list_workspaces(ctx: click.Context) -> None:
    """List all accessible workspaces.

    \b
    Output fields: id, displayName, type, capacityId
    Example: fabio workspace list --query '[].id,displayName'
    """
    data = client.get("/workspaces")
    workspaces = data.get("value", [])

    output(
        ctx,
        workspaces,
        columns=["displayName", "id", "type", "capacityId"],
        headers=["NAME", "ID", "TYPE", "CAPACITY_ID"],
        plain_key="id",
    )


@workspace.command(name="show")
@click.option("--id", "workspace_id", default=None, help="Workspace ID.")
@click.option("--name", "-n", default=None, help="Workspace name (resolved to ID).")
@click.pass_context
def show_workspace(ctx: click.Context, workspace_id: str | None, name: str | None) -> None:
    """Show details for a specific workspace.

    \b
    Provide either --id or --name. If both, --id takes precedence.
    Example: fabio workspace show --name "My Workspace"
    """
    if workspace_id is None and name is None:
        raise FabioError(ErrorCode.MISSING_PARAM, "Provide --id or --name.")

    if workspace_id is None:
        # Resolve name to ID
        workspace_id = _resolve_workspace_name(name)  # type: ignore[arg-type]

    data = client.get(f"/workspaces/{workspace_id}")
    output(ctx, data, plain_key="id")


@workspace.command(name="create")
@click.option("--name", "-n", required=True, help="Workspace display name.")
@click.option("--capacity", "-c", default=None, help="Capacity ID to assign.")
@click.option("--description", "-d", default=None, help="Optional description.")
@click.pass_context
def create_workspace(
    ctx: click.Context,
    name: str,
    capacity: str | None,
    description: str | None,
) -> None:
    """Create a new Fabric workspace.

    \b
    Example: fabio workspace create --name "My Analytics" --capacity <cap-id>
    """
    body: dict[str, object] = {"displayName": name}
    if capacity:
        body["capacityId"] = capacity
    if description:
        body["description"] = description

    data = client.post("/workspaces", body=body)
    output(ctx, data, plain_key="id")


@workspace.command(name="delete")
@click.option("--id", "workspace_id", required=True, help="Workspace ID to delete.")
@click.pass_context
def delete_workspace(ctx: click.Context, workspace_id: str) -> None:
    """Delete a workspace.

    \b
    Example: fabio workspace delete --id <workspace-id>
    """
    client.delete(f"/workspaces/{workspace_id}")
    output(ctx, {"id": workspace_id, "status": "deleted"}, plain_key="id")


def _resolve_workspace_name(name: str) -> str:
    """Resolve a workspace display name to its ID.

    Raises FabioError if not found or ambiguous.
    """
    data = client.get("/workspaces")
    workspaces = data.get("value", [])

    matches = [ws for ws in workspaces if ws.get("displayName") == name]

    if not matches:
        raise FabioError(
            ErrorCode.NOT_FOUND,
            f"Workspace not found: '{name}'",
        )
    if len(matches) > 1:
        ids = [m["id"] for m in matches]
        raise FabioError(
            ErrorCode.CONFLICT,
            f"Multiple workspaces named '{name}': {ids}. Use --id instead.",
        )

    return str(matches[0]["id"])


@workspace.command(name="assign-capacity")
@click.option("--id", "workspace_id", required=True, help="Workspace ID.")
@click.option("--capacity", "-c", required=True, help="Target capacity ID.")
@click.pass_context
def assign_capacity(ctx: click.Context, workspace_id: str, capacity: str) -> None:
    """Assign a workspace to a different Fabric capacity.

    \b
    Example: fabio workspace assign-capacity --id <ws-id> -c <capacity-id>
    """
    client.post(
        f"/workspaces/{workspace_id}/assignToCapacity",
        body={"capacityId": capacity},
    )
    output(
        ctx,
        {"id": workspace_id, "capacityId": capacity, "status": "assigned"},
        plain_key="id",
    )
