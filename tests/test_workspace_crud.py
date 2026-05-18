"""Tests for new workspace create/delete commands."""

from __future__ import annotations

import json
from unittest.mock import patch

from click.testing import CliRunner

from fabio.cli import main


class TestWorkspaceCreate:
    def test_create_workspace(self) -> None:
        runner = CliRunner()
        created = {
            "id": "ws-new",
            "displayName": "New Workspace",
            "type": "Workspace",
            "capacityId": "cap-001",
        }
        with patch("fabio.commands.workspace.client.post", return_value=created) as mock:
            result = runner.invoke(main, ["workspace", "create", "--name", "New Workspace"])

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["id"] == "ws-new"
        mock.assert_called_once_with("/workspaces", body={"displayName": "New Workspace"})

    def test_create_with_capacity(self) -> None:
        runner = CliRunner()
        created = {"id": "ws-new", "displayName": "WS", "capacityId": "cap-1"}
        with patch("fabio.commands.workspace.client.post", return_value=created) as mock:
            result = runner.invoke(
                main,
                ["workspace", "create", "-n", "WS", "--capacity", "cap-1"],
            )

        assert result.exit_code == 0
        call_body = mock.call_args[1]["body"]
        assert call_body["capacityId"] == "cap-1"


class TestWorkspaceDelete:
    def test_delete_workspace(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.workspace.client.delete", return_value={}) as mock:
            result = runner.invoke(main, ["workspace", "delete", "--id", "ws-001"])

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "deleted"
        mock.assert_called_once_with("/workspaces/ws-001")
