"""Token cache configuration with encrypted/unencrypted storage detection.

On Linux, MSAL uses libsecret for encrypted persistent token caching.  If
libsecret is unavailable (SSH sessions, containers, WSL without a keyring
daemon), we fall back to unencrypted storage and warn the user.

On macOS and Windows, the OS keychain is always available so encryption is
never an issue.
"""

from __future__ import annotations

import platform
import shutil

from azure.identity import TokenCachePersistenceOptions
from rich.console import Console

console = Console(stderr=True)

_CACHE_NAME = "fabio"


def _is_libsecret_available() -> bool:
    """Check whether libsecret is usable on the current system."""
    if platform.system() != "Linux":
        # macOS uses Keychain, Windows uses DPAPI -- always available.
        return True

    # libsecret requires the `secret-tool` binary and a running D-Bus session.
    if shutil.which("secret-tool") is None:
        return False

    # Check if DBUS_SESSION_BUS_ADDRESS is set (needed for secret service).
    import os

    return "DBUS_SESSION_BUS_ADDRESS" in os.environ


def get_cache_options(warn: bool = True) -> TokenCachePersistenceOptions:
    """Build TokenCachePersistenceOptions with appropriate encryption setting.

    Parameters
    ----------
    warn:
        If *True* and falling back to unencrypted storage, print a warning
        to stderr.

    Returns
    -------
    TokenCachePersistenceOptions
        Configured for encrypted storage if possible, unencrypted otherwise.
    """
    if _is_libsecret_available():
        return TokenCachePersistenceOptions(name=_CACHE_NAME)

    if warn:
        console.print(
            "[yellow]Warning:[/yellow] libsecret is not available. "
            "Credentials will be stored unencrypted in the file system.\n"
            "Install libsecret and ensure a keyring daemon is running for "
            "encrypted storage.",
        )
    return TokenCachePersistenceOptions(name=_CACHE_NAME, allow_unencrypted_storage=True)
