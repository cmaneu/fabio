"""``fabio lakehouse`` command group.

Commands:
    tables          - List tables in a lakehouse
    files           - List files in a lakehouse
    upload          - Upload a local file to a lakehouse
    download        - Download a file from a lakehouse
    load-table      - Create/load a table from a file in the lakehouse
    copy-file       - Server-side copy a file between lakehouses
    delete-file     - Delete a file from a lakehouse
    move-file       - Server-side move a file between lakehouses
    delete-table    - Delete a Delta table from a lakehouse
    copy-table      - Server-side copy a table between lakehouses
    move-table      - Server-side move a table between lakehouses
    create-shortcut - Create a OneLake/ADLS/S3 shortcut in a lakehouse
    get-shortcut    - Get details of a shortcut
    delete-shortcut - Delete a shortcut
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import click

from fabio import client
from fabio.errors import ErrorCode, FabioError
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
    # OneLake DFS API has a "virtual lakehouse-in-lakehouse" behavior:
    # directory=X returns paths prefixed with X/ where the actual contents
    # are doubled (e.g. directory=Files returns Files/Files/myfile.csv for
    # a file at Files/myfile.csv). We must always use recursive=true to see
    # actual file content and then filter client-side.
    entries = client.list_onelake_files(
        workspace, lakehouse_id, directory=path, recursive=True
    )

    # The real contents of path X are at prefix X/X/ due to the virtual view
    # e.g., for path="Files", actual files appear as "Files/Files/data.csv"
    path_stripped = path.rstrip("/")
    content_prefix = f"{path_stripped}/{path_stripped}/"
    # Also handle subdirectory paths like "Files/raw" -> "Files/raw/raw/"
    # Actually the pattern is: directory=X → contents at X/<basename(X)>/
    # For "Files" → "Files/Files/", for "Tables" → "Tables/Tables/"
    # But for subpaths like "Files/raw", we need to check empirically.
    # The safe approach: use the doubled-prefix for top-level lakehouse dirs
    # and fall back to single-prefix stripping otherwise.
    top_level_dirs = {"Files", "Tables", "Functions", "TableMaintenance"}
    real_prefix = (
        content_prefix if path_stripped in top_level_dirs else f"{path_stripped}/"
    )

    normalized: list[dict[str, object]] = []
    for entry in entries:
        full_name = entry.get("name", "")
        if not full_name.startswith(real_prefix):
            continue
        rel_name = full_name[len(real_prefix):]
        if not rel_name:
            continue
        # If not recursive mode, only show direct children (no / in rel_name)
        if not recursive and "/" in rel_name:
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


@lakehouse.command(name="upload")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "lakehouse_id", required=True, help="Lakehouse item ID.")
@click.option("--source", "-s", required=True, help="Local file path to upload.")
@click.option(
    "--dest",
    "-d",
    default=None,
    help="Destination path in lakehouse (default: Files/<filename>).",
)
@click.pass_context
def upload_file(
    ctx: click.Context,
    workspace: str,
    lakehouse_id: str,
    source: str,
    dest: str | None,
) -> None:
    """Upload a local file to a lakehouse.

    \b
    Examples:
        fabio lakehouse upload -w <ws-id> --id <lh-id> --source data.csv
        fabio lakehouse upload -w <ws-id> --id <lh-id> -s data.csv -d Files/raw/data.csv
    """
    src_path = Path(source)
    if not src_path.exists():
        raise FabioError(ErrorCode.INVALID_INPUT, f"Source file not found: {source}")

    if dest is None:
        dest = f"Files/{src_path.name}"

    content = src_path.read_bytes()
    client.upload_onelake_file(workspace, lakehouse_id, dest, content)

    output(
        ctx,
        {
            "status": "uploaded",
            "source": source,
            "destination": dest,
            "size": len(content),
            "workspace": workspace,
            "lakehouse": lakehouse_id,
        },
        plain_key="destination",
    )


@lakehouse.command(name="download")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "lakehouse_id", required=True, help="Lakehouse item ID.")
@click.option("--path", "-p", required=True, help="File path in lakehouse to download.")
@click.option(
    "--dest",
    "-d",
    default=None,
    help="Local destination path (default: current dir/<filename>).",
)
@click.pass_context
def download_file(
    ctx: click.Context,
    workspace: str,
    lakehouse_id: str,
    path: str,
    dest: str | None,
) -> None:
    """Download a file from a lakehouse to local disk.

    \b
    Examples:
        fabio lakehouse download -w <ws-id> --id <lh-id> --path Files/data.csv
        fabio lakehouse download -w <ws-id> --id <lh-id> -p Files/data.csv -d ./out.csv
    """
    content = client.download_onelake_file(workspace, lakehouse_id, path)

    if dest is None:
        filename = path.rsplit("/", 1)[-1]
        dest = filename

    dest_path = Path(dest)
    dest_path.parent.mkdir(parents=True, exist_ok=True)
    dest_path.write_bytes(content)

    output(
        ctx,
        {
            "status": "downloaded",
            "source": path,
            "destination": str(dest_path),
            "size": len(content),
        },
        plain_key="destination",
    )


@lakehouse.command(name="load-table")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "lakehouse_id", required=True, help="Lakehouse item ID.")
@click.option("--table", "-t", required=True, help="Table name to create/load.")
@click.option(
    "--path",
    "-p",
    required=True,
    help="Relative path in lakehouse (e.g. Files/data.csv).",
)
@click.option(
    "--format",
    "file_format",
    default=None,
    help="File extension hint (csv, parquet, json).",
)
@click.option(
    "--mode",
    "-m",
    default="Overwrite",
    type=click.Choice(["Overwrite", "Append"]),
    help="Load mode (default: Overwrite).",
)
@click.option("--header/--no-header", default=True, help="CSV has header row.")
@click.option("--delimiter", default=",", help="CSV delimiter (default: ',').")
@click.pass_context
def load_table_cmd(
    ctx: click.Context,
    workspace: str,
    lakehouse_id: str,
    table: str,
    path: str,
    file_format: str | None,
    mode: str,
    header: bool,
    delimiter: str,
) -> None:
    """Load a file into a lakehouse table.

    \b
    Creates a Delta table from a file already in the lakehouse.
    Examples:
        fabio lakehouse load-table -w <ws-id> --id <lh-id> -t orders -p Files/orders.csv
        fabio lakehouse load-table -w <ws-id> --id <lh-id> -t sales \
            -p Files/sales.parquet --format parquet
    """
    # Auto-detect format from path if not specified
    if file_format is None:
        suffix = path.rsplit(".", 1)[-1].lower() if "." in path else ""
        if suffix in ("csv", "parquet", "json", "avro"):
            file_format = suffix

    # Build formatOptions with format key (PascalCase value)
    format_options: dict[str, str] | None = None
    if file_format:
        format_options = {"format": file_format.capitalize()}
        if file_format == "csv":
            format_options["header"] = str(header).lower()
            format_options["delimiter"] = delimiter

    data = client.load_table(
        workspace,
        lakehouse_id,
        table,
        path,
        file_extension=file_format,
        format_options=format_options,
        mode=mode,
    )

    # The load-table API may return 200 with empty body or operation info
    result = data if data else {}
    result.update(
        {
            "status": "loaded",
            "table": table,
            "source": path,
            "mode": mode,
        }
    )
    output(ctx, result, plain_key="table")


@lakehouse.command(name="copy-file")
@click.option("--source-workspace", "-sw", required=True, help="Source workspace ID.")
@click.option("--source-id", "-si", required=True, help="Source lakehouse item ID.")
@click.option(
    "--source-path", "-sp", required=True, help="Source file path (e.g. Files/data.csv)."
)
@click.option("--dest-workspace", "-dw", required=True, help="Destination workspace ID.")
@click.option("--dest-id", "-di", required=True, help="Destination lakehouse item ID.")
@click.option(
    "--dest-path",
    "-dp",
    default=None,
    help="Destination file path (default: same as source path).",
)
@click.pass_context
def copy_file(
    ctx: click.Context,
    source_workspace: str,
    source_id: str,
    source_path: str,
    dest_workspace: str,
    dest_id: str,
    dest_path: str | None,
) -> None:
    """Copy a file between lakehouses via server-side copy.

    \b
    Copies are performed server-side (data never transits through the client).
    Works across workspaces. Supports any file in the lakehouse (Files/ or Tables/).

    \b
    Examples:
        fabio lakehouse copy-file \\
            -sw <src-ws> -si <src-lh> -sp Files/data.csv \\
            -dw <dest-ws> -di <dest-lh>

        fabio lakehouse copy-file \\
            -sw <src-ws> -si <src-lh> -sp Files/raw/input.parquet \\
            -dw <dest-ws> -di <dest-lh> -dp Files/staging/input.parquet
    """
    if dest_path is None:
        dest_path = source_path

    result = client.copy_onelake_file(
        source_workspace,
        source_id,
        source_path,
        dest_workspace,
        dest_id,
        dest_path,
    )

    result.update(
        {
            "status": "copied",
            "source": f"{source_workspace}/{source_id}/{source_path}",
            "destination": f"{dest_workspace}/{dest_id}/{dest_path}",
        }
    )
    output(ctx, result, plain_key="destination")


@lakehouse.command(name="delete-file")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "lakehouse_id", required=True, help="Lakehouse item ID.")
@click.option("--path", "-p", required=True, help="File path to delete (e.g. Files/data.csv).")
@click.pass_context
def delete_file(
    ctx: click.Context,
    workspace: str,
    lakehouse_id: str,
    path: str,
) -> None:
    """Delete a file from a lakehouse.

    \b
    Examples:
        fabio lakehouse delete-file -w <ws> --id <lh> -p Files/old_data.csv
        fabio lakehouse delete-file -w <ws> --id <lh> -p Files/staging/temp.parquet
    """
    client.delete_onelake_file(workspace, lakehouse_id, path)
    output(
        ctx,
        {"status": "deleted", "path": path, "workspace": workspace, "lakehouse": lakehouse_id},
        plain_key="path",
    )


@lakehouse.command(name="move-file")
@click.option("--source-workspace", "-sw", required=True, help="Source workspace ID.")
@click.option("--source-id", "-si", required=True, help="Source lakehouse item ID.")
@click.option(
    "--source-path", "-sp", required=True, help="Source file path (e.g. Files/data.csv)."
)
@click.option("--dest-workspace", "-dw", required=True, help="Destination workspace ID.")
@click.option("--dest-id", "-di", required=True, help="Destination lakehouse item ID.")
@click.option(
    "--dest-path",
    "-dp",
    default=None,
    help="Destination file path (default: same as source path).",
)
@click.pass_context
def move_file(
    ctx: click.Context,
    source_workspace: str,
    source_id: str,
    source_path: str,
    dest_workspace: str,
    dest_id: str,
    dest_path: str | None,
) -> None:
    """Move a file between lakehouses (server-side copy + delete source).

    \b
    Performs a server-side copy then deletes the source. Data never transits
    through the client. Safe failure mode: if interrupted after copy, you get
    a duplicate rather than data loss.

    \b
    Examples:
        fabio lakehouse move-file \\
            -sw <src-ws> -si <src-lh> -sp Files/data.csv \\
            -dw <dest-ws> -di <dest-lh>

        fabio lakehouse move-file \\
            -sw <src-ws> -si <src-lh> -sp Files/staging/raw.parquet \\
            -dw <dest-ws> -di <dest-lh> -dp Files/archive/raw.parquet
    """
    if dest_path is None:
        dest_path = source_path

    result = client.move_onelake_file(
        source_workspace,
        source_id,
        source_path,
        dest_workspace,
        dest_id,
        dest_path,
    )

    result.update(
        {
            "status": "moved",
            "source": f"{source_workspace}/{source_id}/{source_path}",
            "destination": f"{dest_workspace}/{dest_id}/{dest_path}",
        }
    )
    output(ctx, result, plain_key="destination")


@lakehouse.command(name="delete-table")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "lakehouse_id", required=True, help="Lakehouse item ID.")
@click.option("--table", "-t", required=True, help="Table name to delete.")
@click.pass_context
def delete_table_cmd(
    ctx: click.Context,
    workspace: str,
    lakehouse_id: str,
    table: str,
) -> None:
    """Delete a Delta table from a lakehouse.

    \b
    Recursively deletes the table directory (Tables/<name>) including
    all Delta log files and data files.

    \b
    Examples:
        fabio lakehouse delete-table -w <ws> --id <lh> -t staging_orders
        fabio lakehouse delete-table -w <ws> --id <lh> --table old_sales
    """
    client.delete_table(workspace, lakehouse_id, table)
    output(
        ctx,
        {"status": "deleted", "table": table, "workspace": workspace, "lakehouse": lakehouse_id},
        plain_key="table",
    )


@lakehouse.command(name="copy-table")
@click.option("--source-workspace", "-sw", required=True, help="Source workspace ID.")
@click.option("--source-id", "-si", required=True, help="Source lakehouse item ID.")
@click.option("--source-table", "-st", required=True, help="Source table name.")
@click.option("--dest-workspace", "-dw", required=True, help="Destination workspace ID.")
@click.option("--dest-id", "-di", required=True, help="Destination lakehouse item ID.")
@click.option(
    "--dest-table", "-dt", default=None, help="Destination table name (default: same as source)."
)
@click.pass_context
def copy_table_cmd(
    ctx: click.Context,
    source_workspace: str,
    source_id: str,
    source_table: str,
    dest_workspace: str,
    dest_id: str,
    dest_table: str | None,
) -> None:
    """Copy a Delta table between lakehouses (server-side).

    \b
    Copies all files (Delta log + parquet) from the source table to the
    destination. Data never transits through the client.

    \b
    Examples:
        fabio lakehouse copy-table \\
            -sw <src-ws> -si <src-lh> -st sales \\
            -dw <dest-ws> -di <dest-lh>

        fabio lakehouse copy-table \\
            -sw <src-ws> -si <src-lh> -st orders \\
            -dw <dest-ws> -di <dest-lh> -dt orders_backup
    """
    if dest_table is None:
        dest_table = source_table

    result = client.copy_table(
        source_workspace,
        source_id,
        source_table,
        dest_workspace,
        dest_id,
        dest_table,
    )

    result.update({"status": "copied"})
    output(ctx, result, plain_key="destTable")


@lakehouse.command(name="move-table")
@click.option("--source-workspace", "-sw", required=True, help="Source workspace ID.")
@click.option("--source-id", "-si", required=True, help="Source lakehouse item ID.")
@click.option("--source-table", "-st", required=True, help="Source table name.")
@click.option("--dest-workspace", "-dw", required=True, help="Destination workspace ID.")
@click.option("--dest-id", "-di", required=True, help="Destination lakehouse item ID.")
@click.option(
    "--dest-table", "-dt", default=None, help="Destination table name (default: same as source)."
)
@click.pass_context
def move_table_cmd(
    ctx: click.Context,
    source_workspace: str,
    source_id: str,
    source_table: str,
    dest_workspace: str,
    dest_id: str,
    dest_table: str | None,
) -> None:
    """Move a Delta table between lakehouses (server-side copy + delete).

    \b
    Copies all table files to the destination, then deletes the source table.
    Safe failure mode: if interrupted after copy, you get a duplicate rather
    than data loss.

    \b
    Examples:
        fabio lakehouse move-table \\
            -sw <src-ws> -si <src-lh> -st staging_data \\
            -dw <dest-ws> -di <dest-lh>

        fabio lakehouse move-table \\
            -sw <src-ws> -si <src-lh> -st raw_events \\
            -dw <dest-ws> -di <dest-lh> -dt archived_events
    """
    if dest_table is None:
        dest_table = source_table

    result = client.move_table(
        source_workspace,
        source_id,
        source_table,
        dest_workspace,
        dest_id,
        dest_table,
    )

    output(ctx, result, plain_key="destTable")


@lakehouse.command(name="create-shortcut")
@click.option("--workspace", "-w", required=True, help="Workspace ID (target lakehouse).")
@click.option("--id", "lakehouse_id", required=True, help="Lakehouse item ID (target).")
@click.option("--name", "-n", required=True, help="Shortcut name.")
@click.option(
    "--path",
    "-p",
    default="Tables",
    help="Parent path for the shortcut (Tables or Files). Default: Tables.",
)
@click.option(
    "--target-type",
    "-t",
    type=click.Choice(["onelake", "adls", "s3"]),
    default="onelake",
    help="Target type (default: onelake).",
)
@click.option(
    "--source-workspace",
    default=None,
    help="[onelake] Source workspace ID.",
)
@click.option(
    "--source-id",
    default=None,
    help="[onelake] Source lakehouse/item ID.",
)
@click.option(
    "--source-path",
    default=None,
    help="[onelake] Path in source item (e.g. Tables/sales).",
)
@click.option(
    "--location",
    default=None,
    help="[adls/s3] Storage account URL or S3 bucket URL.",
)
@click.option(
    "--subpath",
    default=None,
    help="[adls/s3] Sub-path within the storage container/bucket.",
)
@click.option(
    "--connection-id",
    default=None,
    help="[adls/s3] Connection ID for authentication.",
)
@click.pass_context
def create_shortcut(
    ctx: click.Context,
    workspace: str,
    lakehouse_id: str,
    name: str,
    path: str,
    target_type: str,
    source_workspace: str | None,
    source_id: str | None,
    source_path: str | None,
    location: str | None,
    subpath: str | None,
    connection_id: str | None,
) -> None:
    """Create a shortcut in a lakehouse.

    \b
    OneLake shortcuts reference tables/files in another lakehouse:
        fabio lakehouse create-shortcut -w <ws> --id <lh> -n sales \\
            --source-workspace <src-ws> --source-id <src-lh> \\
            --source-path Tables/sales

    \b
    ADLS Gen2 shortcuts reference external Azure storage:
        fabio lakehouse create-shortcut -w <ws> --id <lh> -n raw \\
            --path Files --target-type adls \\
            --location https://account.dfs.core.windows.net \\
            --subpath /container/path --connection-id <conn-id>

    \b
    S3 shortcuts reference Amazon S3 buckets:
        fabio lakehouse create-shortcut -w <ws> --id <lh> -n external \\
            --path Files --target-type s3 \\
            --location https://bucket.s3.region.amazonaws.com \\
            --subpath /prefix --connection-id <conn-id>
    """
    target: dict[str, Any] = {}

    if target_type == "onelake":
        if not source_workspace or not source_id or not source_path:
            raise FabioError(
                ErrorCode.INVALID_INPUT,
                "OneLake shortcuts require --source-workspace, --source-id, and --source-path.",
            )
        target["oneLake"] = {
            "workspaceId": source_workspace,
            "itemId": source_id,
            "path": source_path,
        }
    elif target_type == "adls":
        if not location or not subpath:
            raise FabioError(
                ErrorCode.INVALID_INPUT,
                "ADLS shortcuts require --location and --subpath.",
            )
        adls_target: dict[str, str] = {"location": location, "subpath": subpath}
        if connection_id:
            adls_target["connectionId"] = connection_id
        target["adlsGen2"] = adls_target
    elif target_type == "s3":
        if not location or not subpath:
            raise FabioError(
                ErrorCode.INVALID_INPUT,
                "S3 shortcuts require --location and --subpath.",
            )
        s3_target: dict[str, str] = {"location": location, "subpath": subpath}
        if connection_id:
            s3_target["connectionId"] = connection_id
        target["amazonS3"] = s3_target

    body: dict[str, Any] = {
        "path": path,
        "name": name,
        "target": target,
    }

    api_path = f"/workspaces/{workspace}/items/{lakehouse_id}/shortcuts"
    data = client.post(api_path, body=body)

    # The API returns the created shortcut details
    result: dict[str, Any] = data if data else {}
    result.update({"status": "created", "shortcutName": name, "shortcutPath": path})
    output(ctx, result, plain_key="shortcutName")


@lakehouse.command(name="get-shortcut")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "lakehouse_id", required=True, help="Lakehouse item ID.")
@click.option("--name", "-n", required=True, help="Shortcut name.")
@click.option(
    "--path",
    "-p",
    default="Tables",
    help="Shortcut parent path (Tables or Files). Default: Tables.",
)
@click.pass_context
def get_shortcut(
    ctx: click.Context,
    workspace: str,
    lakehouse_id: str,
    name: str,
    path: str,
) -> None:
    """Get details of a shortcut.

    \b
    Examples:
        fabio lakehouse get-shortcut -w <ws> --id <lh> -n sales
        fabio lakehouse get-shortcut -w <ws> --id <lh> -n raw --path Files
    """
    api_path = f"/workspaces/{workspace}/items/{lakehouse_id}/shortcuts/{path}/{name}"
    data = client.get(api_path)
    output(ctx, data, plain_key="name")


@lakehouse.command(name="delete-shortcut")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "lakehouse_id", required=True, help="Lakehouse item ID.")
@click.option("--name", "-n", required=True, help="Shortcut name.")
@click.option(
    "--path",
    "-p",
    default="Tables",
    help="Shortcut parent path (Tables or Files). Default: Tables.",
)
@click.pass_context
def delete_shortcut(
    ctx: click.Context,
    workspace: str,
    lakehouse_id: str,
    name: str,
    path: str,
) -> None:
    """Delete a shortcut from a lakehouse.

    \b
    Examples:
        fabio lakehouse delete-shortcut -w <ws> --id <lh> -n sales
        fabio lakehouse delete-shortcut -w <ws> --id <lh> -n raw --path Files
    """
    api_path = f"/workspaces/{workspace}/items/{lakehouse_id}/shortcuts/{path}/{name}"
    client.delete(api_path)
    output(
        ctx,
        {"status": "deleted", "shortcutName": name, "shortcutPath": path},
        plain_key="shortcutName",
    )
