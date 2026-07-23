---
title: Authenticate Fabio
description: Choose interactive, service-principal, workload-identity, or cached authentication.
---

Fabio resolves credentials from several sources so local development and automation can use the same commands.

## Interactive sign-in

Device code is the portable default:

```bash
fabio auth login
```

For a browser redirect:

```bash
fabio auth login --browser
```

On Windows, Web Account Manager provides native single sign-on:

```powershell
fabio auth login --wam
```

Inspect or clear the local session with `fabio auth status` and `fabio auth logout`.

## Service principal

For non-interactive CI/CD:

```bash
fabio auth login --service-principal \
  --tenant <tenant-id> \
  --client-id <client-id> \
  --client-secret <client-secret>
```

Prefer environment variables or your CI secret store over command-line secrets. Fabio also supports certificate and federated-token authentication; run `fabio auth login --help` for the exact flags.

## Credential precedence

Fabio checks an explicit `FABIO_ACCESS_TOKEN`, its encrypted login cache, Azure environment credentials, managed identity, Azure CLI, and Azure Developer CLI. A command may require a separate audience for Fabric, OneLake storage, SQL, ARM, Kusto, or Microsoft Graph.

If an error reports `AUTH_REQUIRED`, follow its `hint` rather than changing the requested operation.
