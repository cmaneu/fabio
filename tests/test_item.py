"""Tests for ``fabio item`` commands."""

from __future__ import annotations

import json
from unittest.mock import patch

from click.testing import CliRunner

from fabio.cli import main


def _fake_items() -> dict[str, object]:
    return {
        "value": [
            {"id": "item-001", "displayName": "SalesReport", "type": "Report"},
            {"id": "item-002", "displayName": "Revenue Model", "type": "SemanticModel"},
            {"id": "item-003", "displayName": "MainLakehouse", "type": "Lakehouse"},
        ]
    }


class TestItemList:
    def test_json_output(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.item.client.get", return_value=_fake_items()):
            result = runner.invoke(main, ["item", "list", "--workspace", "ws-001"])

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 3
        assert parsed["data"][0]["displayName"] == "SalesReport"

    def test_filter_by_type(self) -> None:
        runner = CliRunner()
        filtered = {"value": [{"id": "item-001", "displayName": "SalesReport", "type": "Report"}]}
        with patch("fabio.commands.item.client.get", return_value=filtered) as mock_get:
            result = runner.invoke(
                main, ["item", "list", "--workspace", "ws-001", "--type", "Report"]
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["count"] == 1
        # Verify type param was passed
        mock_get.assert_called_once_with("/workspaces/ws-001/items", params={"type": "Report"})

    def test_plain_output(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.item.client.get", return_value=_fake_items()):
            result = runner.invoke(main, ["-o", "plain", "item", "list", "--workspace", "ws-001"])

        assert result.exit_code == 0
        lines = result.output.strip().split("\n")
        assert "item-001" in lines
        assert "item-002" in lines


class TestItemShow:
    def test_show_by_id(self) -> None:
        runner = CliRunner()
        detail = {"id": "item-001", "displayName": "SalesReport", "type": "Report"}
        with patch("fabio.commands.item.client.get", return_value=detail):
            result = runner.invoke(
                main, ["item", "show", "--workspace", "ws-001", "--id", "item-001"]
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["id"] == "item-001"
        assert parsed["data"]["type"] == "Report"

    def test_show_by_name(self) -> None:
        runner = CliRunner()
        detail = {"id": "item-001", "displayName": "SalesReport", "type": "Report"}
        with patch(
            "fabio.commands.item.client.get",
            side_effect=[_fake_items(), detail],
        ):
            result = runner.invoke(
                main, ["item", "show", "--workspace", "ws-001", "--name", "SalesReport"]
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["id"] == "item-001"

    def test_show_not_found(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.item.client.get", return_value=_fake_items()):
            result = runner.invoke(
                main, ["item", "show", "--workspace", "ws-001", "--name", "NoSuchItem"]
            )

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "NOT_FOUND"

    def test_show_requires_id_or_name(self) -> None:
        runner = CliRunner()
        result = runner.invoke(main, ["item", "show", "--workspace", "ws-001"])

        assert result.exit_code != 0
        parsed = json.loads(result.output)
        assert parsed["error"]["code"] == "MISSING_PARAM"


class TestItemCreate:
    def test_create_item(self) -> None:
        runner = CliRunner()
        created = {"id": "item-new", "displayName": "NewLH", "type": "Lakehouse"}
        with patch("fabio.commands.item.client.post", return_value=created) as mock_post:
            result = runner.invoke(
                main,
                ["item", "create", "-w", "ws-001", "--name", "NewLH", "--type", "Lakehouse"],
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["id"] == "item-new"
        mock_post.assert_called_once_with(
            "/workspaces/ws-001/items",
            body={"displayName": "NewLH", "type": "Lakehouse"},
        )

    def test_create_with_description(self) -> None:
        runner = CliRunner()
        created = {"id": "item-new", "displayName": "NewLH", "type": "Lakehouse"}
        with patch("fabio.commands.item.client.post", return_value=created) as mock_post:
            result = runner.invoke(
                main,
                [
                    "item",
                    "create",
                    "--workspace",
                    "ws-001",
                    "--name",
                    "NewLH",
                    "--type",
                    "Lakehouse",
                    "--description",
                    "Main data lake",
                ],
            )

        assert result.exit_code == 0
        call_body = mock_post.call_args[1]["body"]
        assert call_body["description"] == "Main data lake"


class TestItemDelete:
    def test_delete_item(self) -> None:
        runner = CliRunner()
        with patch("fabio.commands.item.client.delete", return_value={}) as mock_del:
            result = runner.invoke(
                main, ["item", "delete", "--workspace", "ws-001", "--id", "item-001"]
            )

        assert result.exit_code == 0
        parsed = json.loads(result.output)
        assert parsed["data"]["status"] == "deleted"
        assert parsed["data"]["id"] == "item-001"
        mock_del.assert_called_once_with("/workspaces/ws-001/items/item-001")
