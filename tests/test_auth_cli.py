"""Tests for ``fabio auth`` CLI commands with structured output.

These tests exercise the Click commands via CliRunner and mock out all
azure-identity interactions so no real browser/device-code flows are needed.
"""

from __future__ import annotations

import json
import time
from typing import TYPE_CHECKING
from unittest.mock import MagicMock, patch

import pytest
from click.testing import CliRunner

from fabio.auth_store import AuthRecord, save_record
from fabio.cli import main

if TYPE_CHECKING:
    from pathlib import Path


def _extract_json(output: str) -> dict:
    """Extract the JSON line from CLI output (may contain Rich warnings)."""
    for line in output.strip().split("\n"):
        line = line.strip()
        if line.startswith("{"):
            return json.loads(line)
    raise ValueError(f"No JSON found in output: {output!r}")


@pytest.fixture()
def runner() -> CliRunner:
    return CliRunner()


@pytest.fixture()
def auth_files(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """Redirect auth files to a temp location."""
    fake_auth = tmp_path / "auth.json"
    fake_azure = tmp_path / "azure_auth_record.json"
    monkeypatch.setattr("fabio.auth_store.AUTH_FILE", fake_auth)
    monkeypatch.setattr("fabio.auth_store.AZURE_RECORD_FILE", fake_azure)
    return fake_auth


def _mock_azure_record(
    username: str = "user@contoso.com", tenant_id: str = "tenant-1"
) -> MagicMock:
    """Create a mock azure-identity AuthenticationRecord."""
    rec = MagicMock()
    rec.username = username
    rec.tenant_id = tenant_id
    rec.authority = f"https://login.microsoftonline.com/{tenant_id}"
    rec.serialize.return_value = '{"mock": "serialized"}'
    return rec


# -- fabio auth login -------------------------------------------------------


class TestLogin:
    @patch("fabio.commands.auth.InteractiveBrowserCredential", autospec=False)
    def test_login_browser_success(
        self, mock_cred_cls: MagicMock, runner: CliRunner, auth_files: Path
    ) -> None:
        mock_cred = MagicMock()
        mock_cred.authenticate.return_value = _mock_azure_record()
        mock_cred_cls.return_value = mock_cred

        result = runner.invoke(main, ["auth", "login"])

        assert result.exit_code == 0
        parsed = _extract_json(result.output)
        assert parsed["data"]["status"] == "authenticated"
        assert parsed["data"]["username"] == "user@contoso.com"
        assert auth_files.exists()

    @patch("fabio.commands.auth.DeviceCodeCredential", autospec=False)
    def test_login_device_code_success(
        self, mock_cred_cls: MagicMock, runner: CliRunner, auth_files: Path
    ) -> None:
        mock_cred = MagicMock()
        mock_cred.authenticate.return_value = _mock_azure_record(
            username="dev@contoso.com", tenant_id="t2"
        )
        mock_cred_cls.return_value = mock_cred

        result = runner.invoke(main, ["auth", "login", "--use-device-code"])

        assert result.exit_code == 0
        parsed = _extract_json(result.output)
        assert parsed["data"]["username"] == "dev@contoso.com"

    @patch("fabio.commands.auth.InteractiveBrowserCredential", autospec=False)
    def test_login_with_tenant(
        self, mock_cred_cls: MagicMock, runner: CliRunner, auth_files: Path
    ) -> None:
        mock_cred = MagicMock()
        mock_cred.authenticate.return_value = _mock_azure_record(tenant_id="custom-tenant")
        mock_cred_cls.return_value = mock_cred

        result = runner.invoke(main, ["auth", "login", "--tenant", "custom-tenant"])

        assert result.exit_code == 0
        call_kwargs = mock_cred_cls.call_args[1]
        assert call_kwargs["tenant_id"] == "custom-tenant"

    @patch("fabio.commands.auth.InteractiveBrowserCredential", autospec=False)
    def test_login_failure(
        self, mock_cred_cls: MagicMock, runner: CliRunner, auth_files: Path
    ) -> None:
        mock_cred_cls.side_effect = RuntimeError("no browser")

        result = runner.invoke(main, ["auth", "login"])

        assert result.exit_code != 0
        parsed = _extract_json(result.output)
        assert parsed["error"]["code"] == "AUTH_FAILED"


# -- fabio auth logout -------------------------------------------------------


class TestLogout:
    def test_logout_with_session(self, runner: CliRunner, auth_files: Path) -> None:
        save_record(AuthRecord(username="u", tenant_id="t", authority="a"))

        result = runner.invoke(main, ["auth", "logout"])

        assert result.exit_code == 0
        parsed = _extract_json(result.output)
        assert parsed["data"]["status"] == "logged_out"
        assert not auth_files.exists()

    def test_logout_without_session(self, runner: CliRunner, auth_files: Path) -> None:
        result = runner.invoke(main, ["auth", "logout"])

        assert result.exit_code == 0
        parsed = _extract_json(result.output)
        assert parsed["data"]["status"] == "no_session"


# -- fabio auth status -------------------------------------------------------


class TestStatus:
    def test_status_logged_in(self, runner: CliRunner, auth_files: Path) -> None:
        save_record(
            AuthRecord(
                username="alice@contoso.com",
                tenant_id="tid-123",
                authority="https://login.microsoftonline.com/tid-123",
                login_time=time.time() - 60,
            )
        )

        result = runner.invoke(main, ["auth", "status"])

        assert result.exit_code == 0
        parsed = _extract_json(result.output)
        assert parsed["data"]["username"] == "alice@contoso.com"
        assert parsed["data"]["tenant_id"] == "tid-123"
        assert parsed["data"]["status"] == "authenticated"

    def test_status_not_logged_in(self, runner: CliRunner, auth_files: Path) -> None:
        result = runner.invoke(main, ["auth", "status"])

        assert result.exit_code != 0
        parsed = _extract_json(result.output)
        assert parsed["error"]["code"] == "AUTH_REQUIRED"


# -- fabio --version ---------------------------------------------------------


class TestVersion:
    def test_version_flag(self, runner: CliRunner) -> None:
        result = runner.invoke(main, ["--version"])

        assert result.exit_code == 0
        assert "0.1.0" in result.output
