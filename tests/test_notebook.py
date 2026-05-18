"""Tests for ``fabio notebook`` commands."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING
from unittest.mock import MagicMock, patch

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


class TestNotebookRun:
    def test_run_returns_job_id(self) -> None:
        """Run without --wait returns immediately with job instance ID."""
        runner = CliRunner()
        mock_resp = MagicMock()
        mock_resp.status_code = 202
        mock_resp.headers = {
            "Location": (
                "https://api.fabric.microsoft.com/v1/workspaces/ws-001"
                "/items/nb-001/jobs/instances/job-123"
            )
        }
        mock_resp.text = ""

        with (
            patch("fabio.commands.notebook.client.require_auth", return_value="tok"),
            patch("fabio.commands.notebook.req.post", return_value=mock_resp),
        ):
            result = runner.invoke(
                main,
                ["notebook", "run", "-w", "ws-001", "--id", "nb-001"],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["jobInstanceId"] == "job-123"
        assert parsed["data"]["status"] == "submitted"

    def test_run_with_wait_polls_until_completed(self) -> None:
        """Run with --wait polls until status is Completed."""
        runner = CliRunner()
        mock_resp = MagicMock()
        mock_resp.status_code = 202
        mock_resp.headers = {
            "Location": (
                "https://api.fabric.microsoft.com/v1/workspaces/ws-001"
                "/items/nb-001/jobs/instances/job-456"
            )
        }
        mock_resp.text = ""

        poll_resp = MagicMock()
        poll_resp.ok = True
        poll_resp.text = '{"status": "Completed"}'
        poll_resp.json.return_value = {"status": "Completed"}

        with (
            patch("fabio.commands.notebook.client.require_auth", return_value="tok"),
            patch("fabio.commands.notebook.req.post", return_value=mock_resp),
            patch("fabio.commands.notebook.req.get", return_value=poll_resp),
            patch("fabio.commands.notebook.time.sleep"),
        ):
            result = runner.invoke(
                main,
                ["notebook", "run", "-w", "ws-001", "--id", "nb-001", "--wait"],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "Completed"

    def test_run_with_wait_reports_failure(self) -> None:
        """Run with --wait reports failure reason."""
        runner = CliRunner()
        mock_resp = MagicMock()
        mock_resp.status_code = 202
        mock_resp.headers = {
            "Location": (
                "https://api.fabric.microsoft.com/v1/workspaces/ws-001"
                "/items/nb-001/jobs/instances/job-789"
            )
        }
        mock_resp.text = ""

        poll_resp = MagicMock()
        poll_resp.ok = True
        poll_resp.text = '{"status": "Failed", "failureReason": "Spark error"}'
        poll_resp.json.return_value = {
            "status": "Failed",
            "failureReason": "Spark error",
        }

        with (
            patch("fabio.commands.notebook.client.require_auth", return_value="tok"),
            patch("fabio.commands.notebook.req.post", return_value=mock_resp),
            patch("fabio.commands.notebook.req.get", return_value=poll_resp),
            patch("fabio.commands.notebook.time.sleep"),
        ):
            result = runner.invoke(
                main,
                ["notebook", "run", "-w", "ws-001", "--id", "nb-001", "--wait"],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "Failed"
        assert parsed["data"]["failureReason"] == "Spark error"

    def test_run_no_location_header_errors(self) -> None:
        """Run fails gracefully if no Location header returned."""
        runner = CliRunner()
        mock_resp = MagicMock()
        mock_resp.status_code = 202
        mock_resp.headers = {}
        mock_resp.text = ""

        with (
            patch("fabio.commands.notebook.client.require_auth", return_value="tok"),
            patch("fabio.commands.notebook.req.post", return_value=mock_resp),
        ):
            result = runner.invoke(
                main,
                ["notebook", "run", "-w", "ws-001", "--id", "nb-001"],
            )

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "API_ERROR"


class TestNotebookStatus:
    def test_status_returns_job_info(self) -> None:
        runner = CliRunner()
        job_data = {
            "id": "job-123",
            "itemId": "nb-001",
            "jobType": "RunNotebook",
            "status": "InProgress",
            "startTimeUtc": "2025-01-01T00:00:00Z",
            "endTimeUtc": None,
        }
        with patch("fabio.commands.notebook.client.get", return_value=job_data):
            result = runner.invoke(
                main,
                [
                    "notebook",
                    "status",
                    "-w",
                    "ws-001",
                    "--id",
                    "nb-001",
                    "-j",
                    "job-123",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "InProgress"
        assert parsed["data"]["id"] == "job-123"

    def test_status_completed(self) -> None:
        runner = CliRunner()
        job_data = {
            "id": "job-456",
            "status": "Completed",
            "startTimeUtc": "2025-01-01T00:00:00Z",
            "endTimeUtc": "2025-01-01T00:05:00Z",
        }
        with patch("fabio.commands.notebook.client.get", return_value=job_data):
            result = runner.invoke(
                main,
                [
                    "notebook",
                    "status",
                    "-w",
                    "ws-001",
                    "--id",
                    "nb-001",
                    "-j",
                    "job-456",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "Completed"


class TestNotebookStop:
    def test_stop_cancels_job(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.notebook.client.post", return_value=None):
            result = runner.invoke(
                main,
                [
                    "notebook",
                    "stop",
                    "-w",
                    "ws-001",
                    "--id",
                    "nb-001",
                    "-j",
                    "job-123",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "cancelled"
        assert parsed["data"]["jobInstanceId"] == "job-123"

    def test_stop_calls_cancel_endpoint(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.notebook.client.post") as mock_post:
            mock_post.return_value = None
            runner.invoke(
                main,
                [
                    "notebook",
                    "stop",
                    "-w",
                    "ws-001",
                    "--id",
                    "nb-001",
                    "-j",
                    "job-999",
                ],
            )

        mock_post.assert_called_once_with(
            "/workspaces/ws-001/items/nb-001/jobs/instances/job-999/cancel"
        )


class TestNotebookDelete:
    def test_delete_notebook(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.notebook.client.delete") as mock_del:
            result = runner.invoke(
                main,
                ["notebook", "delete", "-w", "ws-001", "--id", "nb-001"],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "deleted"
        assert parsed["data"]["notebookId"] == "nb-001"
        mock_del.assert_called_once_with("/workspaces/ws-001/items/nb-001")

    def test_delete_plain_output(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.notebook.client.delete"):
            result = runner.invoke(
                main,
                [
                    "-o",
                    "plain",
                    "notebook",
                    "delete",
                    "-w",
                    "ws-001",
                    "--id",
                    "nb-001",
                ],
            )

        assert result.exit_code == 0
        assert "nb-001" in result.output
