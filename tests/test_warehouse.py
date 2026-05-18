"""Tests for ``fabio warehouse`` commands."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING
from unittest.mock import MagicMock, patch

from click.testing import CliRunner

from fabio.cli import main

if TYPE_CHECKING:
    from pathlib import Path


class TestWarehouseList:
    def test_list_warehouses(self) -> None:
        runner = CliRunner()
        mock_data = {
            "value": [
                {"id": "wh-001", "displayName": "SalesWarehouse"},
                {"id": "wh-002", "displayName": "AnalyticsWarehouse"},
            ]
        }
        with patch("fabio.commands.warehouse.client.get", return_value=mock_data):
            result = runner.invoke(
                main,
                ["warehouse", "list", "-w", "ws-001"],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        assert parsed["data"][0]["displayName"] == "SalesWarehouse"

    def test_list_warehouses_empty(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.warehouse.client.get", return_value={"value": []}):
            result = runner.invoke(
                main,
                ["warehouse", "list", "-w", "ws-001"],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 0


class TestWarehouseQuery:
    def test_query_returns_rows(self) -> None:
        runner = CliRunner()
        wh_data = {
            "id": "wh-001",
            "displayName": "SalesWarehouse",
            "properties": {
                "connectionString": "abc123.datawarehouse.fabric.microsoft.com",
            },
        }

        mock_cursor = MagicMock()
        mock_cursor.description = [("id",), ("name",), ("amount",)]
        mock_cursor.fetchall.return_value = [(1, "Alice", 100.0), (2, "Bob", 200.0)]

        mock_conn = MagicMock()
        mock_conn.cursor.return_value = mock_cursor

        with (
            patch("fabio.commands.warehouse.client.get", return_value=wh_data),
            patch("fabio.commands.warehouse.client.require_auth", return_value="token"),
            patch("pyodbc.connect", return_value=mock_conn),
        ):
            result = runner.invoke(
                main,
                [
                    "warehouse",
                    "query",
                    "-w",
                    "ws-001",
                    "--id",
                    "wh-001",
                    "--sql",
                    "SELECT id, name, amount FROM sales",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        assert parsed["data"][0]["name"] == "Alice"
        assert parsed["data"][1]["amount"] == 200.0

    def test_query_ddl_no_results(self) -> None:
        runner = CliRunner()
        wh_data = {
            "id": "wh-001",
            "displayName": "SalesWarehouse",
            "properties": {
                "connectionString": "abc123.datawarehouse.fabric.microsoft.com",
            },
        }

        mock_cursor = MagicMock()
        mock_cursor.description = None
        mock_cursor.rowcount = 5

        mock_conn = MagicMock()
        mock_conn.cursor.return_value = mock_cursor

        with (
            patch("fabio.commands.warehouse.client.get", return_value=wh_data),
            patch("fabio.commands.warehouse.client.require_auth", return_value="token"),
            patch("pyodbc.connect", return_value=mock_conn),
        ):
            result = runner.invoke(
                main,
                [
                    "warehouse",
                    "query",
                    "-w",
                    "ws-001",
                    "--id",
                    "wh-001",
                    "--sql",
                    "DELETE FROM staging WHERE processed = 1",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "executed"
        assert parsed["data"]["rowcount"] == 5

    def test_query_from_file(self, tmp_path: Path) -> None:
        runner = CliRunner()
        sql_file = tmp_path / "query.sql"
        sql_file.write_text("SELECT COUNT(*) AS cnt FROM orders")

        wh_data = {
            "id": "wh-001",
            "displayName": "TestWH",
            "properties": {
                "connectionString": "x.datawarehouse.fabric.microsoft.com",
            },
        }

        mock_cursor = MagicMock()
        mock_cursor.description = [("cnt",)]
        mock_cursor.fetchall.return_value = [(42,)]

        mock_conn = MagicMock()
        mock_conn.cursor.return_value = mock_cursor

        with (
            patch("fabio.commands.warehouse.client.get", return_value=wh_data),
            patch("fabio.commands.warehouse.client.require_auth", return_value="tok"),
            patch("pyodbc.connect", return_value=mock_conn),
        ):
            result = runner.invoke(
                main,
                [
                    "warehouse",
                    "query",
                    "-w",
                    "ws-001",
                    "--id",
                    "wh-001",
                    "--sql",
                    f"@{sql_file}",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"][0]["cnt"] == 42
        mock_cursor.execute.assert_called_once_with("SELECT COUNT(*) AS cnt FROM orders")

    def test_query_missing_connection_string(self) -> None:
        runner = CliRunner()
        wh_data = {
            "id": "wh-001",
            "displayName": "NoConnWH",
            "properties": {},
        }

        from fabio.errors import FabioError

        with patch(
            "fabio.commands.warehouse.client.get",
            side_effect=[wh_data, FabioError("NOT_FOUND", "not found")],
        ):
            result = runner.invoke(
                main,
                [
                    "warehouse",
                    "query",
                    "-w",
                    "ws-001",
                    "--id",
                    "wh-001",
                    "--sql",
                    "SELECT 1",
                ],
            )

        assert result.exit_code != 0
        assert "connection string" in result.output.lower() or "NOT_FOUND" in result.output

    def test_query_connection_uses_correct_params(self) -> None:
        """Verify pyodbc.connect is called with correct ODBC connection string."""
        runner = CliRunner()
        wh_data = {
            "id": "wh-001",
            "displayName": "MyWarehouse",
            "properties": {
                "connectionString": "guid123.datawarehouse.fabric.microsoft.com",
            },
        }

        mock_cursor = MagicMock()
        mock_cursor.description = [("x",)]
        mock_cursor.fetchall.return_value = [(1,)]

        mock_conn = MagicMock()
        mock_conn.cursor.return_value = mock_cursor

        with (
            patch("fabio.commands.warehouse.client.get", return_value=wh_data),
            patch("fabio.commands.warehouse.client.require_auth", return_value="my-token"),
            patch("pyodbc.connect", return_value=mock_conn) as mock_connect,
        ):
            result = runner.invoke(
                main,
                [
                    "warehouse",
                    "query",
                    "-w",
                    "ws-001",
                    "--id",
                    "wh-001",
                    "--sql",
                    "SELECT 1 AS x",
                ],
            )

        assert result.exit_code == 0, result.output
        call_args = mock_connect.call_args
        conn_str = call_args[0][0]
        assert "ODBC Driver 18 for SQL Server" in conn_str
        assert "guid123.datawarehouse.fabric.microsoft.com" in conn_str
        assert "MyWarehouse" in conn_str
        # Verify token struct is passed
        attrs = call_args[1]["attrs_before"]
        assert 1256 in attrs  # SQL_COPT_SS_ACCESS_TOKEN

    def test_query_falls_back_to_lakehouse_sql_endpoint(self) -> None:
        """If warehouse API fails, try lakehouse SQL endpoint."""
        runner = CliRunner()

        from fabio.errors import FabioError

        lh_data = {
            "id": "lh-001",
            "displayName": "MyLakehouse",
            "properties": {
                "sqlEndpointProperties": {
                    "connectionString": "guid456.datawarehouse.fabric.microsoft.com",
                }
            },
        }

        mock_cursor = MagicMock()
        mock_cursor.description = [("val",)]
        mock_cursor.fetchall.return_value = [(99,)]

        mock_conn = MagicMock()
        mock_conn.cursor.return_value = mock_cursor

        with (
            patch(
                "fabio.commands.warehouse.client.get",
                side_effect=[FabioError("NOT_FOUND", "not a warehouse"), lh_data],
            ),
            patch("fabio.commands.warehouse.client.require_auth", return_value="tok"),
            patch("pyodbc.connect", return_value=mock_conn) as mock_connect,
        ):
            result = runner.invoke(
                main,
                [
                    "warehouse",
                    "query",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--sql",
                    "SELECT 99 AS val",
                ],
            )

        assert result.exit_code == 0, result.output
        conn_str = mock_connect.call_args[0][0]
        assert "guid456.datawarehouse.fabric.microsoft.com" in conn_str
        assert "MyLakehouse" in conn_str

    def test_query_serializes_dates(self) -> None:
        """Date/datetime values are serialized to ISO format."""
        import datetime

        runner = CliRunner()
        wh_data = {
            "id": "wh-001",
            "displayName": "TestWH",
            "properties": {
                "connectionString": "x.datawarehouse.fabric.microsoft.com",
            },
        }

        mock_cursor = MagicMock()
        mock_cursor.description = [("id",), ("created",)]
        mock_cursor.fetchall.return_value = [
            (1, datetime.date(2024, 3, 15)),
            (2, datetime.datetime(2024, 6, 1, 10, 30, 0)),
        ]

        mock_conn = MagicMock()
        mock_conn.cursor.return_value = mock_cursor

        with (
            patch("fabio.commands.warehouse.client.get", return_value=wh_data),
            patch("fabio.commands.warehouse.client.require_auth", return_value="tok"),
            patch("pyodbc.connect", return_value=mock_conn),
        ):
            result = runner.invoke(
                main,
                ["warehouse", "query", "-w", "ws", "--id", "wh", "--sql", "SELECT *"],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"][0]["created"] == "2024-03-15"
        assert parsed["data"][1]["created"] == "2024-06-01T10:30:00"
