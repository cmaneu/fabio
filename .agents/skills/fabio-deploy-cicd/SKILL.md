---
name: fabio-deploy-cicd
description: >-
  Intent-scoped fabio skill for Fabric CI/CD: stateless content-hash deploy (export/validate/plan/apply), Git integration, deployment pipelines, and variable libraries for environment-specific config. Use for promoting Fabric items between environments, Git lifecycle, and parameterized deployments. Triggers: "deploy", "ci/cd", "promote to production", "deploy plan", "deploy apply", "git commit", "git pull", "deployment pipeline", "variable library", "value set".
license: MIT
---

# fabio-deploy-cicd — Deploy & CI/CD — stateless content-hash deployment, Git, pipelines, variable libraries

> **Generated file — do not edit by hand.** This intent-scoped sub-skill of the `fabio` skill is generated from fabio's command schema plus authored judgment. Regenerate with `cargo test generate_subskills -- --ignored`. For install, auth, output envelope, global flags, and agent-safety rules, see the root `fabio` skill.

> **Prefer runtime introspection.** This index is a snapshot; the installed binary is always authoritative. Use `fabio context agent --group <group>` and `fabio context describe <group> <command>` for exact flags and output shapes.

## When to use
- Exporting a workspace to disk and deploying it to another environment.
- Planning (dry-run diff) before applying changes; converging idempotently.
- Git integration: connect, status, commit, pull, checkout, branch-out.
- Managing deployment pipelines (dev/test/prod stages).
- Managing variable libraries and activating environment value sets.

## When NOT to use (route elsewhere)
- Porting from Synapse/Databricks/HDInsight -> use the migration-engineer persona + migration workflows.
- One-off item CRUD -> use the specific workload skill (fabio-lakehouse, fabio-rti-kql, etc.).

## Command index

Generated from fabio's command schema. For full flag details use `fabio context agent --group <group>` or `fabio context describe <group> <command>`.

### fabio deploy
Deploy item definitions from a local directory to a workspace

| Command | Mutates | Description |
|---|---|---|
| `fabio deploy apply` | yes | Execute deployment (create/update/delete items) |
| `fabio deploy export` | no | Export workspace item definitions to a local directory |
| `fabio deploy init-params` | no | Generate a parameters.json scaffold by scanning or diffing exported definitions |
| `fabio deploy plan` | no | Preview what would be deployed (create/update/delete/skip) |
| `fabio deploy validate` | no | Validate source directory locally (no API calls). Checks .platform files, item types, duplicate names/logical IDs, cross-references, and parameters |

### fabio git
Manage Git integration (connect, commit, pull, status)

| Command | Mutates | Description |
|---|---|---|
| `fabio git branch-out` | yes | Create a feature workspace from the current branch (branch out) |
| `fabio git checkout` | yes | Switch to a different branch (disconnect + connect + init) |
| `fabio git commit` | yes | Commit workspace changes to the connected remote branch |
| `fabio git connect` | yes | Connect a workspace to a Git repository |
| `fabio git connection` | no | Show or manage Git connection and credentials |
| `fabio git credentials` | no | Manage Git credentials |
| `fabio git disconnect` | yes | Disconnect a workspace from Git |
| `fabio git init` | yes | Initialize a workspace Git connection (required after connect) |
| `fabio git pull` | yes | Pull remote changes into the workspace (update from Git) |
| `fabio git show-tracked` | no | Show tracked items and their Git sync status |
| `fabio git status` | no | Show workspace Git status (changes, conflicts) |

### fabio deployment-pipeline
Manage deployment pipelines (CI/CD stages, deploy items)

| Command | Mutates | Description |
|---|---|---|
| `fabio deployment-pipeline add-role-assignment` | yes | Add a role assignment to a deployment pipeline |
| `fabio deployment-pipeline assign-workspace` | yes | Assign a workspace to a deployment pipeline stage |
| `fabio deployment-pipeline create` | yes | Create a new deployment pipeline |
| `fabio deployment-pipeline delete` | yes | Delete a deployment pipeline |
| `fabio deployment-pipeline delete-role-assignment` | yes | Delete a role assignment from a deployment pipeline |
| `fabio deployment-pipeline deploy` | yes | Deploy items from one stage to another |
| `fabio deployment-pipeline list` | no | List deployment pipelines |
| `fabio deployment-pipeline list-operations` | no | List deploy operations for a deployment pipeline |
| `fabio deployment-pipeline list-role-assignments` | no | List role assignments for a deployment pipeline |
| `fabio deployment-pipeline list-stage-items` | no | List items in a deployment pipeline stage |
| `fabio deployment-pipeline list-stages` | no | List stages in a deployment pipeline |
| `fabio deployment-pipeline show` | no | Show details of a deployment pipeline |
| `fabio deployment-pipeline show-operation` | no | Show details of a deploy operation |
| `fabio deployment-pipeline show-stage` | no | Show details of a deployment pipeline stage |
| `fabio deployment-pipeline unassign-workspace` | yes | Unassign the workspace from a deployment pipeline stage |
| `fabio deployment-pipeline update` | yes | Update a deployment pipeline |
| `fabio deployment-pipeline update-stage` | yes | Update a deployment pipeline stage configuration |

### fabio variable-library
Manage variable libraries (shared variables)

| Command | Mutates | Description |
|---|---|---|
| `fabio variable-library activate-value-set` | yes | Activate a value set for a variable library in a workspace |
| `fabio variable-library create` | yes | Create a new variable library |
| `fabio variable-library delete` | yes | Delete a variable library |
| `fabio variable-library get-definition` | no | Get the definition of a variable library |
| `fabio variable-library list` | no | List variable librarys in a workspace |
| `fabio variable-library list-value-sets` | no | List value sets defined in a variable library |
| `fabio variable-library show` | no | Show details of a variable library |
| `fabio variable-library update` | yes | Update variable library properties |
| `fabio variable-library update-definition` | yes | Update the definition of a variable library |

## Key gotchas
- Deploy is STATELESS — content-hash diffing against the live workspace, no state file. --workspace accepts a display name or GUID.
- Always 'deploy plan' (dry-run) and review before 'deploy apply'.
- Source format is fabric Git Integration '.platform' directories (100% fabric-cicd compatible).
- --strategy default (per-item, content-hash skip) | bulk (fast initial deploy to empty workspace) | sequential (debugging).
- Name value sets to match --env values — 'deploy apply --env prod' auto-activates the 'prod' value set as a post-hook.

## Safety
- --force-all overwrites ALL matched items regardless of content changes — irreversible; run 'deploy plan' first.
- --delete-orphans removes workspace items not in source; protected data types (Lakehouse/Warehouse/SQLDatabase/Eventhouse/KQLDatabase) require --allow-delete-types.
- Deploy output includes a 'destructive' boolean — surface it to the human before applying.

## See also
- fabio context persona migration-engineer
- fabio context workflow cicd-deploy
- fabio context best-practices deploy-parameters
