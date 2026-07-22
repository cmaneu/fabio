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
#   - Delegated permissions (the MINIMAL set covering fabio's 6 token audiences)
#
# Permission model (why these scopes):
#   fabio acquires access tokens for SIX resources, each via `<resource>/.default`.
#   For an interactive PUBLIC client, `.default` (and the non-interactive
#   refresh-token exchange fabio uses for the non-Fabric audiences) only issues a
#   token if the app has a CONSENTED delegated permission on that resource. So the
#   app must carry one delegated permission per audience:
#
#     1. Power BI Service  (api.fabric.microsoft.com) — Fabric REST + Power BI REST.
#        Fabric authorizes calls by the user's workspace/tenant role, NOT by the
#        granular scope claim, so a small COARSE set covers the whole CLI surface
#        (rather than all ~200 published scopes).
#     2. Azure Storage     (storage.azure.com)        — OneLake DFS/Blob  → user_impersonation
#     3. Azure SQL DB      (database.windows.net)     — TDS (warehouse/sql) → user_impersonation
#     4. Azure Resource Mgmt (management.azure.com)   — capacity ARM ops   → user_impersonation
#     5. Azure Data Explorer (*.kusto.fabric...)      — KQL query/mgmt     → user_impersonation
#     6. Microsoft Graph   (graph.microsoft.com)      — sign-in + label list
#
#   Scope GUIDs are resolved at RUNTIME by name from each resource service
#   principal (portable across tenants/clouds); missing resource SPs are
#   auto-provisioned, and any allow-listed name the resource doesn't publish is
#   reported so drift surfaces immediately.
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

# Well-known first-party resource identifiers (stable across tenants/clouds).
# Delegated scope GUIDs are resolved by NAME at runtime, so only appIds are fixed.
GRAPH_APP_ID="00000003-0000-0000-c000-000000000000"        # Microsoft Graph
POWER_BI_APP_ID="00000009-0000-0000-c000-000000000000"     # Power BI Service (serves api.fabric.microsoft.com)
STORAGE_APP_ID="e406a681-f3d4-42a8-90b6-c2b029497af1"      # Azure Storage (OneLake)
SQL_APP_ID="022907d3-0f1b-48f7-badc-1ba6abab6d66"          # Azure SQL Database (TDS)
ARM_APP_ID="797f4846-ba00-4fd7-ba43-dac1f8f63013"          # Azure Resource Manager
KUSTO_APP_ID="2746ea77-4702-4b45-80ca-3c97e680e8b7"        # Azure Data Explorer (Kusto / KQL)
FABRIC_RESOURCE_URI="https://api.fabric.microsoft.com"

# Curated COARSE Power BI/Fabric delegated scopes that cover the full fabio CLI
# surface. Resolved to GUIDs at runtime by matching these values. Intentionally
# minimal vs. the ~200 scopes the resource publishes.
FABRIC_SCOPE_VALUES=(
  # ── Fabric item/workspace/tenant plane ──
  "Workspace.ReadWrite.All"       # workspaces + generic item management
  "Item.ReadWrite.All"            # most Fabric item types (CRUD + definitions)
  "Item.Execute.All"              # run notebooks / jobs / pipelines
  "Item.Reshare.All"              # sharing / role assignment
  "Capacity.ReadWrite.All"        # capacity assignment
  "Connection.ReadWrite.All"      # connections
  "Gateway.ReadWrite.All"         # gateways / managed private endpoints
  "OneLake.ReadWrite.All"         # OneLake data plane via the Fabric audience
  "Tenant.ReadWrite.All"          # admin plane: admin list, tenant settings, domains
  # ── Power BI data plane (BI commands + Power BI REST passthrough) ──
  "Dataset.ReadWrite.All"         # semantic models
  "Report.ReadWrite.All"          # reports
  "PaginatedReport.ReadWrite.All" # paginated reports
  "Dashboard.ReadWrite.All"       # dashboards
  "Dataflow.ReadWrite.All"        # dataflows Gen2 / datamarts
)

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
# `az ad app update` call. Scope GUIDs are resolved by NAME from each resource
# service principal so the script is portable across tenants and clouds.
echo "[5/7] Adding delegated API permissions..."

# Ensure a first-party resource SP exists in this tenant, then emit a JSON array
# [ {id,type:"Scope"}, ... ] for the requested delegated scope VALUES (resolved
# to GUIDs by name). Warns (on stderr) for any requested value the resource does
# not publish. Avoids bash-4 `mapfile` and empty-array pitfalls for portability.
resource_access_json() {
  local appid="$1"; shift

  # Provision the resource SP if absent (needed to read its published scopes).
  az ad sp show --id "$appid" -o none 2>/dev/null \
    || az ad sp create --id "$appid" -o none 2>/dev/null \
    || true

  local scopes_json
  scopes_json=$(az ad sp show --id "$appid" \
    --query "oauth2PermissionScopes[?isEnabled].{value:value,id:id}" -o json 2>/dev/null || echo "[]")

  local out="[]" v id
  for v in "$@"; do
    id=$(echo "$scopes_json" | jq -r --arg v "$v" 'first(.[] | select(.value==$v) | .id) // empty')
    if [[ -z "$id" ]]; then
      echo "       ! WARNING: '$v' not published by resource $appid (skipped)" >&2
    else
      out=$(echo "$out" | jq --arg id "$id" '. + [ { id: $id, type: "Scope" } ]')
    fi
  done
  echo "$out"
}

# 5a. Resolve the resource SP that serves the Fabric audience (fall back to the
# well-known Power BI Service appId if the identifier URI can't be resolved).
FABRIC_RESOURCE_APPID=$(az ad sp list --all \
  --filter "servicePrincipalNames/any(n:n eq '${FABRIC_RESOURCE_URI}')" \
  --query "[0].appId" -o tsv 2>/dev/null || true)
if [[ -z "${FABRIC_RESOURCE_APPID}" || "${FABRIC_RESOURCE_APPID}" == "None" ]]; then
  FABRIC_RESOURCE_APPID="$POWER_BI_APP_ID"
fi

# 5b. Resolve the curated per-resource delegated scopes to requiredResourceAccess.
FABRIC_ACCESS=$(resource_access_json "$FABRIC_RESOURCE_APPID" "${FABRIC_SCOPE_VALUES[@]}")
STORAGE_ACCESS=$(resource_access_json "$STORAGE_APP_ID" "user_impersonation")
SQL_ACCESS=$(resource_access_json "$SQL_APP_ID" "user_impersonation")
ARM_ACCESS=$(resource_access_json "$ARM_APP_ID" "user_impersonation")
KUSTO_ACCESS=$(resource_access_json "$KUSTO_APP_ID" "user_impersonation")
GRAPH_ACCESS=$(resource_access_json "$GRAPH_APP_ID" "User.Read" "InformationProtectionPolicy.Read")

FABRIC_SCOPE_COUNT=$(echo "$FABRIC_ACCESS" | jq 'length')

# 5c. Compose the full requiredResourceAccess manifest (6 resources) and apply it
# in ONE `az ad app update` call. Resources that resolved no scopes are dropped.
RRA_FILE="$(mktemp)"
trap 'rm -f "$RRA_FILE"' EXIT
jq -n \
  --arg fabric  "$FABRIC_RESOURCE_APPID" \
  --arg storage "$STORAGE_APP_ID" \
  --arg sql     "$SQL_APP_ID" \
  --arg arm     "$ARM_APP_ID" \
  --arg kusto   "$KUSTO_APP_ID" \
  --arg graph   "$GRAPH_APP_ID" \
  --argjson fabricAccess  "$FABRIC_ACCESS" \
  --argjson storageAccess "$STORAGE_ACCESS" \
  --argjson sqlAccess     "$SQL_ACCESS" \
  --argjson armAccess     "$ARM_ACCESS" \
  --argjson kustoAccess   "$KUSTO_ACCESS" \
  --argjson graphAccess   "$GRAPH_ACCESS" '
  [
    { resourceAppId: $fabric,  resourceAccess: $fabricAccess  },
    { resourceAppId: $storage, resourceAccess: $storageAccess },
    { resourceAppId: $sql,     resourceAccess: $sqlAccess     },
    { resourceAppId: $arm,     resourceAccess: $armAccess     },
    { resourceAppId: $kusto,   resourceAccess: $kustoAccess   },
    { resourceAppId: $graph,   resourceAccess: $graphAccess   }
  ]
  | map(select((.resourceAccess | length) > 0))' > "$RRA_FILE"

az ad app update --id "$APP_ID" --required-resource-accesses @"$RRA_FILE" -o none

echo "       + Fabric/Power BI (${FABRIC_RESOURCE_APPID}): ${FABRIC_SCOPE_COUNT} delegated scopes"
echo "       + Azure Storage:        user_impersonation (OneLake)"
echo "       + Azure SQL Database:   user_impersonation (TDS)"
echo "       + Azure Resource Mgmt:  user_impersonation (capacity ARM ops)"
echo "       + Azure Data Explorer:  user_impersonation (KQL / Kusto)"
echo "       + Microsoft Graph:      User.Read, InformationProtectionPolicy.Read"
if [[ "$FABRIC_SCOPE_COUNT" -eq 0 ]]; then
  echo "       ! Could not resolve Fabric delegated scopes for ${FABRIC_RESOURCE_APPID}."
  echo "         Add 'Power BI Service' delegated permissions manually in the portal."
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
