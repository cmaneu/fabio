"""``fabio auth`` command group - login, logout, status.

Auth commands are the one exception to JSON-only output:
login is inherently interactive (opens browser), so it uses
stderr for human messages but still returns structured JSON on stdout.
"""

from __future__ import annotations

import click
from azure.identity import (
    AuthenticationRecord as AzureAuthRecord,
)
from azure.identity import (
    DeviceCodeCredential,
    InteractiveBrowserCredential,
)

from fabio.auth_store import (
    AuthRecord,
    clear_record,
    load_record,
    save_azure_record,
    save_record,
)
from fabio.cache import get_cache_options
from fabio.errors import ErrorCode, FabioError
from fabio.output import output

# Microsoft Fabric / Power BI REST API scope
FABRIC_SCOPE = "https://analysis.windows.net/powerbi/api/.default"


def _record_from_azure_auth(azure_record: AzureAuthRecord) -> AuthRecord:
    """Build a display-friendly AuthRecord from an azure-identity AuthenticationRecord."""
    return AuthRecord(
        username=azure_record.username,
        tenant_id=azure_record.tenant_id,
        authority=azure_record.authority,
    )


def _do_interactive_login(tenant_id: str | None) -> tuple[AuthRecord, str]:
    """Run an interactive browser-based login."""
    cache_options = get_cache_options()
    kwargs: dict[str, object] = {"cache_persistence_options": cache_options}
    if tenant_id:
        kwargs["tenant_id"] = tenant_id

    credential = InteractiveBrowserCredential(**kwargs)
    azure_record = credential.authenticate(scopes=(FABRIC_SCOPE,))
    return _record_from_azure_auth(azure_record), azure_record.serialize()


def _do_device_code_login(tenant_id: str | None) -> tuple[AuthRecord, str]:
    """Run a device-code login flow."""
    cache_options = get_cache_options()
    kwargs: dict[str, object] = {"cache_persistence_options": cache_options}
    if tenant_id:
        kwargs["tenant_id"] = tenant_id

    credential = DeviceCodeCredential(**kwargs)  # type: ignore[arg-type]
    azure_record = credential.authenticate(scopes=(FABRIC_SCOPE,))
    return _record_from_azure_auth(azure_record), azure_record.serialize()


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
@click.pass_context
def login(ctx: click.Context, tenant: str | None, use_device_code: bool) -> None:
    """Sign in to Microsoft Fabric.

    \b
    Opens a browser for interactive login (default), or prints
    a device code for headless environments.

    Example: fabio auth login --tenant contoso.com
    """
    try:
        record, serialized = (
            _do_device_code_login(tenant) if use_device_code else _do_interactive_login(tenant)
        )
    except Exception as exc:
        raise FabioError(ErrorCode.AUTH_FAILED, f"Login failed: {exc}") from exc

    save_record(record)
    save_azure_record(serialized)

    output(
        ctx,
        {
            "status": "authenticated",
            "username": record.username,
            "tenant_id": record.tenant_id,
        },
    )


@auth.command()
@click.pass_context
def logout(ctx: click.Context) -> None:
    """Sign out and clear cached credentials.

    \b
    Example: fabio auth logout
    """
    removed = clear_record()
    output(
        ctx,
        {
            "status": "logged_out" if removed else "no_session",
        },
    )


@auth.command()
@click.pass_context
def status(ctx: click.Context) -> None:
    """Show current authentication status.

    \b
    Example: fabio auth status
    """
    record = load_record()
    if record is None:
        raise FabioError(
            ErrorCode.AUTH_REQUIRED,
            "Not logged in. Run 'fabio auth login' to sign in.",
        )

    output(
        ctx,
        {
            "status": "authenticated",
            "username": record.username,
            "tenant_id": record.tenant_id,
            "authority": record.authority,
            "session_age": record.age_human(),
        },
    )
