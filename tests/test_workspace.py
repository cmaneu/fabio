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

    def test_filter_by_type(self) -> None:
        runner = CliRunner()
        filtered_items: dict[str, object] = {
            "value": [
                {"id": "item-001", "displayName": "SalesReport", "type": "Report"},
            ]
        }
        with patch(
            "fabio.commands.workspace.client.get",
            side_effect=[_fake_workspaces(), filtered_items],
        ) as mock_get:
            result = runner.invoke(
                main,
                ["workspace", "show", "--name", "My Workspace", "--type", "Report"],
            )

        assert result.exit_code == 0
        assert "SalesReport" in result.output
        # Verify the type param was passed to the items API call.
        items_call = mock_get.call_args_list[1]
        assert items_call[1]["params"] == {"type": "Report"}

    def test_type_lakehouse_lists_tables_and_files(self) -> None:
        runner = CliRunner()
        lakehouses: dict[str, object] = {
            "value": [
                {"id": "lh-001", "displayName": "SalesLakehouse", "type": "Lakehouse"},
            ]
        }
        tables: dict[str, object] = {
            "data": [
                {
                    "name": "orders",
                    "type": "Managed",
                    "format": "delta",
                    "location": "abfss://path/orders",
                },
                {
                    "name": "customers",
                    "type": "Managed",
                    "format": "delta",
                    "location": "abfss://path/customers",
                },
            ]
        }
        files = [
            {
                "name": "Files/raw_sales.csv",
                "contentLength": "1024",
                "lastModified": "2025-01-15T10:00:00Z",
                "isDirectory": "false",
            },
        ]
        with (
            patch(
                "fabio.commands.workspace.client.get",
                side_effect=[_fake_workspaces(), lakehouses, tables],
            ),
            patch(
                "fabio.commands.workspace.client.list_onelake_files",
                return_value=files,
            ),
        ):
            result = runner.invoke(
                main,
                ["workspace", "show", "--name", "My Workspace", "--type", "Lakehouse"],
            )

        assert result.exit_code == 0
        assert "orders" in result.output
        assert "customers" in result.output
        assert "SalesLakehouse" in result.output
        assert "raw_sales.csv" in result.output

    def test_item_lakehouse_lists_tables_and_files(self) -> None:
        runner = CliRunner()
        items_with_lakehouse: dict[str, object] = {
            "value": [
                {"id": "lh-001", "displayName": "SalesLakehouse", "type": "Lakehouse"},
            ]
        }
        item_detail = {
            "id": "lh-001",
            "displayName": "SalesLakehouse",
            "type": "Lakehouse",
            "description": "Main lakehouse",
        }
        tables: dict[str, object] = {
            "data": [
                {
                    "name": "products",
                    "type": "Managed",
                    "format": "delta",
                    "location": "abfss://path/products",
                },
            ]
        }
        files = [
            {
                "name": "Files/inventory.parquet",
                "contentLength": "2048",
                "lastModified": "2025-02-10T14:30:00Z",
                "isDirectory": "false",
            },
        ]
        with (
            patch(
                "fabio.commands.workspace.client.get",
                side_effect=[_fake_workspaces(), items_with_lakehouse, item_detail, tables],
            ),
            patch(
                "fabio.commands.workspace.client.list_onelake_files",
                return_value=files,
            ),
        ):
            result = runner.invoke(
                main,
                ["workspace", "show", "--name", "My Workspace", "--item", "SalesLakehouse"],
            )

        assert result.exit_code == 0
        assert "SalesLakehouse" in result.output
        assert "Lakehouse" in result.output
        assert "products" in result.output
        assert "Main lakehouse" in result.output
        assert "inventory.parquet" in result.output

    def test_dir_lists_directory_contents(self) -> None:
        runner = CliRunner()
        items_with_lakehouse: dict[str, object] = {
            "value": [
                {"id": "lh-001", "displayName": "SalesLakehouse", "type": "Lakehouse"},
            ]
        }
        item_detail = {
            "id": "lh-001",
            "displayName": "SalesLakehouse",
            "type": "Lakehouse",
        }
        dir_entries = [
            {
                "name": "Files/raw/orders.csv",
                "contentLength": "4096",
                "lastModified": "2025-03-01T09:00:00Z",
                "isDirectory": "false",
            },
            {
                "name": "Files/raw/2024",
                "isDirectory": "true",
                "lastModified": "2025-03-01T08:00:00Z",
            },
            {
                "name": "Files/raw/2024/jan.parquet",
                "contentLength": "8192",
                "lastModified": "2025-01-31T12:00:00Z",
                "isDirectory": "false",
            },
        ]
        with (
            patch(
                "fabio.commands.workspace.client.get",
                side_effect=[_fake_workspaces(), items_with_lakehouse, item_detail],
            ),
            patch(
                "fabio.commands.workspace.client.list_onelake_files",
                return_value=dir_entries,
            ) as mock_list,
        ):
            result = runner.invoke(
                main,
                [
                    "workspace", "show",
                    "--name", "My Workspace",
                    "--item", "SalesLakehouse",
                    "--dir", "raw",
                ],
            )

        assert result.exit_code == 0
        assert "orders.csv" in result.output
        assert "2024" in result.output
        assert "2024/jan.parquet" in result.output
        # Verify a single recursive call was made.
        mock_list.assert_called_once_with(
            "ws-001", "lh-001", directory="Files/raw", recursive=True
        )

    def test_dir_empty_directory(self) -> None:
        runner = CliRunner()
        items_with_lakehouse: dict[str, object] = {
            "value": [
                {"id": "lh-001", "displayName": "SalesLakehouse", "type": "Lakehouse"},
            ]
        }
        item_detail = {
            "id": "lh-001",
            "displayName": "SalesLakehouse",
            "type": "Lakehouse",
        }
        with (
            patch(
                "fabio.commands.workspace.client.get",
                side_effect=[_fake_workspaces(), items_with_lakehouse, item_detail],
            ),
            patch(
                "fabio.commands.workspace.client.list_onelake_files",
                return_value=[],
            ),
        ):
            result = runner.invoke(
                main,
                [
                    "workspace", "show",
                    "--name", "My Workspace",
                    "--item", "SalesLakehouse",
                    "--dir", "nonexistent",
                ],
            )

        assert result.exit_code == 0
        assert "No files found" in result.output

    def test_dir_requires_item(self) -> None:
        runner = CliRunner()
        result = runner.invoke(
            main,
            ["workspace", "show", "--name", "My Workspace", "--dir", "raw"],
        )

        assert result.exit_code != 0
        assert "--dir requires --item" in result.output
