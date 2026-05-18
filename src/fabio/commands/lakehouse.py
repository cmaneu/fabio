"""``fabio lakehouse`` command group.

Commands:
    tables     - List tables in a lakehouse
    files      - List files in a lakehouse
    upload     - Upload a local file to a lakehouse
    download   - Download a file from a lakehouse
    load-table - Create/load a table from a file in the lakehouse
"""

from __future__ import annotations

from pathlib import Path

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
    default="overwrite",
    type=click.Choice(["overwrite", "append"]),
    help="Load mode (default: overwrite).",
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

    format_options: dict[str, str] | None = None
    if file_format == "csv":
        format_options = {
            "header": str(header).lower(),
            "delimiter": delimiter,
        }

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
