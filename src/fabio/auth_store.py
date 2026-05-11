"""Persistent token / session cache for Fabric authentication.

Tokens are stored as JSON in ``~/.config/fabio/auth.json`` (XDG-compatible on
Linux, ``%APPDATA%/fabio`` on Windows).  The file contains only the metadata
needed to display auth status (account hint, expiry); actual credential caching
is delegated to ``azure-identity``'s built-in persistent cache so that refresh
tokens are handled securely.
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


AUTH_FILE = _config_dir() / "auth.json"


@dataclass
class AuthRecord:
    """Lightweight record persisted alongside the azure-identity cache."""

    username: str
    tenant_id: str
    authority: str
    login_time: float = field(default_factory=time.time)

    def age_human(self) -> str:
        """Return a human-readable string describing how long ago login happened."""
        delta = int(time.time() - self.login_time)
        if delta < 60:
            return f"{delta}s ago"
        if delta < 3600:
            return f"{delta // 60}m ago"
        return f"{delta // 3600}h {(delta % 3600) // 60}m ago"


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


def clear_record() -> bool:
    """Remove the persisted auth record.  Returns *True* if a file was deleted."""
    if AUTH_FILE.exists():
        AUTH_FILE.unlink()
        return True
    return False
