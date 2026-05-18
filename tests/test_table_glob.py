"""Tests for glob pattern support in table operations (delete/copy/move)."""

from __future__ import annotations

import json
from unittest.mock import patch

from click.testing import CliRunner

from fabio.cli import main
from fabio.commands.lakehouse import _resolve_table_glob

MOCK_TABLES_RESPONSE = {
    "data": [
        {"name": "sales_2023", "type": "Managed"},
        {"name": "sales_2024", "type": "Managed"},
        {"name": "orders", "type": "Managed"},
        {"name": "staging_orders", "type": "Managed"},
        {"name": "staging_products", "type": "Managed"},
        {"name": "customers", "type": "Managed"},
    ]
}


class TestResolveTableGlob:
    """Unit tests for _resolve_table_glob helper."""

    def test_star_pattern(self) -> None:
        with patch("fabio.commands.lakehouse.client.get", return_value=MOCK_TABLES_RESPONSE):
            result = _resolve_table_glob("ws-001", "lh-001", "sales_*")
        assert result == ["sales_2023", "sales_2024"]

    def test_question_mark_pattern(self) -> None:
        with patch("fabio.commands.lakehouse.client.get", return_value=MOCK_TABLES_RESPONSE):
            result = _resolve_table_glob("ws-001", "lh-001", "sales_202?")
        assert result == ["sales_2023", "sales_2024"]

    def test_bracket_pattern(self) -> None:
        with patch("fabio.commands.lakehouse.client.get", return_value=MOCK_TABLES_RESPONSE):
            result = _resolve_table_glob("ws-001", "lh-001", "staging_[op]*")
        assert result == ["staging_orders", "staging_products"]

    def test_no_match(self) -> None:
        with patch("fabio.commands.lakehouse.client.get", return_value=MOCK_TABLES_RESPONSE):
            result = _resolve_table_glob("ws-001", "lh-001", "nonexist_*")
        assert result == []

    def test_match_all(self) -> None:
        with patch("fabio.commands.lakehouse.client.get", return_value=MOCK_TABLES_RESPONSE):
            result = _resolve_table_glob("ws-001", "lh-001", "*")
        assert len(result) == 6

    def test_exact_match(self) -> None:
        with patch("fabio.commands.lakehouse.client.get", return_value=MOCK_TABLES_RESPONSE):
            result = _resolve_table_glob("ws-001", "lh-001", "orders")
        assert result == ["orders"]

    def test_empty_table_list(self) -> None:
        with patch("fabio.commands.lakehouse.client.get", return_value={"data": []}):
            result = _resolve_table_glob("ws-001", "lh-001", "sales_*")
        assert result == []


class TestDeleteTableGlob:
    """Tests for delete-table with glob patterns."""

    def test_delete_single_table_no_glob(self) -> None:
        """Non-glob table name works as before."""
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.delete_table") as mock_del:
            result = runner.invoke(
                main,
                ["lakehouse", "delete-table", "-w", "ws-001", "--id", "lh-001", "-t", "orders"],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "deleted"
        assert parsed["data"]["table"] == "orders"
        mock_del.assert_called_once_with("ws-001", "lh-001", "orders")

    def test_delete_table_glob_pattern(self) -> None:
        """Glob pattern deletes all matching tables."""
        runner = CliRunner()
        with (
            patch(
                "fabio.commands.lakehouse.client.get",
                return_value=MOCK_TABLES_RESPONSE,
            ),
            patch("fabio.commands.lakehouse.client.delete_table") as mock_del,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "delete-table",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "-t",
                    "staging_*",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        tables = [r["table"] for r in parsed["data"]]
        assert tables == ["staging_orders", "staging_products"]
        assert mock_del.call_count == 2
        mock_del.assert_any_call("ws-001", "lh-001", "staging_orders")
        mock_del.assert_any_call("ws-001", "lh-001", "staging_products")

    def test_delete_table_glob_no_match(self) -> None:
        """Glob with no matches raises NOT_FOUND error."""
        runner = CliRunner()
        with patch(
            "fabio.commands.lakehouse.client.get",
            return_value=MOCK_TABLES_RESPONSE,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "delete-table",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "-t",
                    "nonexist_*",
                ],
            )

        assert result.exit_code != 0
        assert "NOT_FOUND" in result.output or "No tables match" in result.output


class TestCopyTableGlob:
    """Tests for copy-table with glob patterns."""

    def test_copy_single_table_no_glob(self) -> None:
        """Non-glob copy works as before."""
        runner = CliRunner()
        with patch(
            "fabio.commands.lakehouse.client.copy_table",
            return_value={"filesCopied": 3, "sourceTable": "sales", "destTable": "sales"},
        ) as mock_copy:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "copy-table",
                    "-sw",
                    "src-ws",
                    "-si",
                    "src-lh",
                    "-st",
                    "sales",
                    "-dw",
                    "dest-ws",
                    "-di",
                    "dest-lh",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "copied"
        mock_copy.assert_called_once_with(
            "src-ws", "src-lh", "sales", "dest-ws", "dest-lh", "sales"
        )

    def test_copy_table_glob_pattern(self) -> None:
        """Glob pattern copies all matching tables."""
        runner = CliRunner()
        with (
            patch(
                "fabio.commands.lakehouse.client.get",
                return_value=MOCK_TABLES_RESPONSE,
            ),
            patch(
                "fabio.commands.lakehouse.client.copy_table",
                return_value={"filesCopied": 2},
            ) as mock_copy,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "copy-table",
                    "-sw",
                    "src-ws",
                    "-si",
                    "src-lh",
                    "-st",
                    "sales_*",
                    "-dw",
                    "dest-ws",
                    "-di",
                    "dest-lh",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        tables = [r["destTable"] for r in parsed["data"]]
        assert tables == ["sales_2023", "sales_2024"]
        assert mock_copy.call_count == 2
        mock_copy.assert_any_call(
            "src-ws", "src-lh", "sales_2023", "dest-ws", "dest-lh", "sales_2023"
        )
        mock_copy.assert_any_call(
            "src-ws", "src-lh", "sales_2024", "dest-ws", "dest-lh", "sales_2024"
        )

    def test_copy_table_glob_no_match(self) -> None:
        """Glob with no matches raises NOT_FOUND error."""
        runner = CliRunner()
        with patch(
            "fabio.commands.lakehouse.client.get",
            return_value=MOCK_TABLES_RESPONSE,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "copy-table",
                    "-sw",
                    "src-ws",
                    "-si",
                    "src-lh",
                    "-st",
                    "nonexist_*",
                    "-dw",
                    "dest-ws",
                    "-di",
                    "dest-lh",
                ],
            )

        assert result.exit_code != 0
        assert "NOT_FOUND" in result.output or "No tables match" in result.output

    def test_copy_table_glob_ignores_dest_table(self) -> None:
        """When glob is used, --dest-table is ignored (each table keeps its name)."""
        runner = CliRunner()
        with (
            patch(
                "fabio.commands.lakehouse.client.get",
                return_value=MOCK_TABLES_RESPONSE,
            ),
            patch(
                "fabio.commands.lakehouse.client.copy_table",
                return_value={"filesCopied": 1},
            ) as mock_copy,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "copy-table",
                    "-sw",
                    "src-ws",
                    "-si",
                    "src-lh",
                    "-st",
                    "sales_*",
                    "-dw",
                    "dest-ws",
                    "-di",
                    "dest-lh",
                    "-dt",
                    "ignored_name",
                ],
            )

        assert result.exit_code == 0, result.output
        # Glob mode ignores --dest-table, uses source name
        mock_copy.assert_any_call(
            "src-ws", "src-lh", "sales_2023", "dest-ws", "dest-lh", "sales_2023"
        )


class TestMoveTableGlob:
    """Tests for move-table with glob patterns."""

    def test_move_single_table_no_glob(self) -> None:
        """Non-glob move works as before."""
        runner = CliRunner()
        with patch(
            "fabio.commands.lakehouse.client.move_table",
            return_value={
                "filesCopied": 3,
                "sourceTable": "staging",
                "destTable": "staging",
                "status": "moved",
            },
        ) as mock_move:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "move-table",
                    "-sw",
                    "src-ws",
                    "-si",
                    "src-lh",
                    "-st",
                    "staging",
                    "-dw",
                    "dest-ws",
                    "-di",
                    "dest-lh",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "moved"
        mock_move.assert_called_once_with(
            "src-ws", "src-lh", "staging", "dest-ws", "dest-lh", "staging"
        )

    def test_move_table_glob_pattern(self) -> None:
        """Glob pattern moves all matching tables."""
        runner = CliRunner()
        with (
            patch(
                "fabio.commands.lakehouse.client.get",
                return_value=MOCK_TABLES_RESPONSE,
            ),
            patch(
                "fabio.commands.lakehouse.client.move_table",
                return_value={"filesCopied": 2, "status": "moved"},
            ) as mock_move,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "move-table",
                    "-sw",
                    "src-ws",
                    "-si",
                    "src-lh",
                    "-st",
                    "staging_*",
                    "-dw",
                    "dest-ws",
                    "-di",
                    "dest-lh",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        tables = [r["destTable"] for r in parsed["data"]]
        assert tables == ["staging_orders", "staging_products"]
        assert mock_move.call_count == 2
        mock_move.assert_any_call(
            "src-ws", "src-lh", "staging_orders", "dest-ws", "dest-lh", "staging_orders"
        )
        mock_move.assert_any_call(
            "src-ws", "src-lh", "staging_products", "dest-ws", "dest-lh", "staging_products"
        )

    def test_move_table_glob_no_match(self) -> None:
        """Glob with no matches raises NOT_FOUND error."""
        runner = CliRunner()
        with patch(
            "fabio.commands.lakehouse.client.get",
            return_value=MOCK_TABLES_RESPONSE,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "move-table",
                    "-sw",
                    "src-ws",
                    "-si",
                    "src-lh",
                    "-st",
                    "nonexist_*",
                    "-dw",
                    "dest-ws",
                    "-di",
                    "dest-lh",
                ],
            )

        assert result.exit_code != 0
        assert "NOT_FOUND" in result.output or "No tables match" in result.output
