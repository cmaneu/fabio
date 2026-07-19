#!/usr/bin/env bash
# scripts/create-fabio-app.sh — Create the Entra ID app registration that fabio
# uses to sign in interactive USERS (device code, browser PKCE, Windows WAM).
#
# This is the PUBLIC CLIENT app baked into fabio as DEFAULT_PUBLIC_CLIENT_ID in
# src/token_cache.rs. It is NOT the CI service-principal app — for that, use
# scripts/setup-ci-auth.sh instead.
#
# The app is created as:
#   - Multitenant           (sign-in audience: AzureADMultipleOrgs)
#   - Public client         (allowPublicClient = true → enables device code flow)
#   - Redirect URIs         (loopback for browser PKCE, native client, WAM broker)
#   - Delegated permissions (Microsoft Graph User.Read + Fabric/Power BI scopes)
#
# By default the script also patches src/token_cache.rs so the new app ID becomes
# the compiled-in default. You can instead (or additionally) point an existing
# build at the new app at runtime with:  export FABIO_CLIENT_ID=<new-app-id>
#
# Prerequisites:
#   - az CLI authenticated with permission to create app registrations
#     (Application Administrator / Cloud Application Administrator, or owner).
#
# Usage:
#   ./scripts/create-fabio-app.sh [--name <display-name>] [--no-patch-source]
#                                 [--admin-consent] [--print-only]
#
# Examples:
#   ./scripts/create-fabio-app.sh
#   ./scripts/create-fabio-app.sh --name "Fabio CLI" --admin-consent

set -euo pipefail

# ── Defaults ────────────────────────────────────────────────────────────────
APP_NAME="Fabio CLI"
PATCH_SOURCE=1
ADMIN_CONSENT=0
PRINT_ONLY=0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC_FILE="${SCRIPT_DIR}/../src/token_cache.rs"

# Well-known first-party resource identifiers.
GRAPH_APP_ID="00000003-0000-0000-c000-000000000000"        # Microsoft Graph
GRAPH_USER_READ="e1fe6dd8-ba31-4d61-89e7-88639da4683d"     # Graph User.Read (delegated)
POWER_BI_APP_ID="00000009-0000-0000-c000-000000000000"     # Power BI Service (serves api.fabric.microsoft.com)
FABRIC_RESOURCE_URI="https://api.fabric.microsoft.com"

# ── Parse args ──────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --name)            APP_NAME="${2:?--name requires a value}"; shift 2 ;;
    --no-patch-source) PATCH_SOURCE=0; shift ;;
    --admin-consent)   ADMIN_CONSENT=1; shift ;;
    --print-only)      PRINT_ONLY=1; shift ;;
    -h|--help)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

command -v az >/dev/null 2>&1 || { echo "ERROR: az CLI is not installed." >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "ERROR: jq is not installed." >&2; exit 1; }

echo "=== Create fabio public-client app registration ==="
echo "Display name: $APP_NAME"
echo ""

# ── 1. Tenant context ───────────────────────────────────────────────────────
TENANT_ID=$(az account show --query tenantId -o tsv)
echo "[1/7] Tenant: $TENANT_ID"

# ── 2. Create the app (multitenant public client + loopback redirect) ───────
echo "[2/7] Creating app registration..."
APP_ID=$(az ad app create \
  --display-name "$APP_NAME" \
  --sign-in-audience AzureADMultipleOrgs \
  --is-fallback-public-client true \
  --public-client-redirect-uris "http://localhost" \
  --query appId -o tsv)
echo "       App ID: $APP_ID"

# ── 3. Configure all public-client redirect URIs ────────────────────────────
# - http://localhost                     : browser PKCE flow (loopback, any port)
# - .../common/oauth2/nativeclient       : MSAL native client fallback
# - ms-appx-web://.../brokerplugin/<id>  : Windows WAM broker (SSO)
echo "[3/7] Configuring redirect URIs (loopback, native client, WAM broker)..."
az ad app update --id "$APP_ID" --public-client-redirect-uris \
  "http://localhost" \
  "https://login.microsoftonline.com/common/oauth2/nativeclient" \
  "ms-appx-web://microsoft.aad.brokerplugin/${APP_ID}" \
  -o none
echo "       Redirect URIs set"

# ── 4. Create the service principal in this tenant ──────────────────────────
echo "[4/7] Creating service principal (home tenant)..."
az ad sp create --id "$APP_ID" -o none 2>/dev/null || echo "       (service principal already exists)"

# ── 5. Delegated API permissions ────────────────────────────────────────────
# Built as a single requiredResourceAccess manifest and applied in ONE
# `az ad app update` call. Adding scopes individually (az ad app permission add
# per scope) is far too slow — Fabric/Power BI publishes ~200 delegated scopes.
echo "[5/7] Adding delegated API permissions..."

# 5a. Resolve the resource service principal that serves the Fabric audience.
# Prefer an SP that advertises the fabric identifier URI; fall back to Power BI.
FABRIC_RESOURCE_APPID=$(az ad sp list --all \
  --filter "servicePrincipalNames/any(n:n eq '${FABRIC_RESOURCE_URI}')" \
  --query "[0].appId" -o tsv 2>/dev/null || true)
if [[ -z "${FABRIC_RESOURCE_APPID}" || "${FABRIC_RESOURCE_APPID}" == "None" ]]; then
  FABRIC_RESOURCE_APPID="$POWER_BI_APP_ID"
fi

# 5b. Enumerate every enabled delegated scope the resource publishes, so that a
# `.default` token acquisition (as fabio uses) resolves to the full set.
FABRIC_SCOPE_IDS=$(az ad sp show --id "$FABRIC_RESOURCE_APPID" \
  --query "oauth2PermissionScopes[?isEnabled].id" -o json 2>/dev/null || echo "[]")
FABRIC_SCOPE_COUNT=$(echo "$FABRIC_SCOPE_IDS" | jq 'length')

# 5c. Compose the manifest: Fabric/Power BI scopes + Microsoft Graph User.Read.
RRA_FILE="$(mktemp)"
trap 'rm -f "$RRA_FILE"' EXIT
echo "$FABRIC_SCOPE_IDS" | jq \
  --arg fabric "$FABRIC_RESOURCE_APPID" \
  --arg graph "$GRAPH_APP_ID" \
  --arg uread "$GRAPH_USER_READ" '
  [
    { resourceAppId: $fabric, resourceAccess: [ .[] | { id: ., type: "Scope" } ] },
    { resourceAppId: $graph,  resourceAccess: [ { id: $uread, type: "Scope" } ] }
  ]' > "$RRA_FILE"

az ad app update --id "$APP_ID" --required-resource-accesses @"$RRA_FILE" -o none
echo "       + Microsoft Graph: User.Read"
if [[ "$FABRIC_SCOPE_COUNT" -eq 0 ]]; then
  echo "       ! Could not enumerate Fabric delegated scopes for ${FABRIC_RESOURCE_APPID}."
  echo "         Add 'Power BI Service' delegated permissions manually in the portal."
else
  echo "       + Fabric/Power BI (${FABRIC_RESOURCE_APPID}): ${FABRIC_SCOPE_COUNT} delegated scopes"
fi

# ── 6. Optional admin consent (this tenant only) ────────────────────────────
if [[ "$ADMIN_CONSENT" -eq 1 ]]; then
  echo "[6/7] Granting admin consent in tenant $TENANT_ID..."
  # Consent can lag replication of the freshly-added permissions; retry briefly.
  for attempt in 1 2 3; do
    if az ad app permission admin-consent --id "$APP_ID" -o none 2>/dev/null; then
      echo "       Admin consent granted"
      break
    fi
    echo "       consent attempt $attempt failed (permissions may still be replicating); retrying..."
    sleep 10
  done
else
  echo "[6/7] Skipping admin consent (pass --admin-consent to grant, or consent on first sign-in)."
fi

# ── 7. Patch the source with the new default app ID ─────────────────────────
if [[ "$PATCH_SOURCE" -eq 1 && "$PRINT_ONLY" -eq 0 ]]; then
  echo "[7/7] Patching DEFAULT_PUBLIC_CLIENT_ID in src/token_cache.rs..."
  if [[ ! -f "$SRC_FILE" ]]; then
    echo "       ! Source file not found: $SRC_FILE (skipping patch)"
  else
    # Replace only the quoted value on the DEFAULT_PUBLIC_CLIENT_ID line.
    sed -i.bak -E \
      "s/(DEFAULT_PUBLIC_CLIENT_ID: &str = \")[^\"]+(\";)/\1${APP_ID}\2/" \
      "$SRC_FILE"
    rm -f "${SRC_FILE}.bak"
    if grep -q "DEFAULT_PUBLIC_CLIENT_ID: &str = \"${APP_ID}\";" "$SRC_FILE"; then
      echo "       Patched: DEFAULT_PUBLIC_CLIENT_ID = ${APP_ID}"
    else
      echo "       ! Patch did not apply — update src/token_cache.rs manually."
    fi
  fi
else
  echo "[7/7] Skipping source patch."
fi

# ── Summary ─────────────────────────────────────────────────────────────────
echo ""
echo "=== Done ==="
echo "App (client) ID : $APP_ID"
echo "Tenant ID       : $TENANT_ID"
echo "Audience        : AzureADMultipleOrgs (multitenant)"
echo ""
echo "Use it without rebuilding:"
echo "  export FABIO_CLIENT_ID=$APP_ID"
echo ""
echo "Then verify sign-in:"
echo "  fabio auth login --device-code"
echo ""
echo "To clean up later:"
echo "  az ad app delete --id $APP_ID"
