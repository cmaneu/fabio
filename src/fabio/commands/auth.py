"""``fabio auth`` command group - login, logout, status."""

from __future__ import annotations

import base64
import json
import sys

import click
from azure.identity import DeviceCodeCredential, InteractiveBrowserCredential
from rich.console import Console

from fabio.auth_store import AuthRecord, clear_record, load_record, save_record

# Microsoft Fabric / Power BI REST API scope
FABRIC_SCOPE = "https://analysis.windows.net/powerbi/api/.default"

console = Console()


def _decode_jwt_claims(token_str: str) -> dict[str, object]:
    """Decode the payload segment of a JWT without signature verification."""
    payload_b64 = token_str.split(".")[1]
    payload_b64 += "=" * (-len(payload_b64) % 4)
    return json.loads(base64.urlsafe_b64decode(payload_b64))  # type: ignore[no-any-return]


def _record_from_token(token_str: str, fallback_tenant: str | None) -> AuthRecord:
    """Build an *AuthRecord* by inspecting JWT claims."""
    claims = _decode_jwt_claims(token_str)
    username = (
        claims.get("upn") or claims.get("preferred_username") or claims.get("sub", "unknown")
    )
    tid = claims.get("tid", fallback_tenant or "unknown")
    return AuthRecord(
        username=str(username),
        tenant_id=str(tid),
        authority=f"https://login.microsoftonline.com/{tid}",
    )


def _do_interactive_login(tenant_id: str | None) -> AuthRecord:
    """Run an interactive browser-based login and return an AuthRecord."""
    kwargs: dict[str, object] = {}
    if tenant_id:
        kwargs["tenant_id"] = tenant_id

    credential = InteractiveBrowserCredential(**kwargs)
    token = credential.get_token(FABRIC_SCOPE)
    return _record_from_token(token.token, tenant_id)


def _do_device_code_login(tenant_id: str | None) -> AuthRecord:
    """Run a device-code login flow and return an AuthRecord."""
    kwargs: dict[str, object] = {}
    if tenant_id:
        kwargs["tenant_id"] = tenant_id

    credential = DeviceCodeCredential(**kwargs)  # type: ignore[arg-type]
    token = credential.get_token(FABRIC_SCOPE)
    return _record_from_token(token.token, tenant_id)


# ---------------------------------------------------------------------------
# CLI commands
# ---------------------------------------------------------------------------


@click.group()
def auth() -> None:
    """Authenticate with Microsoft Fabric."""


@auth.command()
@click.option("--tenant", "-t", default=None, help="Azure AD tenant ID or domain.")
@click.option(
    "--device-code",
    is_flag=True,
    default=False,
    help="Use device-code flow instead of opening a browser.",
)
def login(tenant: str | None, device_code: bool) -> None:
    """Sign in to Microsoft Fabric."""
    try:
        record = _do_device_code_login(tenant) if device_code else _do_interactive_login(tenant)
    except Exception as exc:
        console.print(f"[red]Login failed:[/red] {exc}")
        sys.exit(1)

    save_record(record)
    console.print(f"[green]Logged in as[/green] {record.username}")


@auth.command()
def logout() -> None:
    """Sign out and clear cached credentials."""
    removed = clear_record()
    if removed:
        console.print("[green]Logged out successfully.[/green]")
    else:
        console.print("[yellow]No active session found.[/yellow]")


@auth.command()
def status() -> None:
    """Show current authentication status."""
    record = load_record()
    if record is None:
        console.print(
            "[yellow]Not logged in.[/yellow] Run [bold]fabio auth login[/bold] to sign in."
        )
        sys.exit(1)

    console.print("[green]Logged in[/green]")
    console.print(f"  Account:   {record.username}")
    console.print(f"  Tenant:    {record.tenant_id}")
    console.print(f"  Authority: {record.authority}")
    console.print(f"  Session:   {record.age_human()}")
