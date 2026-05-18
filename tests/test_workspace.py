"""Tests for the new ``fabio workspace`` commands with structured output."""

from __future__ import annotations

import json
from unittest.mock import patch

from click.testing import CliRunner

from fabio.cli import main


def _fake_workspaces() -> dict[str, object]:
    return {
        "value": [
            {
                "id": "ws-001",
                "displayName": "My Workspace",
                "type": "Workspace",
                "capacityId": "cap-aaa",
            },
            {
                "id": "ws-002",
                "displayName": "Shared Analytics",
                "type": "Workspace",
                "capacityId": "cap-bbb",
            },
        ]
    }


class TestWorkspaceList:
    def test_json_output(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.workspace.client.get", return_value=_fake_workspaces()):
            result = runner.invoke(main, ["workspace", "list"])

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 2
        assert parsed["data"][0]["displayName"] == "My Workspace"
        assert parsed["data"][1]["id"] == "ws-002"

    def test_plain_output(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.workspace.client.get", return_value=_fake_workspaces()):
            result = runner.invoke(main, ["-o", "plain", "workspace", "list"])

        assert result.exit_code == 0
        lines = result.output.strip().split("\n")
        assert "ws-001" in lines
        assert "ws-002" in lines

    def test_query_filter(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.workspace.client.get", return_value=_fake_workspaces()):
            result = runner.invoke(main, ["--query", "[].id,displayName", "workspace", "list"])

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        # Only id and displayName should be present
        for item in parsed["data"]:
            assert set(item.keys()) <= {"id", "displayName"}

    def test_empty_workspaces(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.workspace.client.get", return_value={"value": []}):
            result = runner.invoke(main, ["workspace", "list"])

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"] == []
        assert parsed["count"] == 0

    def test_structured_error_on_auth_failure(self) -> None:
        runner = CliRunner()
        from fabio.errors import ErrorCode, FabioError

        with patch(
            "fabio.commands.workspace.client.get",
            side_effect=FabioError(ErrorCode.AUTH_REQUIRED, "Not authenticated"),
        ):
            result = runner.invoke(main, ["workspace", "list"])

        assert result.exit_code != 0
        # Error goes to stderr
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "AUTH_REQUIRED"


class TestWorkspaceShow:
    def test_show_by_name(self) -> None:
        runner = CliRunner()
        workspace_detail = {
            "id": "ws-001",
            "displayName": "My Workspace",
            "type": "Workspace",
            "capacityId": "cap-aaa",
        }
        with patch(
            "fabio.commands.workspace.client.get",
            side_effect=[_fake_workspaces(), workspace_detail],
        ):
            result = runner.invoke(main, ["workspace", "show", "--name", "My Workspace"])

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["id"] == "ws-001"
        assert parsed["data"]["displayName"] == "My Workspace"

    def test_show_by_id(self) -> None:
        runner = CliRunner()
        workspace_detail = {
            "id": "ws-001",
            "displayName": "My Workspace",
            "type": "Workspace",
        }
        with patch(
            "fabio.commands.workspace.client.get",
            return_value=workspace_detail,
        ):
            result = runner.invoke(main, ["workspace", "show", "--id", "ws-001"])

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["id"] == "ws-001"

    def test_show_not_found(self) -> None:
        runner = CliRunner()
        with patch(
            "fabio.commands.workspace.client.get",
            return_value=_fake_workspaces(),
        ):
            result = runner.invoke(main, ["workspace", "show", "--name", "Nonexistent"])

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "NOT_FOUND"

    def test_show_requires_id_or_name(self) -> None:
        runner = CliRunner()
        result = runner.invoke(main, ["workspace", "show"])

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "MISSING_PARAM"
