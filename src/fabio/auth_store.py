"""Persistent authentication state for Fabric CLI.

Stores the azure-identity ``AuthenticationRecord`` (serialized) alongside a
lightweight metadata record.  The ``AuthenticationRecord`` enables silent token
acquisition on subsequent CLI invocations without re-prompting the user.

Files are stored in ``~/.config/fabio/`` (XDG on Linux, ``%APPDATA%/fabio`` on
Windows).
"""

from __future__ import annotations

import json
import os
import platform
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path


def _config_dir() -> Path:
    """Return the platform-appropriate config directory for fabio."""
    if platform.system() == "Windows":
        base = os.environ.get("APPDATA", str(Path.home() / "AppData" / "Roaming"))
    else:
        base = os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config"))
    return Path(base) / "fabio"


CONFIG_DIR = _config_dir()
AUTH_FILE = CONFIG_DIR / "auth.json"
AZURE_RECORD_FILE = CONFIG_DIR / "azure_auth_record.json"


@dataclass
class AuthRecord:
    """Lightweight metadata persisted for display purposes (status command)."""

    username: str
    tenant_id: str
    authority: str
    login_time: float = field(default_factory=time.time)
    auth_method: str = "browser"  # "browser" or "device_code"

    def age_human(self) -> str:
        """Return a human-readable string describing how long ago login happened."""
        delta = int(time.time() - self.login_time)
        if delta < 60:
            return f"{delta}s ago"
        if delta < 3600:
            return f"{delta // 60}m ago"
        return f"{delta // 3600}h {(delta % 3600) // 60}m ago"


# -- Metadata record persistence ---------------------------------------------


def save_record(record: AuthRecord) -> None:
    """Persist an *AuthRecord* to disk."""
    AUTH_FILE.parent.mkdir(parents=True, exist_ok=True)
    AUTH_FILE.write_text(json.dumps(asdict(record), indent=2))


def load_record() -> AuthRecord | None:
    """Load the saved *AuthRecord*, or *None* if no session exists."""
    if not AUTH_FILE.exists():
        return None
    try:
        data = json.loads(AUTH_FILE.read_text())
        return AuthRecord(**data)
    except (json.JSONDecodeError, TypeError, KeyError):
        return None


# -- Azure-identity AuthenticationRecord persistence --------------------------


def save_azure_record(serialized: str) -> None:
    """Persist the serialized azure-identity AuthenticationRecord."""
    AZURE_RECORD_FILE.parent.mkdir(parents=True, exist_ok=True)
    AZURE_RECORD_FILE.write_text(serialized)


def load_azure_record() -> str | None:
    """Load the serialized azure-identity AuthenticationRecord, or None."""
    if not AZURE_RECORD_FILE.exists():
        return None
    content = AZURE_RECORD_FILE.read_text().strip()
    return content if content else None


# -- Cleanup ------------------------------------------------------------------


def clear_record() -> bool:
    """Remove all persisted auth state.  Returns *True* if anything was deleted."""
    removed = False
    for path in (AUTH_FILE, AZURE_RECORD_FILE):
        if path.exists():
            path.unlink()
            removed = True
    return removed
