# Fabio CLI - Session Context

## Goal
- Design and implement an agent-first CLI (`fabio`) to manage Microsoft Fabric artifacts and data, inspired by AWS/gcloud/Azure CLI principles, with structured JSON output, composability via stdin/stdout, and machine-readable errors.

## Agent-Native CLI Principles

Fabio must always respect these 10 principles for agent-native CLIs:
https://trevinsays.com/p/10-principles-for-agent-native-clis

1. **Non-interactive by default** — No prompts; all inputs via flags/env/files. Non-TTY must fail fast.
2. **Structured, parseable output** — `--json` on every command; stdout = data, stderr = diagnostics; stable exit codes.
3. **Errors that teach and enumerate** — Errors include valid enum values, corrected command examples, and machine-readable codes with hints.
4. **Safe retries and explicit mutation boundaries** — `--dry-run` for mutations; idempotency-safe; stable returned IDs.
5. **Bounded responses** — `--limit` for list commands; default to concise output; truncation metadata in envelope.
6. **Cross-CLI vocabulary consistency** — Canonical agent verbs: `list`, `show`, `create`, `delete`, `copy`, `move`.
7. **Three-layer introspection** — `fabio agent-context` provides machine-readable command schema (flags, types, mutability, examples).
8. **Async-aware execution** — `--wait` for async jobs; local job ledger (`fabio jobs list/get/prune`); status polling.
9. **Persistent identity through profiles** — Named profiles (`fabio profile save/use/list/show/delete`); `--profile` flag.
10. **Two-way I/O** — Feedback channel (`fabio feedback send/list`); artifact delivery via stdout/file.

## Constraints & Preferences
- **Windows-first compatibility** — All code must work on Windows. Use `Path::new().join()` (never hardcoded `/` for filesystem paths), `dirs::home_dir()` (never manual `HOME`/`USERPROFILE`), `.lines()` for text parsing (handles CRLF), no Unix-specific APIs. `.gitattributes` enforces LF line endings.
- **Throttling reduction** — Reduce the likelihood of API throttling by:
  - Use bulk and batch operations when available (e.g., `item bulk-create`, `item bulk-delete`, workspace role batch-assign, domain batch-assign).
  - Prefer list APIs over repeated single-resource requests (e.g., use a single list call + client-side filter rather than N individual show calls).
- CLI designed for AI agents first (structured output, no interactive prompts, explicit params)
- JSON output by default with `--output json|table|plain` flag
- Composable: manage inputs and produce outputs for piping
- Machine-readable error codes in structured JSON envelope
- Rust (edition 2024, rust-version 1.85), uses clap derive, tokio, reqwest, azure_identity, serde, comfy-table, thiserror/anyhow
- Linting: clippy pedantic+nursery (zero warnings), rustfmt
- CI: GitHub Actions (cargo fmt, clippy, test, build release) on ubuntu/macos/windows
- Installable via `cargo install --git https://github.com/iemejia/fabio.git`

## Progress
### Done
- **Full Rust implementation** (265 subcommands across 37 groups): auth, workspace, item, lakehouse, capacity, notebook, warehouse, data-agent, ontology, environment, data-pipeline, copy-job, dataflow, report, semantic-model, eventhouse, eventstream, kql-database, kql-queryset, kql-dashboard, mirrored-database, reflex, ml-model, ml-experiment, spark, spark-job-definition, graphql-api, git, connection, deployment-pipeline, domain, job-scheduler, onelake-security, managed-private-endpoint, profile, jobs, feedback + agent-context
- Core output system: JSON envelope (`{"data":..., "count":N}` or `{"error":{"code":...,"message":...}}`), table, plain formats
- Structured error system: `ErrorCode` enum (AUTH_REQUIRED, NOT_FOUND, RATE_LIMITED, CAPACITY_INACTIVE, API_ERROR, TIMEOUT, etc.) + `FabioError`
- Global options fully wired: `--output/-o`, `--query/-q` (dot-notation field extraction), `--quiet` (suppresses stdout), `--profile`, `--dry-run`, `--limit`, `--all`, `--continuation-token`
- HTTP client: async get/post/put/patch/delete with LRO polling (`Location` + `x-ms-operation-id` + resource follow)
- OneLake operations: DFS upload (create+append+flush), download, file listing; Blob API copy (server-side async)
- **Parallel file/table operations**: Upload, copy, move support glob patterns with concurrent execution and rate-limit retry
- **Sync command**: `lakehouse sync` copies new/modified files between lakehouses using ETag/MD5 comparison
- **LRO polling**: 2s interval, 120s max, handles 200/202, checks `status` field until Succeeded/Failed
- **Server-side file copy/move**: Blob API `PUT` with `x-ms-copy-source`, move = copy + delete
- **Server-side table copy/move/delete**: Root listing + prefix filter, per-file Blob copy, recursive DFS delete
- **Shortcuts**: Create/get/delete OneLake, ADLS Gen2, S3 shortcuts
- **Notebook run**: Captures job instance ID from Location header, status/stop via Jobs API
- **Notebook `--wait` flag**: Polls job status every 5s until Completed/Failed/Cancelled, with configurable `--timeout` (default 600s)
- **Item copy/move**: getDefinition LRO + create in dest workspace LRO; move = copy + delete source
- **Warehouse**: list/show/create/update/delete/query (endpoint resolved, stdin/file/flag SQL input)
- **Git integration**: status, commit, pull, connect, disconnect, initialize, switch (branch), connection/credentials management, show-tracked
- **Ontology management**: list, show, create, update, delete, get-definition, update-definition (RDF file support)
- **Environment**: list, show, create, update, delete, publish, cancel-publish, get-spark-settings, get-staging-spark-settings
- **Data Pipeline**: list, show, create, update, delete, run (triggers Pipeline job)
- **Eventhouse**: list, show, create, update, delete
- **Eventstream**: list, show, create, update, delete, get-definition, update-definition
- **KQL Database**: list, show, create, update, delete, get-definition, update-definition (ReadWrite/ReadOnlyFollowing)
- **KQL Queryset**: list, show, create, update, delete, get-definition, update-definition, run (executes saved query tabs against configured data source)
- **KQL Dashboard**: list, show, create, update, delete, get-definition, update-definition (RealTimeDashboard.json)
- **Mirrored Database**: list, show, create, update, delete, get/update-definition, start, stop, status, table-status
- **Reflex**: list, show, create, update, delete, get-definition, update-definition (Data Activator triggers)
- **ML Model**: list, show, create, update, delete (CRUD only, no definition support)
- **ML Experiment**: list, show, create, update, delete (CRUD only, no definition support)
- **Copy Job**: list, show, create, update, delete, get-definition, update-definition (data movement)
- **Dataflow**: list, show, create, update, delete, get-definition, update-definition (Power BI transformation)
- **GraphQL API**: list, show, create, update, delete, get-definition, update-definition (schema.graphql)
- **Report**: list, show, create (from definition file), update, delete, get-definition, update-definition
- **Semantic Model**: list, show, create (from model.bim), update, delete, get-definition, update-definition
- **Spark Job Definition**: list, show, create, update, delete, get-definition, update-definition, run
- **Capacity**: list, show (inspect available capacities)
- **Connection**: list, show, create, update, delete, list-supported-types
- **Deployment Pipeline**: list, show, create, update, delete, list-stages, list-stage-items, assign-workspace, unassign-workspace, deploy
- **Domain**: list, show, create, update, delete, list-workspaces, assign-workspaces, unassign-workspaces, assign-by-capacity, assign-by-principal
- **Job Scheduler**: list-instances, get-instance, run-on-demand, cancel-instance, list-schedules, get-schedule, create-schedule, update-schedule, delete-schedule
- **Spark**: get-settings, update-settings, list-pools, get-pool, create-pool, update-pool, delete-pool
- **OneLake Security**: list, show, upsert, delete (data access roles for row/column-level security)
- **Managed Private Endpoint**: list, show, create, delete (workspace private networking)
- **Pagination**: `--all` fetches all pages, `--continuation-token` resumes from a specific token, `--limit` truncates client-side
- **Agent-native compliance** (all 10 principles implemented):
  - Principle 1: Non-interactive by default
  - Principle 2: Structured parseable output
  - Principle 3: Errors that teach and enumerate
  - Principle 4: Safe retries (`--dry-run`)
  - Principle 5: Bounded responses (`--limit`, `--continuation-token`, truncation metadata)
  - Principle 6: Consistent vocabulary (list/show/create/delete/copy/move)
  - Principle 7: `fabio agent-context` machine-readable schema
  - Principle 8: Async-aware (`--wait`, jobs ledger)
  - Principle 9: Named profiles (`fabio profile save/use/list/show/delete`)
  - Principle 10: Two-way I/O (`fabio feedback send/list`)
- **SQL Database**: list/show/create/update/delete/query/connection-string/import (TDS + type inference)
- **SQL Database import**: Reads CSV/JSON files, infers column types (Int/BigInt/Float/Bit/Date/NVarChar), generates CREATE TABLE + batched INSERTs via TDS. Supports --drop-if-exists, --no-create-table, --batch-size.
- **630 Rust tests** (177 unit + 453 E2E integration), zero clippy warnings, rustfmt clean
- **CI/CD**: GitHub Actions (6-target matrix: x64+arm64 for linux/macos/windows), Dependabot auto-merge, CodeQL, Secret Scanning
- **Release workflow**: Triggered on tags, builds 6 binaries, publishes GitHub Release with SHA256 checksums
- Release binary: ~9.4 MB, stripped, full LTO, panic=abort

### Blocked
- (none)

## Key Decisions
- JSON envelope always wraps output: lists get `{"data":[...],"count":N}`, objects get `{"data":{...}}`
- Errors on stderr as `{"error":{"code":"...","message":"..."}}` with non-zero exit
- `--query` supports simple dot-notation field projection (not full JMESPath; users can pipe to `jq`)
- `--quiet` suppresses all stdout; errors still go to stderr
- OneLake upload uses DFS create+append+flush 3-step pattern
- Notebook creation builds minimal .ipynb JSON, base64-encodes for Fabric API; `source` must be list of strings
- Item copy fetches definition from source via LRO, posts to destination workspace via LRO
- LRO polling: 2s default interval, 120s max wait, handles `Location`/`x-ms-operation-id` headers
- `post()` accepts `poll: bool` for LRO-aware operations
- Load-table requires PascalCase values (`"Overwrite"`, `"Csv"`) and `format` inside `formatOptions`
- **Load-table only supports Csv and Parquet**: The Fabric REST API `formatOptions` discriminated union only has `Csv` (with `header`/`delimiter`) and `Parquet` (format only). JSON is NOT supported — must convert to CSV/Parquet first. Sending CSV-specific fields (header, delimiter) with Parquet format causes API rejection.
- **SQL Database import**: Uses type inference with `Unknown` initial state → first non-empty observation sets the type, subsequent observations widen (Int→BigInt→Float→NVarChar, never narrows)
- **Server-side copy**: OneLake Blob API supports `PUT` with `x-ms-copy-source`; returns 202 with pending status. Poll via HEAD.
- **No native move/rename**: OneLake rejects `x-ms-rename-source`. Move = copy + delete.
- **Table file listing**: Must list from root (no `directory` param) to get real paths prefixed with item ID.
- **Recursive delete**: DFS `DELETE /{ws}/{lh}/Tables/{name}?recursive=true` works for directories.
- All destructive actions use consistent verb `delete` (not `remove`)
- Cross-workspace ops use `--source-workspace`/`--dest-workspace` with `visible_alias` short forms
- Auth relies on `DefaultAzureCredential` chain (az login, environment, managed identity)
- `azure_identity`/`azure_core` with `default-features = false` (no OpenSSL dependency)
- `unsafe_code = "forbid"` in lints
- **KQL Queryset definition format**: Uses `RealTimeQueryset.json` (NOT `RawQueryset.kql`). JSON structure: `{"queryset":{"version":"1.0.0","dataSources":[{"id","clusterUri","type","databaseName"}],"tabs":[{"id","content","title","dataSourceId"}]}}`. The `content` field holds the KQL query text with `\n` for newlines.
- **KQL Queryset run**: Fetches definition via LRO, decodes `RealTimeQueryset.json`, selects tab by name or index, resolves data source (clusterUri + databaseName), executes via Kusto REST API. Tab selection is case-insensitive by title.

## Critical Context
- User's tenant: set locally via secure environment configuration (redacted)
- Active capacity: set locally via secure environment configuration (redacted)
- Inactive capacity: set locally via secure environment configuration (redacted)
- Source workspace/lakehouse: set locally via secure environment configuration (redacted)
- Destination workspace/lakehouse: set locally via secure environment configuration (redacted)
- Notebook ID: set locally via secure environment configuration (redacted)
- Fabric REST base URL: `https://api.fabric.microsoft.com/v1`
- OneLake DFS base URL: `https://onelake.dfs.fabric.microsoft.com`
- OneLake Blob base URL: `https://onelake.blob.fabric.microsoft.com`
- Fabric scope: `https://analysis.windows.net/powerbi/api/.default`
- Storage scope: `https://storage.azure.com/.default`
- Spark rate limit on small capacity: LRO reports 430 `TooManyRequestsForCapacity` (non-standard code)
- Test env vars: `FABIO_TEST_SOURCE_WORKSPACE`, `FABIO_TEST_SOURCE_LAKEHOUSE`, `FABIO_TEST_DEST_WORKSPACE`, `FABIO_TEST_DEST_LAKEHOUSE`, `FABIO_TEST_NOTEBOOK_ID`, `FABIO_TEST_CAPACITY_ID`
- Fabric REST API specs (OpenAPI): `https://github.com/Azure/azure-rest-api-specs/` (look under `specification/fabric/`)

## Relevant Files
- `Cargo.toml`: Project config, dependencies, clippy/lints config, release profile (LTO+strip)
- `rust-toolchain.toml`: stable channel, rustfmt+clippy components
- `src/main.rs`: Entry point, `#![recursion_limit = "256"]`, tokio async main, error handling dispatch
- `src/cli.rs`: Clap derive CLI definition, OutputFormat enum, Command enum with 37 subcommand groups
- `src/errors.rs`: ErrorCode enum + FabioError struct with thiserror
- `src/output.rs`: render_list_with_token, render_object, render_error (respects --quiet/--query), apply_query, dry_run_guard, unit tests
- `src/parallel.rs`: Parallel execution framework for concurrent file/table operations with rate-limit retry
- `src/client.rs`: FabricClient with async HTTP (get/post/put/patch/delete), LRO polling, OneLake DFS/Blob ops, run_notebook
- `src/commands/mod.rs`: Command dispatch
- `src/commands/auth.rs`: login/logout/status (DefaultAzureCredential chain)
- `src/commands/workspace.rs`: 13 subcommands (CRUD + capacity + identity + role assignments)
- `src/commands/item.rs`: 10 subcommands (CRUD + copy/move + definitions + list-connections)
- `src/commands/lakehouse.rs`: 20 subcommands (CRUD + tables, files, upload, download, load-table, copy-file, delete-file, move-file, delete-table, copy-table, move-table, sync, create-shortcut, get-shortcut, delete-shortcut)
- `src/commands/notebook.rs`: create/get-definition/run (with --wait/--timeout)/status/stop/delete
- `src/commands/warehouse.rs`: list/show/create/update/delete/query (endpoint resolved, stdin/file/flag SQL input)
- `src/commands/sql_database.rs`: list/show/create/update/delete/query/connection-string/import (TDS + type inference)
- `src/commands/tds_utils.rs`: shared `column_value_to_json()` with `to_utf8_string()` fix
- `src/commands/dataagent.rs`: list/show/create/update/delete/query
- `src/commands/git.rs`: status/commit/pull/connect/disconnect/initialize/switch/connection/credentials/show-tracked
- `src/commands/ontology.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/environment.rs`: list/show/create/update/delete/publish/cancel-publish/get-spark-settings/get-staging-spark-settings
- `src/commands/data_pipeline.rs`: list/show/create/update/delete/run
- `src/commands/report.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/semantic_model.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/eventhouse.rs`: list/show/create/update/delete
- `src/commands/eventstream.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/kql_database.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/kql_queryset.rs`: CRUD + get-definition/update-definition + run (fetch definition, select tab, execute against Kusto REST API)
- `src/commands/kql_dashboard.rs`: list/show/create/update/delete/get-definition/update-definition (RealTimeDashboard.json)
- `src/commands/mirrored_database.rs`: list/show/create/update/delete/get-definition/update-definition/start/stop/status/table-status
- `src/commands/reflex.rs`: list/show/create/update/delete/get-definition/update-definition (Data Activator)
- `src/commands/ml_model.rs`: list/show/create/update/delete (CRUD only)
- `src/commands/ml_experiment.rs`: list/show/create/update/delete (CRUD only)
- `src/commands/copy_job.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/dataflow.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/graphql_api.rs`: list/show/create/update/delete/get-definition/update-definition (schema.graphql)
- `src/commands/spark.rs`: get-settings/update-settings/list-pools/get-pool/create-pool/update-pool/delete-pool
- `src/commands/spark_job_definition.rs`: list/show/create/update/delete/get-definition/update-definition/run
- `src/commands/capacity.rs`: list/show
- `src/commands/connection.rs`: list/show/create/update/delete/list-supported-types
- `src/commands/deployment_pipeline.rs`: list/show/create/update/delete/list-stages/list-stage-items/assign-workspace/unassign-workspace/deploy
- `src/commands/domain.rs`: list/show/create/update/delete/list-workspaces/assign-workspaces/unassign-workspaces/assign-by-capacity/assign-by-principal
- `src/commands/job_scheduler.rs`: list-instances/get-instance/run-on-demand/cancel-instance/list-schedules/get-schedule/create-schedule/update-schedule/delete-schedule
- `src/commands/onelake_security.rs`: list/show/upsert/delete (data access roles)
- `src/commands/managed_private_endpoint.rs`: list/show/create/delete
- `src/commands/profile.rs`: save/use/list/show/delete (named profiles with defaults)
- `src/commands/jobs.rs`: list/get/prune (local async job ledger)
- `src/commands/feedback.rs`: send/list (two-way I/O for CLI friction reporting)
- `src/commands/agent_context.rs`: Machine-readable command schema for AI agents
- `tests/common/mod.rs`: Shared E2E test harness (TestConfig, helpers)
- `tests/e2e_auth.rs`: Auth integration tests
- `tests/e2e_workspace.rs`: Workspace CRUD + assign-capacity tests
- `tests/e2e_global_options.rs`: --query, --quiet, --output format tests
- `tests/e2e_item.rs`: Item list/show/create/delete/copy/move tests
- `tests/e2e_lakehouse.rs`: Tables/files/upload/download tests
- `tests/e2e_lakehouse_files.rs`: File copy/move/delete tests
- `tests/e2e_lakehouse_tables.rs`: Table load/copy/move/delete tests
- `tests/e2e_lakehouse_shortcuts.rs`: Shortcut create/get/delete tests
- `tests/e2e_notebook.rs`: Notebook create/get-definition/run/run --wait/status/stop/delete tests
- `tests/e2e_warehouse.rs`: Warehouse list/show/query/query-stdin tests
- `tests/e2e_sql_database.rs`: SQL Database CRUD + query + import tests
- `tests/e2e_dataagent.rs`: Data agent tests
- `tests/e2e_git.rs`: Git command group tests
- `tests/e2e_ontology.rs`: Ontology CRUD + definition tests
- `tests/e2e_agent_native.rs`: Agent-native compliance tests (principles 1-10)
- `tests/e2e_sync.rs`: Lakehouse sync tests
- `tests/e2e_connection.rs`: Connection CRUD + list-supported-types tests
- `tests/e2e_environment.rs`: Environment CRUD tests
- `tests/e2e_data_pipeline.rs`: Data pipeline CRUD + run tests
- `tests/e2e_eventhouse.rs`: Eventhouse CRUD tests
- `tests/e2e_eventstream.rs`: Eventstream CRUD tests
- `tests/e2e_kql_database.rs`: KQL database tests
- `tests/e2e_kql_queryset.rs`: KQL queryset tests
- `tests/e2e_kql_dashboard.rs`: KQL dashboard tests
- `tests/e2e_mirrored_database.rs`: Mirrored database tests
- `tests/e2e_reflex.rs`: Reflex CRUD tests
- `tests/e2e_graphql_api.rs`: GraphQL API CRUD tests
- `tests/e2e_ml_model.rs`: ML model CRUD tests
- `tests/e2e_ml_experiment.rs`: ML experiment CRUD tests
- `tests/e2e_copy_job.rs`: Copy job CRUD tests
- `tests/e2e_dataflow.rs`: Dataflow CRUD tests
- `tests/e2e_report.rs`: Report CRUD tests
- `tests/e2e_semantic_model.rs`: Semantic model CRUD tests
- `tests/e2e_spark_job_definition.rs`: Spark job definition tests
- `tests/e2e_deployment_pipeline.rs`: Deployment pipeline tests
- `tests/e2e_domain.rs`: Domain management tests
- `tests/e2e_job_scheduler.rs`: Job scheduler tests
- `tests/e2e_spark.rs`: Spark settings and pool tests
- `tests/e2e_capacity.rs`: Capacity list/show tests
- `tests/e2e_onelake_security.rs`: OneLake security tests
- `tests/e2e_managed_private_endpoint.rs`: Managed private endpoint tests
- `.github/workflows/ci.yml`: Rust CI (fmt, clippy, test, build) on 6 targets (x64+arm64 x linux/macos/windows)
- `.github/workflows/release.yml`: Release workflow (tag-triggered, 6 binaries, SHA256 checksums, GitHub Release)
- `.github/workflows/dependabot-auto-merge.yml`: Auto-merge Dependabot PRs on CI pass
- `.github/dependabot.yml`: Cargo + GitHub Actions dependency updates

## OneLake API Behaviors Discovered
- Blob API copy (`x-ms-copy-source`): works for server-side file copy, async (202 with pending status)
- DFS rename (`x-ms-rename-source`): NOT supported (returns `UnsupportedHeader`)
- DFS recursive delete (`?recursive=true`): works for directories
- DFS listing with `directory` param on a table path shows virtual lakehouse structure (not real files)
- Root listing (no `directory` param): returns real paths prefixed with item ID
- Table files live at `Tables/{name}/_delta_log/` and `Tables/{name}/*.parquet`
- **DFS directory parameter "virtual lakehouse-in-lakehouse" view**: When `directory=X` is specified, the API returns ALL paths prefixed with `X/`, where top-level lakehouse dirs appear doubled (e.g., `Files/Files/myfile.csv` for a file at `Files/myfile.csv`). With `recursive=false`, only immediate virtual children show. Fix: always use `recursive=true` and strip the doubled prefix client-side.
- **Notebook Jobs API**: `POST /workspaces/{ws}/items/{id}/jobs/instances?jobType=RunNotebook` returns 202 + Location header with job instance URL. Status endpoint returns `NotStarted`, `InProgress`, `Completed`, `Failed`, `Cancelled`. Cancel via `POST .../cancel`.
- **Spark cold start on small capacity**: First notebook run can take 2-5 minutes to transition from `NotStarted` to `InProgress` due to Spark session allocation.

## Data Agent API Behaviors Discovered
- **Definition schema is minimal**: The `getDefinition`/`updateDefinition` API only controls `$schema`, `aiInstructions`, and `experimental` fields. Data sources are NOT configured through definitions — they are managed internally by Fabric (portal-only).
- **Definition schema URLs**:
  - `dataAgent/2.1.0/schema.json` — top-level, only has `$schema`
  - `stageConfiguration/1.0.0/schema.json` — has `$schema`, `aiInstructions`, `experimental`
  - `publishInfo/1.0.0/schema.json` — has `$schema`, `description`
- **Definition parts structure** (observed):
  - `Files/Config/data_agent.json` — schema version reference only
  - `Files/Config/draft/stage_config.json` — AI instructions (draft stage)
  - `Files/Config/published/stage_config.json` — AI instructions (published stage)
  - `Files/Config/publish_info.json` — publish metadata
  - `.platform` — git integration metadata (type, displayName, description, logicalId)
- **V3 Management Plane**: `GET /workspaces/{ws}/dataAgents/{id}/settings` endpoint EXISTS but returns `FeatureNotAvailable` (HTTP 403) with message "Data Agent V3 Public Management Plane is not enabled." This implies a tenant-level feature flag controls access.
- **Publishing is portal-only**: No REST API endpoint exposes publish functionality. Tried: `POST .../publish`, `POST .../jobs/instances?jobType=Publish`, `POST .../jobs/instances?jobType=PublishDataAgent` — all return 404 or InvalidJobType. The portal "Publish" button activates the server-side chat endpoint.
- **Published URL**: Only available from the portal Settings page AFTER publishing. Not exposed in `GET /dataAgents/{id}` response (which only returns `id`, `type`, `displayName`, `description`, `workspaceId`). Will be in `/settings` once V3 is enabled.
- **Published URL pattern**: `https://api.fabric.microsoft.com/v1/workspaces/{wsId}/dataagents/{agentId}/aiassistant/openai` — this is the OpenAI Assistants-compatible endpoint activated by publishing from the portal.
- **Chat protocol**: Data agents expose an OpenAI Assistants-compatible API at the published URL. Flow: `POST /assistants` → `POST /threads` → `POST /threads/{id}/messages` → `POST /threads/{id}/runs` → poll until terminal → `GET /threads/{id}/messages`. Query param: `?api-version=2024-05-01-preview`.
- **Authentication for chat**: Uses same Fabric bearer token (`https://analysis.windows.net/powerbi/api/.default` scope), sent as `Authorization: Bearer {token}`.
- **PATCH /dataAgents/{id}**: Only accepts `displayName` and `description` fields. Passing `properties` or other fields returns `InvalidInput: UpdateArtifactRequest should have at least one valid field to update`.
- **Admin tenant settings API**: `GET /v1/admin/tenantsettings` returns error (insufficient privileges required). PowerBI admin API (`api.powerbi.com/v1.0/myorg/admin/tenantsettings`) returns 404.
- **Data source configuration via definition IS supported**: Include `datasource.json` parts at path `Files/Config/draft/{type}-{DisplayName}/datasource.json`. The server normalizes the path (e.g., `lakehouse-SalesLH` → `lakehouse-tables-SalesLH`). Schema: `dataSource/1.0.0/schema.json`. The server adds `$schema` URL automatically.
- **Data source path convention**: `Files/Config/{stage}/{type}-{DisplayName}/datasource.json` where stage is `draft` or `published`, and type is the full type value (e.g., `lakehouse-tables`, `data-warehouse`, `kusto`). The server normalizes shorthand prefixes.
- **Data source type enum**: `unknown`, `lakehouse_tables`, `lakehouse`, `data_warehouse`, `kusto`, `semantic_model`, `graph`, `mirrored_database`, `mirrored_azure_databricks`.
- **Elements array for table/column selection**: The `elements` field in `datasource.json` defines which tables and columns the agent can access. Each element has `display_name`, `type` (e.g., `lakehouse_tables.table`, `lakehouse_tables.column`), `is_selected`, `description`, `data_type`, and `children` (nested). Setting `is_selected: true` makes them available to the agent.
- **Element type enum**: `lakehouse_tables.table`, `lakehouse_tables.column`, `warehouse_tables.table`, `warehouse_tables.column`, `kusto.table`, `kusto.column`, `kusto.functions`, `semantic_model.table`, `semantic_model.column`, `semantic_model.measure`, `graph.nodeType`, `graph.edgeType`, `mirrored_database.table`, `mirrored_database.column`, plus schema-level types.
- **Server strips unknown fields from datasource.json**: Custom fields like `tables`, `connectionInfo` are stripped; only schema-defined fields are kept (`$schema`, `artifactId`, `workspaceId`, `displayName`, `type`, `userDescription`, `dataSourceInstructions`, `metadata`, `elements`).
- **Server strips experimental.dataSources**: Putting data sources in `experimental` field of `stage_config.json` is ignored; the experimental object is emptied. Data sources MUST use dedicated `datasource.json` files at the correct paths.
- **Update PATCH returns full object**: `PATCH /workspaces/{ws}/dataAgents/{id}` returns the full updated item object (not just `{status: "updated"}`).

## Semantic Model API Behaviors Discovered
- **TMDL vs model.bim**: Direct Lake semantic models REQUIRE TMDL format (v4.0 pbism). The older model.bim JSON format (compat level 1550) does NOT support DirectLake mode partitions.
- **model.bim requires V3 (compat 1604)**: Import-mode models created via the Fabric Items API MUST use `compatibilityLevel: 1604` and `"defaultPowerBIDataSourceVersion": "powerBI_V3"`. Compat level 1550 returns "Import from JSON supported for V3 models only".
- **TMDL enum value for data source version**: Must be `powerBI_V3` (not `powerBIDataSourceVersion3`). The latter returns `InvalidValueFormat` parsing error.
- **definition.pbism is always required**: Fabric Items API for semantic model creation always requires a `definition.pbism` file in the definition parts. Without it, creation fails silently or produces a broken model.
- **TMDL definition.pbism format**: `{"version": "4.0", "datasetReference": {"byPath": null, "byConnection": null}}`
- **model.bim definition.pbism format**: `{"version": "3.0", "datasetReference": {"byPath": null, "byConnection": null}}`
- **TMDL file structure**: A Direct Lake TMDL semantic model requires: `definition.pbism`, `model.tmdl` (model-level settings + expressions), and `definition/tables/{TableName}.tmdl` (one per table). The expression in `model.tmdl` provides the lakehouse connection via `DatabaseQuery` with a placeholder connection string.
- **Direct Lake partition annotation**: Each table partition needs `mode: directLake` in the TMDL source definition. Without it, the model defaults to Import mode.
- **Connection flag**: `semantic-model create --connection <lakehouse-sql-endpoint-id>` wires the Direct Lake connection. The connection ID is the SQL Analytics Endpoint ID (not the lakehouse ID itself).
- **Creation is LRO**: Semantic model creation uses the standard Fabric LRO pattern (202 + Location header polling).
- **Format auto-detection**: `.tmdl` files → TMDL format (v4.0 pbism); `.bim` file → model.bim format (v3.0 pbism). The CLI auto-detects from the file extension.

## Report API Behaviors Discovered
- **definition.pbir is the report definition entry point**: Not `report.json`. The report definition file at `definition.pbir` references the semantic model binding.
- **definition.pbir format**: `{"version": "4.0", "datasetReference": {"byConnection": {"connectionString": null, "pbiServiceModelId": null, "pbiModelVirtualServerName": "sobe_wowvirtualserver", "pbiModelDatabaseName": "<semantic-model-id>", "name": "EntityDataSource", "connectionType": "pbiServiceXmlaStyleLive"}}}` — the `pbiModelDatabaseName` is the semantic model ID.
- **Blank report.json**: A minimal valid report is `{"config": "{\"version\":\"5.56\"}", "layoutOptimization": 0, "pods": [{"config": "{\"name\":\"Page 1\"}"}]}`
- **report create --dataset**: Generates both `definition.pbir` (with semantic model binding) and `report.json` (blank page) automatically. No definition file needed from the user.
- **Definition path changed**: The report definition entry point is `definition.pbir` (not `report.json`). Both `create` and `update-definition` use this path.
- **updateDefinition ALWAYS requires definition.pbir**: The API rejects requests missing the `definition.pbir` part, even if only updating `report.json`. Always include both parts when updating visuals.
- **PBIR-Legacy visual containers**: Reports use `report.json` with `sections[].visualContainers[]` array. Each visual container has `x`, `y`, `z`, `width`, `height`, `config` (JSON string), `filters`, and `tabOrder`.
- **Visual config structure**: The `config` JSON string contains `name`, `layouts[]`, and `singleVisual` with `visualType`, `projections`, `properties`, `objects`, and `dataTransforms`.
- **Supported visualType values**: `card` (KPI cards), `barChart` (bar charts), `tableEx` (data tables), `columnChart`, `lineChart`, `pieChart`, `donutChart`, etc.
- **Projections role names**: Card: `Values`; Bar/Column chart: `Category` + `Y`; Table: `Values`; Line chart: `Category` + `Y`.
- **queryRef format**: `TableName.ColumnName` for columns, `TableName.MeasureName` for measures. Must match the semantic model's exact table and field names.
- **dataTransforms for field binding**: Include `projectionOrdering`, `queryMetadata.Select[]` (with `Restatement`, `Name`, `Type`), and `selects[]` (with `displayName`, `queryName`, `roles`, `type`). Type values: 1=text, 2=numeric/measure, 260=aggregate.
- **Server preserves dataTransforms**: The API correctly stores and returns `dataTransforms` in visual configs, confirming programmatic visual creation is supported.
- **Server preserves original binding**: When `updateDefinition` is called with a new `definition.pbir` that has null values, the server uses the connection string from the original creation. The binding is stable.
- **publish-to-web**: `POST https://api.powerbi.com/v1.0/myorg/groups/{groupId}/reports/{reportId}/publishtoweb` returns 404 for Fabric reports. Attempted with various body formats (`{"accessLevel":"View","allowFullScreen":true}`). Likely requires: (1) tenant admin to enable "Publish to web" in admin portal, AND (2) may only work with classic Power BI reports (not Fabric-native reports created via Items API).
- **PowerBI API scope**: Report publish-to-web uses `api.powerbi.com` (not `api.fabric.microsoft.com`). Requires the same bearer token (`https://analysis.windows.net/powerbi/api/.default` scope).

## Git Integration API Behaviors Discovered
- **GitHub provider REQUIRES credentials**: `fabio git connect --provider github` ALWAYS requires `--connection-id` pointing to a pre-configured `GitHubSourceControl` connection. Without it, returns: `"The property myGitCredentials is required for the GitProviderType GitHub."`. Azure DevOps can use "Automatic" credentials without a connection ID.
- **Fabric Git does NOT track table data**: Delta tables created via `load-table` are NOT version-controlled. Only item definitions (`.platform`, metadata files, notebook code) are tracked. `git status` shows NO changes after creating a table. CI/CD best practice: version-control the Notebook/Pipeline that creates the table.
- **Lakehouse definition does NOT include table schema**: `lakehouse.metadata.json` remains `{}` even after tables are created. The definition only tracks: `.platform` (type metadata), `alm.settings.json` (shortcuts/data access roles config), `shortcuts.metadata.json`.
- **Git status API is LRO-aware**: `GET /workspaces/{ws}/git/status` uses the LRO pattern. Returns `{"changes": [...], "workspaceHead": "<sha>", "remoteCommitHash": "<sha>"}`.
- **Initialize strategy for new workspaces**: Use `prefer-workspace` when connecting a workspace with existing items to an empty repo. Use `prefer-remote` when the repo already has content to pull into the workspace.
- **Commit auto-fetches workspaceHead**: The commit API requires `workspaceHead` but fabio auto-fetches it from `git status` if not provided. Agents don't need to track it manually.
- **Item naming in git**: Folders use `{DisplayName}.{ItemType}` convention: `SalesLakehouse.Lakehouse`, `CreateSalesTable.Notebook`.
- **Notebook format in git**: `{Name}.Notebook/.platform` + `{Name}.Notebook/notebook-content.py`. Cell separators: `# CELL ********************`.
- **ObjectId vs LogicalId**: First commit assigns only `objectId`. After commit, items gain a `logicalId` (stored in `.platform`) for cross-workspace portability.
- **remoteChange is null**: When there's no remote change, the field is `null` (not `"None"`), but `workspaceChange` uses string values like `"Added"`, `"Modified"`, `"None"`.
- **Git connection state**: `fabio git connection show` returns `gitConnectionState: "ConnectedAndInitialized"` with `gitSyncDetails.head` and `lastSyncTime`.
- **Commit is LRO**: Returns 202 with operation ID. With `--wait`, polls until `Succeeded`/`Failed`. Returns `percentComplete: 100` on success.
- **Full CI/CD workflow via fabio**: Validated complete flow: `workspace create` → `workspace assign-capacity` → `lakehouse create` → `git connect` → `git init` → `git commit` → (create items) → `git commit`.

## Cross-Database Query Behaviors Discovered
- **Lakehouse SQL endpoint supports three-part naming**: From a lakehouse SQL endpoint, you can query other databases in the same workspace using `[DatabaseName].[schema].[table]` syntax. Example: `SELECT * FROM SalesDB.dbo.orders` works from the ProductCatalog lakehouse SQL endpoint.
- **SQL Database does NOT support three-part naming**: Fabric SQL Database (`.database.fabric.microsoft.com`) rejects cross-database references with error 40515: "Reference to database and/or server name is not supported in this version of SQL Server."
- **Cross-database direction is one-way**: Lakehouse/Warehouse SQL endpoint → SQL Database works. SQL Database → Lakehouse/Warehouse does NOT work.
- **Warehouse and Lakehouse can cross-query each other**: Both share the same `.datawarehouse.fabric.microsoft.com` TDS endpoint and can query any database visible in `sys.databases` (all lakehouses, warehouses, and SQL Databases in the same workspace).
- **Practical pattern for cross-database analytics**: Use the lakehouse SQL endpoint as the query hub. It can JOIN local Delta tables with SQL Database tables in a single query: `SELECT l.col FROM dbo.local_table l JOIN SqlDb.dbo.remote_table r ON l.id = r.id`.
- **Date columns from cross-DB queries**: TDS returns date columns as "N days since 0001-01-01" format when crossing database boundaries. May need client-side conversion.
- **SQL Database requires F4+ capacity**: On F2 capacity, SQL Database TDS connections fail with error 18456 State 240 ("Validation of user's permissions failed"). This is not a permissions issue — it's insufficient compute to serve the TDS endpoint. F4 resolves the issue completely.
- **SQL Database auto-creates a SQLEndpoint item**: Creating a SQL Database automatically creates a companion SQLEndpoint item with the same display name. This is the mirrored read-only analytics endpoint.
- **Initial catalog must be set explicitly**: Fabric TDS connection strings from the REST API contain only the server hostname (no `database=` or `Initial Catalog=`). The TDS client must set the initial catalog to the item's `displayName` to connect to the correct database context. Without it, the server defaults to an arbitrary database in the workspace.

## KQL Queryset API Behaviors Discovered
- **Definition uses `RealTimeQueryset.json`** (NOT `RawQueryset.kql`): The definition part path is `RealTimeQueryset.json` containing a JSON object with `queryset.version`, `queryset.dataSources[]`, and `queryset.tabs[]`.
- **Empty queryset returns `{}`**: A newly created queryset has `RealTimeQueryset.json` with payload `e30=` (base64 for `{}`). Must check for empty object before attempting to run.
- **Data source type is always `AzureDataExplorer`**: Even for Fabric Eventhouses, the `type` field in data sources is `"AzureDataExplorer"` (not `"Eventhouse"` or `"Fabric"`).
- **clusterUri for Fabric Eventhouse**: Uses the Kusto query URI format `https://<id>.<region>.kusto.fabric.microsoft.com`. This is the same URI used for direct KQL database queries.
- **Tab content uses literal `\n`**: In the JSON definition, KQL query newlines are stored as literal `\n` characters within the string (not `\\n` escape sequences). Multi-line queries work correctly.
- **Tab selection is case-insensitive by title**: The portal stores tab titles as-is, but `kql-queryset run` matches case-insensitively for agent ergonomics.
- **No server-side run API exists**: KQL Querysets have no Jobs API or `/run` endpoint. Execution requires client-side: get definition → extract tab content → POST to Kusto REST API.
- **getDefinition is LRO**: Like other Fabric definition APIs, `POST .../getDefinition` returns 202 and requires polling.
- **updateDefinition is LRO**: Returns 202 with empty body on success (after polling). The response body from LRO completion is empty/null.
- **Server normalizes CRLF**: If you upload a definition with LF line endings, the server may return it with CRLF (`\r\n`). Decode must handle both.
- **Multiple data sources supported**: A queryset can reference multiple clusters/databases. Each tab has a `dataSourceId` field linking to a specific data source.

## GraphQL API Behaviors Discovered
- **Query endpoint**: `POST /workspaces/{ws}/graphqlApis/{id}/graphql` with body `{"query": "...", "variables": {...}, "operationName": "..."}`.
- **Scope is standard Fabric scope**: Uses `https://analysis.windows.net/powerbi/api/.default` (same as all Fabric APIs, NOT a GraphQL-specific scope).
- **Response envelope**: Returns `{"data": {...}}` on success, `{"errors": [...]}` on failure, or both for partial results.
- **Introspection blocked by default**: `__schema` and `__type` introspection queries return a security error unless explicitly enabled in tenant settings.
- **Definition format**: `graphql-definition.json` with `datasources[]` array. Each datasource has `sourceItemId`, `sourceWorkspaceId`, `sourceType` (e.g., `SqlAnalyticsEndpoint`, `Warehouse`), and `objects[]` with field mappings.
- **updateDefinition is LRO**: Returns 202 and must be polled. Creating a GraphQL API with a datasource requires the LRO pattern.
- **sourceType values**: `SqlAnalyticsEndpoint` (for lakehouses), `Warehouse`, `SqlDatabase`. The source item ID is the SQL analytics endpoint ID (not the lakehouse/warehouse item ID directly).
- **Object field mappings**: Each object in `objects[]` maps GraphQL types to source table columns. Field names are auto-generated from table column names.
- **No schema.graphql in initial definition**: Newly created GraphQL APIs have no `schema.graphql` part until a datasource is configured and the schema is generated.

## Warehouse API Behaviors Discovered
- **Connection string format**: `<unique-id>.datawarehouse.fabric.microsoft.com` — no port, no protocol prefix. TDS client connects via port 1433 (default).
- **Views appear in INFORMATION_SCHEMA.TABLES**: Both tables and views show up. Distinguish via `TABLE_TYPE` column (`BASE TABLE` vs `VIEW`).
- **System views are visible**: `queryinsights.*` and `sys.*` views appear alongside user objects. Filter with `WHERE TABLE_SCHEMA = 'dbo'` for user objects only.
- **Date columns via TDS**: Date values come through as "N days since 0001-01-01" string representation in the mssql-rs crate. Conversion: `chrono::NaiveDate::from_num_days_from_ce(days + 1)`.
- **Cross-workspace queries NOT supported**: Three-part naming only works within the same workspace. Cross-workspace requires explicit data copy or shortcuts.

## Semantic Model + Report Creation Workflow
- **DirectQuery to warehouse**: model.bim with `compatibilityLevel: 1604`, partition `mode: "directQuery"`, M expression using `Sql.Database("<connectionInfo>", "<displayName>")`.
- **M expression pattern for warehouse**: `let Source = Sql.Database("server.datawarehouse.fabric.microsoft.com", "WarehouseName"), table = Source{[Schema="dbo",Item="table_name"]}[Data] in table`.
- **Measures in model.bim**: Defined at table level in `measures[]` array with `name` and `expression` (DAX). Works for both Import and DirectQuery models.
- **Report creation with `--dataset`**: Simplest path — generates `definition.pbir` + blank `report.json` automatically. No need to craft definition files manually.
- **Report is blank canvas**: CLI-created reports have a single blank page. Visual authoring requires the Fabric portal or Power BI Desktop. The report is immediately viewable and editable in the portal.
- **Semantic model ID links report to data**: The `definition.pbir` file's `pbiModelDatabaseName` field is the semantic model ID (UUID), not the display name.
- **End-to-end creation order**: Warehouse (data source) → Semantic Model (definition + connection) → Report (bound to semantic model). Each step depends on the previous item's ID.

## Next Steps
- Add ODBC support to warehouse query (`odbc-api` crate)
- Consider adding `--filter` flag for list commands
