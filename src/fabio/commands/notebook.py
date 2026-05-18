"""``fabio notebook`` command group.

Commands:
    create - Create a notebook with content
    get    - Get notebook definition/content
"""

from __future__ import annotations

import base64
import json
from pathlib import Path

import click

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
