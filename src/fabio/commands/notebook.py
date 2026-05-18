"""``fabio notebook`` command group.

Commands:
    create         - Create a notebook with content
    get-definition - Get notebook definition/content
    run            - Run a notebook (submit job)
    status         - Get the status of a notebook job
    stop           - Cancel a running notebook job
    delete         - Delete a notebook
"""

from __future__ import annotations

import base64
import json
import time
from pathlib import Path
from typing import Any

import click
import requests as req

from fabio import client
from fabio.errors import ErrorCode, FabioError
from fabio.output import output


@click.group()
def notebook() -> None:
    """Manage Fabric notebooks."""


def _build_notebook_definition(
    code: str,
    *,
    lakehouse_id: str | None = None,
    lakehouse_name: str | None = None,
    workspace_id: str | None = None,
) -> dict[str, object]:
    """Build a Fabric notebook definition payload.

    Creates a minimal .ipynb-compatible notebook with one code cell.
    """
    # Build source as list of lines (ipynb spec requires list of strings)
    source_lines = [line + "\n" for line in code.split("\n")]
    # Remove trailing newline from last line
    if source_lines and source_lines[-1] == "\n":
        source_lines = source_lines[:-1]

    # Build a standard Jupyter notebook structure
    nb: dict[str, object] = {
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": {
            "kernel_info": {"name": "synapse_pyspark"},
            "language_info": {"name": "python"},
        },
        "cells": [
            {
                "cell_type": "code",
                "metadata": {},
                "source": source_lines,
                "outputs": [],
            }
        ],
    }

    # If lakehouse is specified, add it to metadata for default lakehouse
    if lakehouse_id and workspace_id:
        nb["metadata"] = {
            **nb["metadata"],  # type: ignore[dict-item]
            "dependencies": {
                "lakehouse": {
                    "default_lakehouse": lakehouse_id,
                    "default_lakehouse_name": lakehouse_name or "",
                    "default_lakehouse_workspace_id": workspace_id,
                }
            },
        }

    nb_json = json.dumps(nb, indent=2)
    nb_b64 = base64.b64encode(nb_json.encode()).decode()

    return {
        "format": "ipynb",
        "parts": [
            {
                "path": "notebook-content.py",
                "payload": nb_b64,
                "payloadType": "InlineBase64",
            }
        ],
    }


@notebook.command(name="create")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--name", "-n", required=True, help="Notebook display name.")
@click.option(
    "--code",
    "-c",
    default=None,
    help="Inline Python/PySpark code for the notebook.",
)
@click.option(
    "--file",
    "-f",
    "code_file",
    default=None,
    help="Path to a .py or .ipynb file with notebook content.",
)
@click.option(
    "--lakehouse",
    default=None,
    help="Default lakehouse ID to attach.",
)
@click.option(
    "--lakehouse-name",
    default=None,
    help="Default lakehouse display name.",
)
@click.option("--description", "-d", default=None, help="Notebook description.")
@click.pass_context
def create_notebook(
    ctx: click.Context,
    workspace: str,
    name: str,
    code: str | None,
    code_file: str | None,
    lakehouse: str | None,
    lakehouse_name: str | None,
    description: str | None,
) -> None:
    """Create a notebook in a workspace.

    \b
    Provide code inline with --code, or from a file with --file.
    Examples:
        fabio notebook create -w <ws-id> -n "Process Data" \
            --code "df = spark.read.csv('Files/data.csv')"
        fabio notebook create -w <ws-id> -n "ETL" --file etl.py --lakehouse <lh-id>
    """
    if code is None and code_file is None:
        raise FabioError(
            ErrorCode.MISSING_PARAM,
            "Provide --code or --file with notebook content.",
        )

    if code_file is not None:
        p = Path(code_file)
        if not p.exists():
            raise FabioError(
                ErrorCode.INVALID_INPUT,
                f"File not found: {code_file}",
            )
        code = p.read_text()

    assert code is not None

    definition = _build_notebook_definition(
        code,
        lakehouse_id=lakehouse,
        lakehouse_name=lakehouse_name,
        workspace_id=workspace,
    )

    body: dict[str, object] = {
        "displayName": name,
        "type": "Notebook",
        "definition": definition,
    }
    if description:
        body["description"] = description

    data = client.post(f"/workspaces/{workspace}/items", body=body, poll=True)
    output(ctx, data, plain_key="id")


@notebook.command(name="get-definition")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "notebook_id", required=True, help="Notebook item ID.")
@click.pass_context
def get_definition(ctx: click.Context, workspace: str, notebook_id: str) -> None:
    """Get the definition (content) of a notebook.

    \b
    Returns the notebook definition including encoded content.
    Example: fabio notebook get-definition -w <ws-id> --id <nb-id>
    """
    data = client.get_item_definition(workspace, notebook_id)
    output(ctx, data)


# Job polling defaults for --wait
_JOB_POLL_INTERVAL = 5  # seconds
_JOB_MAX_WAIT = 3600  # 1 hour max


@notebook.command(name="run")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "notebook_id", required=True, help="Notebook item ID.")
@click.option(
    "--wait/--no-wait",
    default=False,
    help="Wait for the job to complete (default: return immediately).",
)
@click.pass_context
def run_notebook(
    ctx: click.Context,
    workspace: str,
    notebook_id: str,
    wait: bool,
) -> None:
    """Run a notebook (submit a job).

    \b
    By default, returns immediately with the job instance ID.
    Use --wait to block until the job completes or fails.

    \b
    Examples:
        fabio notebook run -w <ws-id> --id <nb-id>
        fabio notebook run -w <ws-id> --id <nb-id> --wait
    """
    path = f"/workspaces/{workspace}/items/{notebook_id}/jobs/instances"
    token = client.require_auth()

    try:
        resp = req.post(
            f"{client.FABRIC_BASE_URL}{path}",
            params={"jobType": "RunNotebook"},
            json={},
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
            },
            timeout=30,
        )
    except req.Timeout as exc:
        raise FabioError(ErrorCode.TIMEOUT, "Request timed out") from exc
    except req.ConnectionError as e:
        raise FabioError(ErrorCode.API_ERROR, f"Connection error: {e}") from e

    if resp.status_code not in (200, 202):
        client._handle_response(resp)

    # Extract job instance ID from Location header
    location = resp.headers.get("Location", "")
    job_id = location.rsplit("/", 1)[-1] if location else ""

    if not job_id:
        raise FabioError(
            ErrorCode.API_ERROR,
            "No job instance ID returned from run request.",
        )

    result: dict[str, Any] = {
        "jobInstanceId": job_id,
        "status": "submitted",
        "workspace": workspace,
        "notebookId": notebook_id,
    }

    if not wait:
        output(ctx, result, plain_key="jobInstanceId")
        return

    # Poll until completion
    job_url = (
        f"{client.FABRIC_BASE_URL}/workspaces/{workspace}"
        f"/items/{notebook_id}/jobs/instances/{job_id}"
    )
    elapsed = 0.0
    status = "NotStarted"

    while elapsed < _JOB_MAX_WAIT and status in ("NotStarted", "InProgress"):
        time.sleep(_JOB_POLL_INTERVAL)
        elapsed += _JOB_POLL_INTERVAL

        try:
            poll_resp = req.get(
                job_url,
                headers={"Authorization": f"Bearer {token}"},
                timeout=30,
            )
        except (req.Timeout, req.ConnectionError):
            continue

        if poll_resp.ok and poll_resp.text.strip():
            job_data = poll_resp.json()
            status = job_data.get("status", "Unknown")

    if status in ("NotStarted", "InProgress"):
        result["status"] = status
        result["message"] = (
            f"Job still {status} after {int(elapsed)}s. Check with 'notebook status'."
        )
    else:
        result["status"] = status
        if status == "Failed":
            result["failureReason"] = job_data.get("failureReason", "Unknown")

    output(ctx, result, plain_key="jobInstanceId")


@notebook.command(name="status")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "notebook_id", required=True, help="Notebook item ID.")
@click.option("--job", "-j", required=True, help="Job instance ID.")
@click.pass_context
def job_status(
    ctx: click.Context,
    workspace: str,
    notebook_id: str,
    job: str,
) -> None:
    """Get the status of a notebook job.

    \b
    Returns the job state: NotStarted, InProgress, Completed, Failed, Cancelled.

    \b
    Examples:
        fabio notebook status -w <ws-id> --id <nb-id> -j <job-id>
    """
    path = f"/workspaces/{workspace}/items/{notebook_id}/jobs/instances/{job}"
    data = client.get(path)
    output(ctx, data, plain_key="status")


@notebook.command(name="stop")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "notebook_id", required=True, help="Notebook item ID.")
@click.option("--job", "-j", required=True, help="Job instance ID to cancel.")
@click.pass_context
def stop_notebook(
    ctx: click.Context,
    workspace: str,
    notebook_id: str,
    job: str,
) -> None:
    """Cancel a running notebook job.

    \b
    Examples:
        fabio notebook stop -w <ws-id> --id <nb-id> -j <job-id>
    """
    path = f"/workspaces/{workspace}/items/{notebook_id}/jobs/instances/{job}/cancel"
    client.post(path)
    output(
        ctx,
        {"status": "cancelled", "jobInstanceId": job, "notebookId": notebook_id},
        plain_key="jobInstanceId",
    )


@notebook.command(name="delete")
@click.option("--workspace", "-w", required=True, help="Workspace ID.")
@click.option("--id", "notebook_id", required=True, help="Notebook item ID.")
@click.pass_context
def delete_notebook(
    ctx: click.Context,
    workspace: str,
    notebook_id: str,
) -> None:
    """Delete a notebook from a workspace.

    \b
    Examples:
        fabio notebook delete -w <ws-id> --id <nb-id>
    """
    client.delete(f"/workspaces/{workspace}/items/{notebook_id}")
    output(
        ctx,
        {"status": "deleted", "notebookId": notebook_id, "workspace": workspace},
        plain_key="notebookId",
    )
