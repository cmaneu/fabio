"""Thin HTTP client for the Microsoft Fabric REST API.

Returns raw data (dicts/lists) on success, raises FabioError on failure.
No click dependencies - this module is pure logic.
"""

from __future__ import annotations

from typing import Any

import requests
from azure.core.exceptions import ClientAuthenticationError
from azure.identity import (
    AuthenticationRecord as AzureAuthRecord,
)
from azure.identity import (
    InteractiveBrowserCredential,
)

from fabio.auth_store import load_azure_record, load_record
from fabio.cache import get_cache_options
from fabio.errors import ErrorCode, FabioError

FABRIC_BASE_URL = "https://api.fabric.microsoft.com/v1"
ONELAKE_DFS_URL = "https://onelake.dfs.fabric.microsoft.com"
FABRIC_SCOPE = "https://analysis.windows.net/powerbi/api/.default"
STORAGE_SCOPE = "https://storage.azure.com/.default"


def require_auth(scope: str = FABRIC_SCOPE) -> str:
    """Validate authentication and return a valid access token.

    Raises FabioError if not authenticated or token expired.
    """
    record = load_record()
    azure_record_serialized = load_azure_record()

    if record is None or azure_record_serialized is None:
        raise FabioError(
            ErrorCode.AUTH_REQUIRED,
            "Not authenticated. Run 'fabio auth login' first.",
        )

    azure_record = AzureAuthRecord.deserialize(azure_record_serialized)

    credential = InteractiveBrowserCredential(
        authentication_record=azure_record,
        cache_persistence_options=get_cache_options(warn=False),
        disable_automatic_authentication=True,
    )

    try:
        token = credential.get_token(scope)
    except ClientAuthenticationError as exc:
        raise FabioError(
            ErrorCode.AUTH_EXPIRED,
            "Session expired. Run 'fabio auth login' to re-authenticate.",
        ) from exc

    return token.token


def _handle_response(resp: requests.Response) -> dict[str, Any]:
    """Check response and raise structured errors on failure."""
    if resp.ok:
        if resp.status_code == 204:
            return {}
        return resp.json()  # type: ignore[no-any-return]

    status = resp.status_code
    try:
        body = resp.json()
        message = body.get("error", {}).get("message", resp.text)
    except Exception:
        message = resp.text

    if status == 401:
        raise FabioError(ErrorCode.AUTH_EXPIRED, message, status=status)
    elif status == 403:
        raise FabioError(ErrorCode.FORBIDDEN, message, status=status)
    elif status == 404:
        raise FabioError(ErrorCode.NOT_FOUND, message, status=status)
    elif status == 409:
        raise FabioError(ErrorCode.CONFLICT, message, status=status)
    elif status == 429:
        raise FabioError(ErrorCode.RATE_LIMITED, message, status=status)
    elif status >= 500:
        raise FabioError(ErrorCode.SERVER_ERROR, message, status=status)
    else:
        raise FabioError(ErrorCode.API_ERROR, f"HTTP {status}: {message}", status=status)


def get(path: str, params: dict[str, str] | None = None) -> dict[str, Any]:
    """Make an authenticated GET request to the Fabric API.

    Returns parsed JSON response body.
    Raises FabioError on any failure.
    """
    token = require_auth()
    url = f"{FABRIC_BASE_URL}{path}"
    try:
        resp = requests.get(
            url,
            params=params,
            headers={"Authorization": f"Bearer {token}"},
            timeout=30,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, f"Request timed out: GET {path}") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    return _handle_response(resp)


def post(path: str, body: dict[str, Any] | None = None) -> dict[str, Any]:
    """Make an authenticated POST request to the Fabric API."""
    token = require_auth()
    url = f"{FABRIC_BASE_URL}{path}"
    try:
        resp = requests.post(
            url,
            json=body,
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
            },
            timeout=30,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, f"Request timed out: POST {path}") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    return _handle_response(resp)


def patch(path: str, body: dict[str, Any] | None = None) -> dict[str, Any]:
    """Make an authenticated PATCH request to the Fabric API."""
    token = require_auth()
    url = f"{FABRIC_BASE_URL}{path}"
    try:
        resp = requests.patch(
            url,
            json=body,
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
            },
            timeout=30,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, f"Request timed out: PATCH {path}") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    return _handle_response(resp)


def delete(path: str) -> dict[str, Any]:
    """Make an authenticated DELETE request to the Fabric API."""
    token = require_auth()
    url = f"{FABRIC_BASE_URL}{path}"
    try:
        resp = requests.delete(
            url,
            headers={"Authorization": f"Bearer {token}"},
            timeout=30,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, f"Request timed out: DELETE {path}") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    return _handle_response(resp)


def list_onelake_files(
    workspace_id: str,
    lakehouse_id: str,
    directory: str = "Files",
    *,
    recursive: bool = False,
) -> list[dict[str, Any]]:
    """List files in a lakehouse via the OneLake DFS API.

    Returns list of path entries.
    Raises FabioError on failure.
    """
    token = require_auth(scope=STORAGE_SCOPE)
    url = f"{ONELAKE_DFS_URL}/{workspace_id}/{lakehouse_id}"
    try:
        resp = requests.get(
            url,
            params={
                "resource": "filesystem",
                "directory": directory,
                "recursive": "true" if recursive else "false",
            },
            headers={"Authorization": f"Bearer {token}"},
            timeout=30,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, "OneLake request timed out") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    if not resp.ok:
        if resp.status_code == 404:
            return []
        _handle_response(resp)

    data = resp.json()
    return data.get("paths", [])  # type: ignore[no-any-return]
