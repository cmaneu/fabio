"""Tests for ``fabio workspace ls`` command."""

from __future__ import annotations

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


class TestWorkspaceLs:
    def test_lists_workspaces(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.workspace.client.get", return_value=_fake_workspaces()):
            result = runner.invoke(main, ["workspace", "ls"])

        assert result.exit_code == 0
        assert "My Workspace" in result.output
        assert "Shared Analytics" in result.output
        assert "ws-001" in result.output
        assert "ws-002" in result.output

    def test_empty_workspaces(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.workspace.client.get", return_value={"value": []}):
            result = runner.invoke(main, ["workspace", "ls"])

        assert result.exit_code == 0
        assert "No workspaces found" in result.output

    def test_api_error(self) -> None:
        runner = CliRunner()
        import click

        with patch(
            "fabio.commands.workspace.client.get",
            side_effect=click.ClickException("Fabric API error 401: Unauthorized"),
        ):
            result = runner.invoke(main, ["workspace", "ls"])

        assert result.exit_code != 0
        assert "401" in result.output
