---
title: Build a Fabric CI/CD pipeline
description: Build a two-environment GitHub Actions pipeline with isolated PR workspaces, Python checks, preview links, and automatic production deployment.
---

In this tutorial you will put a small Microsoft Fabric project under continuous
integration and deployment. Every pull request is checked and deployed to its own
temporary workspace on a development capacity. After the checks pass, the workflow
adds a link to that workspace to the pull request. Merging to `main` deploys the
same definitions to a protected production workspace.

You will use the
[complete sample repository layout](https://github.com/iemejia/fabio/tree/main/samples/cicd),
which contains a Lakehouse, a PySpark notebook, a Direct Lake semantic model, a
Power BI report, tests, and the GitHub Actions workflow.

## What you will build

```text
feature branch ──pull request──► validate + Ruff + unit tests
                                      │
                                      ▼
                               temporary PR workspace
                                      │
                                      └──► PR comment with Fabric link

main ───────────────push───────► plan + deploy + verify
                                      │
                                      ▼
                               production workspace
```

The project uses three kinds of workspaces:

1. A persistent **development** workspace for authoring and validating changes.
2. A persistent **production** workspace deployed from `main`.
3. One short-lived **preview** workspace per open pull request.

Only definitions are stored in Git. The notebook creates the sample data after
the Lakehouse is deployed.

## Prerequisites

You need:

- A Microsoft Fabric tenant and a development capacity that can host preview workspaces.
- Permission to create Fabric workspaces and assign them to that capacity.
- A production capacity and permission to administer the production workspace.
- A GitHub repository with Actions enabled.
- An Entra application configured for
  [GitHub workload identity federation](guides/authentication/#workload-identity-federation-github-actions-oidc).
- Fabio, Python 3.11 or later, Git, and the GitHub CLI installed locally.

For a same-repository pull request, the workflow can use OIDC and comment on the
PR without storing a client secret. Pull requests from forks run the offline
validation job only because GitHub does not grant them deployment credentials.

## 1. Start from the sample

Copy the contents of `samples/cicd` into the root of a new repository:

```text
.
├── .github/workflows/fabric-cicd.yml
├── fabric-items/
│   ├── LoadSales.Notebook/
│   ├── SalesLakehouse.Lakehouse/
│   ├── SalesModel.SemanticModel/
│   └── SalesReport.Report/
├── tests/
├── parameters.json
├── pyproject.toml
└── requirements-dev.txt
```

The `.platform` files give every Fabric item a stable logical ID. Fabio uses
those IDs to bind the notebook to the deployed Lakehouse and the report to the
deployed semantic model, even when the target workspace starts empty.

Install the development dependency and run the checks:

```bash
python -m pip install -r requirements-dev.txt
ruff check fabric-items tests
python -m unittest discover -s tests
fabio deploy validate --source ./fabric-items \
  --parameters ./parameters.json --env dev
fabio report validate --source ./fabric-items/SalesReport.Report
```

The Python test parses the notebook source and checks that it writes the expected
`sales` table. `deploy validate` and `report validate` are offline, so these
checks do not need Fabric credentials.

## 2. Create development and production workspaces

List the capacities available to you:

```bash
fabio capacity list -o table
```

Create two persistent workspaces. Replace the capacity IDs with values from the
previous command:

```bash
fabio workspace create \
  --name "sales-cicd-dev" \
  --capacity-id "<development-capacity-id>"

fabio workspace create \
  --name "sales-cicd-prod" \
  --capacity-id "<production-capacity-id>"
```

Save each `data.id` value. You will use the production ID in GitHub. The preview
workflow uses the development capacity ID to create temporary workspaces named
`<repository>-pr-<number>`.

Deploy to the development workspace once to prove the source works. The first
apply creates and runs the data-loading items; the second creates the Direct Lake
model and report after the `sales` table exists:

```bash
fabio deploy plan --source ./fabric-items \
  --workspace "<development-workspace-id>" \
  --parameters ./parameters.json --env dev

fabio deploy apply --source ./fabric-items \
  --workspace "<development-workspace-id>" \
  --parameters ./parameters.json --env dev \
  --item-types Lakehouse,Notebook --no-post-hooks

fabio notebook list --workspace "<development-workspace-id>" -o table
fabio notebook run --workspace "<development-workspace-id>" \
  --id "<LoadSales-notebook-id>" --wait --timeout 1200

fabio deploy apply --source ./fabric-items \
  --workspace "<development-workspace-id>" \
  --parameters ./parameters.json --env dev --verify
```

Review `data.summary` before each apply. The final response's
`data.verification.converged` should be `true`.

## 3. Configure passwordless GitHub authentication

Enable service-principal access to Fabric APIs in the tenant and grant the Entra
application only the permissions it needs:

- Permission to create preview workspaces and assign the development capacity.
- Contributor access to the production workspace.

Create these GitHub environments under **Settings → Environments**:

| Environment | Variables |
|---|---|
| `preview` | `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `FABRIC_DEV_CAPACITY_ID` |
| `production` | `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `FABRIC_PROD_WORKSPACE_ID` |

The IDs are configuration, not passwords, so the sample reads them from GitHub
environment variables. Add required reviewers to the `production` environment
if a human must approve each release.

Add one federated credential to the Entra application for each environment. Use
these subject identifiers, replacing the placeholders:

```text
repo:<owner>/<repository>:environment:preview
repo:<owner>/<repository>:environment:production
```

Use audience `api://AzureADTokenExchange`. The workflow requests a short-lived
OIDC token and passes it to `fabio auth login --federated-token`; no client secret
is committed or stored.

## 4. Understand the workflow

The sample's `.github/workflows/fabric-cicd.yml` has four jobs.

### Validate source changes

For an open PR or a push to `main`, `validate`:

1. Installs the pinned Ruff version and Fabio.
2. Lints the notebook and test source.
3. Runs the Python unit tests.
4. Runs offline Fabric and PBIR validation.

This job has only `contents: read` permission and does not authenticate to Fabric.

### Create and deploy a PR preview

After validation, `preview`:

1. Authenticates with the `preview` environment's OIDC identity.
2. Deletes an older workspace for the same PR, if present.
3. Creates a clean workspace on `FABRIC_DEV_CAPACITY_ID`.
4. Runs `deploy plan`.
5. Deploys the Lakehouse and notebook, then runs `LoadSales`.
6. Deploys the semantic model and report and verifies convergence.
7. Creates or updates a PR comment with the workspace URL.

Recreating the workspace prevents an item deleted from the branch from surviving
in an older preview. Workflow concurrency allows only the newest commit for a PR
to deploy.

### Delete the preview

When the PR closes, `cleanup-preview` deletes the workspace with that PR's exact
name and updates the existing comment. The job does not enumerate or delete
unrelated workspaces.

### Deploy production

When a commit reaches `main`, `production` repeats the plan, data load, complete
deployment, and convergence verification against `FABRIC_PROD_WORKSPACE_ID`.
GitHub environment protection can pause this job for approval.

## 5. Open a pull request

Commit the sample and push a feature branch:

```bash
git add .
git commit -m "feat: add sales report"
git push -u origin feature/sales-report
gh pr create --fill
```

Open the Actions run and confirm:

- Ruff, unit tests, `deploy validate`, and `report validate` pass.
- The deployment plan contains the four expected items.
- The `LoadSales` notebook completes.
- The final apply reports convergence.
- The PR receives an **open the PR workspace** link.

Follow the link and inspect the Lakehouse, notebook, semantic model, and report.
Push another commit to see the workflow replace the preview with a clean deployment.

## 6. Merge and verify production

Require the `validate` and `preview` jobs in the `main` branch ruleset. Merge the
PR after both pass. The push to `main` starts the production job automatically.

After it completes:

```bash
fabio deploy plan --source ./fabric-items \
  --workspace "<production-workspace-id>" \
  --parameters ./parameters.json --env production
```

A converged deployment has zero creates, updates, or deletes. Closing or merging
the PR also starts `cleanup-preview`, so only the persistent development and
production workspaces remain.

## Next steps

- Add production environment reviewers and restrict deployments to `main`.
- Replace the sample rows with your own ingestion logic and add notebook tests.
- Add environment-specific values to `parameters.json` or a Fabric Variable Library.
- Read the [GitHub Actions deployment guide](guides/github-actions-cicd/) for
  branch-per-environment promotion, rollback, and hardening options.
- Read the [`deploy` command reference](reference/commands/deploy/) for filters,
  saved plans, and deployment strategies.
