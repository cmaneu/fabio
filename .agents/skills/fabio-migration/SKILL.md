---
name: fabio-migration
description: >-
  Intent-scoped fabio skill for migrating existing analytics workloads to Microsoft Fabric from Azure Synapse, Databricks, HDInsight, and Azure Data Factory. Covers assessment, API/path translation, target setup, deploy, and parity validation. Use when porting notebooks, jobs, or pipelines onto Fabric. Triggers: "migrate to fabric", "synapse to fabric", "databricks to fabric", "hdinsight to fabric", "port notebooks", "mssparkutils", "dbutils", "unity catalog", "dbfs", "linked service".
license: MIT
---

# fabio-migration — Migration — port Synapse, Databricks, HDInsight, and ADF to Fabric

> **Generated file — do not edit by hand.** This intent-scoped sub-skill of the `fabio` skill is generated from fabio's command schema plus authored judgment. Regenerate with `cargo test generate_subskills -- --ignored`. For install, auth, output envelope, global flags, and agent-safety rules, see the root `fabio` skill.

> **Prefer runtime introspection.** This index is a snapshot; the installed binary is always authoritative. Use `fabio context agent --group <group>` and `fabio context describe <group> <command>` for exact flags and output shapes.

## When to use
- Assessing a source workspace and mapping its artifacts to Fabric items.
- Translating utility APIs and storage paths (see best-practices migration-api-shims).
- Setting up the target environment (workspace, lakehouse, connections, variable libraries).
- Deploying migrated artifacts via stateless content-hash CI/CD and validating parity.
- Follow the workflow for the specific source: synapse-migration, databricks-migration, hdinsight-migration, pipeline-migration.

## When NOT to use (route elsewhere)
- Greenfield build with no source system -> use the data-engineer persona.
- Ongoing CI/CD of already-migrated items -> use fabio-deploy-cicd.

## Command index

Generated from fabio's command schema. For full flag details use `fabio context agent --group <group>` or `fabio context describe <group> <command>`.

### fabio connection
Manage connections (cloud, on-premises, virtual network)

| Command | Mutates | Description |
|---|---|---|
| `fabio connection add-role-assignment` | yes | Add a role assignment to a connection |
| `fabio connection create` | yes | Create a new connection |
| `fabio connection delete` | yes | Delete a connection |
| `fabio connection delete-role-assignment` | yes | Delete a role assignment from a connection |
| `fabio connection list` | no | List all connections you have permission to access |
| `fabio connection list-role-assignments` | no | List role assignments for a connection |
| `fabio connection list-supported-types` | no | List supported connection types (gateway types catalog) |
| `fabio connection show` | no | Show details of a specific connection |
| `fabio connection show-role-assignment` | no | Show a specific role assignment for a connection |
| `fabio connection test-connection` | no | Test a connection (not supported for `StreamingVirtualNetworkGateway` connections) |
| `fabio connection update` | yes | Update a connection's name, credentials, or privacy level |
| `fabio connection update-role-assignment` | yes | Update a role assignment for a connection |

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

### fabio deploy
Deploy item definitions from a local directory to a workspace

| Command | Mutates | Description |
|---|---|---|
| `fabio deploy apply` | yes | Execute deployment (create/update/delete items) |
| `fabio deploy export` | no | Export workspace item definitions to a local directory |
| `fabio deploy init-params` | no | Generate a parameters.json scaffold by scanning or diffing exported definitions |
| `fabio deploy plan` | no | Preview what would be deployed (create/update/delete/skip) |
| `fabio deploy validate` | no | Validate source directory locally (no API calls). Checks .platform files, item types, duplicate names/logical IDs, cross-references, and parameters |

## Key gotchas
- mssparkutils.* / dbutils.* -> notebookutils.* (same method surface).
- DBFS / WASB / ADLS Gen1/Gen2 / S3 paths -> OneLake abfss paths, or a Lakehouse shortcut to keep data in place.
- Unity Catalog / Hive metastore tables -> Lakehouse Delta tables (Delta is cross-compatible — often re-point rather than re-write).
- Linked Services -> Fabric Connections; ADF global parameters -> Variable Library value sets; Oozie/ADF triggers -> job schedules.
- Spark pool / cluster sizing does NOT migrate — Fabric manages compute per capacity.

## Safety
- STOP after assessment: present feature-parity gaps and get user sign-off on the target architecture before writing code.
- Do NOT decommission source systems until parity validation passes.
- Deploy migrated items with 'deploy plan' (dry-run) reviewed before 'deploy apply'.

## See also
- fabio context persona migration-engineer
- fabio context best-practices migration-api-shims
- fabio context workflow synapse-migration
- fabio context workflow databricks-migration
- fabio context workflow hdinsight-migration
- fabio context workflow pipeline-migration
