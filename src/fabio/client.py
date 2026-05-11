"""Thin HTTP client for the Microsoft Fabric REST API."""

from __future__ import annotations

import sys
from typing import Any

import click
import requests
from azure.identity import InteractiveBrowserCredential
from rich.console import Console

from fabio.auth_store import load_record

FABRIC_BASE_URL = "https://api.fabric.microsoft.com/v1"
FABRIC_SCOPE = "https://analysis.windows.net/powerbi/api/.default"

console = Console()


def _get_access_token() -> str:
    """Acquire a valid access token, or exit if not logged in."""
    record = load_record()
    if record is None:
        console.print("[red]Not authenticated.[/red] Run [bold]fabio auth login[/bold] first.")
        sys.exit(1)

    credential = InteractiveBrowserCredential(tenant_id=record.tenant_id)
    token = credential.get_token(FABRIC_SCOPE)
    return token.token


def get(path: str, params: dict[str, str] | None = None) -> dict[str, Any]:
    """Make an authenticated GET request to the Fabric API.

    Parameters
    ----------
    path:
        Relative path (e.g. ``/workspaces``).
    params:
        Optional query parameters.

    Returns
    -------
    dict:
        Parsed JSON response body.

    Raises
    ------
    click.ClickException:
        On HTTP errors with a user-friendly message.
    """
    token = _get_access_token()
    url = f"{FABRIC_BASE_URL}{path}"
    resp = requests.get(
        url,
        params=params,
        headers={"Authorization": f"Bearer {token}"},
        timeout=30,
    )
    if not resp.ok:
        raise click.ClickException(f"Fabric API error {resp.status_code}: {resp.text}")
    return resp.json()  # type: ignore[no-any-return]
