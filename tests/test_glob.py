"""Tests for glob pattern support in lakehouse file operations."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING
from unittest.mock import patch

if TYPE_CHECKING:
    from pathlib import Path

from click.testing import CliRunner

from fabio.cli import main
from fabio.commands.lakehouse import (
    _glob_base_dir,
    _glob_match,
    _has_glob,
    _resolve_local_glob,
)


class TestGlobHelpers:
    """Unit tests for glob helper functions."""

    def test_has_glob_star(self) -> None:
        assert _has_glob("*.csv") is True
        assert _has_glob("data/**/*.parquet") is True

    def test_has_glob_question(self) -> None:
        assert _has_glob("file?.txt") is True

    def test_has_glob_bracket(self) -> None:
        assert _has_glob("file[0-9].txt") is True

    def test_has_glob_no_metachar(self) -> None:
        assert _has_glob("Files/data.csv") is False
        assert _has_glob("plain_file") is False

    def test_glob_base_dir_simple(self) -> None:
        assert _glob_base_dir("Files/*.csv") == "Files"
        assert _glob_base_dir("Files/raw/*.parquet") == "Files/raw"
        assert _glob_base_dir("Tables/sales/*.parquet") == "Tables/sales"

    def test_glob_base_dir_doublestar(self) -> None:
        assert _glob_base_dir("Files/**/*.csv") == "Files"
        assert _glob_base_dir("Files/raw/**/*.parquet") == "Files/raw"

    def test_glob_base_dir_star_at_start(self) -> None:
        assert _glob_base_dir("*.csv") == "Files"  # fallback

    def test_glob_match_simple(self) -> None:
        assert _glob_match("Files/data.csv", "Files/*.csv") is True
        assert _glob_match("Files/data.parquet", "Files/*.csv") is False
        assert _glob_match("Files/raw/data.csv", "Files/*.csv") is False

    def test_glob_match_recursive(self) -> None:
        assert _glob_match("Files/raw/data.csv", "Files/**/*.csv") is True
        assert _glob_match("Files/raw/nested/deep.csv", "Files/**/*.csv") is True
        assert _glob_match("Files/data.csv", "Files/**/*.csv") is True
        assert _glob_match("Files/data.parquet", "Files/**/*.csv") is False

    def test_glob_match_question_mark(self) -> None:
        assert _glob_match("Files/file1.csv", "Files/file?.csv") is True
        assert _glob_match("Files/file12.csv", "Files/file?.csv") is False

    def test_glob_match_brackets(self) -> None:
        assert _glob_match("Files/data1.csv", "Files/data[0-9].csv") is True
        assert _glob_match("Files/datax.csv", "Files/data[0-9].csv") is False


class TestResolveLocalGlob:
    """Unit tests for local glob resolution."""

    def test_matches_files(self, tmp_path: Path) -> None:
        (tmp_path / "a.csv").write_text("a")
        (tmp_path / "b.csv").write_text("b")
        (tmp_path / "c.txt").write_text("c")
        result = _resolve_local_glob(str(tmp_path / "*.csv"))
        names = [f.name for f in result]
        assert sorted(names) == ["a.csv", "b.csv"]
        assert "c.txt" not in names

    def test_recursive_pattern(self, tmp_path: Path) -> None:
        sub = tmp_path / "sub"
        sub.mkdir()
        (tmp_path / "top.csv").write_text("t")
        (sub / "deep.csv").write_text("d")
        (sub / "other.txt").write_text("o")
        result = _resolve_local_glob(str(tmp_path / "**/*.csv"))
        names = [f.name for f in result]
        assert "top.csv" in names
        assert "deep.csv" in names
        assert "other.txt" not in names

    def test_no_matches_returns_empty(self, tmp_path: Path) -> None:
        (tmp_path / "data.txt").write_text("x")
        result = _resolve_local_glob(str(tmp_path / "*.csv"))
        assert result == []

    def test_skips_directories(self, tmp_path: Path) -> None:
        (tmp_path / "dir.csv").mkdir()  # directory with .csv name
        (tmp_path / "file.csv").write_text("x")
        result = _resolve_local_glob(str(tmp_path / "*.csv"))
        assert len(result) == 1
        assert result[0].name == "file.csv"


class TestUploadGlob:
    """Tests for upload command with glob patterns."""

    def test_upload_glob_multiple_files(self, tmp_path: Path) -> None:
        runner = CliRunner()
        (tmp_path / "a.csv").write_text("aaa")
        (tmp_path / "b.csv").write_text("bbb")
        (tmp_path / "c.txt").write_text("ccc")

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
                    "-sp",
                    str(tmp_path / "*.csv"),
                    "-dp",
                    "Files/data/",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        destinations = sorted(item["destination"] for item in parsed["data"])
        assert destinations == ["Files/data/a.csv", "Files/data/b.csv"]
        assert mock_upload.call_count == 2

    def test_upload_glob_preserves_subdir_structure(self, tmp_path: Path) -> None:
        runner = CliRunner()
        sub = tmp_path / "raw"
        sub.mkdir()
        (sub / "x.csv").write_text("x")

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
                    "-sp",
                    str(tmp_path / "**/*.csv"),
                    "-dp",
                    "Files/",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 1
        assert parsed["data"][0]["destination"] == "Files/raw/x.csv"

    def test_upload_glob_no_match_errors(self, tmp_path: Path) -> None:
        runner = CliRunner()
        (tmp_path / "data.txt").write_text("x")

        result = runner.invoke(
            main,
            [
                "lakehouse",
                "upload",
                "-w",
                "ws-001",
                "--id",
                "lh-001",
                "-sp",
                str(tmp_path / "*.csv"),
            ],
        )
        assert result.exit_code != 0
        err = json.loads(result.output)
        assert err["error"]["code"] == "NOT_FOUND"


class TestDownloadGlob:
    """Tests for download command with glob patterns."""

    def test_download_glob_multiple_files(self, tmp_path: Path) -> None:
        runner = CliRunner()
        # Simulate remote files with doubled prefix
        remote_entries = [
            {
                "name": "Files/Files/data1.csv",
                "contentLength": "5",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
            {
                "name": "Files/Files/data2.csv",
                "contentLength": "5",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
            {
                "name": "Files/Files/other.txt",
                "contentLength": "5",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
        ]
        with (
            patch(
                "fabio.commands.lakehouse.client.list_onelake_files",
                return_value=remote_entries,
            ),
            patch(
                "fabio.commands.lakehouse.client.download_onelake_file",
                return_value=b"hello",
            ) as mock_dl,
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
                    "-sp",
                    "Files/*.csv",
                    "-dp",
                    str(tmp_path),
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        # Downloaded the 2 csv files, not the .txt
        assert mock_dl.call_count == 2
        downloaded_sources = sorted(c.args[2] for c in mock_dl.call_args_list)
        assert downloaded_sources == ["Files/data1.csv", "Files/data2.csv"]

    def test_download_glob_no_match_errors(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.list_onelake_files", return_value=[]):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "download",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "-sp",
                    "Files/*.csv",
                ],
            )
        assert result.exit_code != 0
        err = json.loads(result.output)
        assert err["error"]["code"] == "NOT_FOUND"


class TestDeleteFileGlob:
    """Tests for delete-file command with glob patterns."""

    def test_delete_glob_multiple(self) -> None:
        runner = CliRunner()
        remote_entries = [
            {
                "name": "Files/temp/Files/temp/a.csv",
                "contentLength": "10",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
            {
                "name": "Files/temp/Files/temp/b.csv",
                "contentLength": "20",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
            {
                "name": "Files/temp/Files/temp/keep.txt",
                "contentLength": "30",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
        ]
        with (
            patch(
                "fabio.commands.lakehouse.client.list_onelake_files",
                return_value=remote_entries,
            ),
            patch("fabio.commands.lakehouse.client.delete_onelake_file") as mock_del,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "delete-file",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "-p",
                    "Files/temp/*.csv",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        assert mock_del.call_count == 2
        deleted_paths = sorted(c.args[2] for c in mock_del.call_args_list)
        assert deleted_paths == ["Files/temp/a.csv", "Files/temp/b.csv"]


class TestCopyFileGlob:
    """Tests for copy-file command with glob patterns."""

    def test_copy_glob_multiple(self) -> None:
        runner = CliRunner()
        remote_entries = [
            {
                "name": "Files/raw/Files/raw/x.csv",
                "contentLength": "10",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
            {
                "name": "Files/raw/Files/raw/y.csv",
                "contentLength": "20",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
        ]
        with (
            patch(
                "fabio.commands.lakehouse.client.list_onelake_files",
                return_value=remote_entries,
            ),
            patch(
                "fabio.commands.lakehouse.client.copy_onelake_file",
                side_effect=lambda *a, **kw: {"copyStatus": "success"},
            ) as mock_copy,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "copy-file",
                    "-sw",
                    "ws-001",
                    "-si",
                    "lh-001",
                    "-sp",
                    "Files/raw/*.csv",
                    "-dw",
                    "ws-002",
                    "-di",
                    "lh-002",
                    "-dp",
                    "Files/staging/",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        assert mock_copy.call_count == 2
        # Check destination paths
        destinations = sorted(item["destination"] for item in parsed["data"])
        assert destinations == ["Files/staging/x.csv", "Files/staging/y.csv"]


class TestMoveFileGlob:
    """Tests for move-file command with glob patterns."""

    def test_move_glob_multiple(self) -> None:
        runner = CliRunner()
        remote_entries = [
            {
                "name": "Files/staging/Files/staging/report.parquet",
                "contentLength": "100",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
        ]
        with (
            patch(
                "fabio.commands.lakehouse.client.list_onelake_files",
                return_value=remote_entries,
            ),
            patch(
                "fabio.commands.lakehouse.client.move_onelake_file",
                side_effect=lambda *a, **kw: {"copyStatus": "success"},
            ) as mock_move,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "move-file",
                    "-sw",
                    "ws-001",
                    "-si",
                    "lh-001",
                    "-sp",
                    "Files/staging/*.parquet",
                    "-dw",
                    "ws-002",
                    "-di",
                    "lh-002",
                    "-dp",
                    "Files/archive/",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 1
        assert parsed["data"][0]["destination"] == "Files/archive/report.parquet"
        mock_move.assert_called_once_with(
            "ws-001",
            "lh-001",
            "Files/staging/report.parquet",
            "ws-002",
            "lh-002",
            "Files/archive/report.parquet",
        )


class TestResolveRemoteGlob:
    """Tests for _resolve_remote_glob via commands (integration)."""

    def test_recursive_glob_pattern(self) -> None:
        """Test ** pattern matches files in subdirectories."""
        runner = CliRunner()
        remote_entries = [
            {
                "name": "Files/Files/top.csv",
                "contentLength": "10",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
            {
                "name": "Files/Files/sub/deep.csv",
                "contentLength": "20",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
            {
                "name": "Files/Files/sub/nested/very_deep.csv",
                "contentLength": "30",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
            {
                "name": "Files/Files/other.txt",
                "contentLength": "5",
                "lastModified": "Mon, 18 May 2026 10:00:00 GMT",
                "isDirectory": "false",
            },
        ]
        with (
            patch(
                "fabio.commands.lakehouse.client.list_onelake_files",
                return_value=remote_entries,
            ),
            patch("fabio.commands.lakehouse.client.delete_onelake_file") as mock_del,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "delete-file",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "-p",
                    "Files/**/*.csv",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        # Should match all 3 .csv files (not .txt)
        assert parsed["count"] == 3
        deleted = sorted(item["path"] for item in parsed["data"])
        assert deleted == [
            "Files/sub/deep.csv",
            "Files/sub/nested/very_deep.csv",
            "Files/top.csv",
        ]
