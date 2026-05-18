"""Tests for lakehouse sync-files command."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING
from unittest.mock import patch

if TYPE_CHECKING:
    from pathlib import Path

from click.testing import CliRunner

from fabio.cli import main
from fabio.commands.lakehouse import _compute_sync_plan, _list_local_files


class TestComputeSyncPlan:
    """Unit tests for the sync plan computation logic."""

    def test_new_files_in_source(self) -> None:
        source = {
            "a.txt": {"size": 100, "mtime": 1000.0},
            "b.txt": {"size": 200, "mtime": 1000.0},
        }
        dest: dict[str, dict[str, object]] = {}
        plan = _compute_sync_plan(source, dest)
        assert plan["transfer"] == ["a.txt", "b.txt"]
        assert plan["skip"] == []
        assert plan["delete"] == []

    def test_identical_files_skipped(self) -> None:
        source = {
            "a.txt": {"size": 100, "mtime": 1000.0},
        }
        dest = {
            "a.txt": {"size": 100, "mtime": 1000.0},
        }
        plan = _compute_sync_plan(source, dest)
        assert plan["transfer"] == []
        assert plan["skip"] == ["a.txt"]
        assert plan["delete"] == []

    def test_size_change_triggers_transfer(self) -> None:
        source = {
            "a.txt": {"size": 150, "mtime": 1000.0},
        }
        dest = {
            "a.txt": {"size": 100, "mtime": 1000.0},
        }
        plan = _compute_sync_plan(source, dest)
        assert plan["transfer"] == ["a.txt"]
        assert plan["skip"] == []

    def test_newer_source_triggers_transfer(self) -> None:
        source = {
            "a.txt": {"size": 100, "mtime": 1005.0},
        }
        dest = {
            "a.txt": {"size": 100, "mtime": 1000.0},
        }
        plan = _compute_sync_plan(source, dest)
        assert plan["transfer"] == ["a.txt"]

    def test_mtime_tolerance_skips(self) -> None:
        # Source is only 0.5s newer - within 1.0s tolerance
        source = {
            "a.txt": {"size": 100, "mtime": 1000.5},
        }
        dest = {
            "a.txt": {"size": 100, "mtime": 1000.0},
        }
        plan = _compute_sync_plan(source, dest)
        assert plan["transfer"] == []
        assert plan["skip"] == ["a.txt"]

    def test_delete_flag_detects_orphans(self) -> None:
        source = {
            "a.txt": {"size": 100, "mtime": 1000.0},
        }
        dest = {
            "a.txt": {"size": 100, "mtime": 1000.0},
            "orphan.txt": {"size": 50, "mtime": 900.0},
        }
        plan = _compute_sync_plan(source, dest, delete=True)
        assert plan["skip"] == ["a.txt"]
        assert plan["delete"] == ["orphan.txt"]

    def test_delete_flag_false_ignores_orphans(self) -> None:
        source = {"a.txt": {"size": 100, "mtime": 1000.0}}
        dest = {
            "a.txt": {"size": 100, "mtime": 1000.0},
            "orphan.txt": {"size": 50, "mtime": 900.0},
        }
        plan = _compute_sync_plan(source, dest, delete=False)
        assert plan["delete"] == []

    def test_mixed_scenario(self) -> None:
        source = {
            "new.txt": {"size": 100, "mtime": 1000.0},
            "changed.txt": {"size": 200, "mtime": 2000.0},
            "same.txt": {"size": 300, "mtime": 1000.0},
        }
        dest = {
            "changed.txt": {"size": 150, "mtime": 1000.0},
            "same.txt": {"size": 300, "mtime": 1000.0},
            "gone.txt": {"size": 50, "mtime": 500.0},
        }
        plan = _compute_sync_plan(source, dest, delete=True)
        assert plan["transfer"] == ["changed.txt", "new.txt"]
        assert plan["skip"] == ["same.txt"]
        assert plan["delete"] == ["gone.txt"]


class TestListLocalFiles:
    """Unit tests for local file listing."""

    def test_lists_files_recursively(self, tmp_path: Path) -> None:
        (tmp_path / "a.txt").write_text("hello")
        sub = tmp_path / "sub"
        sub.mkdir()
        (sub / "b.txt").write_text("world!")
        result = _list_local_files(tmp_path)
        assert "a.txt" in result
        assert "sub/b.txt" in result
        assert result["a.txt"]["size"] == 5
        assert result["sub/b.txt"]["size"] == 6

    def test_nonexistent_dir_returns_empty(self, tmp_path: Path) -> None:
        result = _list_local_files(tmp_path / "missing")
        assert result == {}

    def test_skips_directories(self, tmp_path: Path) -> None:
        (tmp_path / "a.txt").write_text("x")
        (tmp_path / "subdir").mkdir()
        result = _list_local_files(tmp_path)
        assert "a.txt" in result
        assert "subdir" not in result


class TestSyncFilesCommand:
    """Integration tests for the sync-files CLI command."""

    def test_push_dry_run(self, tmp_path: Path) -> None:
        runner = CliRunner()
        (tmp_path / "a.txt").write_text("hello")
        (tmp_path / "b.txt").write_text("world")

        # Remote is empty
        with patch("fabio.commands.lakehouse.client.list_onelake_files", return_value=[]):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "sync-files",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--direction",
                    "push",
                    "--local",
                    str(tmp_path),
                    "--remote",
                    "Files/data",
                    "--dry-run",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["dryRun"] is True
        assert parsed["data"]["filesTransferred"] == 2
        assert sorted(parsed["data"]["toTransfer"]) == ["a.txt", "b.txt"]

    def test_push_uploads_new_files(self, tmp_path: Path) -> None:
        runner = CliRunner()
        (tmp_path / "new.txt").write_text("new content")

        with (
            patch("fabio.commands.lakehouse.client.list_onelake_files", return_value=[]),
            patch("fabio.commands.lakehouse.client.upload_onelake_file") as mock_upload,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "sync-files",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--direction",
                    "push",
                    "--local",
                    str(tmp_path),
                    "--remote",
                    "Files",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["filesTransferred"] == 1
        mock_upload.assert_called_once_with("ws-001", "lh-001", "Files/new.txt", b"new content")

    def test_push_skips_unchanged_files(self, tmp_path: Path) -> None:
        runner = CliRunner()
        f = tmp_path / "same.txt"
        f.write_text("same content")
        stat = f.stat()

        # Remote file has same size and mtime >= local
        remote_entries = [
            {
                "name": "Files/Files/same.txt",
                "contentLength": str(stat.st_size),
                "lastModified": "Mon, 18 May 2026 23:59:59 GMT",
                "isDirectory": "false",
            },
        ]
        with patch(
            "fabio.commands.lakehouse.client.list_onelake_files", return_value=remote_entries
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "sync-files",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--direction",
                    "push",
                    "--local",
                    str(tmp_path),
                    "--remote",
                    "Files",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["filesTransferred"] == 0
        assert parsed["data"]["filesSkipped"] == 1

    def test_push_with_delete(self, tmp_path: Path) -> None:
        runner = CliRunner()
        (tmp_path / "keep.txt").write_text("keep")

        # Remote has a file not in local
        remote_entries = [
            {
                "name": "Files/data/Files/data/keep.txt",
                "contentLength": "4",
                "lastModified": "Mon, 18 May 2026 23:59:59 GMT",
                "isDirectory": "false",
            },
            {
                "name": "Files/data/Files/data/orphan.txt",
                "contentLength": "10",
                "lastModified": "Mon, 18 May 2026 23:59:59 GMT",
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
                    "sync-files",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--direction",
                    "push",
                    "--local",
                    str(tmp_path),
                    "--remote",
                    "Files/data",
                    "--delete",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["filesDeleted"] == 1
        mock_del.assert_called_once_with("ws-001", "lh-001", "Files/data/orphan.txt")

    def test_pull_downloads_files(self, tmp_path: Path) -> None:
        runner = CliRunner()
        dest = tmp_path / "pulled"

        remote_entries = [
            {
                "name": "Files/Files/data.csv",
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
                "fabio.commands.lakehouse.client.download_onelake_file",
                return_value=b"csv,data\n",
            ) as mock_dl,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "sync-files",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--direction",
                    "pull",
                    "--local",
                    str(dest),
                    "--remote",
                    "Files",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["filesTransferred"] == 1
        mock_dl.assert_called_once_with("ws-001", "lh-001", "Files/data.csv")
        # File written locally
        assert (dest / "data.csv").read_bytes() == b"csv,data\n"

    def test_pull_creates_subdirectories(self, tmp_path: Path) -> None:
        runner = CliRunner()
        dest = tmp_path / "out"

        remote_entries = [
            {
                "name": "Files/Files/sub/deep/file.txt",
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
            ),
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "sync-files",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--direction",
                    "pull",
                    "--local",
                    str(dest),
                    "--remote",
                    "Files",
                ],
            )

        assert result.exit_code == 0
        assert (dest / "sub" / "deep" / "file.txt").read_bytes() == b"hello"

    def test_pull_with_delete(self, tmp_path: Path) -> None:
        runner = CliRunner()
        # Pre-create a local file that doesn't exist remotely
        dest = tmp_path / "local"
        dest.mkdir()
        (dest / "orphan.txt").write_text("will be deleted")

        with (
            patch("fabio.commands.lakehouse.client.list_onelake_files", return_value=[]),
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "sync-files",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--direction",
                    "pull",
                    "--local",
                    str(dest),
                    "--remote",
                    "Files",
                    "--delete",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["filesDeleted"] == 1
        assert not (dest / "orphan.txt").exists()

    def test_push_local_not_exist_errors(self, tmp_path: Path) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            [
                "lakehouse",
                "sync-files",
                "-w",
                "ws-001",
                "--id",
                "lh-001",
                "--direction",
                "push",
                "--local",
                str(tmp_path / "nonexistent"),
                "--remote",
                "Files",
            ],
        )
        assert result.exit_code != 0
        err = json.loads(result.output)
        assert err["error"]["code"] == "INVALID_INPUT"

    def test_subdirectory_remote_path(self, tmp_path: Path) -> None:
        """Test sync with a non-top-level remote path (Files/sub)."""
        runner = CliRunner()
        (tmp_path / "x.txt").write_text("x")

        # Doubled prefix for Files/sub
        remote_entries = [
            {
                "name": "Files/sub/Files/sub/existing.txt",
                "contentLength": "5",
                "lastModified": "Mon, 18 May 2026 23:59:59 GMT",
                "isDirectory": "false",
            },
        ]
        with (
            patch(
                "fabio.commands.lakehouse.client.list_onelake_files",
                return_value=remote_entries,
            ),
            patch("fabio.commands.lakehouse.client.upload_onelake_file") as mock_upload,
        ):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "sync-files",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--direction",
                    "push",
                    "--local",
                    str(tmp_path),
                    "--remote",
                    "Files/sub",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        # x.txt is new, existing.txt only on remote (not deleted without --delete)
        assert parsed["data"]["filesTransferred"] == 1
        mock_upload.assert_called_once_with("ws-001", "lh-001", "Files/sub/x.txt", b"x")
