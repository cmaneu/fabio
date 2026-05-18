"""``fabio lakehouse`` command group.

Commands:
    tables - List tables in a lakehouse
    files  - List files in a lakehouse
    ls     - List directory contents (recursive)
"""

from __future__ import annotations

import click

from fabio import client
from fabio.output import output


@click.group()
def lakehouse() -> None:
    """Manage lakehouse tables and files."""


@lakehouse.command(name="tables")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "lakehouse_id", required=True, help="Lakehouse item ID.")
@click.pass_context
def list_tables(ctx: click.Context, workspace: str, lakehouse_id: str) -> None:
    """List tables in a lakehouse.

    \b
    Output fields: name, type, format, location
    Examples:
        fabio lakehouse tables --workspace <ws-id> --id <lh-id>
        fabio lakehouse tables -w <ws-id> --id <lh-id> --query '[].name'
    """
    data = client.get(f"/workspaces/{workspace}/lakehouses/{lakehouse_id}/tables")
    tables = data.get("data", [])

    output(
        ctx,
        tables,
        columns=["name", "type", "format", "location"],
        headers=["NAME", "TYPE", "FORMAT", "LOCATION"],
        plain_key="name",
    )


@lakehouse.command(name="files")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "lakehouse_id", required=True, help="Lakehouse item ID.")
@click.option("--path", "-p", default="Files", help="Directory path (default: Files).")
@click.option("--recursive", "-r", is_flag=True, default=False, help="List recursively.")
@click.pass_context
def list_files(
    ctx: click.Context,
    workspace: str,
    lakehouse_id: str,
    path: str,
    recursive: bool,
) -> None:
    """List files in a lakehouse directory.

    \b
    Output fields: name, contentLength, lastModified, isDirectory
    Examples:
        fabio lakehouse files --workspace <ws-id> --id <lh-id>
        fabio lakehouse files -w <ws-id> --id <lh-id> --path Files/raw -r
    """
    entries = client.list_onelake_files(
        workspace, lakehouse_id, directory=path, recursive=recursive
    )

    # Normalize entries: strip the directory prefix for cleaner output
    prefix = f"{path}/" if not path.endswith("/") else path
    normalized: list[dict[str, object]] = []
    for entry in entries:
        full_name = entry.get("name", "")
        rel_name = full_name[len(prefix) :] if full_name.startswith(prefix) else full_name
        if not rel_name:
            continue
        normalized.append(
            {
                "name": rel_name,
                "contentLength": entry.get("contentLength", ""),
                "lastModified": entry.get("lastModified", ""),
                "isDirectory": entry.get("isDirectory", "false") == "true",
            }
        )

    output(
        ctx,
        normalized,
        columns=["name", "contentLength", "lastModified", "isDirectory"],
        headers=["NAME", "SIZE", "MODIFIED", "DIR"],
        plain_key="name",
    )
