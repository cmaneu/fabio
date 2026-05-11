"""Tests for ``fabio workspace`` commands."""

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
            result = runner.invoke(main, ["workspace", "list"])

        assert result.exit_code == 0
        assert "My Workspace" in result.output
        assert "Shared Analytics" in result.output
        assert "ws-001" in result.output
        assert "ws-002" in result.output

    def test_empty_workspaces(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.workspace.client.get", return_value={"value": []}):
            result = runner.invoke(main, ["workspace", "list"])

        assert result.exit_code == 0
        assert "No workspaces found" in result.output

    def test_api_error(self) -> None:
        runner = CliRunner()
        import click

        with patch(
            "fabio.commands.workspace.client.get",
            side_effect=click.ClickException("Fabric API error 401: Unauthorized"),
        ):
            result = runner.invoke(main, ["workspace", "list"])

        assert result.exit_code != 0
        assert "401" in result.output


# -- fabio workspace show ---------------------------------------------------


def _fake_items() -> dict[str, object]:
    return {
        "value": [
            {
                "id": "item-001",
                "displayName": "SalesReport",
                "type": "Report",
            },
            {
                "id": "item-002",
                "displayName": "Revenue Model",
                "type": "SemanticModel",
            },
            {
                "id": "item-003",
                "displayName": "Ingestion Pipeline",
                "type": "DataPipeline",
            },
        ]
    }


class TestWorkspaceShow:
    def test_shows_artifacts(self) -> None:
        runner = CliRunner()
        with patch(
            "fabio.commands.workspace.client.get",
            side_effect=[_fake_workspaces(), _fake_items()],
        ):
            result = runner.invoke(main, ["workspace", "show", "--name", "My Workspace"])

        assert result.exit_code == 0
        assert "SalesReport" in result.output
        assert "Report" in result.output
        assert "Revenue Model" in result.output
        assert "SemanticModel" in result.output
        assert "Ingestion Pipeline" in result.output
        assert "DataPipeline" in result.output

    def test_workspace_not_found(self) -> None:
        runner = CliRunner()
        with patch(
            "fabio.commands.workspace.client.get",
            return_value=_fake_workspaces(),
        ):
            result = runner.invoke(main, ["workspace", "show", "--name", "Nonexistent"])

        assert result.exit_code != 0
        assert "Workspace not found" in result.output

    def test_empty_artifacts(self) -> None:
        runner = CliRunner()
        with patch(
            "fabio.commands.workspace.client.get",
            side_effect=[_fake_workspaces(), {"value": []}],
        ):
            result = runner.invoke(main, ["workspace", "show", "--name", "My Workspace"])

        assert result.exit_code == 0
        assert "No artifacts found" in result.output

    def test_requires_name_option(self) -> None:
        runner = CliRunner()
        result = runner.invoke(main, ["workspace", "show"])

        assert result.exit_code != 0
        assert "Missing option" in result.output or "--name" in result.output

    def test_show_specific_item(self) -> None:
        runner = CliRunner()
        item_detail = {
            "id": "item-001",
            "displayName": "SalesReport",
            "type": "Report",
            "description": "Monthly sales figures",
        }
        with patch(
            "fabio.commands.workspace.client.get",
            side_effect=[_fake_workspaces(), _fake_items(), item_detail],
        ):
            result = runner.invoke(
                main,
                ["workspace", "show", "--name", "My Workspace", "--item", "SalesReport"],
            )

        assert result.exit_code == 0
        assert "SalesReport" in result.output
        assert "Report" in result.output
        assert "Monthly sales figures" in result.output
        assert "My Workspace" in result.output

    def test_show_item_not_found(self) -> None:
        runner = CliRunner()
        with patch(
            "fabio.commands.workspace.client.get",
            side_effect=[_fake_workspaces(), _fake_items()],
        ):
            result = runner.invoke(
                main,
                ["workspace", "show", "--name", "My Workspace", "--item", "NoSuchItem"],
            )

        assert result.exit_code != 0
        assert "Item not found" in result.output
