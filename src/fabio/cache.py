"""Token cache configuration with encrypted/unencrypted storage detection.

Uses the shared MSAL token cache (same as az CLI, azd, and other Microsoft
developer tools).  This means if the user is already logged in via ``az login``,
fabio can silently acquire tokens without re-prompting.

On Linux, MSAL uses libsecret to talk to a secret service provider (keyring
daemon such as gnome-keyring) for encrypted persistent token caching.  If no
secret service is reachable (SSH sessions, containers, WSL without a keyring
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
        _libsecret_available = not result.stderr and result.returncode in (0, 1)
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError):
        _libsecret_available = False

    return _libsecret_available


def get_cache_options(warn: bool = True) -> TokenCachePersistenceOptions:
    """Build TokenCachePersistenceOptions using the shared MSAL developer tools cache.

    By not specifying a custom cache name, we use the default shared cache
    that is also used by az CLI, azd, and other Microsoft developer tools.
    This means users who are already authenticated via ``az login`` can use
    fabio without re-authenticating.

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
        return TokenCachePersistenceOptions()

    if warn:
        console.print(
            "[yellow]Warning:[/yellow] No keyring daemon running. "
            "Credentials will be stored unencrypted.",
        )
    return TokenCachePersistenceOptions(allow_unencrypted_storage=True)
