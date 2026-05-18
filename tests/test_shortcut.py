"""Tests for lakehouse shortcut commands (create, get, delete)."""

from __future__ import annotations

import json
from unittest.mock import patch

from click.testing import CliRunner

from fabio.cli import main


class TestCreateShortcutOneLake:
    """Tests for 'lakehouse create-shortcut' with OneLake target."""

    def test_create_onelake_shortcut(self) -> None:
        runner = CliRunner()
        mock_response = {
            "name": "sales",
            "path": "Tables",
            "target": {
                "oneLake": {
                    "workspaceId": "src-ws-001",
                    "itemId": "src-lh-001",
                    "path": "Tables/sales",
                }
            },
        }

        with patch(
            "fabio.commands.lakehouse.client.post", return_value=mock_response
        ) as mock_post:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "create-shortcut",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--name",
                    "sales",
                    "--source-workspace",
                    "src-ws-001",
                    "--source-id",
                    "src-lh-001",
                    "--source-path",
                    "Tables/sales",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "created"
        assert parsed["data"]["shortcutName"] == "sales"
        assert parsed["data"]["shortcutPath"] == "Tables"

        mock_post.assert_called_once_with(
            "/workspaces/ws-001/items/lh-001/shortcuts",
            body={
                "path": "Tables",
                "name": "sales",
                "target": {
                    "oneLake": {
                        "workspaceId": "src-ws-001",
                        "itemId": "src-lh-001",
                        "path": "Tables/sales",
                    }
                },
            },
        )

    def test_create_onelake_shortcut_files_path(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.post", return_value={}) as mock_post:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "create-shortcut",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--name",
                    "raw-data",
                    "--path",
                    "Files",
                    "--source-workspace",
                    "src-ws-001",
                    "--source-id",
                    "src-lh-001",
                    "--source-path",
                    "Files/raw",
                ],
            )

        assert result.exit_code == 0, result.output
        call_body = mock_post.call_args[1]["body"]
        assert call_body["path"] == "Files"
        assert call_body["name"] == "raw-data"
        assert call_body["target"]["oneLake"]["path"] == "Files/raw"

    def test_create_onelake_shortcut_missing_source_workspace(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            [
                "lakehouse",
                "create-shortcut",
                "-w",
                "ws-001",
                "--id",
                "lh-001",
                "--name",
                "sales",
                "--source-id",
                "src-lh-001",
                "--source-path",
                "Tables/sales",
            ],
        )

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "INVALID_INPUT"
        assert "--source-workspace" in parsed["error"]["message"]

    def test_create_onelake_shortcut_missing_source_id(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            [
                "lakehouse",
                "create-shortcut",
                "-w",
                "ws-001",
                "--id",
                "lh-001",
                "--name",
                "sales",
                "--source-workspace",
                "src-ws-001",
                "--source-path",
                "Tables/sales",
            ],
        )

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "INVALID_INPUT"

    def test_create_onelake_shortcut_missing_source_path(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            [
                "lakehouse",
                "create-shortcut",
                "-w",
                "ws-001",
                "--id",
                "lh-001",
                "--name",
                "sales",
                "--source-workspace",
                "src-ws-001",
                "--source-id",
                "src-lh-001",
            ],
        )

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "INVALID_INPUT"


class TestCreateShortcutADLS:
    """Tests for 'lakehouse create-shortcut' with ADLS Gen2 target."""

    def test_create_adls_shortcut(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.post", return_value={}) as mock_post:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "create-shortcut",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--name",
                    "external-data",
                    "--path",
                    "Files",
                    "--target-type",
                    "adls",
                    "--location",
                    "https://myaccount.dfs.core.windows.net",
                    "--subpath",
                    "/container/data",
                    "--connection-id",
                    "conn-123",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "created"

        call_body = mock_post.call_args[1]["body"]
        assert call_body["target"]["adlsGen2"]["location"] == (
            "https://myaccount.dfs.core.windows.net"
        )
        assert call_body["target"]["adlsGen2"]["subpath"] == "/container/data"
        assert call_body["target"]["adlsGen2"]["connectionId"] == "conn-123"

    def test_create_adls_shortcut_without_connection(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.post", return_value={}) as mock_post:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "create-shortcut",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--name",
                    "public-data",
                    "--path",
                    "Files",
                    "--target-type",
                    "adls",
                    "--location",
                    "https://myaccount.dfs.core.windows.net",
                    "--subpath",
                    "/public/data",
                ],
            )

        assert result.exit_code == 0, result.output
        call_body = mock_post.call_args[1]["body"]
        assert "connectionId" not in call_body["target"]["adlsGen2"]

    def test_create_adls_shortcut_missing_location(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            [
                "lakehouse",
                "create-shortcut",
                "-w",
                "ws-001",
                "--id",
                "lh-001",
                "--name",
                "fail",
                "--target-type",
                "adls",
                "--subpath",
                "/container/data",
            ],
        )

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "INVALID_INPUT"
        assert "ADLS" in parsed["error"]["message"]


class TestCreateShortcutS3:
    """Tests for 'lakehouse create-shortcut' with S3 target."""

    def test_create_s3_shortcut(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.post", return_value={}) as mock_post:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "create-shortcut",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--name",
                    "s3-data",
                    "--path",
                    "Files",
                    "--target-type",
                    "s3",
                    "--location",
                    "https://mybucket.s3.us-east-1.amazonaws.com",
                    "--subpath",
                    "/prefix/data",
                    "--connection-id",
                    "s3-conn-001",
                ],
            )

        assert result.exit_code == 0, result.output
        call_body = mock_post.call_args[1]["body"]
        assert call_body["target"]["amazonS3"]["location"] == (
            "https://mybucket.s3.us-east-1.amazonaws.com"
        )
        assert call_body["target"]["amazonS3"]["subpath"] == "/prefix/data"
        assert call_body["target"]["amazonS3"]["connectionId"] == "s3-conn-001"

    def test_create_s3_shortcut_missing_subpath(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            [
                "lakehouse",
                "create-shortcut",
                "-w",
                "ws-001",
                "--id",
                "lh-001",
                "--name",
                "fail",
                "--target-type",
                "s3",
                "--location",
                "https://mybucket.s3.us-east-1.amazonaws.com",
            ],
        )

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "INVALID_INPUT"
        assert "S3" in parsed["error"]["message"]


class TestGetShortcut:
    """Tests for 'lakehouse get-shortcut'."""

    def test_get_shortcut(self) -> None:
        runner = CliRunner()
        mock_response = {
            "name": "sales",
            "path": "Tables",
            "target": {
                "oneLake": {
                    "workspaceId": "src-ws-001",
                    "itemId": "src-lh-001",
                    "path": "Tables/sales",
                }
            },
        }

        with patch("fabio.commands.lakehouse.client.get", return_value=mock_response):
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "get-shortcut",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--name",
                    "sales",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["name"] == "sales"
        assert parsed["data"]["target"]["oneLake"]["workspaceId"] == "src-ws-001"

    def test_get_shortcut_files_path(self) -> None:
        runner = CliRunner()
        with patch(
            "fabio.commands.lakehouse.client.get", return_value={"name": "raw"}
        ) as mock_get:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "get-shortcut",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--name",
                    "raw",
                    "--path",
                    "Files",
                ],
            )

        assert result.exit_code == 0, result.output
        mock_get.assert_called_once_with("/workspaces/ws-001/items/lh-001/shortcuts/Files/raw")


class TestDeleteShortcut:
    """Tests for 'lakehouse delete-shortcut'."""

    def test_delete_shortcut(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.delete", return_value={}) as mock_delete:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "delete-shortcut",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--name",
                    "sales",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "deleted"
        assert parsed["data"]["shortcutName"] == "sales"
        mock_delete.assert_called_once_with(
            "/workspaces/ws-001/items/lh-001/shortcuts/Tables/sales"
        )

    def test_delete_shortcut_files_path(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.lakehouse.client.delete", return_value={}) as mock_delete:
            result = runner.invoke(
                main,
                [
                    "lakehouse",
                    "delete-shortcut",
                    "-w",
                    "ws-001",
                    "--id",
                    "lh-001",
                    "--name",
                    "raw",
                    "--path",
                    "Files",
                ],
            )

        assert result.exit_code == 0, result.output
        mock_delete.assert_called_once_with("/workspaces/ws-001/items/lh-001/shortcuts/Files/raw")
