"""Thin HTTP client for the Microsoft Fabric REST API.

Returns raw data (dicts/lists) on success, raises FabioError on failure.
No click dependencies - this module is pure logic.
"""

from __future__ import annotations

import time
from typing import Any

import requests
from azure.core.exceptions import ClientAuthenticationError
from azure.identity import (
    AuthenticationRecord as AzureAuthRecord,
)
from azure.identity import (
    DeviceCodeCredential,
    InteractiveBrowserCredential,
)

from fabio.auth_store import load_azure_record, load_record
from fabio.cache import get_cache_options
from fabio.errors import ErrorCode, FabioError

FABRIC_BASE_URL = "https://api.fabric.microsoft.com/v1"
ONELAKE_DFS_URL = "https://onelake.dfs.fabric.microsoft.com"
ONELAKE_BLOB_URL = "https://onelake.blob.fabric.microsoft.com"
FABRIC_SCOPE = "https://analysis.windows.net/powerbi/api/.default"
STORAGE_SCOPE = "https://storage.azure.com/.default"

# LRO polling defaults
LRO_POLL_INTERVAL = 2  # seconds between polls
LRO_MAX_WAIT = 120  # seconds max total wait


def require_auth(scope: str = FABRIC_SCOPE) -> str:
    """Validate authentication and return a valid access token.

    Raises FabioError if not authenticated or token expired.
    Uses the same credential type (browser or device-code) that was
    used during login, so that silent token refresh works correctly.
    """
    record = load_record()
    azure_record_serialized = load_azure_record()

    if record is None or azure_record_serialized is None:
        raise FabioError(
            ErrorCode.AUTH_REQUIRED,
            "Not authenticated. Run 'fabio auth login' first.",
        )

    azure_record = AzureAuthRecord.deserialize(azure_record_serialized)
    cache_options = get_cache_options(warn=False)

    auth_method = getattr(record, "auth_method", "browser")
    if auth_method == "device_code":
        credential = DeviceCodeCredential(
            authentication_record=azure_record,
            cache_persistence_options=cache_options,
            disable_automatic_authentication=True,
        )
    else:
        credential = InteractiveBrowserCredential(  # type: ignore[assignment]
            authentication_record=azure_record,
            cache_persistence_options=cache_options,
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
        if resp.status_code == 204 or not resp.text.strip():
            return {}
        return resp.json()  # type: ignore[no-any-return]

    status = resp.status_code
    try:
        body = resp.json()
        message = body.get("error", {}).get("message", resp.text)
        error_code = body.get("error", {}).get("code", "") or body.get("errorCode", "")
    except Exception:
        message = resp.text
        error_code = ""

    # Detect Fabric-specific capacity errors before generic status mapping
    if "CapacityNotActive" in error_code or "CapacityNotActive" in message:
        raise FabioError(
            ErrorCode.CAPACITY_INACTIVE,
            "Fabric capacity is paused or inactive. Resume it in the Azure portal.",
            status=status,
        )

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


def _poll_lro(resp: requests.Response, token: str) -> dict[str, Any]:
    """Poll a long-running operation until it completes.

    Fabric LROs return 202 with a Location header or Retry-After.
    We poll the Location URL until status is Succeeded/Failed.
    """
    location = resp.headers.get("Location")
    operation_id = resp.headers.get("x-ms-operation-id")

    if not location and not operation_id:
        # No LRO info, just return empty
        return {}

    poll_url = location or f"{FABRIC_BASE_URL}/operations/{operation_id}"
    retry_after = int(resp.headers.get("Retry-After", str(LRO_POLL_INTERVAL)))

    elapsed = 0.0
    while elapsed < LRO_MAX_WAIT:
        time.sleep(retry_after)
        elapsed += retry_after

        try:
            poll_resp = requests.get(
                poll_url,
                headers={"Authorization": f"Bearer {token}"},
                timeout=30,
            )
        except (requests.Timeout, requests.ConnectionError) as exc:
            raise FabioError(ErrorCode.TIMEOUT, f"LRO poll failed: {exc}") from exc

        if not poll_resp.ok:
            _handle_response(poll_resp)

        if not poll_resp.text.strip():
            continue

        result = poll_resp.json()
        status = result.get("status", "").lower()

        if status == "succeeded":
            # Check if there's a resource location to fetch the final result
            resource_location = poll_resp.headers.get("Location")
            if resource_location:
                final_resp = requests.get(
                    resource_location,
                    headers={"Authorization": f"Bearer {token}"},
                    timeout=30,
                )
                if final_resp.ok and final_resp.text.strip():
                    return final_resp.json()  # type: ignore[no-any-return]
            return result  # type: ignore[no-any-return]
        elif status == "failed":
            error_msg = result.get("error", {}).get("message", "Operation failed")
            raise FabioError(ErrorCode.API_ERROR, f"LRO failed: {error_msg}")
        elif status in ("running", "notstarted"):
            retry_after = int(poll_resp.headers.get("Retry-After", str(LRO_POLL_INTERVAL)))
            continue
        else:
            # Unknown status, keep polling
            continue

    raise FabioError(
        ErrorCode.TIMEOUT,
        f"Operation timed out after {LRO_MAX_WAIT}s. Operation ID: {operation_id}",
    )


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


def post(path: str, body: dict[str, Any] | None = None, *, poll: bool = False) -> dict[str, Any]:
    """Make an authenticated POST request to the Fabric API.

    If poll=True, handles 202 responses by polling the LRO until completion.
    """
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

    if poll and resp.status_code == 202:
        return _poll_lro(resp, token)

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


def upload_onelake_file(
    workspace_id: str,
    lakehouse_id: str,
    dest_path: str,
    content: bytes,
) -> None:
    """Upload a file to a lakehouse via the OneLake DFS API.

    Uses the create + append + flush pattern for the DFS API.
    """
    token = require_auth(scope=STORAGE_SCOPE)
    base_url = f"{ONELAKE_DFS_URL}/{workspace_id}/{lakehouse_id}/{dest_path}"

    # Step 1: Create the file resource
    try:
        resp = requests.put(
            base_url,
            params={"resource": "file"},
            headers={"Authorization": f"Bearer {token}"},
            timeout=30,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, "OneLake create timed out") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    if not resp.ok:
        _handle_response(resp)

    # Step 2: Append content
    try:
        resp = requests.patch(
            base_url,
            params={"action": "append", "position": "0"},
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/octet-stream",
            },
            data=content,
            timeout=60,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, "OneLake append timed out") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    if not resp.ok:
        _handle_response(resp)

    # Step 3: Flush (commit)
    try:
        resp = requests.patch(
            base_url,
            params={"action": "flush", "position": str(len(content))},
            headers={"Authorization": f"Bearer {token}"},
            timeout=30,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, "OneLake flush timed out") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    if not resp.ok:
        _handle_response(resp)


def download_onelake_file(
    workspace_id: str,
    lakehouse_id: str,
    file_path: str,
) -> bytes:
    """Download a file from a lakehouse via the OneLake DFS API."""
    token = require_auth(scope=STORAGE_SCOPE)
    url = f"{ONELAKE_DFS_URL}/{workspace_id}/{lakehouse_id}/{file_path}"

    try:
        resp = requests.get(
            url,
            headers={"Authorization": f"Bearer {token}"},
            timeout=60,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, "OneLake download timed out") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    if not resp.ok:
        _handle_response(resp)

    return resp.content


def load_table(
    workspace_id: str,
    lakehouse_id: str,
    table_name: str,
    relative_path: str,
    *,
    file_extension: str | None = None,
    format_options: dict[str, Any] | None = None,
    mode: str = "Overwrite",
    recursive: bool = False,
) -> dict[str, Any]:
    """Load a file into a lakehouse table via the Tables API.

    Parameters
    ----------
    relative_path:
        Path relative to lakehouse root (e.g. "Files/data.csv").
    file_extension:
        File extension hint (e.g. "Csv", "Parquet").
    format_options:
        Format-specific options (e.g. {"format": "Csv", "header": "true"}).
    mode:
        Load mode: "Overwrite" or "Append" (PascalCase).
    recursive:
        Whether to recursively load from a directory.
    """
    path = f"/workspaces/{workspace_id}/lakehouses/{lakehouse_id}/tables/{table_name}/load"

    body: dict[str, Any] = {
        "relativePath": relative_path,
        "pathType": "File",
        "mode": mode,
        "recursive": recursive,
    }
    if file_extension:
        body["fileExtension"] = file_extension
    if format_options:
        body["formatOptions"] = format_options

    return post(path, body=body, poll=True)


def get_item_definition(
    workspace_id: str,
    item_id: str,
) -> dict[str, Any]:
    """Get the definition of an item (for notebooks, reports, etc).

    POST /workspaces/{ws}/items/{item}/getDefinition
    Uses LRO polling since the API may return 202.
    """
    path = f"/workspaces/{workspace_id}/items/{item_id}/getDefinition"
    return post(path, poll=True)


def update_item_definition(
    workspace_id: str,
    item_id: str,
    definition: dict[str, Any],
) -> dict[str, Any]:
    """Update the definition of an item.

    POST /workspaces/{ws}/items/{item}/updateDefinition
    """
    path = f"/workspaces/{workspace_id}/items/{item_id}/updateDefinition"
    return post(path, body={"definition": definition})


# Copy polling defaults
COPY_POLL_INTERVAL = 1  # seconds between polls
COPY_MAX_WAIT = 300  # seconds max total wait (large files can take time)


def copy_onelake_file(
    src_workspace_id: str,
    src_item_id: str,
    src_path: str,
    dest_workspace_id: str,
    dest_item_id: str,
    dest_path: str,
) -> dict[str, Any]:
    """Copy a file between lakehouses via server-side copy (OneLake Blob API).

    Uses PUT with x-ms-copy-source to trigger an async server-side copy.
    The data never transits through the client. Polls until copy completes.

    Returns dict with copy status and metadata.
    """
    token = require_auth(scope=STORAGE_SCOPE)

    source_url = f"{ONELAKE_BLOB_URL}/{src_workspace_id}/{src_item_id}/{src_path}"
    dest_url = f"{ONELAKE_BLOB_URL}/{dest_workspace_id}/{dest_item_id}/{dest_path}"

    # Initiate server-side copy
    try:
        resp = requests.put(
            dest_url,
            headers={
                "Authorization": f"Bearer {token}",
                "x-ms-copy-source": source_url,
                "x-ms-version": "2024-08-04",
            },
            timeout=30,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, "OneLake copy initiation timed out") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    if not resp.ok:
        _handle_response(resp)

    copy_status = resp.headers.get("x-ms-copy-status", "")
    copy_id = resp.headers.get("x-ms-copy-id", "")

    # If copy completed synchronously (small files)
    if copy_status == "success":
        return {"copyId": copy_id, "copyStatus": "success"}

    # Poll for async copy completion via HEAD on destination
    elapsed = 0.0
    while elapsed < COPY_MAX_WAIT and copy_status == "pending":
        time.sleep(COPY_POLL_INTERVAL)
        elapsed += COPY_POLL_INTERVAL

        try:
            head_resp = requests.head(
                dest_url,
                headers={
                    "Authorization": f"Bearer {token}",
                    "x-ms-version": "2024-08-04",
                },
                timeout=30,
            )
        except (requests.Timeout, requests.ConnectionError):
            continue

        copy_status = head_resp.headers.get("x-ms-copy-status", "pending")

    if copy_status == "success":
        return {"copyId": copy_id, "copyStatus": "success"}
    elif copy_status == "failed":
        desc = resp.headers.get("x-ms-copy-status-description", "Unknown error")
        raise FabioError(ErrorCode.API_ERROR, f"Server-side copy failed: {desc}")
    elif copy_status == "pending":
        raise FabioError(
            ErrorCode.TIMEOUT,
            f"Copy still pending after {COPY_MAX_WAIT}s. Copy ID: {copy_id}",
        )
    else:
        return {"copyId": copy_id, "copyStatus": copy_status}


def delete_onelake_file(
    workspace_id: str,
    item_id: str,
    file_path: str,
) -> None:
    """Delete a file from a lakehouse via the OneLake DFS API.

    Raises FabioError if the file doesn't exist or on failure.
    """
    token = require_auth(scope=STORAGE_SCOPE)
    url = f"{ONELAKE_DFS_URL}/{workspace_id}/{item_id}/{file_path}"

    try:
        resp = requests.delete(
            url,
            headers={"Authorization": f"Bearer {token}"},
            timeout=30,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, "OneLake delete timed out") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    if not resp.ok:
        _handle_response(resp)


def move_onelake_file(
    src_workspace_id: str,
    src_item_id: str,
    src_path: str,
    dest_workspace_id: str,
    dest_item_id: str,
    dest_path: str,
) -> dict[str, Any]:
    """Move a file between lakehouses (server-side copy + delete source).

    Performs a server-side copy via the Blob API, then deletes the source
    file via the DFS API. Data never transits through the client.

    Safe failure mode: if delete fails after copy, you get a duplicate
    rather than data loss.
    """
    # Step 1: Server-side copy
    result = copy_onelake_file(
        src_workspace_id,
        src_item_id,
        src_path,
        dest_workspace_id,
        dest_item_id,
        dest_path,
    )

    # Step 2: Delete source (only after copy confirmed success)
    delete_onelake_file(src_workspace_id, src_item_id, src_path)

    result["copyStatus"] = "moved"
    return result


def _list_table_files(
    workspace_id: str,
    item_id: str,
    table_name: str,
) -> list[str]:
    """List all files belonging to a Delta table in a lakehouse.

    Uses the root filesystem listing and filters for the table prefix.
    Returns relative paths like 'Tables/sales/_delta_log/00000.json'.
    """
    token = require_auth(scope=STORAGE_SCOPE)
    url = f"{ONELAKE_DFS_URL}/{workspace_id}/{item_id}"

    try:
        resp = requests.get(
            url,
            params={"resource": "filesystem", "recursive": "true"},
            headers={"Authorization": f"Bearer {token}"},
            timeout=60,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, "OneLake list timed out") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    if not resp.ok:
        _handle_response(resp)

    paths = resp.json().get("paths", [])
    prefix = f"{item_id}/Tables/{table_name}/"
    table_files: list[str] = []

    for entry in paths:
        name = entry.get("name", "")
        is_dir = entry.get("isDirectory", "false") == "true"
        if not is_dir and name.startswith(prefix):
            # Strip the item_id prefix to get relative path
            rel_path = name[len(f"{item_id}/") :]
            table_files.append(rel_path)

    return table_files


def delete_onelake_directory(
    workspace_id: str,
    item_id: str,
    dir_path: str,
) -> None:
    """Recursively delete a directory from a lakehouse via OneLake DFS API."""
    token = require_auth(scope=STORAGE_SCOPE)
    url = f"{ONELAKE_DFS_URL}/{workspace_id}/{item_id}/{dir_path}"

    try:
        resp = requests.delete(
            url,
            params={"recursive": "true"},
            headers={"Authorization": f"Bearer {token}"},
            timeout=60,
        )
    except requests.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, "OneLake directory delete timed out") from exc
    except requests.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    if not resp.ok:
        _handle_response(resp)


def delete_table(
    workspace_id: str,
    item_id: str,
    table_name: str,
) -> None:
    """Delete a Delta table from a lakehouse (recursive directory delete)."""
    delete_onelake_directory(workspace_id, item_id, f"Tables/{table_name}")


def copy_table(
    src_workspace_id: str,
    src_item_id: str,
    src_table: str,
    dest_workspace_id: str,
    dest_item_id: str,
    dest_table: str,
) -> dict[str, Any]:
    """Copy a Delta table between lakehouses via server-side file copy.

    Lists all files in the source table directory, then copies each file
    to the destination using the Blob API (server-side, no client transit).

    Returns dict with file count and status.
    """
    # List all files in the source table
    source_files = _list_table_files(src_workspace_id, src_item_id, src_table)

    if not source_files:
        raise FabioError(
            ErrorCode.NOT_FOUND,
            f"Table '{src_table}' not found or has no files.",
        )

    token = require_auth(scope=STORAGE_SCOPE)
    src_prefix = f"Tables/{src_table}/"
    dest_prefix = f"Tables/{dest_table}/"
    copied = 0

    for file_path in source_files:
        # Replace source table prefix with dest table prefix
        relative = file_path[len(src_prefix) :]
        dest_path = f"{dest_prefix}{relative}"

        source_url = f"{ONELAKE_BLOB_URL}/{src_workspace_id}/{src_item_id}/{file_path}"
        dest_url = f"{ONELAKE_BLOB_URL}/{dest_workspace_id}/{dest_item_id}/{dest_path}"

        try:
            resp = requests.put(
                dest_url,
                headers={
                    "Authorization": f"Bearer {token}",
                    "x-ms-copy-source": source_url,
                    "x-ms-version": "2024-08-04",
                },
                timeout=30,
            )
        except (requests.Timeout, requests.ConnectionError) as exc:
            raise FabioError(ErrorCode.API_ERROR, f"Copy failed for {file_path}: {exc}") from exc

        if not resp.ok:
            _handle_response(resp)

        # Wait for async copy if pending
        copy_status = resp.headers.get("x-ms-copy-status", "success")
        if copy_status == "pending":
            elapsed = 0.0
            while elapsed < COPY_MAX_WAIT and copy_status == "pending":
                time.sleep(COPY_POLL_INTERVAL)
                elapsed += COPY_POLL_INTERVAL
                try:
                    head = requests.head(
                        dest_url,
                        headers={
                            "Authorization": f"Bearer {token}",
                            "x-ms-version": "2024-08-04",
                        },
                        timeout=30,
                    )
                    copy_status = head.headers.get("x-ms-copy-status", "pending")
                except (requests.Timeout, requests.ConnectionError):
                    continue

            if copy_status != "success":
                raise FabioError(
                    ErrorCode.API_ERROR,
                    f"Copy timed out for {file_path}. Status: {copy_status}",
                )

        copied += 1

    return {"filesCopied": copied, "sourceTable": src_table, "destTable": dest_table}


def move_table(
    src_workspace_id: str,
    src_item_id: str,
    src_table: str,
    dest_workspace_id: str,
    dest_item_id: str,
    dest_table: str,
) -> dict[str, Any]:
    """Move a Delta table between lakehouses (server-side copy + delete).

    Copies all table files to the destination, then deletes the source
    table directory. Safe failure mode: duplicate rather than data loss.
    """
    result = copy_table(
        src_workspace_id,
        src_item_id,
        src_table,
        dest_workspace_id,
        dest_item_id,
        dest_table,
    )

    delete_table(src_workspace_id, src_item_id, src_table)

    result["status"] = "moved"
    return result
