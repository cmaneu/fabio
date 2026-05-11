"""Tests for fabio.cache - encrypted/unencrypted storage detection."""

from __future__ import annotations

from unittest.mock import patch

from fabio.cache import _is_libsecret_available, get_cache_options


class TestLibsecretDetection:
    @patch("fabio.cache.platform.system", return_value="Darwin")
    def test_macos_always_available(self, _mock: object) -> None:
        assert _is_libsecret_available() is True

    @patch("fabio.cache.platform.system", return_value="Windows")
    def test_windows_always_available(self, _mock: object) -> None:
        assert _is_libsecret_available() is True

    @patch("fabio.cache.platform.system", return_value="Linux")
    @patch("fabio.cache.shutil.which", return_value="/usr/bin/secret-tool")
    @patch.dict("os.environ", {"DBUS_SESSION_BUS_ADDRESS": "unix:path=/run/user/1000/bus"})
    def test_linux_with_libsecret(self, _mock_which: object, _mock_sys: object) -> None:
        assert _is_libsecret_available() is True

    @patch("fabio.cache.platform.system", return_value="Linux")
    @patch("fabio.cache.shutil.which", return_value=None)
    def test_linux_without_secret_tool(self, _mock_which: object, _mock_sys: object) -> None:
        assert _is_libsecret_available() is False

    @patch("fabio.cache.platform.system", return_value="Linux")
    @patch("fabio.cache.shutil.which", return_value="/usr/bin/secret-tool")
    @patch.dict("os.environ", {}, clear=True)
    def test_linux_without_dbus(self, _mock_which: object, _mock_sys: object) -> None:
        assert _is_libsecret_available() is False


class TestGetCacheOptions:
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
    def test_warns_on_unencrypted_fallback(self, mock_console: object, _mock: object) -> None:
        get_cache_options(warn=True)
        from unittest.mock import MagicMock

        assert isinstance(mock_console, MagicMock)
        mock_console.print.assert_called_once()
        call_args = mock_console.print.call_args[0][0]
        assert "unencrypted" in call_args

    @patch("fabio.cache._is_libsecret_available", return_value=False)
    @patch("fabio.cache.console")
    def test_no_warning_when_suppressed(self, mock_console: object, _mock: object) -> None:
        get_cache_options(warn=False)
        from unittest.mock import MagicMock

        assert isinstance(mock_console, MagicMock)
        mock_console.print.assert_not_called()
