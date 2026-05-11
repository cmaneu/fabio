"""``fabio auth`` command group - login, logout, status."""

from __future__ import annotations

import sys

import click
from azure.identity import (
    AuthenticationRecord as AzureAuthRecord,
)
from azure.identity import (
    DeviceCodeCredential,
    InteractiveBrowserCredential,
    TokenCachePersistenceOptions,
)
from rich.console import Console

from fabio.auth_store import (
    AuthRecord,
    clear_record,
    load_record,
    save_azure_record,
    save_record,
)

# Microsoft Fabric / Power BI REST API scope
FABRIC_SCOPE = "https://analysis.windows.net/powerbi/api/.default"

console = Console()

# Enable persistent MSAL token cache so refresh tokens survive across sessions.
_cache_options = TokenCachePersistenceOptions(name="fabio")


def _record_from_azure_auth(azure_record: AzureAuthRecord) -> AuthRecord:
    """Build a display-friendly AuthRecord from an azure-identity AuthenticationRecord."""
    return AuthRecord(
        username=azure_record.username,
        tenant_id=azure_record.tenant_id,
        authority=azure_record.authority,
    )


def _do_interactive_login(tenant_id: str | None) -> tuple[AuthRecord, str]:
    """Run an interactive browser-based login.

    Returns the display AuthRecord and the serialized azure AuthenticationRecord.
    """
    kwargs: dict[str, object] = {"cache_persistence_options": _cache_options}
    if tenant_id:
        kwargs["tenant_id"] = tenant_id

    credential = InteractiveBrowserCredential(**kwargs)
    # Force token acquisition to populate the cache and authentication_record.
    credential.get_token(FABRIC_SCOPE)

    azure_record = credential.authentication_record  # type: ignore[attr-defined]
    return _record_from_azure_auth(azure_record), azure_record.serialize()


def _do_device_code_login(tenant_id: str | None) -> tuple[AuthRecord, str]:
    """Run a device-code login flow.

    Returns the display AuthRecord and the serialized azure AuthenticationRecord.
    """
    kwargs: dict[str, object] = {"cache_persistence_options": _cache_options}
    if tenant_id:
        kwargs["tenant_id"] = tenant_id

    credential = DeviceCodeCredential(**kwargs)  # type: ignore[arg-type]
    credential.get_token(FABRIC_SCOPE)

    azure_record = credential.authentication_record  # type: ignore[attr-defined]
    return _record_from_azure_auth(azure_record), azure_record.serialize()


# ---------------------------------------------------------------------------
# CLI commands
# ---------------------------------------------------------------------------


@click.group()
def auth() -> None:
    """Authenticate with Microsoft Fabric."""


@auth.command()
@click.option("--tenant", "-t", default=None, help="Azure AD tenant ID or domain.")
@click.option(
    "--use-device-code",
    is_flag=True,
    default=False,
    help="Use device-code flow instead of opening a browser.",
)
def login(tenant: str | None, use_device_code: bool) -> None:
    """Sign in to Microsoft Fabric."""
    try:
        record, serialized = (
            _do_device_code_login(tenant) if use_device_code else _do_interactive_login(tenant)
        )
    except Exception as exc:
        console.print(f"[red]Login failed:[/red] {exc}")
        sys.exit(1)

    save_record(record)
    save_azure_record(serialized)
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
