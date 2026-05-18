"""Tests for ``fabio lakehouse`` commands."""

from __future__ import annotations

import json
from unittest.mock import patch

from click.testing import CliRunner

from fabio.cli import main


class TestLakehouseTables:
    def test_list_tables(self) -> None:
        runner = CliRunner()
        tables_data = {
            "data": [
                {
                    "name": "orders",
                    "type": "Managed",
                    "format": "delta",
                    "location": "abfss://x/orders",
                },
                {
                    "name": "customers",
                    "type": "Managed",
                    "format": "delta",
                    "location": "abfss://x/customers",
                },
            ]
        }
        with patch("fabio.commands.lakehouse.client.get", return_value=tables_data):
            result = runner.invoke(
                main, ["lakehouse", "tables", "--workspace", "ws-001", "--id", "lh-001"]
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        assert parsed["data"][0]["name"] == "orders"
        assert parsed["data"][1]["name"] == "customers"

    def test_empty_tables(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.get", return_value={"data": []}):
            result = runner.invoke(
                main, ["lakehouse", "tables", "--workspace", "ws-001", "--id", "lh-001"]
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"] == []
        assert parsed["count"] == 0

    def test_plain_output(self) -> None:
        runner = CliRunner()
        tables_data = {
            "data": [
                {"name": "orders", "type": "Managed", "format": "delta", "location": "abfss://x"},
            ]
        }
        with patch("fabio.commands.lakehouse.client.get", return_value=tables_data):
            result = runner.invoke(
                main,
                ["-o", "plain", "lakehouse", "tables", "-w", "ws-001", "--id", "lh-001"],
            )

        assert result.exit_code == 0
        assert result.output.strip() == "orders"


class TestLakehouseFiles:
    def test_list_files(self) -> None:
        runner = CliRunner()
        # OneLake DFS API returns paths with doubled prefix for top-level dirs
        # e.g. directory=Files returns "Files/Files/myfile.csv"
        files = [
            {
                "name": "Files/Files",
                "isDirectory": "true",
                "lastModified": "2025-01-10T08:00:00Z",
            },
            {
                "name": "Files/Files/raw_sales.csv",
                "contentLength": "1024",
                "lastModified": "2025-01-15T10:00:00Z",
                "isDirectory": "false",
            },
            {
                "name": "Files/Files/data",
                "isDirectory": "true",
                "lastModified": "2025-01-10T08:00:00Z",
            },
            {
                "name": "Files/Functions",
                "isDirectory": "true",
                "lastModified": "2025-01-10T08:00:00Z",
            },
        ]
        with patch("fabio.commands.lakehouse.client.list_onelake_files", return_value=files):
            result = runner.invoke(
                main, ["lakehouse", "files", "--workspace", "ws-001", "--id", "lh-001"]
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        assert parsed["data"][0]["name"] == "raw_sales.csv"
        assert parsed["data"][0]["isDirectory"] is False
        assert parsed["data"][1]["name"] == "data"
        assert parsed["data"][1]["isDirectory"] is True

    def test_list_files_recursive(self) -> None:
        runner = CliRunner()
        # For non-top-level paths like "Files/raw", prefix is just "Files/raw/"
        files = [
            {
                "name": "Files/raw/orders.csv",
                "contentLength": "4096",
                "lastModified": "2025-03-01T09:00:00Z",
                "isDirectory": "false",
            },
            {
                "name": "Files/raw/nested/deep.csv",
                "contentLength": "100",
                "lastModified": "2025-03-01T09:00:00Z",
                "isDirectory": "false",
            },
        ]
        with patch(
            "fabio.commands.lakehouse.client.list_onelake_files", return_value=files
        ) as mock_list:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "files",
                    "--workspace",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--path",
                    "Files/raw",
                    "-r",
                ],
            )

        assert result.exit_code == 0
        # Always calls with recursive=True (filters client-side)
        mock_list.assert_called_once_with(
            "ws-001", "lh-001", directory="Files/raw", recursive=True
        )
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        assert parsed["data"][0]["name"] == "orders.csv"
        assert parsed["data"][1]["name"] == "nested/deep.csv"

    def test_empty_directory(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.list_onelake_files", return_value=[]):
            result = runner.invoke(
                main, ["lakehouse", "files", "--workspace", "ws-001", "--id", "lh-001"]
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"] == []
        assert parsed["count"] == 0

    def test_plain_output(self) -> None:
        runner = CliRunner()
        # Doubled prefix for top-level "Files" directory
        files = [
            {
                "name": "Files/Files/report.csv",
                "contentLength": "512",
                "lastModified": "",
                "isDirectory": "false",
            },
        ]
        with patch("fabio.commands.lakehouse.client.list_onelake_files", return_value=files):
            result = runner.invoke(
                main,
                ["-o", "plain", "lakehouse", "files", "-w", "ws-001", "--id", "lh-001"],
            )

        assert result.exit_code == 0
        assert "report.csv" in result.output.strip()
