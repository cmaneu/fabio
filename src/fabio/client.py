"""Thin HTTP client for the Microsoft Fabric REST API.

All public functions in this module validate that the user is authenticated
before making requests.  Tokens are acquired silently from the persistent
MSAL cache populated during ``fabio auth login``.
"""

from __future__ import annotations

import sys
from typing import Any

import click
import requests
from azure.core.exceptions import ClientAuthenticationError
from azure.identity import (
    AuthenticationRecord as AzureAuthRecord,
)
from azure.identity import (
    InteractiveBrowserCredential,
    TokenCachePersistenceOptions,
)
from rich.console import Console

from fabio.auth_store import load_azure_record, load_record

FABRIC_BASE_URL = "https://api.fabric.microsoft.com/v1"
FABRIC_SCOPE = "https://analysis.windows.net/powerbi/api/.default"

console = Console()

_cache_options = TokenCachePersistenceOptions(name="fabio")


def require_auth() -> str:
    """Validate authentication and return a valid access token.

    Exits with a helpful error message if:
    - The user has never logged in.
    - The cached credentials have expired and cannot be refreshed silently.
    """
    record = load_record()
    azure_record_serialized = load_azure_record()

    if record is None or azure_record_serialized is None:
        console.print("[red]Not authenticated.[/red] Run [bold]fabio auth login[/bold] first.")
        sys.exit(1)

    # Deserialize the azure-identity AuthenticationRecord so the credential
    # can look up cached refresh tokens without user interaction.
    azure_record = AzureAuthRecord.deserialize(azure_record_serialized)

    credential = InteractiveBrowserCredential(
        authentication_record=azure_record,
        cache_persistence_options=_cache_options,
        # Disable automatic interactive auth -- if the cache is empty/expired
        # we want to fail explicitly rather than pop open a browser.
        disable_automatic_authentication=True,
    )

    try:
        token = credential.get_token(FABRIC_SCOPE)
    except ClientAuthenticationError:
        console.print(
            "[red]Session expired.[/red] Run [bold]fabio auth login[/bold] to re-authenticate."
        )
        sys.exit(1)

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
    token = require_auth()
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
