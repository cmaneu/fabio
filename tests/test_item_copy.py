"""Tests for ``fabio item copy`` and ``fabio item move`` commands."""

from __future__ import annotations

import json
from unittest.mock import patch

from click.testing import CliRunner

from fabio.cli import main


class TestItemCopy:
    def test_copy_item(self) -> None:
        runner = CliRunner()
        source_item = {
            "id": "item-001",
            "displayName": "SalesReport",
            "type": "Report",
        }
        definition = {
            "definition": {
                "format": "ipynb",
                "parts": [{"path": "content.py", "payload": "abc123"}],
            }
        }
        created = {
            "id": "item-new",
            "displayName": "SalesReport",
            "type": "Report",
        }
        with (
            patch(
                "fabio.commands.item.client.get",
                return_value=source_item,
            ),
            patch(
                "fabio.commands.item.client.get_item_definition",
                return_value=definition,
            ),
            patch(
                "fabio.commands.item.client.post",
                return_value=created,
            ) as mock_post,
        ):
            result = runner.invoke(
                main,
                [
                    "item",
                    "copy",
                    "--source-workspace",
                    "ws-001",
                    "--id",
                    "item-001",
                    "--dest-workspace",
                    "ws-002",
                ],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["id"] == "item-new"
        assert parsed["data"]["copySource"]["workspace"] == "ws-001"
        # Verify it was created in the destination
        mock_post.assert_called_once()
        call_args = mock_post.call_args
        assert "/workspaces/ws-002/items" in call_args[0][0]

    def test_copy_with_rename(self) -> None:
        runner = CliRunner()
        source_item = {
            "id": "nb-001",
            "displayName": "Original",
            "type": "Notebook",
        }
        definition = {"definition": {"format": "ipynb", "parts": []}}
        created = {
            "id": "nb-new",
            "displayName": "Renamed Copy",
            "type": "Notebook",
        }
        with (
            patch(
                "fabio.commands.item.client.get",
                return_value=source_item,
            ),
            patch(
                "fabio.commands.item.client.get_item_definition",
                return_value=definition,
            ),
            patch(
                "fabio.commands.item.client.post",
                return_value=created,
            ) as mock_post,
        ):
            result = runner.invoke(
                main,
                [
                    "item",
                    "copy",
                    "-sw",
                    "ws-001",
                    "--id",
                    "nb-001",
                    "-dw",
                    "ws-002",
                    "--name",
                    "Renamed Copy",
                ],
            )

        assert result.exit_code == 0
        call_body = mock_post.call_args[1]["body"]
        assert call_body["displayName"] == "Renamed Copy"


class TestItemMove:
    def test_move_item(self) -> None:
        runner = CliRunner()
        source_item = {
            "id": "item-001",
            "displayName": "SalesReport",
            "type": "Report",
        }
        definition = {
            "definition": {
                "format": "ipynb",
                "parts": [{"path": "content.py", "payload": "abc123"}],
            }
        }
        created = {
            "id": "item-new",
            "displayName": "SalesReport",
            "type": "Report",
        }
        with (
            patch(
                "fabio.commands.item.client.get",
                return_value=source_item,
            ),
            patch(
                "fabio.commands.item.client.get_item_definition",
                return_value=definition,
            ),
            patch(
                "fabio.commands.item.client.post",
                return_value=created,
            ),
            patch(
                "fabio.commands.item.client.delete",
            ) as mock_delete,
        ):
            result = runner.invoke(
                main,
                [
                    "item",
                    "move",
                    "--source-workspace",
                    "ws-001",
                    "--id",
                    "item-001",
                    "--dest-workspace",
                    "ws-002",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["id"] == "item-new"
        assert parsed["data"]["status"] == "moved"
        assert parsed["data"]["copySource"]["workspace"] == "ws-001"
        mock_delete.assert_called_once_with("/workspaces/ws-001/items/item-001")

    def test_move_with_rename(self) -> None:
        runner = CliRunner()
        source_item = {
            "id": "nb-001",
            "displayName": "Original",
            "type": "Notebook",
        }
        definition = {"definition": {"format": "ipynb", "parts": []}}
        created = {
            "id": "nb-moved",
            "displayName": "New Name",
            "type": "Notebook",
        }
        with (
            patch(
                "fabio.commands.item.client.get",
                return_value=source_item,
            ),
            patch(
                "fabio.commands.item.client.get_item_definition",
                return_value=definition,
            ),
            patch(
                "fabio.commands.item.client.post",
                return_value=created,
            ) as mock_post,
            patch(
                "fabio.commands.item.client.delete",
            ) as mock_delete,
        ):
            result = runner.invoke(
                main,
                [
                    "item",
                    "move",
                    "-sw",
                    "ws-001",
                    "--id",
                    "nb-001",
                    "-dw",
                    "ws-002",
                    "--name",
                    "New Name",
                ],
            )

        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "moved"
        call_body = mock_post.call_args[1]["body"]
        assert call_body["displayName"] == "New Name"
        mock_delete.assert_called_once_with("/workspaces/ws-001/items/nb-001")
