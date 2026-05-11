"""Token cache configuration with encrypted/unencrypted storage detection.

On Linux, MSAL uses libsecret for encrypted persistent token caching.  If
libsecret is unavailable (SSH sessions, containers, WSL without a keyring
daemon), we fall back to unencrypted storage and warn the user.

On macOS and Windows, the OS keychain is always available so encryption is
never an issue.
"""

from __future__ import annotations

import platform
import subprocess

from azure.identity import TokenCachePersistenceOptions
from rich.console import Console

console = Console(stderr=True)

_CACHE_NAME = "fabio"

# Cache the probe result so we only check once per process.
_libsecret_available: bool | None = None


def _is_libsecret_available() -> bool:
    """Check whether libsecret is actually usable on the current system.

    On non-Linux platforms this always returns True (macOS Keychain / Windows
    DPAPI are always functional).  On Linux we attempt a real ``secret-tool``
    lookup to verify the secret service is reachable.
    """
    global _libsecret_available

    if _libsecret_available is not None:
        return _libsecret_available

    if platform.system() != "Linux":
        _libsecret_available = True
        return True

    # Attempt to probe the secret service via secret-tool.  A successful
    # lookup (even with no results) means the keyring daemon is reachable.
    # If secret-tool writes to stderr it means the service is broken.
    try:
        result = subprocess.run(
            ["secret-tool", "lookup", "fabio-probe", "test"],
            capture_output=True,
            timeout=5,
        )
        # If stderr has content, the secret service is not functional
        # (e.g. "The name org.freedesktop.secrets was not provided...").
        # Exit 0 = found item, exit 1 = not found (both mean service works).
        _libsecret_available = (
            not result.stderr and result.returncode in (0, 1)
        )
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError):
        _libsecret_available = False

    return _libsecret_available


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
