"""Tests for ``fabio notebook`` commands."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING
from unittest.mock import patch

if TYPE_CHECKING:
    from pathlib import Path

from click.testing import CliRunner

from fabio.cli import main


class TestNotebookCreate:
    def test_create_with_inline_code(self) -> None:
        runner = CliRunner()
        created = {
            "id": "nb-001",
            "displayName": "Process Data",
            "type": "Notebook",
        }
        with patch("fabio.commands.notebook.client.post", return_value=created) as mock_post:
            result = runner.invoke(
                main,
                [
                    "notebook",
                    "create",
                    "-w",
                    "ws-001",
                    "-n",
                    "Process Data",
                    "--code",
                    "df = spark.read.csv('Files/data.csv')",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["id"] == "nb-001"
        # Verify the body has correct structure
        call_body = mock_post.call_args[1]["body"]
        assert call_body["displayName"] == "Process Data"
        assert call_body["type"] == "Notebook"
        assert "definition" in call_body

    def test_create_from_file(self, tmp_path: Path) -> None:
        runner = CliRunner()
        code_file = tmp_path / "etl.py"
        code_file.write_text("# ETL Script\ndf = spark.sql('SELECT 1')\n")

        created = {"id": "nb-002", "displayName": "ETL", "type": "Notebook"}
        with patch("fabio.commands.notebook.client.post", return_value=created):
            result = runner.invoke(
                main,
                [
                    "notebook",
                    "create",
                    "-w",
                    "ws-001",
                    "-n",
                    "ETL",
                    "--file",
                    str(code_file),
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["id"] == "nb-002"

    def test_create_with_lakehouse(self) -> None:
        runner = CliRunner()
        created = {"id": "nb-003", "displayName": "NB", "type": "Notebook"}
        with patch("fabio.commands.notebook.client.post", return_value=created) as mock_post:
            result = runner.invoke(
                main,
                [
                    "notebook",
                    "create",
                    "-w",
                    "ws-001",
                    "-n",
                    "NB",
                    "--code",
                    "print('hi')",
                    "--lakehouse",
                    "lh-001",
                    "--lakehouse-name",
                    "MyLakehouse",
                ],
            )

        assert result.exit_code == 0
        # Verify lakehouse is in definition metadata
        call_body = mock_post.call_args[1]["body"]
        assert call_body["definition"] is not None

    def test_create_requires_code_or_file(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            ["notebook", "create", "-w", "ws-001", "-n", "Empty"],
        )

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "MISSING_PARAM"

    def test_create_file_not_found(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            [
                "notebook",
                "create",
                "-w",
                "ws-001",
                "-n",
                "NB",
                "--file",
                "/nonexistent/file.py",
            ],
        )

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "INVALID_INPUT"


class TestNotebookGetDefinition:
    def test_get_definition(self) -> None:
        runner = CliRunner()
        definition = {
            "definition": {
                "format": "ipynb",
                "parts": [{"path": "notebook-content.py", "payload": "..."}],
            }
        }
        with patch(
            "fabio.commands.notebook.client.get_item_definition",
            return_value=definition,
        ):
            result = runner.invoke(
                main,
                ["notebook", "get-definition", "-w", "ws-001", "--id", "nb-001"],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert "definition" in parsed["data"]
