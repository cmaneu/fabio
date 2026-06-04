#!/usr/bin/env bash
# setup-ci-auth.sh — One-time setup for fabio GitHub Actions authentication
#
# Creates an Entra ID app registration with:
#   - Client secret (for env-var-based auth)
#   - OIDC federated credential (for secretless auth via azure/login)
#   - Fabric workspace Contributor role
#   - GitHub repo secrets
#
# Prerequisites:
#   - az CLI authenticated with permissions to create app registrations
#   - gh CLI authenticated with repo admin permissions
#   - fabio CLI authenticated (for workspace role assignment)
#
# Usage:
#   ./scripts/setup-ci-auth.sh <github-repo> <fabric-workspace-id>
#
# Example:
#   ./scripts/setup-ci-auth.sh iemejia/fabio d497e89d-5464-486e-9d14-9e64bc18b908

set -euo pipefail

REPO="${1:?Usage: $0 <github-repo> <fabric-workspace-id>}"
WORKSPACE_ID="${2:?Usage: $0 <github-repo> <fabric-workspace-id>}"
APP_NAME="${3:-fabio-ci-$(echo "$REPO" | tr '/' '-')}"

echo "=== Setup CI Auth for $REPO ==="
echo "App name:     $APP_NAME"
echo "Workspace ID: $WORKSPACE_ID"
echo ""

# 1. Get tenant and subscription info
TENANT_ID=$(az account show --query tenantId -o tsv)
SUBSCRIPTION_ID=$(az account show --query id -o tsv)
echo "[1/6] Tenant: $TENANT_ID"

# 2. Create app registration
echo "[2/6] Creating app registration..."
APP_ID=$(az ad app create --display-name "$APP_NAME" --query appId -o tsv)
echo "       App ID: $APP_ID"

# 3. Create service principal
echo "[3/6] Creating service principal..."
SP_ID=$(az ad sp create --id "$APP_ID" --query id -o tsv)
echo "       SP ID: $SP_ID"

# 4. Create client secret (1 year expiry)
echo "[4/6] Creating client secret..."
CLIENT_SECRET=$(az ad app credential reset --id "$APP_ID" --display-name "ci" --years 1 --query password -o tsv)
echo "       Secret created (expires in 1 year)"

# 5. Add OIDC federated credential for main branch
echo "[5/6] Adding OIDC federated credential..."
az ad app federated-credential create --id "$APP_ID" --parameters "{
  \"name\": \"github-main\",
  \"issuer\": \"https://token.actions.githubusercontent.com\",
  \"subject\": \"repo:${REPO}:ref:refs/heads/main\",
  \"audiences\": [\"api://AzureADTokenExchange\"],
  \"description\": \"GitHub Actions OIDC for ${REPO} main branch\"
}" -o none

echo "       Federated credential added (subject: repo:${REPO}:ref:refs/heads/main)"

# 6. Grant Fabric workspace access
echo "[6/6] Granting Contributor role on workspace..."
fabio workspace add-role-assignment \
  --id "$WORKSPACE_ID" \
  --principal-id "$SP_ID" \
  --principal-type ServicePrincipal \
  --role Contributor \
  --quiet

echo "       Contributor role granted"

# Set GitHub secrets
echo ""
echo "=== Setting GitHub secrets on $REPO ==="
gh secret set AZURE_CLIENT_ID --repo "$REPO" --body "$APP_ID"
gh secret set AZURE_TENANT_ID --repo "$REPO" --body "$TENANT_ID"
gh secret set AZURE_CLIENT_SECRET --repo "$REPO" --body "$CLIENT_SECRET"
echo "       Secrets set: AZURE_CLIENT_ID, AZURE_TENANT_ID, AZURE_CLIENT_SECRET"

echo ""
echo "=== Done ==="
echo ""
echo "Test with:"
echo "  gh workflow run test-auth.yml -f method=both --repo $REPO"
echo ""
echo "To clean up later:"
echo "  az ad app delete --id $APP_ID"
echo "  fabio workspace delete-role-assignment --id $WORKSPACE_ID --principal-id $SP_ID"
