"""Tests for fabio.client - Fabric API client with structured errors."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING
from unittest.mock import MagicMock, patch

import pytest
from click.testing import CliRunner

from fabio.auth_store import AuthRecord, save_azure_record, save_record
from fabio.cli import main
from fabio.errors import ErrorCode, FabioError

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
    def test_raises_when_not_logged_in(self, auth_files: Path) -> None:
        """require_auth raises FabioError when unauthenticated."""
        from fabio.client import require_auth

        with pytest.raises(FabioError) as exc_info:
            require_auth()

        assert exc_info.value.code == ErrorCode.AUTH_REQUIRED

    def test_raises_when_azure_record_missing(self, auth_files: Path) -> None:
        """Fail if metadata exists but azure record is missing."""
        save_record(AuthRecord(username="u", tenant_id="t", authority="a"))

        from fabio.client import require_auth

        with pytest.raises(FabioError) as exc_info:
            require_auth()

        assert exc_info.value.code == ErrorCode.AUTH_REQUIRED

    @patch("fabio.client.InteractiveBrowserCredential", autospec=False)
    @patch("fabio.client.AzureAuthRecord", autospec=False)
    def test_raises_on_expired_session(
        self,
        mock_azure_record_cls: MagicMock,
        mock_cred_cls: MagicMock,
        auth_files: Path,
    ) -> None:
        """Raises FabioError when the cached token is expired."""
        from azure.core.exceptions import ClientAuthenticationError

        save_record(AuthRecord(username="u", tenant_id="t", authority="a"))
        save_azure_record('{"serialized": "data"}')

        mock_azure_record_cls.deserialize.return_value = MagicMock()
        mock_cred = MagicMock()
        mock_cred.get_token.side_effect = ClientAuthenticationError("expired")
        mock_cred_cls.return_value = mock_cred

        from fabio.client import require_auth

        with pytest.raises(FabioError) as exc_info:
            require_auth()

        assert exc_info.value.code == ErrorCode.AUTH_EXPIRED

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
        mock_resp.status_code = 200
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
        parsed = json.loads(result.output)
        assert parsed["data"][0]["displayName"] == "TestWS"
        # Verify the token was passed in the Authorization header.
        call_kwargs = mock_requests_get.call_args[1]
        assert call_kwargs["headers"]["Authorization"] == "Bearer valid-token"


class TestHandleResponse:
    """Test the _handle_response function directly."""

    def test_404_raises_not_found(self) -> None:
        from fabio.client import _handle_response

        mock_resp = MagicMock()
        mock_resp.ok = False
        mock_resp.status_code = 404
        mock_resp.text = "Not found"
        mock_resp.json.side_effect = Exception("no json")

        with pytest.raises(FabioError) as exc_info:
            _handle_response(mock_resp)
        assert exc_info.value.code == ErrorCode.NOT_FOUND

    def test_429_raises_rate_limited(self) -> None:
        from fabio.client import _handle_response

        mock_resp = MagicMock()
        mock_resp.ok = False
        mock_resp.status_code = 429
        mock_resp.text = "Too many requests"
        mock_resp.json.side_effect = Exception("no json")

        with pytest.raises(FabioError) as exc_info:
            _handle_response(mock_resp)
        assert exc_info.value.code == ErrorCode.RATE_LIMITED

    def test_500_raises_server_error(self) -> None:
        from fabio.client import _handle_response

        mock_resp = MagicMock()
        mock_resp.ok = False
        mock_resp.status_code = 500
        mock_resp.text = "Internal error"
        mock_resp.json.side_effect = Exception("no json")

        with pytest.raises(FabioError) as exc_info:
            _handle_response(mock_resp)
        assert exc_info.value.code == ErrorCode.SERVER_ERROR

    def test_204_returns_empty_dict(self) -> None:
        from fabio.client import _handle_response

        mock_resp = MagicMock()
        mock_resp.ok = True
        mock_resp.status_code = 204

        result = _handle_response(mock_resp)
        assert result == {}

    def test_capacity_not_active_raises_capacity_inactive(self) -> None:
        from fabio.client import _handle_response

        mock_resp = MagicMock()
        mock_resp.ok = False
        mock_resp.status_code = 404
        mock_resp.text = "CapacityNotActive"
        mock_resp.json.return_value = {
            "errorCode": "CapacityNotActive",
            "message": "Capacity 64fd7fa6 is not active",
        }

        with pytest.raises(FabioError) as exc_info:
            _handle_response(mock_resp)
        assert exc_info.value.code == ErrorCode.CAPACITY_INACTIVE
        assert "paused or inactive" in exc_info.value.message

    def test_capacity_not_active_nested_error(self) -> None:
        from fabio.client import _handle_response

        mock_resp = MagicMock()
        mock_resp.ok = False
        mock_resp.status_code = 404
        mock_resp.text = ""
        mock_resp.json.return_value = {
            "error": {
                "code": "ItemNotFound",
                "message": "Internal error CapacityNotActive.Capacity xyz is not active",
            }
        }

        with pytest.raises(FabioError) as exc_info:
            _handle_response(mock_resp)
        assert exc_info.value.code == ErrorCode.CAPACITY_INACTIVE


class TestPollLro:
    """Test the LRO polling logic."""

    @patch("fabio.client.time.sleep")
    @patch("fabio.client.requests.get")
    def test_poll_succeeds(self, mock_get: MagicMock, mock_sleep: MagicMock) -> None:
        from fabio.client import _poll_lro

        # Simulated 202 response with Location header
        resp_202 = MagicMock()
        resp_202.status_code = 202
        resp_202.headers = {
            "Location": "https://api.fabric.microsoft.com/v1/operations/op-1",
            "Retry-After": "1",
        }

        # First poll: running
        poll_running = MagicMock()
        poll_running.ok = True
        poll_running.text = '{"status":"Running"}'
        poll_running.json.return_value = {"status": "Running"}
        poll_running.headers = {"Retry-After": "1"}

        # Second poll: succeeded
        poll_done = MagicMock()
        poll_done.ok = True
        poll_done.text = '{"status":"Succeeded"}'
        poll_done.json.return_value = {"status": "Succeeded"}
        poll_done.headers = {}

        mock_get.side_effect = [poll_running, poll_done]

        result = _poll_lro(resp_202, "token-123")
        assert result["status"] == "Succeeded"
        assert mock_get.call_count == 2

    @patch("fabio.client.time.sleep")
    @patch("fabio.client.requests.get")
    def test_poll_fails(self, mock_get: MagicMock, mock_sleep: MagicMock) -> None:
        from fabio.client import _poll_lro

        resp_202 = MagicMock()
        resp_202.status_code = 202
        resp_202.headers = {
            "Location": "https://api.fabric.microsoft.com/v1/operations/op-2",
            "Retry-After": "1",
        }

        poll_failed = MagicMock()
        poll_failed.ok = True
        poll_failed.text = '{"status":"Failed","error":{"message":"bad input"}}'
        poll_failed.json.return_value = {
            "status": "Failed",
            "error": {"message": "bad input"},
        }
        poll_failed.headers = {}

        mock_get.return_value = poll_failed

        with pytest.raises(FabioError) as exc_info:
            _poll_lro(resp_202, "token-123")
        assert exc_info.value.code == ErrorCode.API_ERROR
        assert "bad input" in exc_info.value.message

    def test_poll_no_location_returns_empty(self) -> None:
        from fabio.client import _poll_lro

        resp_202 = MagicMock()
        resp_202.status_code = 202
        resp_202.headers = {}

        result = _poll_lro(resp_202, "token-123")
        assert result == {}


class TestCopyOneLakeFile:
    """Test the server-side copy logic."""

    @patch("fabio.client.require_auth", return_value="token-abc")
    @patch("fabio.client.requests.put")
    @patch("fabio.client.requests.head")
    @patch("fabio.client.time.sleep")
    def test_copy_sync_success(
        self,
        mock_sleep: MagicMock,
        mock_head: MagicMock,
        mock_put: MagicMock,
        mock_auth: MagicMock,
    ) -> None:
        """Copy completes synchronously (small file)."""
        from fabio.client import copy_onelake_file

        mock_resp = MagicMock()
        mock_resp.ok = True
        mock_resp.status_code = 202
        mock_resp.headers = {
            "x-ms-copy-status": "success",
            "x-ms-copy-id": "copy-001",
        }
        mock_put.return_value = mock_resp

        result = copy_onelake_file("ws-a", "lh-a", "Files/f.csv", "ws-b", "lh-b", "Files/f.csv")

        assert result["copyStatus"] == "success"
        assert result["copyId"] == "copy-001"
        mock_head.assert_not_called()

    @patch("fabio.client.require_auth", return_value="token-abc")
    @patch("fabio.client.requests.put")
    @patch("fabio.client.requests.head")
    @patch("fabio.client.time.sleep")
    def test_copy_async_polls_until_success(
        self,
        mock_sleep: MagicMock,
        mock_head: MagicMock,
        mock_put: MagicMock,
        mock_auth: MagicMock,
    ) -> None:
        """Copy is async, polls HEAD until success."""
        from fabio.client import copy_onelake_file

        mock_resp = MagicMock()
        mock_resp.ok = True
        mock_resp.status_code = 202
        mock_resp.headers = {
            "x-ms-copy-status": "pending",
            "x-ms-copy-id": "copy-002",
        }
        mock_put.return_value = mock_resp

        # First poll: still pending
        head_pending = MagicMock()
        head_pending.headers = {"x-ms-copy-status": "pending"}
        # Second poll: success
        head_done = MagicMock()
        head_done.headers = {"x-ms-copy-status": "success"}
        mock_head.side_effect = [head_pending, head_done]

        result = copy_onelake_file("ws-a", "lh-a", "Files/f.csv", "ws-b", "lh-b", "Files/f.csv")

        assert result["copyStatus"] == "success"
        assert mock_head.call_count == 2

    @patch("fabio.client.require_auth", return_value="token-abc")
    @patch("fabio.client.requests.put")
    def test_copy_source_not_found(
        self,
        mock_put: MagicMock,
        mock_auth: MagicMock,
    ) -> None:
        """Copy fails when source file doesn't exist."""
        from fabio.client import copy_onelake_file

        mock_resp = MagicMock()
        mock_resp.ok = False
        mock_resp.status_code = 404
        mock_resp.text = "Source not found"
        mock_resp.json.side_effect = Exception("no json")
        mock_put.return_value = mock_resp

        with pytest.raises(FabioError) as exc_info:
            copy_onelake_file("ws-a", "lh-a", "Files/x.csv", "ws-b", "lh-b", "Files/x.csv")
        assert exc_info.value.code == ErrorCode.NOT_FOUND
