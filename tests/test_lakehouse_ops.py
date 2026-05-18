"""Tests for lakehouse upload, download, and load-table commands."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING
from unittest.mock import patch

if TYPE_CHECKING:
    from pathlib import Path

from click.testing import CliRunner

from fabio.cli import main


class TestLakehouseUpload:
    def test_upload_file(self, tmp_path: Path) -> None:
        runner = CliRunner()
        # Create a temp CSV
        csv_file = tmp_path / "data.csv"
        csv_file.write_text("id,name\n1,Alice\n2,Bob\n")

        with patch("fabio.commands.lakehouse.client.upload_onelake_file") as mock_upload:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "upload",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--source",
                    str(csv_file),
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "uploaded"
        assert parsed["data"]["destination"] == "Files/data.csv"
        mock_upload.assert_called_once_with(
            "ws-001", "lh-001", "Files/data.csv", csv_file.read_bytes()
        )

    def test_upload_custom_dest(self, tmp_path: Path) -> None:
        runner = CliRunner()
        csv_file = tmp_path / "input.csv"
        csv_file.write_text("a,b\n1,2\n")

        with patch("fabio.commands.lakehouse.client.upload_onelake_file"):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "upload",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--source",
                    str(csv_file),
                    "--dest",
                    "Files/raw/input.csv",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["destination"] == "Files/raw/input.csv"

    def test_upload_missing_file(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            [
                "lakehouse",
                "upload",
                "-w",
                "ws-001",
                "--id",
                "lh-001",
                "--source",
                "/nonexistent/file.csv",
            ],
        )

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "INVALID_INPUT"


class TestLakehouseDownload:
    def test_download_file(self, tmp_path: Path) -> None:
        runner = CliRunner()
        content = b"id,name\n1,Alice\n"
        dest = tmp_path / "out.csv"

        with patch(
            "fabio.commands.lakehouse.client.download_onelake_file",
            return_value=content,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "download",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--path",
                    "Files/data.csv",
                    "--dest",
                    str(dest),
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "downloaded"
        assert dest.read_bytes() == content


class TestLakehouseLoadTable:
    def test_load_csv_table(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.load_table", return_value={}) as mock_load:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "load-table",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--table",
                    "orders",
                    "--path",
                    "Files/orders.csv",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "loaded"
        assert parsed["data"]["table"] == "orders"
        mock_load.assert_called_once_with(
            "ws-001",
            "lh-001",
            "orders",
            "Files/orders.csv",
            file_extension="csv",
            format_options={"format": "Csv", "header": "true", "delimiter": ","},
            mode="Overwrite",
        )

    def test_load_parquet_table(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.load_table", return_value={}) as mock_load:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "load-table",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--table",
                    "sales",
                    "--path",
                    "Files/sales.parquet",
                ],
            )

        assert result.exit_code == 0
        # Parquet should have format_options with just format key
        mock_load.assert_called_once_with(
            "ws-001",
            "lh-001",
            "sales",
            "Files/sales.parquet",
            file_extension="parquet",
            format_options={"format": "Parquet"},
            mode="Overwrite",
        )


class TestLakehouseCopyFile:
    def test_copy_file_basic(self) -> None:
        runner = CliRunner()
        with patch(
            "fabio.commands.lakehouse.client.copy_onelake_file",
            return_value={"copyId": "abc-123", "copyStatus": "success"},
        ) as mock_copy:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "copy-file",
                    "-sw",
                    "src-ws",
                    "-si",
                    "src-lh",
                    "-sp",
                    "Files/data.csv",
                    "-dw",
                    "dest-ws",
                    "-di",
                    "dest-lh",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "copied"
        assert parsed["data"]["copyStatus"] == "success"
        assert "src-ws/src-lh/Files/data.csv" in parsed["data"]["source"]
        assert "dest-ws/dest-lh/Files/data.csv" in parsed["data"]["destination"]
        mock_copy.assert_called_once_with(
            "src-ws",
            "src-lh",
            "Files/data.csv",
            "dest-ws",
            "dest-lh",
            "Files/data.csv",
        )

    def test_copy_file_custom_dest_path(self) -> None:
        runner = CliRunner()
        with patch(
            "fabio.commands.lakehouse.client.copy_onelake_file",
            return_value={"copyId": "def-456", "copyStatus": "success"},
        ) as mock_copy:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "copy-file",
                    "-sw",
                    "src-ws",
                    "-si",
                    "src-lh",
                    "-sp",
                    "Files/raw/input.parquet",
                    "-dw",
                    "dest-ws",
                    "-di",
                    "dest-lh",
                    "-dp",
                    "Files/staging/input.parquet",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert "dest-ws/dest-lh/Files/staging/input.parquet" in parsed["data"]["destination"]
        mock_copy.assert_called_once_with(
            "src-ws",
            "src-lh",
            "Files/raw/input.parquet",
            "dest-ws",
            "dest-lh",
            "Files/staging/input.parquet",
        )

    def test_copy_file_same_workspace(self) -> None:
        runner = CliRunner()
        with patch(
            "fabio.commands.lakehouse.client.copy_onelake_file",
            return_value={"copyId": "ghi-789", "copyStatus": "success"},
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "copy-file",
                    "-sw",
                    "ws-001",
                    "-si",
                    "lh-src",
                    "-sp",
                    "Files/report.csv",
                    "-dw",
                    "ws-001",
                    "-di",
                    "lh-dest",
                    "-dp",
                    "Files/report_backup.csv",
                ],
            )

        assert result.exit_code == 0, result.output
