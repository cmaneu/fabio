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
)
from rich.console import Console

from fabio.auth_store import load_azure_record, load_record
from fabio.cache import get_cache_options

FABRIC_BASE_URL = "https://api.fabric.microsoft.com/v1"
ONELAKE_DFS_URL = "https://onelake.dfs.fabric.microsoft.com"
FABRIC_SCOPE = "https://analysis.windows.net/powerbi/api/.default"
STORAGE_SCOPE = "https://storage.azure.com/.default"

console = Console()


def require_auth(scope: str = FABRIC_SCOPE) -> str:
    """Validate authentication and return a valid access token.

    Parameters
    ----------
    scope:
        The OAuth scope to request (default: Fabric API scope).

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
        cache_persistence_options=get_cache_options(warn=False),
        # Disable automatic interactive auth -- if the cache is empty/expired
        # we want to fail explicitly rather than pop open a browser.
        disable_automatic_authentication=True,
    )

    try:
        token = credential.get_token(scope)
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


def list_onelake_files(
    workspace_id: str, lakehouse_id: str, directory: str = "Files"
) -> list[dict[str, Any]]:
    """List files in a lakehouse via the OneLake DFS API.

    Parameters
    ----------
    workspace_id:
        The workspace ID.
    lakehouse_id:
        The lakehouse item ID.
    directory:
        Directory path within the lakehouse (default ``Files``).

    Returns
    -------
    list:
        List of path entries (name, isDirectory, contentLength, lastModified).
    """
    token = require_auth(scope=STORAGE_SCOPE)
    url = f"{ONELAKE_DFS_URL}/{workspace_id}/{lakehouse_id}"
    resp = requests.get(
        url,
        params={"resource": "filesystem", "directory": directory, "recursive": "false"},
        headers={"Authorization": f"Bearer {token}"},
        timeout=30,
    )
    if not resp.ok:
        if resp.status_code == 404:
            return []
        raise click.ClickException(f"OneLake API error {resp.status_code}: {resp.text}")
    data = resp.json()
    return data.get("paths", [])  # type: ignore[no-any-return]
