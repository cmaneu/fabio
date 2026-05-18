"""``fabio warehouse`` command group.

Commands:
    list  - List warehouses in a workspace
    query - Execute a SQL query against a warehouse
"""

from __future__ import annotations

import struct
from typing import Any

import click

from fabio import client
from fabio.errors import ErrorCode, FabioError
from fabio.output import output

DATABASE_SCOPE = "https://database.windows.net/.default"
SQL_COPT_SS_ACCESS_TOKEN = 1256


@click.group()
def warehouse() -> None:
    """Manage Fabric warehouses and run SQL queries."""


@warehouse.command(name="list")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.pass_context
def list_warehouses(ctx: click.Context, workspace: str) -> None:
    """List warehouses in a workspace.

    \b
    Examples:
        fabio warehouse list --workspace <id>
    """
    data = client.get(f"/workspaces/{workspace}/warehouses")
    items = data.get("value", [])
    output(
        ctx,
        items,
        columns=["displayName", "id"],
        headers=["NAME", "ID"],
        plain_key="id",
    )


@warehouse.command(name="query")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "warehouse_id", required=True, help="Warehouse or Lakehouse item ID.")
@click.option("--sql", "-s", required=True, help="SQL query to execute.")
@click.pass_context
def query_cmd(
    ctx: click.Context,
    workspace: str,
    warehouse_id: str,
    sql: str,
) -> None:
    """Execute a SQL query against a Fabric warehouse or SQL endpoint.

    \b
    Connects via TDS (ODBC Driver 18) using Azure AD token auth.
    Returns query results as structured JSON.

    \b
    Examples:
        fabio warehouse query -w <ws> --id <wh> --sql "SELECT TOP 10 * FROM sales"
        fabio warehouse query -w <ws> --id <wh> -s "SELECT COUNT(*) AS cnt FROM orders"
        fabio warehouse query -w <ws> --id <wh> -s @query.sql  (read from file)
    """
    try:
        import pyodbc  # type: ignore[import-not-found]
    except ImportError as exc:
        raise FabioError(
            ErrorCode.INVALID_INPUT,
            "pyodbc package required. Install with: uv pip install pyodbc",
        ) from exc

    # If SQL starts with @, read from file
    if sql.startswith("@"):
        from pathlib import Path

        sql_file = Path(sql[1:])
        if not sql_file.exists():
            raise FabioError(ErrorCode.NOT_FOUND, f"SQL file not found: {sql_file}")
        sql = sql_file.read_text()

    # Get warehouse or lakehouse details to find connection string
    wh_data: dict[str, Any] = {}
    connection_string = ""

    # Try warehouse endpoint first
    try:
        wh_data = client.get(f"/workspaces/{workspace}/warehouses/{warehouse_id}")
        connection_string = (
            wh_data.get("properties", {}).get("connectionString", "")
            if wh_data.get("properties")
            else ""
        )
    except FabioError:
        pass

    # Fall back: try lakehouse SQL endpoint
    if not connection_string:
        try:
            lh_data = client.get(f"/workspaces/{workspace}/lakehouses/{warehouse_id}")
            sql_props = lh_data.get("properties", {}).get("sqlEndpointProperties", {})
            connection_string = sql_props.get("connectionString", "")
            if connection_string:
                wh_data = lh_data
        except FabioError:
            pass

    if not connection_string:
        raise FabioError(
            ErrorCode.NOT_FOUND,
            "Could not determine SQL connection string. "
            "Verify the item is a warehouse or lakehouse with a SQL endpoint.",
        )

    # Extract server and database from connection string
    server, database = _parse_connection_string(connection_string, wh_data)

    # Get access token for SQL and format for ODBC
    token = client.require_auth(DATABASE_SCOPE)
    token_bytes = token.encode("UTF-16-LE")
    token_struct = struct.pack(f"<I{len(token_bytes)}s", len(token_bytes), token_bytes)

    odbc_conn_str = (
        f"DRIVER={{ODBC Driver 18 for SQL Server}};"
        f"SERVER={server};"
        f"DATABASE={database}"
    )

    try:
        conn = pyodbc.connect(
            odbc_conn_str,
            attrs_before={SQL_COPT_SS_ACCESS_TOKEN: token_struct},
            autocommit=True,
        )
    except Exception as exc:
        raise FabioError(
            ErrorCode.API_ERROR,
            f"Failed to connect to SQL endpoint: {exc}",
        ) from exc

    try:
        cursor = conn.cursor()
        cursor.execute(sql)

        if cursor.description:
            columns = [desc[0] for desc in cursor.description]
            rows: list[dict[str, Any]] = []
            for row in cursor.fetchall():
                rows.append(dict(zip(columns, row, strict=False)))
            # Serialize non-JSON-native types (dates, decimals)
            rows = [_serialize_row(r) for r in rows]
            output(
                ctx,
                rows,
                columns=columns[:6],
                headers=[c.upper() for c in columns[:6]],
                plain_key=columns[0] if columns else "result",
            )
        else:
            # DDL/DML statement with no results
            output(
                ctx,
                {"status": "executed", "rowcount": cursor.rowcount},
                plain_key="rowcount",
            )
    except pyodbc.ProgrammingError as exc:
        raise FabioError(ErrorCode.API_ERROR, f"SQL error: {exc}") from exc
    except Exception as exc:
        raise FabioError(ErrorCode.API_ERROR, f"Query execution failed: {exc}") from exc
    finally:
        conn.close()


def _serialize_row(row: dict[str, Any]) -> dict[str, Any]:
    """Convert non-JSON-serializable values (dates, Decimal) to strings."""
    import datetime
    from decimal import Decimal

    result: dict[str, Any] = {}
    for key, val in row.items():
        if isinstance(val, (datetime.date, datetime.datetime)):
            result[key] = val.isoformat()
        elif isinstance(val, Decimal):
            result[key] = float(val)
        elif isinstance(val, bytes):
            result[key] = val.hex()
        else:
            result[key] = val
    return result


def _parse_connection_string(connection_string: str, wh_data: dict[str, Any]) -> tuple[str, str]:
    """Extract server and database from a Fabric SQL connection string.

    The connection string is typically just the server hostname, e.g.:
        <guid>.datawarehouse.fabric.microsoft.com

    The database name is the warehouse display name.
    """
    server = connection_string.strip()

    # Remove any protocol prefix
    if server.startswith("jdbc:"):
        server = server.split("//", 1)[-1].split(";")[0]

    # Remove port suffix if present
    if "," in server:
        server = server.split(",")[0]

    database = wh_data.get("displayName", "")
    if not database:
        raise FabioError(
            ErrorCode.INVALID_INPUT,
            "Could not determine database name from warehouse metadata.",
        )

    return server, database
