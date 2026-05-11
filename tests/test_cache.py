"""Tests for fabio.cache - encrypted/unencrypted storage detection."""

from __future__ import annotations

import subprocess
from unittest.mock import MagicMock, patch

from fabio.cache import _is_libsecret_available, get_cache_options


class TestLibsecretDetection:
    """Each test resets the cached probe result to ensure isolation."""

    def setup_method(self) -> None:
        import fabio.cache

        fabio.cache._libsecret_available = None

    @patch("fabio.cache.platform.system", return_value="Darwin")
    def test_macos_always_available(self, _mock: object) -> None:
        assert _is_libsecret_available() is True

    @patch("fabio.cache.platform.system", return_value="Windows")
    def test_windows_always_available(self, _mock: object) -> None:
        assert _is_libsecret_available() is True

    @patch("fabio.cache.platform.system", return_value="Linux")
    @patch("fabio.cache.subprocess.run")
    def test_linux_with_working_secret_service(
        self, mock_run: MagicMock, _mock_sys: object
    ) -> None:
        # secret-tool exits 1 when item not found -- service is reachable
        mock_run.return_value = MagicMock(returncode=1)
        assert _is_libsecret_available() is True

    @patch("fabio.cache.platform.system", return_value="Linux")
    @patch("fabio.cache.subprocess.run", side_effect=FileNotFoundError)
    def test_linux_without_secret_tool(self, _mock_run: object, _mock_sys: object) -> None:
        assert _is_libsecret_available() is False

    @patch("fabio.cache.platform.system", return_value="Linux")
    @patch("fabio.cache.subprocess.run", side_effect=subprocess.TimeoutExpired("cmd", 5))
    def test_linux_secret_tool_timeout(self, _mock_run: object, _mock_sys: object) -> None:
        assert _is_libsecret_available() is False

    @patch("fabio.cache.platform.system", return_value="Linux")
    @patch("fabio.cache.subprocess.run")
    def test_linux_secret_tool_dbus_failure(self, mock_run: MagicMock, _mock_sys: object) -> None:
        # secret-tool exits with code other than 0 or 1 when D-Bus is broken
        mock_run.return_value = MagicMock(returncode=2)
        assert _is_libsecret_available() is False

    @patch("fabio.cache.platform.system", return_value="Linux")
    @patch("fabio.cache.subprocess.run", side_effect=OSError("no dbus"))
    def test_linux_oserror(self, _mock_run: object, _mock_sys: object) -> None:
        assert _is_libsecret_available() is False


class TestGetCacheOptions:
    def setup_method(self) -> None:
        import fabio.cache

        fabio.cache._libsecret_available = None

    @patch("fabio.cache._is_libsecret_available", return_value=True)
    def test_encrypted_when_available(self, _mock: object) -> None:
        opts = get_cache_options()
        assert opts.allow_unencrypted_storage is not True

    @patch("fabio.cache._is_libsecret_available", return_value=False)
    def test_unencrypted_fallback(self, _mock: object) -> None:
        opts = get_cache_options(warn=False)
        assert opts.allow_unencrypted_storage is True

    @patch("fabio.cache._is_libsecret_available", return_value=False)
    @patch("fabio.cache.console")
    def test_warns_on_unencrypted_fallback(self, mock_console: MagicMock, _mock: object) -> None:
        get_cache_options(warn=True)
        mock_console.print.assert_called_once()
        call_args = mock_console.print.call_args[0][0]
        assert "unencrypted" in call_args

    @patch("fabio.cache._is_libsecret_available", return_value=False)
    @patch("fabio.cache.console")
    def test_no_warning_when_suppressed(self, mock_console: MagicMock, _mock: object) -> None:
        get_cache_options(warn=False)
        mock_console.print.assert_not_called()
