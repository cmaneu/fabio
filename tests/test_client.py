"""Tests for fabio.client - Fabric API client with auth validation."""

from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import MagicMock, patch

import pytest
from click.testing import CliRunner

from fabio.auth_store import AuthRecord, save_azure_record, save_record
from fabio.cli import main

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture()
def auth_files(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """Redirect auth files to a temp location."""
    fake_auth = tmp_path / "auth.json"
    fake_azure = tmp_path / "azure_auth_record.json"
    monkeypatch.setattr("fabio.auth_store.AUTH_FILE", fake_auth)
    monkeypatch.setattr("fabio.auth_store.AZURE_RECORD_FILE", fake_azure)
    return fake_auth


class TestRequireAuth:
    def test_exits_when_not_logged_in(self, auth_files: Path) -> None:
        """Commands that need auth should fail gracefully when unauthenticated."""
        runner = CliRunner()
        # workspace list calls client.get -> require_auth internally.
        result = runner.invoke(main, ["workspace", "list"])

        assert result.exit_code != 0
        assert "Not authenticated" in result.output

    def test_exits_when_azure_record_missing(self, auth_files: Path) -> None:
        """Fail if metadata exists but azure record is missing."""
        save_record(AuthRecord(username="u", tenant_id="t", authority="a"))

        runner = CliRunner()
        result = runner.invoke(main, ["workspace", "list"])

        assert result.exit_code != 0
        assert "Not authenticated" in result.output

    @patch("fabio.client.InteractiveBrowserCredential", autospec=False)
    @patch("fabio.client.AzureAuthRecord", autospec=False)
    def test_exits_on_expired_session(
        self,
        mock_azure_record_cls: MagicMock,
        mock_cred_cls: MagicMock,
        auth_files: Path,
    ) -> None:
        """Fail gracefully when the cached token is expired."""
        from azure.core.exceptions import ClientAuthenticationError

        save_record(AuthRecord(username="u", tenant_id="t", authority="a"))
        save_azure_record('{"serialized": "data"}')

        mock_azure_record_cls.deserialize.return_value = MagicMock()
        mock_cred = MagicMock()
        mock_cred.get_token.side_effect = ClientAuthenticationError("expired")
        mock_cred_cls.return_value = mock_cred

        runner = CliRunner()
        result = runner.invoke(main, ["workspace", "list"])

        assert result.exit_code != 0
        assert "Session expired" in result.output

    @patch("fabio.client.requests.get")
    @patch("fabio.client.InteractiveBrowserCredential", autospec=False)
    @patch("fabio.client.AzureAuthRecord", autospec=False)
    def test_successful_authenticated_request(
        self,
        mock_azure_record_cls: MagicMock,
        mock_cred_cls: MagicMock,
        mock_requests_get: MagicMock,
        auth_files: Path,
    ) -> None:
        """Happy path: auth succeeds and API returns data."""
        save_record(AuthRecord(username="u", tenant_id="t", authority="a"))
        save_azure_record('{"serialized": "data"}')

        mock_azure_record_cls.deserialize.return_value = MagicMock()
        mock_cred = MagicMock()
        mock_cred.get_token.return_value = MagicMock(token="valid-token")
        mock_cred_cls.return_value = mock_cred

        mock_resp = MagicMock()
        mock_resp.ok = True
        mock_resp.json.return_value = {
            "value": [
                {
                    "displayName": "TestWS",
                    "id": "ws-1",
                    "type": "Workspace",
                    "capacityId": "c1",
                }
            ]
        }
        mock_requests_get.return_value = mock_resp

        runner = CliRunner()
        result = runner.invoke(main, ["workspace", "list"])

        assert result.exit_code == 0
        assert "TestWS" in result.output
        # Verify the token was passed in the Authorization header.
        call_kwargs = mock_requests_get.call_args[1]
        assert call_kwargs["headers"]["Authorization"] == "Bearer valid-token"
