"""Tests for ``fabio auth`` CLI commands.

These tests exercise the Click commands via CliRunner and mock out all
azure-identity interactions so no real browser/device-code flows are needed.
"""

from __future__ import annotations

import base64
import json
import time
from typing import TYPE_CHECKING, Any
from unittest.mock import MagicMock, patch

import pytest
from click.testing import CliRunner

from fabio.auth_store import AuthRecord, save_record
from fabio.cli import main

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture()
def runner() -> CliRunner:
    return CliRunner()


@pytest.fixture()
def auth_file(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """Redirect AUTH_FILE to a temp location."""
    fake = tmp_path / "auth.json"
    monkeypatch.setattr("fabio.auth_store.AUTH_FILE", fake)
    # Also patch the reference in auth command module (it imports at call time
    # through the helper functions so the monkeypatch above is sufficient).
    return fake


def _make_fake_token(claims: dict[str, Any]) -> str:
    """Build a fake JWT-shaped string with the given payload claims."""
    header = base64.urlsafe_b64encode(b'{"alg":"none"}').decode().rstrip("=")
    payload = base64.urlsafe_b64encode(json.dumps(claims).encode()).decode().rstrip("=")
    return f"{header}.{payload}.sig"


# -- fabio auth login -------------------------------------------------------


class TestLogin:
    @patch("fabio.commands.auth.InteractiveBrowserCredential", autospec=False)
    def test_login_browser_success(
        self, mock_cred_cls: MagicMock, runner: CliRunner, auth_file: Path
    ) -> None:
        token_str = _make_fake_token({"upn": "user@contoso.com", "tid": "tenant-1"})
        mock_cred = MagicMock()
        mock_cred.get_token.return_value = MagicMock(token=token_str)
        mock_cred_cls.return_value = mock_cred

        result = runner.invoke(main, ["auth", "login"])

        assert result.exit_code == 0
        assert "user@contoso.com" in result.output
        assert auth_file.exists()

    @patch("fabio.commands.auth.DeviceCodeCredential", autospec=False)
    def test_login_device_code_success(
        self, mock_cred_cls: MagicMock, runner: CliRunner, auth_file: Path
    ) -> None:
        token_str = _make_fake_token({"preferred_username": "dev@contoso.com", "tid": "t2"})
        mock_cred = MagicMock()
        mock_cred.get_token.return_value = MagicMock(token=token_str)
        mock_cred_cls.return_value = mock_cred

        result = runner.invoke(main, ["auth", "login", "--device-code"])

        assert result.exit_code == 0
        assert "dev@contoso.com" in result.output

    @patch("fabio.commands.auth.InteractiveBrowserCredential", autospec=False)
    def test_login_with_tenant(
        self, mock_cred_cls: MagicMock, runner: CliRunner, auth_file: Path
    ) -> None:
        token_str = _make_fake_token({"sub": "s", "tid": "custom-tenant"})
        mock_cred = MagicMock()
        mock_cred.get_token.return_value = MagicMock(token=token_str)
        mock_cred_cls.return_value = mock_cred

        result = runner.invoke(main, ["auth", "login", "--tenant", "custom-tenant"])

        assert result.exit_code == 0
        mock_cred_cls.assert_called_once_with(tenant_id="custom-tenant")

    @patch("fabio.commands.auth.InteractiveBrowserCredential", autospec=False)
    def test_login_failure(
        self, mock_cred_cls: MagicMock, runner: CliRunner, auth_file: Path
    ) -> None:
        mock_cred_cls.side_effect = RuntimeError("no browser")

        result = runner.invoke(main, ["auth", "login"])

        assert result.exit_code != 0
        assert "Login failed" in result.output


# -- fabio auth logout -------------------------------------------------------


class TestLogout:
    def test_logout_with_session(self, runner: CliRunner, auth_file: Path) -> None:
        save_record(AuthRecord(username="u", tenant_id="t", authority="a"))

        result = runner.invoke(main, ["auth", "logout"])

        assert result.exit_code == 0
        assert "Logged out" in result.output
        assert not auth_file.exists()

    def test_logout_without_session(self, runner: CliRunner, auth_file: Path) -> None:
        result = runner.invoke(main, ["auth", "logout"])

        assert result.exit_code == 0
        assert "No active session" in result.output


# -- fabio auth status -------------------------------------------------------


class TestStatus:
    def test_status_logged_in(self, runner: CliRunner, auth_file: Path) -> None:
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
        assert "alice@contoso.com" in result.output
        assert "tid-123" in result.output

    def test_status_not_logged_in(self, runner: CliRunner, auth_file: Path) -> None:
        result = runner.invoke(main, ["auth", "status"])

        assert result.exit_code != 0
        assert "Not logged in" in result.output


# -- fabio --version ---------------------------------------------------------


class TestVersion:
    def test_version_flag(self, runner: CliRunner) -> None:
        result = runner.invoke(main, ["--version"])

        assert result.exit_code == 0
        assert "0.1.0" in result.output
