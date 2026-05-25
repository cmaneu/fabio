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
- **Full Rust implementation** (267 subcommands across 37 groups): auth, workspace, item, lakehouse, capacity, notebook, warehouse, data-agent, ontology, environment, data-pipeline, copy-job, dataflow, report, semantic-model, eventhouse, eventstream, kql-database, kql-queryset, kql-dashboard, mirrored-database, reflex, ml-model, ml-experiment, spark, spark-job-definition, graphql-api, git, connection, deployment-pipeline, domain, job-scheduler, onelake-security, managed-private-endpoint, profile, jobs, feedback + agent-context
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
- **Ontology management**: list, show, create, update, delete, get-definition, update-definition (RDF file support, --dir for Fabric definition format, --decode for readable output)
- **Environment**: list, show, create, update, delete, publish, cancel-publish, get-spark-settings, get-staging-spark-settings
- **Data Pipeline**: list, show, create, update, delete, run (triggers Pipeline job)
- **Eventhouse**: list, show, create, update, delete
- **Eventstream**: list, show, create, update, delete, get-definition, update-definition, get-topology, pause, resume, get/pause/resume-source, get-source-connection, get/pause/resume-destination, get-destination-connection, add-source, add-destination
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
- **Map**: list, show, create, update, delete, get-definition, update-definition (geospatial visualization with Azure Maps)
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
- **700 Rust tests** (199 unit + 501 E2E integration), zero clippy warnings, rustfmt clean
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
- `src/commands/map.rs`: list/show/create/update/delete/get-definition/update-definition (geospatial Azure Maps)
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
- `tests/e2e_reflex.rs`: Reflex CRUD + definition (get/update with simulator pipeline) tests
- `tests/e2e_graphql_api.rs`: GraphQL API CRUD tests
- `tests/e2e_ml_model.rs`: ML model CRUD tests
- `tests/e2e_ml_experiment.rs`: ML experiment CRUD tests
- `tests/e2e_copy_job.rs`: Copy job CRUD tests
- `tests/e2e_dataflow.rs`: Dataflow CRUD tests
- `tests/e2e_report.rs`: Report CRUD tests
- `tests/e2e_semantic_model.rs`: Semantic model CRUD tests
- `tests/e2e_map.rs`: Map CRUD + definition tests
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

## Ontology API Behaviors Discovered
- **Definition format**: Fabric ontology uses a proprietary JSON definition format (NOT RDF). Structure: `definition.json` (root, usually `{}`), `EntityTypes/{ID}/definition.json`, `EntityTypes/{ID}/DataBindings/{UUID}.json`, `RelationshipTypes/{ID}/definition.json`.
- **Schema URLs**: Entity types use `https://developer.microsoft.com/json-schemas/fabric/item/ontology/entityType/1.0.0/schema.json`, data bindings use `.../dataBinding/1.0.0/schema.json`, relationship types use `.../relationshipType/1.0.0/schema.json`.
- **Data binding format**: Requires `dataBindingConfiguration` wrapper (NOT flat fields). Structure: `{"id":"<uuid>","dataBindingConfiguration":{"dataBindingType":"NonTimeSeries","sourceTableProperties":{...},"propertyBindings":[...]}}`. The `sourceTableProperties` uses `itemId` (not `lakehouseId`) and `sourceTableName` (not `tableName`).
- **Data binding ID must be UUID format**: The `id` field in data bindings must be a valid UUID (e.g., `c0000001-0001-0001-0001-000000000001`). Non-UUID values (e.g., `db-equipment-001`) are silently dropped.
- **Property bindings use `targetPropertyId`**: Each entry in `propertyBindings` requires `sourceColumnName` and `targetPropertyId` (NOT `propertyId`). The `targetPropertyId` must match a property `id` in the entity type definition.
- **`sourceSchema` field in `sourceTableProperties`**: Include `"sourceSchema": "dbo"` alongside `sourceType`, `workspaceId`, `itemId`, `sourceTableName`. Required for lakehouse table bindings.
- **Data binding type enum**: `NonTimeSeries` (for lakehouse tables) or `TimeSeries` (requires `timestampColumnName`).
- **Source type enum in sourceTableProperties**: `LakehouseTable` or `KustoTable` (for Eventhouse).
- **CRITICAL: JSON key ordering sensitivity**: The Fabric Ontology API uses ordered JSON deserialization for data bindings. The `sourceType` field MUST be the first key in `sourceTableProperties`. If other keys (like `itemId`) come before `sourceType` (e.g., alphabetical order from serde_json without `preserve_order`), the API throws: `"Import of the {0} artifact '{1}' threw an exception with this message: {2}"`. The CLI normalizes key order automatically via `normalize_data_binding()`.
- **Entity type required fields**: `id`, `namespace` (must be `"usertypes"`), `name`, `namespaceType` (must be `"Custom"`). Optional: `baseEntityTypeId`, `entityIdParts`, `displayNamePropertyId`, `visibility` (must be `"Visible"`), `properties`, `timeseriesProperties`.
- **Property value types**: `String`, `Boolean`, `DateTime`, `Object`, `BigInt`, `Double`.
- **Relationship type required fields**: `id`, `namespace`, `name`, `namespaceType`, `source.entityTypeId`, `target.entityTypeId`.
- **Server auto-adds `$schema` URLs**: When you upload definitions, the server adds the appropriate `$schema` URL to the response. You don't need to include it in your upload.
- **Server adds `untypedProperties: []`**: Entity types returned by `getDefinition` include an extra `untypedProperties` array not present in the upload.
- **getDefinition/updateDefinition are LRO**: Both use the standard Fabric LRO polling pattern (202 + Location header).
- **`--decode` flag**: Adds `decodedPayload` field alongside original `payload` (JSON objects or text strings). Preserves backward compatibility.
- **`--dir` flag**: Reads Fabric ontology directory structure (`EntityTypes/`, `RelationshipTypes/` with `definition.json`, `DataBindings/`, `Documents/`, `Overviews/`, `ResourceLinks/`).
- **`preserve_order` feature**: `serde_json` is configured with `preserve_order` to support JSON key-order normalization for data bindings.

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
- **Graph datasource type IS supported via definition API**: `type: "graph"` in `datasource.json` is accepted and persisted. Path: `Files/Config/draft/graph-{DisplayName}/datasource.json`. Server auto-adds `$schema`, `metadata: null`, `elements: []`. The `artifactId` should be the Graph Model item ID.
- **Full definition required for datasource persistence**: Single-part `updateDefinition` with only the datasource file is silently dropped (202 accepted but not persisted). Must include ALL parts together: `data_agent.json` + `stage_config.json` + `datasource.json`. This applies to all datasource types (not just graph).
- **Graph datasource path convention**: `Files/Config/draft/graph-{DisplayName}/datasource.json` — server does NOT normalize the prefix (unlike lakehouse which becomes `lakehouse-tables-`). The `graph-` prefix is kept as-is.
- **Graph datasource fields**: `artifactId` (Graph Model ID), `workspaceId`, `displayName`, `type` ("graph"), `userDescription`, `dataSourceInstructions`. Server adds: `$schema`, `metadata`, `elements`.

## Semantic Model API Behaviors Discovered
- **TMDL vs model.bim**: Direct Lake semantic models REQUIRE TMDL format (v4.0 pbism). The older model.bim JSON format (compat level 1550) does NOT support DirectLake mode partitions.
- **model.bim requires V3 (compat 1604)**: Import-mode models created via the Fabric Items API MUST use `compatibilityLevel: 1604` and `"defaultPowerBIDataSourceVersion": "powerBI_V3"`. Compat level 1550 returns "Import from JSON supported for V3 models only".
- **TMDL enum value for data source version**: Must be `powerBI_V3` (not `powerBIDataSourceVersion3`). The latter returns `InvalidValueFormat` parsing error.
- **definition.pbism is always required**: Fabric Items API for semantic model creation always requires a `definition.pbism` file in the definition parts. Without it, creation fails silently or produces a broken model.
- **TMDL definition.pbism format**: `{"$schema":"https://developer.microsoft.com/json-schemas/fabric/item/semanticModel/definitionProperties/1.0.0/schema.json","version":"4.2","settings":{}}` — v4.2 with the Fabric schema URL.
- **model.bim definition.pbism format**: `{"version": "3.0"}` — no `datasetReference` property (rejected by schema validator).
- **TMDL file structure**: A Direct Lake TMDL semantic model requires: `definition.pbism`, `model.tmdl` (model-level settings + expressions), and `definition/tables/{TableName}.tmdl` (one per table). The expression in `model.tmdl` provides the lakehouse connection via `DatabaseQuery` with a placeholder connection string.
- **Direct Lake partition annotation**: Each table partition needs `mode: directLake` in the TMDL source definition. Without it, the model defaults to Import mode.
- **Connection flag**: `semantic-model create --connection <lakehouse-sql-endpoint-id>` wires the Direct Lake connection. The connection ID is the SQL Analytics Endpoint ID (not the lakehouse ID itself).
- **Creation is LRO**: Semantic model creation uses the standard Fabric LRO pattern (202 + Location header polling).
- **Format auto-detection**: `.tmdl` files → TMDL format (v4.0 pbism); `.bim` file → model.bim format (v3.0 pbism). The CLI auto-detects from the file extension.
- **DirectQuery requires interactive credential binding**: DirectQuery models to Fabric warehouses need OAuth2 credentials configured via portal "Manage connections and gateways". The Power BI REST API `GetBoundGatewayDataSources` returns empty for API-created models. `BindToGateway` with virtual gateway `00000000-...` succeeds but doesn't configure credentials. OAuth2 credential type is "not supported for this API" when creating connections. The `executeQueries` DAX API works (uses caller's token directly), but report viewers fail (service needs stored credentials for the double-hop).
- **Direct Lake avoids credential issues**: Direct Lake models read directly from OneLake Delta files — no SQL connection credentials needed. The framing refresh uses the workspace identity automatically. Prefer Direct Lake over DirectQuery for programmatically-created reports.
- **Direct Lake Sql.Database() second parameter must be SQL endpoint ID**: The M expression `Sql.Database("<server>", "<database>")` must use the SQL Analytics Endpoint ID (not the lakehouse ID). Using the lakehouse ID causes `DM_InvalidRequest_DatamartNotFound` with `artifactType: 2000`.
- **Direct Lake needs refresh to frame**: After creation or updateDefinition, a `POST /refreshes` with `{"type": "Full"}` is required. Without framing, DAX queries fail with error code `3242524690`.
- **Direct Lake entity partition format**: `partition 'Name' = entity` with `mode: directLake`, `source` block containing `entityName: <table_name>`, `schemaName: dbo`, `expressionSource: DatabaseQuery`.
- **TMDL models are "definition-managed" (read-only in portal web editor)**: Models created via Fabric Items API with a `definition` are marked as definition-managed. The portal web modeler shows "This dataset is read-only" and blocks schema editing. Fix: call `POST /v1.0/myorg/groups/{ws}/datasets/{id}/Default.TakeOver` (with empty `{}` body) after creation. This converts the model to "service-managed" while preserving Direct Lake functionality, DAX queries, and refresh capability. The model keeps `targetStorageMode: Abf` (required for Direct Lake).
- **Do NOT change targetStorageMode to PremiumFiles for Direct Lake**: Switching to `PremiumFiles` breaks Direct Lake refresh ("cannot access source column" errors). Direct Lake REQUIRES `Abf` storage mode. The `PATCH /datasets/{id}` with `{"targetStorageMode": "PremiumFiles"}` only works for Import-mode models.
- **TakeOver preserves full functionality**: After TakeOver, `updateDefinition` still works (can redeploy TMDL), `refreshes` still work, DAX queries still work. TakeOver + refresh is the correct post-creation step for editable Direct Lake models.
- **definition.pbism v4.2 schema**: The correct pbism for TMDL models deployed via Fabric Items API is `{"$schema":"https://developer.microsoft.com/json-schemas/fabric/item/semanticModel/definitionProperties/1.0.0/schema.json","version":"4.2","settings":{}}` — NOT the older `{"version":"3.0","datasetReference":{...}}` format (which fails with schema validation error).
- **model.bim pbism format**: For model.bim, use just `{"version": "3.0"}` (no `datasetReference` — that property is rejected by schema validator).

## Report API Behaviors Discovered
- **definition.pbir is the report definition entry point**: Not `report.json`. The report definition file at `definition.pbir` references the semantic model binding.
- **definition.pbir format**: `{"version": "4.0", "datasetReference": {"byConnection": {"connectionString": null, "pbiServiceModelId": null, "pbiModelVirtualServerName": "sobe_wowvirtualserver", "pbiModelDatabaseName": "<semantic-model-id>", "name": "EntityDataSource", "connectionType": "pbiServiceXmlaStyleLive"}}}` — the `pbiModelDatabaseName` is the semantic model ID.
- **Blank report.json**: A minimal valid report is `{"config": "{\"version\":\"5.56\"}", "layoutOptimization": 0, "pods": [{"config": "{\"name\":\"Page 1\"}"}]}`
- **report create --dataset**: Generates both `definition.pbir` (with semantic model binding) and `report.json` (blank page) automatically. No definition file needed from the user.
- **Definition path changed**: The report definition entry point is `definition.pbir` (not `report.json`). Both `create` and `update-definition` use this path.
- **updateDefinition ALWAYS requires definition.pbir**: The API rejects requests missing the `definition.pbir` part, even if only updating `report.json`. Always include both parts when updating visuals.
- **updateDefinition CAN switch formats**: Format conversion works in both directions — send PBIR parts to convert to PBIR; send report.json to convert to PBIR-Legacy. Invalid schema fields cause silent rejection.
- **PBIR-Legacy is REQUIRED for programmatic visuals that render data**: Despite PBIR being the "future" format, only PBIR-Legacy with `prototypeQuery` produces visuals that actually display data. The portal itself creates PBIR-Legacy reports. Use `report.json` with `sections[].visualContainers[]` for programmatic report creation.
- **PBIR version.json requires semver**: The `version` field must match `^[1-9][0-9]*\.(0|[1-9][0-9]*)\.0$` (e.g., `"4.0.0"`, NOT `"4.0"`).
- **PBIR report.json requires layoutOptimization as string**: Must be `"None"` (string), not `0` (integer). Unlike PBIR-Legacy which uses integer 0.
- **PBIR-Legacy visual containers**: Reports use `report.json` with `sections[].visualContainers[]` array. Each visual container has `x`, `y`, `z`, `width`, `height`, `config` (JSON string), `filters`, and `tabOrder`.
- **Visual config structure**: The `config` JSON string contains `name`, `layouts[]`, and `singleVisual` with `visualType`, `projections`, `properties`, `objects`, and `dataTransforms`.
- **Supported visualType values**: `card` (KPI cards), `barChart` (bar charts), `tableEx` (data tables), `columnChart`, `lineChart`, `pieChart`, `donutChart`, etc.
- **Projections role names**: Card: `Values`; Bar/Column chart: `Category` + `Y`; Table: `Values`; Line chart: `Category` + `Y`.
- **queryRef format**: `TableName.ColumnName` for columns, `TableName.MeasureName` for measures. Must match the semantic model's exact table and field names.
- **dataTransforms for field binding**: Include `projectionOrdering`, `queryMetadata.Select[]` (with `Restatement`, `Name`, `Type`), and `selects[]` (with `displayName`, `queryName`, `roles`, `type`). Type values: 1=text, 2=numeric/measure, 260=aggregate.
- **Server preserves dataTransforms**: The API correctly stores and returns `dataTransforms` in visual configs, confirming programmatic visual creation is supported.
- **prototypeQuery is REQUIRED for visuals to render data**: Without `prototypeQuery` in `singleVisual`, the visual container appears but shows NO data. The `prototypeQuery` is a semantic query that tells the Power BI renderer how to construct the DAX query for the visual. Format: `{"Version": 2, "From": [{"Name": "<alias>", "Entity": "<TableName>", "Type": 0}], "Select": [...]}`. Each `Select` entry uses `Column` or `Measure` with `SourceRef.Source` referencing the `From` alias. The `dataTransforms.selects[].expr` must also use `SourceRef.Source` (not `SourceRef.Entity`).
- **PBIR format does NOT support programmatic visual data rendering**: PBIR visuals with `query.queryState` are stored correctly but render NO data in the portal. The PBIR schema does not allow `prototypeQuery` (rejected by schema validator). PBIR appears to require internal metadata that only Power BI Desktop or the portal editor generates. **Use PBIR-Legacy with `prototypeQuery` for programmatic report creation with working visuals.**
- **Server preserves original binding**: When `updateDefinition` is called with a new `definition.pbir` that has null values, the server uses the connection string from the original creation. The binding is stable.
- **publish-to-web**: `POST https://api.powerbi.com/v1.0/myorg/groups/{groupId}/reports/{reportId}/publishtoweb` returns 404 for Fabric reports. Attempted with various body formats (`{"accessLevel":"View","allowFullScreen":true}`). Likely requires: (1) tenant admin to enable "Publish to web" in admin portal, AND (2) may only work with classic Power BI reports (not Fabric-native reports created via Items API).
- **PowerBI API scope**: Report publish-to-web uses `api.powerbi.com` (not `api.fabric.microsoft.com`). Requires the same bearer token (`https://analysis.windows.net/powerbi/api/.default` scope).

## Power BI File Formats Overview

Power BI has multiple file formats spanning different eras and use cases. Understanding these is critical for choosing the right approach when creating or managing semantic models and reports via the Fabric REST API.

| File Format | Purpose | Human Readable? | Fabric REST API Support | Era |
|---|---|---|---|---|
| `.pbix` | Standard Power BI report (binary) | No | Not directly (import only) | Original |
| `.pbit` | Power BI template (no data) | Partially | Not directly | Early |
| `.pbip` | Power BI Project (folder structure) | Yes | Maps to definition parts | 2023+ |
| `.pbir` | Report definition entry point | Yes | Required for all report ops | 2024+ |
| `model.bim` | Tabular model definition (JSON) | Yes | Supported via Items API | Legacy + supported |
| `TMDL` | Tabular Model Definition Language | Yes | Supported via Items API | Current |
| `.rdl` | Paginated report (XML) | XML | Limited | SSRS heritage |

### Format Selection for Fabric REST API

| Scenario | Format | Notes |
|---|---|---|
| Direct Lake semantic model | TMDL | Required for `mode: directLake` partitions |
| Import-mode semantic model | `model.bim` | Must use `compatibilityLevel: 1604` + `powerBI_V3` |
| Report with working visuals | PBIR-Legacy (`report.json`) | Only format supporting `prototypeQuery` for data rendering |
| Report for source control | PBIR (`definition/` folder) | Better diffs but limited programmatic visual support |
| Semantic model source control | TMDL (folder-based) | One `.tmdl` file per table, better Git diffs |

### Evolution Timeline

| Era | Main Formats | Fabric CLI Relevance |
|---|---|---|
| Early Power BI | `.pbix`, `.pbit` | Import-only, not definition-managed |
| Enterprise tabular | `model.bim` | `fabio semantic-model create --file model.bim` |
| Modern DevOps/Git | `.pbip`, `.pbir`, TMDL | `fabio semantic-model create --file *.tmdl`, `fabio report create/update-definition` |
| Paginated reporting | `.rdl` | `fabio item get-definition` (limited) |

### Key Constraints

- **Direct Lake requires TMDL**: `model.bim` cannot express `mode: directLake` partitions. Always use TMDL for Direct Lake.
- **model.bim requires V3**: `compatibilityLevel: 1604` and `defaultPowerBIDataSourceVersion: powerBI_V3` are mandatory.
- **PBIR cannot render data programmatically**: PBIR format visuals with `query.queryState` store correctly but display no data in the portal. Use PBIR-Legacy with `prototypeQuery` for programmatic report creation.
- **PBIR is the future**: PBIR will become the only supported format at GA. PBIR-Legacy is deprecated but still required for programmatic visual data rendering.
- **definition.pbir is always required**: Both PBIR and PBIR-Legacy reports need this file for semantic model binding.

## Power BI Report Definition Formats Reference

Power BI reports use one of two definition formats: **PBIR-Legacy** (single `report.json` file) or **PBIR** (individual files per visual/page in a `definition/` folder). Both formats use `definition.pbir` as the entry point for semantic model binding.

### Format Detection

The Fabric Items API returns the format in `getDefinition` response:
- `"format": "PBIR-Legacy"` → Single `report.json` contains all pages and visuals
- `"format": "PBIR"` → `definition/` folder with structured files per visual

New reports created in the Fabric Service default to PBIR. Existing reports are auto-converted to PBIR when edited in the Service (unless opted out via tenant setting). PBIR will become the only supported format at GA.

### definition.pbir (Common to Both Formats)

The `definition.pbir` file is **always required** and defines the semantic model binding. Two schema versions exist:

**Version 2 (Recommended for Fabric REST API deployments):**
```json
{
  "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/report/definitionProperties/2.0.0/schema.json",
  "version": "4.0",
  "datasetReference": {
    "byConnection": {
      "connectionString": "semanticmodelid=<SEMANTIC-MODEL-UUID>"
    }
  }
}
```
When deploying via Fabric REST API, only `semanticmodelid=<UUID>` is needed in `connectionString`. The server auto-resolves workspace/name.

**Version 1 (Legacy, full connection details):**
```json
{
  "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/report/definitionProperties/1.0.0/schema.json",
  "version": "4.0",
  "datasetReference": {
    "byConnection": {
      "connectionString": "Data Source=powerbi://api.powerbi.com/v1.0/myorg/<WorkspaceName>;initial catalog=\"<ModelName>\";integrated security=ClaimsToken;semanticmodelid=<UUID>",
      "pbiServiceModelId": null,
      "pbiModelVirtualServerName": "sobe_wowvirtualserver",
      "pbiModelDatabaseName": "<SEMANTIC-MODEL-UUID>",
      "connectionType": "pbiServiceXmlaStyleLive",
      "name": "EntityDataSource"
    }
  }
}
```

**Local path reference (PBIP only, not for API deployment):**
```json
{
  "version": "4.0",
  "datasetReference": {
    "byPath": {
      "path": "../Sales.Dataset"
    }
  }
}
```

| Version | Supported formats |
|---------|-------------------|
| 1.0     | PBIR-Legacy only (`report.json`) |
| 4.0+    | PBIR-Legacy (`report.json`) or PBIR (`definition/` folder) |

### PBIR-Legacy Format (`report.json`)

A single JSON file containing ALL report pages, visuals, filters, and formatting. Not publicly documented for editing — modifications may break on Desktop reload. Used by `fabio report update-definition --file <pbir> --report-json <report.json>`.

#### File Structure (API parts)
```
definition.pbir          # Semantic model binding (always required)
report.json              # All pages + visuals in one file
.platform                # Git integration metadata
```

#### report.json Top-Level Structure
```json
{
  "config": "<JSON-string: version, theme, activeSectionIndex>",
  "layoutOptimization": 0,
  "resourcePackages": [],
  "sections": [
    {
      "name": "ReportSection",
      "displayName": "Page Title",
      "displayOption": 1,
      "width": 1280.0,
      "height": 720.0,
      "ordinal": 0,
      "config": "<JSON-string: name, layouts>",
      "filters": "[]",
      "visualContainers": [ ... ]
    }
  ]
}
```

#### visualContainers[] Entry (PBIR-Legacy)
```json
{
  "x": 30.0,
  "y": 20.0,
  "z": 1000,
  "width": 250.0,
  "height": 110.0,
  "config": "<JSON-string: see Visual Config below>",
  "filters": "[]",
  "tabOrder": 0
}
```
- `x`, `y`: position on page canvas (pixels)
- `z`: stacking order (higher = on top)
- `width`, `height`: visual dimensions
- `config`: JSON-encoded string containing the visual definition
- `filters`: JSON-encoded array of visual-level filters
- `tabOrder`: keyboard navigation order

#### Visual Config Structure (PBIR-Legacy, inside `config` string)
```json
{
  "name": "unique_visual_name",
  "layouts": [{"id": 0, "position": {"x": 30, "y": 20, "z": 1000, "width": 250, "height": 110, "tabOrder": 0}}],
  "singleVisual": {
    "visualType": "barChart",
    "projections": {
      "Category": [{"queryRef": "TableName.columnName"}],
      "Y": [{"queryRef": "TableName.MeasureName"}]
    },
    "objects": {},
    "dataTransforms": {
      "projectionOrdering": {"Category": [0], "Y": [1]},
      "queryMetadata": {
        "Select": [
          {"Restatement": "columnName", "Name": "TableName.columnName", "Type": 1},
          {"Restatement": "MeasureName", "Name": "TableName.MeasureName", "Type": 2}
        ]
      },
      "selects": [
        {"displayName": "columnName", "queryName": "TableName.columnName", "roles": {"Category": true}, "type": {"category": null, "underlyingType": 1}},
        {"displayName": "MeasureName", "queryName": "TableName.MeasureName", "roles": {"Y": true}, "type": {"category": null, "underlyingType": 260}}
      ]
    }
  }
}
```

#### queryRef Format
- Columns: `TableName.columnName` (e.g., `Sales Summary.country`)
- Measures: `TableName.MeasureName` (e.g., `Sales Summary.Total Revenue`)
- Must match semantic model table/column/measure names exactly (case-sensitive)

#### dataTransforms Type Values
| Type | underlyingType | Description |
|------|---------------|-------------|
| 1    | 1             | Text/categorical (columns) |
| 2    | 260           | Numeric/measure/aggregate |

#### Projection Role Names by Visual Type
| visualType | Roles |
|------------|-------|
| `card` | `Values` (single measure or column) |
| `multiRowCard` | `Values` (multiple fields) |
| `barChart` | `Category` + `Y` |
| `columnChart` | `Category` + `Y` |
| `lineChart` | `Category` + `Y` (+ optional `Series`) |
| `pieChart` | `Category` + `Y` |
| `donutChart` | `Category` + `Y` |
| `tableEx` | `Values` (array of columns) |
| `matrix` | `Rows` + `Columns` + `Values` |
| `map` | `Category` (location) + `Size` + `Color` |
| `scatterChart` | `Category` + `X` + `Y` + `Size` |
| `slicer` | `Values` |
| `kpi` | `Indicator` + `TrendAxis` + `Goal` |

### PBIR Format (`definition/` folder)

A structured folder with individual JSON files per visual, page, and bookmark. Publicly documented with JSON schemas. Supports external editing and merge-friendly diffs.

#### File Structure (API parts)
```
definition.pbir                              # Semantic model binding
definition/
├── version.json                             # Required: PBIR version
├── report.json                              # Required: report-level settings
├── reportExtensions.json                    # Optional: report-level measures
├── pages/
│   ├── pages.json                           # Page ordering and active page
│   └── <pageName>/
│       ├── page.json                        # Required: page settings
│       └── visuals/
│           └── <visualName>/
│               ├── visual.json              # Required: visual definition
│               └── mobile.json              # Optional: mobile layout
└── bookmarks/
    ├── bookmarks.json                       # Bookmark ordering/groups
    └── <bookmarkName>.bookmark.json         # Individual bookmark state
.platform                                    # Git integration metadata
```

#### definition/version.json
```json
{
  "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/report/definition/versionMetadata/1.0.0/schema.json",
  "version": "4.0.0"
}
```
Note: `version` must match `^[1-9][0-9]*\.(0|[1-9][0-9]*)\.0$` (semver with trailing `.0`).

#### definition/report.json (PBIR — NOT the same as PBIR-Legacy report.json)
```json
{
  "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/report/definition/report/1.0.0/schema.json",
  "layoutOptimization": "None",
  "themeCollection": {
    "baseTheme": {
      "name": "CY24SU06",
      "reportVersionAtImport": "5.55",
      "type": "SharedResources"
    }
  },
  "annotations": [
    {"name": "defaultPage", "value": "<pageName>"}
  ]
}
```

#### definition/pages/pages.json
```json
{
  "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/report/definition/pagesMetadata/1.0.0/schema.json",
  "pageOrder": ["page1Name", "page2Name"],
  "activePageName": "page1Name"
}
```

#### definition/pages/<pageName>/page.json
```json
{
  "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/report/definition/page/1.2.0/schema.json",
  "name": "salesOverview",
  "displayName": "Sales Overview",
  "displayOption": "FitToPage",
  "height": 720,
  "width": 1280
}
```

**displayOption values**: `FitToPage`, `FitToWidth`, `ActualSize`

#### definition/pages/<pageName>/visuals/<visualName>/visual.json
```json
{
  "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/report/definition/visualContainer/2.0.0/schema.json",
  "name": "barByCountry",
  "position": {
    "x": 30,
    "y": 150,
    "z": 3000,
    "width": 580,
    "height": 380,
    "tabOrder": 2000
  },
  "visual": {
    "visualType": "barChart",
    "query": {
      "queryState": {
        "Category": {
          "projections": [
            {
              "field": {
                "Column": {"Expression": {"SourceRef": {"Entity": "Sales Summary"}}, "Property": "country"}
              },
              "queryRef": "Sales Summary.country"
            }
          ]
        },
        "Y": {
          "projections": [
            {
              "field": {
                "Measure": {"Expression": {"SourceRef": {"Entity": "Sales Summary"}}, "Property": "Total Revenue"}
              },
              "queryRef": "Sales Summary.Total Revenue"
            }
          ]
        }
      }
    }
  }
}
```

#### PBIR Field Expression Types (in `field` property)

**Column reference:**
```json
{"Column": {"Expression": {"SourceRef": {"Entity": "TableName"}}, "Property": "columnName"}}
```

**Measure reference:**
```json
{"Measure": {"Expression": {"SourceRef": {"Entity": "TableName"}}, "Property": "measureName"}}
```

**Aggregation (e.g., SUM of a column):**
```json
{"Aggregation": {"Expression": {"Column": {"Expression": {"SourceRef": {"Entity": "TableName"}}, "Property": "columnName"}}, "Function": 0}}
```
Aggregation Function values: 0=Sum, 1=Avg, 2=Count, 3=Min, 4=Max, 5=CountNonNull, 6=Median, 7=StandardDeviation, 8=Variance

#### PBIR Naming Convention
- Page/visual/bookmark folder names default to 20-char unique IDs (e.g., `90c2e07d8e84e7d5c026`)
- Can be renamed to human-friendly names (letters, digits, underscores, hyphens)
- The `name` property inside each JSON must match the folder name and be unique

#### PBIR Annotations
Custom name-value pairs for external tools (ignored by Power BI Desktop):
```json
"annotations": [{"name": "myCustomKey", "value": "myCustomValue"}]
```
Supported on `visual.json`, `page.json`, and `report.json`.

### Key Differences Between Formats

| Aspect | PBIR-Legacy | PBIR |
|--------|-------------|------|
| File structure | Single `report.json` | `definition/` folder tree |
| Visual definition | JSON string in `visualContainers[].config` | `visual.json` per visual |
| Field binding | `projections` + `dataTransforms` | `query.queryState` with semantic expressions |
| Schema validation | No public schema | Full JSON schemas with IntelliSense |
| External editing | Not supported (may break) | Officially supported |
| Merge conflicts | Entire report in one file | Per-visual file diffs |
| Size limits | N/A | 1000 pages, 1000 visuals/page, 300MB total |
| Future | Deprecated at GA | Only supported format at GA |
| API export format | Matches what's stored in service | Matches what's stored in service |

### Fabric REST API Usage

**Creating a report (both formats):**
```
POST /workspaces/{ws}/reports
Body: {"displayName": "My Report", "definition": {"parts": [...]}}
```

**Updating definition (both formats):**
```
POST /workspaces/{ws}/reports/{id}/updateDefinition
Body: {"definition": {"parts": [...]}}
```

Required parts depend on format:
- **PBIR-Legacy**: `definition.pbir` (always required) + `report.json`
- **PBIR**: `definition.pbir` + `definition/version.json` + `definition/report.json` + `definition/pages/pages.json` + page/visual files

**fabio CLI commands:**
```bash
# Create report bound to semantic model (auto-generates blank definition)
fabio report create --workspace $WS --name "My Report" --dataset $SEMANTIC_MODEL_ID

# Update with visuals (PBIR-Legacy)
fabio report update-definition --workspace $WS --id $REPORT_ID \
  --file definition.pbir --report-json report.json

# Get definition (returns format + all parts base64-encoded)
fabio report get-definition --workspace $WS --id $REPORT_ID
```

### JSON Schema URLs (PBIR)
- Visual container: `https://developer.microsoft.com/json-schemas/fabric/item/report/definition/visualContainer/2.0.0/schema.json`
- Visual configuration: `https://developer.microsoft.com/json-schemas/fabric/item/report/definition/visualConfiguration/2.0.0/schema-embedded.json`
- Page: `https://developer.microsoft.com/json-schemas/fabric/item/report/definition/page/1.2.0/schema.json`
- Semantic query: `https://developer.microsoft.com/json-schemas/fabric/item/report/definition/semanticQuery/1.2.0/schema.json`
- Report: `https://developer.microsoft.com/json-schemas/fabric/item/report/definition/report/1.0.0/schema.json`
- definition.pbir: `https://developer.microsoft.com/json-schemas/fabric/item/report/definitionProperties/2.0.0/schema.json`
- All schemas: `https://github.com/microsoft/json-schemas/tree/main/fabric/item/report/definition`

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
- **Report visuals are fully programmable**: CLI-created reports can include working visuals (cards, bar charts, tables) that render data — no portal interaction needed. The key requirement is including `prototypeQuery` in each visual's `singleVisual` config.
- **Semantic model ID links report to data**: The `definition.pbir` file's `pbiModelDatabaseName` field is the semantic model ID (UUID), not the display name.
- **End-to-end creation order**: Warehouse (data source) → Semantic Model (definition + connection) → Report (bound to semantic model). Each step depends on the previous item's ID.

## EventStream API Behaviors Discovered
- **Definition format**: `eventstream.json` contains the topology with `sources`, `destinations`, `streams`, `operators`, and `compatibilityLevel` fields. Separate `eventstreamProperties.json` controls retention and throughput.
- **Definition update is LRO**: `POST .../updateDefinition` returns 202 and requires polling. The response body after LRO completion is empty/null.
- **Source types**: `CustomEndpoint`, `AzureEventHub`, `AzureIoTHub`, `SampleData`, `AmazonKinesis`, `ApacheKafka`, `ConfluentCloud`, `GooglePubSub`, plus CDC types (`AzureSQLDBCDC`, `MySQLCDC`, `PostgreSQLCDC`) and Fabric events (`FabricWorkspaceItemEvents`, `FabricJobEvents`, `FabricOneLakeEvents`).
- **Destination types**: `Eventhouse`, `Lakehouse`, `CustomEndpoint`, `Activator`.
- **CustomEndpoint source exposes Event Hub-compatible interface**: Creates an Azure Event Hub-compatible endpoint. Connection info retrieved via `GET .../sources/{sourceId}/connection` returns `fullyQualifiedNamespace`, `eventHubName`, and `accessKeys` with SAS connection strings.
- **Eventhouse destination `itemId` is the KQL Database ID**: Despite documentation examples showing Eventhouse ID, the topology `itemId` field must be the **KQL Database item ID** (not the Eventhouse ID). Using the Eventhouse ID causes errors ("Unable to extract cluster URL from the Eventhouse KQL database item ID").
- **Two ingestion modes for Eventhouse destination**:
  - `ProcessedIngestion`: Auto-creates the destination table with extra system columns (`EventEnqueuedUtcTime`, `EventProcessedUtcTime`, `PartitionId`). Does NOT require pre-created table or mapping. Requires `inputSerialization` in properties.
  - `DirectIngestion`: Uses a pre-created KQL table and JSON mapping rule. Requires `connectionName` (arbitrary unique string) and `mappingRuleName`. Only maps fields defined in the mapping — no extra system columns.
- **DirectIngestion requires pre-created table + mapping**: Use `.create-merge table` and `.create-or-alter table ... ingestion json mapping` via `kql-database query` BEFORE configuring the destination.
- **Destination status transitions**: `Creating` → `Running` (or `Warning`). The `Warning` state appears when the Eventhouse ID is used instead of KQL Database ID. With correct KQL Database ID, destination transitions to `Running` within ~90 seconds.
- **Source status transitions**: `Creating` → `Running`. Custom Endpoint sources become Running quickly (~15-30 seconds).
- **Stream status**: Always shows `Created` (not `Running`). This is expected — streams are routing constructs, not active processes.
- **Graph-like topology**: Nodes reference each other by `name` via `inputNodes` arrays. A source feeds into a stream, which feeds into a destination or operator. The `name` field must be unique across all nodes (sources, destinations, streams, operators).
- **Default stream naming convention**: `{eventstream-name}-stream` for the default stream fed by the primary source.
- **No REST API for individual source/destination CRUD**: Sources and destinations can only be created/deleted via `update-definition` (full definition replacement). The individual `GET .../sources/{id}` and `GET .../destinations/{id}` endpoints are read-only.
- **`databaseName` field is optional in topology properties**: The server stores it but it's not required for either DirectIngestion or ProcessedIngestion. The `itemId` (KQL Database ID) is sufficient for routing.
- **`connectionName` for DirectIngestion**: Any unique string up to 40 characters. Recommended pattern: `es-eh-conn-{random4}`.
- **ProcessedIngestion auto-creates table**: When using ProcessedIngestion mode, the destination table (e.g., `SensorEvents2`) is automatically created in the KQL database when the first events flow through. No need to pre-create it.
- **Ingestion latency**: ProcessedIngestion: ~60 seconds from event send to queryable. DirectIngestion: ~60-90 seconds. Both modes batch events for efficiency.
- **Event Hub SDK for sending**: Use `azure-eventhub` Python SDK (or equivalent) with the SAS connection string from `get-source-connection`. Standard Event Hub producer pattern works.
- **Pause/Resume for stream control**: `POST .../pause` and `POST .../resume` control the entire eventstream. Individual sources/destinations can be paused/resumed independently.
- **`eventstreamProperties.json`**: Controls `retentionTimeInDays` (1-90, default 1) and `eventThroughputLevel` (`Low`, `Medium`, `High`). Optional in definition updates.
- **Compatibility level**: Current version is `"1.1"`. Always include it in the definition.
- **New commands added**: `fabio eventstream add-source` and `fabio eventstream add-destination` — high-level helpers that fetch current definition, merge in the new node, auto-create default streams, and push the updated definition. Simplifies agent workflow vs. manually crafting full definition JSON.

## RTI (Real-Time Intelligence) End-to-End Workflow
- **Creation order**: Workspace → Eventhouse → KQL Database (with `--eventhouse-id`) → EventStream → Configure topology (add-source + add-destination) → Send events → Query via KQL.
- **Required items**: Workspace (with Fabric capacity assigned), Eventhouse, KQL Database, EventStream.
- **Pre-requisites for DirectIngestion**: Create table schema and JSON ingestion mapping in KQL database BEFORE configuring the EventStream destination.
- **Querying EventStream data**: Query the KQL database directly using `fabio kql-database query`. The EventStream itself is not queryable — it's a routing/processing layer.
- **fabio commands for full RTI pipeline**:
  ```
  fabio workspace create --name "my-rti-workspace"
  fabio workspace assign-capacity --id <ws-id> --capacity <cap-id>
  fabio eventhouse create --workspace <ws-id> --name "MyEventhouse"
  fabio kql-database create --workspace <ws-id> --name "MyDB" --eventhouse-id <eh-id>
  fabio kql-database query --workspace <ws-id> --id <db-id> --kql ".create-merge table ..."
  fabio kql-database query --workspace <ws-id> --id <db-id> --kql ".create-or-alter table ... ingestion json mapping ..."
  fabio eventstream create --workspace <ws-id> --name "MyStream"
  fabio eventstream add-source --workspace <ws-id> --id <es-id> --name "app-source" --source-type CustomEndpoint
  fabio eventstream add-destination --workspace <ws-id> --id <es-id> --name "kql-dest" --destination-type Eventhouse --input-node "app-source-stream" --properties '{"dataIngestionMode":"DirectIngestion","workspaceId":"<ws-id>","itemId":"<kql-db-id>","tableName":"<table>","connectionName":"es-conn-1","mappingRuleName":"<mapping>"}'
  # Send events via Event Hub SDK using connection from:
  fabio eventstream get-source-connection --workspace <ws-id> --id <es-id> --source-id <src-id>
  # Query data:
  fabio kql-database query --workspace <ws-id> --id <db-id> --kql "MyTable | take 10"
  ```

## Graph Model API Behaviors Discovered
- **Job type for refresh is `RefreshGraph` (PascalCase)**: The Jobs API uses `?jobType=RefreshGraph` query parameter. The legacy path-based format (`/jobs/refreshGraph/instances`) returns `InvalidJobType`. Must use `POST /workspaces/{ws}/graphModels/{id}/jobs/instances?jobType=RefreshGraph`.
- **Execute query requires `?preview=true`**: The `executeQuery` endpoint requires `?preview=true` query parameter (NOT `?beta=true`). Without it, returns "InvalidParameter: 'preview' is a required parameter".
- **`getQueryableGraphType` also requires `?preview=true`**: Same pattern as executeQuery. Returns 204 No Content when graph has no queryable type (not yet loaded).
- **Fresh graph model only has `.platform` in definition**: A newly created graph model's `getDefinition` only returns the `.platform` metadata file. No `GraphModel.json` part exists until an ontology is linked.
- **Ontology linking via definition on creation**: Pass `GraphModel.json` part in the `definition` at creation time with `{"ontologyId": "<ontology-id>"}`. The API accepts this via LRO (202) but does NOT return the `GraphModel.json` part in subsequent `getDefinition` calls — the link is stored internally.
- **`updateDefinition` with `GraphModel.json` is silently accepted but not persisted**: The server accepts `updateDefinition` with arbitrary content in `GraphModel.json` but doesn't persist it in `getDefinition`. Ontology linking appears to be a creation-time-only operation through the definition.
- **`queryReadiness` field values**: `None` (no graph loaded), potentially `Ready` after successful refresh. Observed in `properties.queryReadiness`.
- **`lastDataLoadingStatus` field**: Contains `status` (`NotStarted`, `InProgress`, `Completed`, `Failed`), `lastUpdateTime`, and `jobInstanceId`. Null before first refresh.
- **Graph must be loaded before queries**: `executeQuery` on an unloaded graph returns error `GraphNotQueryable` with message `GraphIsNotLoaded`.
- **Graph model `show` includes properties**: Unlike many other item types, `GET /graphModels/{id}` returns `properties` with `queryReadiness` and `lastDataLoadingStatus`.
- **`--ontology` flag on create**: fabio wraps the ontology ID in a `GraphModel.json` definition part with `{"ontologyId":"<id>"}` and includes it in the creation request body.
- **Creation with definition is LRO**: When `definition` is included in the creation body, the API returns 202 and requires polling (unlike simple creation without definition which returns the object directly).
- **Refresh requires portal initialization (VersionConfig)**: Graph model refresh via REST API fails with `InternalError: "Job failed to start: VersionConfig does not exist or failed to retrieve ETag."` when the graph model has NOT been initialized through the Fabric portal. The REST API can create a graph model and link an ontology, but the internal loading infrastructure (`VersionConfig`) is only provisioned by the portal's graph editor. This is similar to Data Agent publishing being portal-only.
- **Refresh fails regardless of ontology state**: Even with a properly configured ontology (entity types + data bindings to lakehouse tables), the refresh fails if the graph has never been opened in the portal. Creating fresh graph models with `--ontology` pointing to a fully-bound ontology still produces the `VersionConfig` error.
- **Jobs API reveals actual failure**: The `show` command shows `lastDataLoadingStatus.status: "NotStarted"` even when the job has already `Failed`. Must check the Jobs API directly (`GET /jobs/instances/{jobId}`) to see the real status with `failureReason`.

## Graph Query Set API Behaviors Discovered
- **Definition file is `exportedDefinition.json`**: NOT `definition.json`. The definition uses `exportedDefinition.json` path with structure: `{"dependencies":[],"indirectDependencies":[],"ArtifactContents":[],"ConfigurationCategories":[]}`.
- **`exportedDefinition.json` is read-only (export only)**: The server accepts `updateDefinition` but consistently strips `ArtifactContents`, `dependencies`, and `ConfigurationCategories` values. The content always returns as empty arrays. Query set content is managed only through the portal UI.
- **PATCH update fails on empty query sets**: `PATCH /graphQuerySets/{id}` with `displayName` change returns `GraphQuerySetUpdate.UserError.GraphQuerySetEmpty: Query set payload is empty, cannot update artifact`. This is a server-side limitation — must have content before renaming.
- **Create returns item immediately**: Unlike graph models with definition, graph query set creation returns the item object directly (not LRO).
- **Delete works regardless of content**: Even empty query sets can be deleted successfully.
- **`getDefinition` is LRO**: Returns 202 and requires polling, same as other Fabric definition APIs.

## Map API Behaviors Discovered
- **Definition file is `map.json`**: NOT `definition.json`. The definition part path is `map.json` containing the full map configuration (basemap, data sources, layers).
- **Schema URL**: `https://developer.microsoft.com/json-schemas/fabric/item/map/definition/2.0.0/schema.json` — the current version is 2.0.0.
- **Definition structure**: `{"$schema":"...","basemap":{},"dataSources":[],"iconSources":[],"layerSources":[],"layerSettings":[]}`. A newly created map has all arrays empty and `basemap: {}`.
- **getDefinition is LRO**: Returns 202 and requires polling. Returns `map.json` + `.platform` parts.
- **updateDefinition returns item object**: Unlike other items that return null/empty on update, map `updateDefinition` returns the full item object (id, type, displayName, description, workspaceId).
- **Server adds `refreshIntervalMs: 0`**: Layer sources automatically get `refreshIntervalMs: 0` added if not specified.
- **Data source types**: `Lakehouse`, `KqlDatabase`, `Ontology` (workspace items with `itemType`, `workspaceId`, `itemId`) or `Connection` (with `connectionId`).
- **Layer source types**: `table` (for lakehouse Delta tables). References a data source via `itemId` and uses `relativePath` (e.g., `Tables/my_table`).
- **Layer settings options**: `type` (`vector` or `raster`), `pointLayerType` (`bubble`, `heatmap`, `marker`), with corresponding sub-options (`bubbleOptions`, `heatmapOptions`, `markerOptions`, `lineOptions`, `polygonOptions`, `polygonExtrusionOptions`).
- **Geospatial columns**: Layers reference geographic data via `latitudeColumnName`/`longitudeColumnName` (for point data) or `geometryColumnName` (for GeoJSON/WKT geometry columns). These appear at both the `layerSettings` level and inside `options`.
- **Bubble options for data-driven sizing**: Use `sizeType: "data-driven"` with `sizeProperty: "<column_name>"` to size bubbles proportional to a numeric column. `sizeType: "fixed"` with `fixedSize` for uniform sizing.
- **Basemap styles**: `road`, `satellite_road_labels`, `grayscale_light`, `grayscale_dark`, `night`, `road_shaded_relief`, `high_contrast_dark`, `high_contrast_light`.
- **Controls**: `zoom`, `pitch`, `compass`, `scale`, `traffic`, `style` — each boolean to enable/disable.
- **Filters support**: Layer settings support `filters` array with types: `text`, `boolean`, `number`, `datetime`. Each filter has an `id` (UUID), `field`, `locked` flag, and type-specific value fields.
- **Map visual IDs must be UUID format**: `layerSources[].id` and `layerSettings[].id` must be valid UUIDs.
- **Create is LRO**: Returns 202 and requires polling (item returned after LRO completes).
- **Conflict on duplicate names**: Creating a map with an existing name returns `409 Conflict` with message "Requested '<name>' is already in use".

## Reflex (Activator) API Behaviors Discovered
- **Definition file is `ReflexEntities.json`**: Contains a JSON array of entity objects. Empty reflex = `[]`.
- **Entity structure**: Each entity has `uniqueIdentifier` (GUID, required), `payload` (object, required), and `type` (string, required). Entities reference each other by `uniqueIdentifier`.
- **Entity types**: `container-v1`, `simulatorSource-v1`, `kqlSource-v1`, `realTimeHubSource-v1`, `eventstreamSource-v1`, `fabricItemAction-v1`, `timeSeriesView-v1`.
- **`timeSeriesView-v1` subtypes**: Determined by `payload.definition.type`: `Event`, `Object`, `Attribute`, `Rule`. This single entity type covers events, objects, attributes, and rules.
- **Processing pipeline hierarchy**: Container → Data Source → Event View → Object View → Attribute Views + Rule Views. Each entity references its parent via `payload.parentContainer.targetUniqueIdentifier` and (for attributes/rules) `payload.parentObject.targetUniqueIdentifier`.
- **`definition.instance` is a JSON-encoded string**: The `instance` field contains a stringified JSON template definition (not a nested object). Must be escaped when building the definition file.
- **Template structure**: `{"templateId":"<name>","templateVersion":"1.1","steps":[{"name":"<step>","id":"<guid>","rows":[{"name":"<row>","kind":"<kind>","arguments":[...]}]}]}`.
- **Template IDs for events**: `SourceEvent` (selects from data source), `SplitEvent` (splits by object identity).
- **Template IDs for attributes**: `IdentityPartAttribute` (object identity field), `IdentityTupleAttribute` (composite identity), `BasicEventAttribute` (extracts field value).
- **Template IDs for rules**: `EventTrigger` (fires on event occurrence), `AttributeTrigger` (fires on threshold condition).
- **Rule action types (in ActStep)**: `TeamsMessage` (Teams notification), `EmailMessage` (email notification), `FabricItemInvocation` (runs a Pipeline/Notebook).
- **TeamsMessage action arguments**: `messageLocale`, `recipients` (array), `headline` (array), `optionalMessage` (array), `additionalInformation` (array). All array values use `{"type":"string","value":"..."}` format.
- **EmailMessage action arguments**: `messageLocale`, `sentTo` (array), `copyTo` (array), `bCCTo` (array), `subject` (array), `headline` (array), `optionalMessage` (array), `additionalInformation` (array).
- **FabricItemInvocation action**: References a `fabricItemAction-v1` entity by `uniqueIdentifier`. The action entity defines `fabricItem.itemId`, `fabricItem.workspaceId`, `fabricItem.itemType`, and `jobType`.
- **Rule settings**: `definition.settings.shouldRun` (boolean, enables/disables rule), `definition.settings.shouldApplyRuleOnUpdate` (boolean, apply to historical data).
- **Simulator source types**: `PackageShipment` (with `version: "V2_0"`). Supports `runSettings.startTime` and `runSettings.stopTime` (ISO 8601).
- **KQL source**: Requires `query.queryString` (KQL), `eventhouseItem.targetUniqueIdentifier` (references Eventhouse item), and `runSettings.executionIntervalInSeconds`.
- **Real-time Hub source**: Requires `connection.scope`, `connection.tenantId`, `connection.workspaceId`, `connection.eventGroupType`, and `filterSettings.eventTypes[]`.
- **Eventstream source**: Requires `metadata.eventstreamArtifactId`.
- **updateDefinition does NOT accept `format` field**: Unlike `createItem` which accepts `"format": "json"` in the definition, `updateDefinition` rejects it with `InvalidDefinitionFormat`. Only send `{"definition":{"parts":[...]}}`.
- **updateDefinition returns 200 (not 202 LRO)**: For valid definitions, the API returns 200 immediately. Invalid content returns 400 with `Activator_Alm_GenericError` (500 from internal service).
- **`.platform` part is optional for updateDefinition**: Only `ReflexEntities.json` is required. `.platform` is accepted if `?updateMetadata=true` is set.
- **Container `type` field values**: `samples` (for simulator-based), `kqlQueries` (for KQL-based), and likely others for Real-time Hub and Eventstream containers.
- **AttributeTrigger rule steps**: `ScalarSelectStep` (selects attribute + summary), `ScalarDetectStep` (condition check), optional `DimensionalFilterStep` (filter by another attribute), `ActStep` (action to execute).
- **NumberBecomes operators**: `BecomesGreaterThan`, `BecomesLessThan`, `BecomesGreaterThanOrEqualTo`, `BecomesLessThanOrEqualTo`.
- **NumberSummary operators**: `Average`, `Min`, `Max`, `Sum`, `Count`.
- **TimeDrivenWindowSpec**: `width` and `hop` in milliseconds (e.g., 600000 = 10 minutes).
- **EventTrigger template step structure is undocumented**: The `EventTrigger` template requires an `EventSelector` row, but the correct step/row placement is not documented. Attempts with `EventDetectStep` + `EventSelector` (kind: `Event` or `EventSelector`) all fail with "Expected at least 1 occurrences of EventSelector, but got: 0". Microsoft docs recommend: "configure a Reflex in the Fabric UI, then use Get Item Definition to retrieve the definition." Use `AttributeTrigger` for programmatic rule creation (fully validated).
- **KQL source (`kqlSource-v1`) requires portal initialization**: Always fails via REST API with `Activator_Alm_UserError: "The importArtifactRequest field is required"`. Similar to Graph Model refresh requiring portal `VersionConfig` initialization. Configure KQL sources through the Fabric portal, then manage definitions programmatically afterward.
- **Real-time Hub event subscriptions create server-side state**: When a `realTimeHubSource-v1` is pushed via `updateDefinition`, the server creates an event subscription. If the Reflex is later updated without the same source (or with incorrect UUIDs), subsequent `updateDefinition` calls fail with "eventSubscriptions/{id} not found". Fix: delete the Reflex and create a fresh one.
- **Duplicate entity UUID tracking**: The server tracks entity UUIDs across definition updates. Reusing a UUID from a previously-deleted entity in the same Reflex causes "duplicate" errors. Always use fresh UUIDs when replacing entities.
- **Real-time Hub filter immutability**: Once an RTH source is created with specific `filterSettings.eventTypes`, the filters cannot be updated. The server returns: "Updating event subscription filters is not supported yet. Please create a new source." Must use a completely new `uniqueIdentifier` and fresh subscription.
- **Validated working pipeline patterns**:
  - Simulator source + AttributeTrigger + EmailMessage action (HTTP 200)
  - Simulator source + AttributeTrigger + TeamsMessage action (HTTP 200)
  - Real-time Hub source with workspace events (HTTP 200, creates subscription)
  - `updateDefinition` replaces entire entity set atomically (not incremental)

## Workspace API Behaviors Discovered
- **Endpoint scope**: All workspace operations are tenant-level at `/workspaces/{id}` (no parent scope).
- **Capacity assignment body**: `POST /workspaces/{id}/assignToCapacity` with `{"capacityId": "<id>"}`. Unassign uses empty body `{}` to `POST /workspaces/{id}/unassignFromCapacity`.
- **Capacity assignment is idempotent**: Re-assigning the same capacity succeeds without error.
- **Identity provisioning is LRO**: `POST /workspaces/{id}/provisionIdentity` uses `poll: true` (may return 202). Deprovision is fire-and-forget.
- **Role assignment validation**: Roles are case-insensitive against `["Admin", "Member", "Contributor", "Viewer"]`. Principal types: `["User", "Group", "ServicePrincipal", "ServicePrincipalProfile"]`.
- **Role assignment body**: `{"principal": {"id": "<principal_id>", "type": "<principal_type>"}, "role": "<role>"}`.
- **Folder management**: Workspaces support folders via `/workspaces/{ws}/folders` (CRUD + move). Move body: `{"targetFolderId": "<id>" | null}` (null moves to root).
- **Tags**: `POST /workspaces/{ws}/applyTags` and `/unapplyTags` with body `{"tagIds": [...]}`.
- **Domain assignment**: `POST /workspaces/{ws}/assignToDomain` with `{"domainId": "<id>"}`. Unassign uses empty body.
- **OneLake settings**: `GET /workspaces/{ws}/onelake/settings` returns tier, diagnostics, immutability. Modify via individual POST endpoints (`/modifyDefaultTier`, `/modifyDiagnostics`, `/modifyImmutabilityPolicy`).
- **Default tier values**: `"Hot"` or `"Cold"` (PascalCase).
- **Lifecycle policies**: Export/import via `/workspaces/{ws}/onelake/lifecycle/exportPolicy` and `/importPolicy`.
- **Network policy**: `GET/PUT /workspaces/{ws}/networking/communicationPolicy`.
- **Create body**: `{"displayName": "<name>", "description"?: "<desc>"}` — minimal, no capacity needed at creation time.

## Item API Behaviors Discovered
- **Type filter on list**: `GET /workspaces/{ws}/items?type={ItemType}` filters server-side. Type values are PascalCase (e.g., `Lakehouse`, `Notebook`, `Warehouse`).
- **Valid item types for create**: `CopyJob`, `Dashboard`, `DataAgent`, `DataPipeline`, `Dataflow`, `Environment`, `Eventhouse`, `Eventstream`, `GraphQLApi`, `KQLDashboard`, `KQLDatabase`, `KQLQueryset`, `Lakehouse`, `MLExperiment`, `MLModel`, `MirroredDatabase`, `MirroredWarehouse`, `Notebook`, `Ontology`, `Paginated Report`, `Reflex`, `Report`, `SQLDatabase`, `SQLEndpoint`, `SemanticModel`, `SparkJobDefinition`, `Warehouse`. Sorted, PascalCase. Hinted on invalid type errors.
- **Copy pattern**: `getDefinition` (LRO) from source → `GET` source metadata → `POST /workspaces/{dest}/items` with definition (LRO). Result includes new item's `id`, `displayName`, `type`.
- **Move pattern**: Copy + `DELETE /workspaces/{source}/items/{id}`. Atomic: delete only after successful copy.
- **Definition format query param**: `POST /workspaces/{ws}/items/{id}/getDefinition?format={fmt}` supports format selection.
- **Update definition metadata**: `POST /workspaces/{ws}/items/{id}/updateDefinition?updateMetadata=true` updates `.platform` metadata alongside definition parts.
- **Bulk operations (all LRO)**:
  - `POST /workspaces/{ws}/items/bulkExportDefinitions` — exports multiple item definitions
  - `POST /workspaces/{ws}/items/bulkImportDefinitions` — imports multiple item definitions
  - `POST /workspaces/{ws}/items/bulkMove` — moves multiple items between folders/workspaces
- **External data shares**: CRUD at `/workspaces/{ws}/items/{id}/externalDataShares`. Create body: `{"paths": [...], "recipient": {"tenantId": "<id>"}}`. Accept invitations at `/externalDataShares/invitations/{id}/accept`.
- **Identity assignment**: `POST /workspaces/{ws}/items/{id}/identities/default/assign`.
- **Tags**: `POST /workspaces/{ws}/items/{id}/applyTags` and `/unapplyTags` with `{"tagIds": [...]}`.

## Lakehouse API Behaviors Discovered
- **Load table format validation**: Only `"Csv"` and `"Parquet"` are valid (PascalCase). JSON is NOT supported by the Fabric REST API. Mode values: `"Overwrite"`, `"Append"` (PascalCase).
- **Load table body (Csv)**: `{"relativePath": "<path>", "pathType": "File", "mode": "Overwrite", "formatOptions": {"format": "Csv", "header": true, "delimiter": ","}}`. The `format` key is INSIDE `formatOptions` (discriminated union pattern).
- **Load table body (Parquet)**: `{"relativePath": "<path>", "pathType": "File", "mode": "Overwrite", "formatOptions": {"format": "Parquet"}}`. Do NOT include `header`/`delimiter` with Parquet — API rejects mixed format options.
- **Upload-table workflow**: Upload file to `Files/.staging/{filename}` → POST load-table → delete staging file (best-effort cleanup).
- **Table listing uses `"data"` key**: Unlike other list endpoints that use `"value"`, `GET /workspaces/{ws}/lakehouses/{id}/tables` returns `{"data": [...]}`.
- **Shortcut creation**: `POST /workspaces/{ws}/items/{id}/shortcuts` with body `{"name": "<name>", "path": "<target_path>", "target": {<target_type>: <target_config>}}`.
- **Bulk shortcut creation**: `POST /workspaces/{ws}/items/{id}/shortcuts/bulkCreate?shortcutConflictPolicy={policy}` with `{"createShortcutRequests": [...]}`. LRO.
- **Shortcut get/delete path**: `GET/DELETE /workspaces/{ws}/items/{id}/shortcuts/{path}/{name}` — path and name are URL path segments.
- **Enable schemas on create**: `{"displayName": "...", "creationPayload": {"enableSchemas": true}}` enables multi-schema lakehouse.
- **Sync algorithm**: Lists both source and destination from root (avoiding DFS virtual view doubling), builds file maps keyed by relative path, compares ETags (default) or Content-MD5 (`--checksum`), copies files with different/missing ETags, optionally deletes orphan files at destination (`--delete`).
- **Parallel execution**: All multi-file operations (upload, copy-file, move-file, delete-table, copy-table, move-table, sync) use concurrent execution with rate-limit retry.
- **Glob patterns**: Local globs via `glob::glob()`, remote globs via listing + pattern match, table globs via table list API + pattern match.
- **Materialized views**: `POST /workspaces/{ws}/lakehouses/{id}/jobs/refreshMaterializedLakeViews/instances` triggers refresh. Schedule management at `.../jobs/refreshMaterializedLakeViews/schedules`.
- **Table maintenance**: `POST /workspaces/{ws}/lakehouses/{id}/jobs/tableMaintenance/instances`.
- **Livy sessions**: `GET /workspaces/{ws}/lakehouses/{id}/livySessions` lists active sessions.
- **Get/Update definition**: LRO via `/workspaces/{ws}/lakehouses/{id}/getDefinition` and `/updateDefinition`.

## Notebook API Behaviors Discovered
- **Creation uses generic items endpoint**: `POST /workspaces/{ws}/items` with `{"type": "Notebook", "displayName": "...", "definition": {...}}`. NOT `/notebooks`.
- **Delete uses generic items endpoint**: `DELETE /workspaces/{ws}/items/{id}` (not `/notebooks/{id}`).
- **ipynb format**: Definition uses `"format": "ipynb"` with part path `notebook-content.py`. The payload is a base64-encoded Jupyter notebook JSON.
- **Cell source must be list of strings**: Each cell's `source` field is an array of strings (one per line with `\n` suffix), NOT a single string.
- **Lakehouse binding via `trident` metadata**: `--lakehouse` flag injects `metadata.trident.lakehouse` into the ipynb JSON with `default_lakehouse`, `default_lakehouse_name`, `default_lakehouse_workspace_id`, `known_lakehouses`.
- **Run mechanism**: `client.run_notebook(workspace, id)` → `POST /workspaces/{ws}/items/{id}/jobs/instances?jobType=RunNotebook`. Returns 202 + Location header with job instance URL.
- **Status polling (--wait)**: Polls `GET /workspaces/{ws}/items/{id}/jobs/instances/{job_id}` every 5 seconds. Default timeout 600s.
- **Terminal statuses**: `Completed`, `Failed`, `Cancelled`. Continue polling on `NotStarted`, `InProgress`, `Deduped`.
- **Failure info**: Extracted from `failureReason.message` in job instance response.
- **Cancel**: `POST /workspaces/{ws}/items/{id}/jobs/instances/{job_id}/cancel`.
- **Get job instance (beta)**: `GET /workspaces/{ws}/notebooks/{id}/jobs/execute/instances/{job_id}?beta=true` — uses notebook-specific path with beta flag.
- **Livy sessions**: `GET /workspaces/{ws}/notebooks/{id}/livySessions` lists active Livy sessions for a notebook.
- **Spark cold start**: First notebook run on small capacity can take 2-5 minutes to transition from `NotStarted` to `InProgress`.

## Environment API Behaviors Discovered
- **Staging/publish workflow**: Changes are staged first, then published as a separate step. All modifications go to staging area.
- **Publish is fire-and-forget**: `POST /workspaces/{ws}/environments/{id}/staging/publish` with empty body `{}`. Not LRO — returns immediately.
- **Cancel publish**: `POST /workspaces/{ws}/environments/{id}/staging/cancelPublish` with empty body.
- **Spark settings dual endpoints**: `GET .../sparkcompute` (published) vs `GET .../staging/sparkcompute` (pending changes). Update goes to staging: `PATCH .../staging/sparkcompute`.
- **Definition file**: Part path is `environment.metadata.json`.
- **Library management**: Published at `/libraries`, staging at `/staging/libraries`. Delete uses query param: `DELETE .../staging/libraries?libraryToDelete={name}`.
- **External libraries**: Export via `GET .../libraries/exportExternalLibraries`. Import via `POST .../staging/libraries/importExternalLibraries`. Remove via `POST .../staging/libraries/removeExternalLibrary` with `{"libraryToRemove": "<name>"}`.
- **Get/Update definition are LRO**: Both use `poll: true`.
- **Create is LRO**: Returns 202, requires polling.

## Mirrored Database API Behaviors Discovered
- **Definition file**: Part path is `mirroring.json`.
- **Start/stop mirroring**: `POST /workspaces/{ws}/mirroredDatabases/{id}/startMirroring` and `/stopMirroring` with empty body `{}`. Fire-and-forget (no LRO).
- **Status endpoints use GET (not POST)**: Despite verb-like paths, `GET .../getMirroringStatus` and `GET .../getTablesMirroringStatus` are GET requests.
- **Create uses type-specific endpoint**: `POST /workspaces/{ws}/mirroredDatabases` (not generic `/items`). No `"type"` field needed in body — endpoint implies type.
- **Create is LRO**: Returns 202, requires polling.
- **Get/Update definition are LRO**: Both use `poll: true`.

## Deployment Pipeline API Behaviors Discovered
- **Tenant-level scope**: All endpoints use `/deploymentPipelines/{id}` (NO `/workspaces/` prefix). Pipelines are not workspace-scoped.
- **Deploy body**: `{"sourceStageId": "<id>", "targetStageId"?: "<id>", "items"?: [...], "note"?: "<text>"}`. `targetStageId` optional (defaults to next stage). `items` optional (defaults to all items).
- **Deploy is LRO**: `POST /deploymentPipelines/{id}/deploy` with `poll: true`. May return empty/null response (treated as "accepted").
- **Items array format**: `[{"itemId": "...", "itemType": "Notebook"}]` — PascalCase item types.
- **Stage management**: `GET .../stages` lists stages. `GET .../stages/{stageId}/items` lists items in stage. Items have `itemDisplayName`, `itemId`, `itemType` fields.
- **Workspace assignment**: `POST .../stages/{stageId}/assignWorkspace` with `{"workspaceId": "<id>"}`. Unassign uses empty body.
- **Operations history**: `GET .../operations` lists past deployments. `GET .../operations/{opId}` shows details.
- **Role assignments**: `GET/POST .../roleAssignments`. Delete uses principal ID: `DELETE .../roleAssignments/{principalId}`.
- **Role assignment body**: `{"principal": {"id": "<id>", "type": "<type>"}, "role": "<role>"}`.
- **Permissions**: Deploy requires "Contributor"; all other mutations require "Admin".

## Domain API Behaviors Discovered
- **Admin scope**: All domain endpoints use `/admin/domains/{id}` prefix. Requires admin privileges.
- **Batch workspace assignment**: `POST /admin/domains/{id}/assignWorkspaces` with `{"workspacesIds": [...]}`. Unassign uses same pattern at `/unassignWorkspaces`.
- **Assign by capacity**: `POST /admin/domains/{id}/assignWorkspacesByCapacities` with `{"capacitiesIds": [...]}`.
- **Assign by principal**: `POST /admin/domains/{id}/assignWorkspacesByPrincipals` with body containing principals array and `type` field.
- **List domain workspaces**: `GET /admin/domains/{id}/workspaces` returns workspaces associated with domain.
- **Create body**: `{"displayName": "<name>", "description"?: "<desc>"}`.
- **Update uses PATCH**: `PATCH /admin/domains/{id}` with `{"displayName"?: "...", "description"?: "..."}`.

## Connection API Behaviors Discovered
- **Tenant-level scope**: All connection endpoints use `/connections/{id}` (no workspace prefix). Connections are shared across workspaces.
- **Connectivity types**: `ShareableCloud`, `OnPremises`, `VirtualNetworkGateway`, `PersonalCloud`.
- **Credential types**: `Basic`, `OAuth2`, `Key`, `Anonymous`, `ServicePrincipal`, `SharedAccessSignature`.
- **Privacy levels**: `None`, `Public`, `Organizational`, `Private`.
- **Parameters format conversion**: User provides JSON object `{"key": "value"}` which is converted to array format `[{"dataType": "Text", "name": "key", "value": "value"}]` for the API.
- **Create body structure**: `{"displayName": "...", "connectivityType": "...", "connectionDetails": {"type": "...", "creationMethod": "...", "parameters": [...]}, "credentialDetails": {"singleSignOnType": "None", "connectionEncryption": "NotEncrypted", "skipTestConnection": bool, "credentials": {"credentialType": "..."}}, "privacyLevel": "..."}`.
- **Test connection**: `POST /connections/{id}/testConnection` with empty body `{}`.
- **Role assignments**: Full CRUD at `/connections/{id}/roleAssignments/{assignmentId}`. Roles: `Owner`, `User`, `UserWithReshare`.
- **Role assignment body**: `{"principal": {"id": "...", "type": "User|Group|ServicePrincipal"}, "role": "Owner|User|UserWithReshare"}`.
- **List supported types**: `GET /connections/supportedConnectionTypes` returns all available connection type definitions.

## Spark API Behaviors Discovered
- **Workspace-level settings**: `GET/PATCH /workspaces/{ws}/spark/settings`.
- **Workspace pools**: CRUD at `/workspaces/{ws}/spark/pools/{poolId}`.
- **Capacity-level settings (beta)**: `GET/PATCH /capacities/{capId}/spark/settings?beta=true`.
- **Capacity pools (beta)**: CRUD at `/capacities/{capId}/spark/pools/{poolId}?beta=true`.
- **Livy sessions**: `GET /workspaces/{ws}/spark/livySessions` and `GET .../livySessions/{id}`.
- **Pool create body**: Accepts JSON from `--file` or `--content` with pool configuration (name, node size, auto-scale settings, dynamic executor allocation).
- **Settings update**: PATCH with JSON body from `--file` or `--content`.
- **Beta flag required for capacity-level operations**: All capacity-scoped Spark endpoints require `?beta=true` query parameter.

## Spark Job Definition API Behaviors Discovered
- **Definition file**: Uses type-specific endpoint `/workspaces/{ws}/sparkJobDefinitions/{id}/getDefinition` and `/updateDefinition`.
- **Run job type**: `POST /workspaces/{ws}/items/{id}/jobs/instances?jobType=sparkjob` (lowercase `sparkjob`).
- **Create is LRO**: `POST /workspaces/{ws}/sparkJobDefinitions` with `poll: true`.
- **Get/Update definition are LRO**: Both use `poll: true`.
- **Definition format**: JSON content with Spark job configuration (main file path, arguments, language, etc.).

## Data Pipeline API Behaviors Discovered
- **Run job type**: `POST /workspaces/{ws}/items/{id}/jobs/instances?jobType=Pipeline` (PascalCase `Pipeline`).
- **Definition file**: Uses `/workspaces/{ws}/dataPipelines/{id}/getDefinition` and `/updateDefinition`. Both LRO.
- **Schedule management**: `POST /workspaces/{ws}/dataPipelines/{id}/jobs/execute/schedules` creates a schedule. Note: uses `/jobs/execute/schedules` (not `/jobs/Pipeline/schedules`).
- **Create is LRO**: `POST /workspaces/{ws}/dataPipelines` with `poll: true`.

## KQL Database API Behaviors Discovered
- **Query endpoint routing**: Management commands (starting with `.`) use `/v1/rest/mgmt`; data queries use `/v2/rest/query`. Both at the Kusto query URI.
- **Query body**: `{"db": "<database_name>", "csl": "<kql_text>"}`.
- **Token scoping**: Acquires token scoped to `{kusto_uri}/.default` (not the standard Fabric scope).
- **Query URI resolution priority**: `properties.queryServiceUri` → `properties.queryUri` → `properties.databaseUrl` → `--query-uri` override. Falls back to error with hint.
- **Database name**: Uses `displayName` from the KQL database item metadata.
- **V1 response format**: `{"Tables": [{"TableName": "...", "Columns": [...], "Rows": [[...], ...]}]}`. Uses first table as primary result.
- **V2 response format**: Array of frames. Finds `DataTable` frame with `TableKind: "PrimaryResult"`. Checks `DataSetCompletion` frame for `HasErrors`.
- **Shortcuts**: `GET /workspaces/{ws}/items/{id}/shortcuts` lists shortcuts on KQL databases.
- **Create types**: `ReadWrite` and `ReadOnlyFollowing`. ReadWrite requires `--eventhouse-id` in creation payload. ReadOnlyFollowing requires source database reference.
- **Get/Update definition are LRO**: Both use `poll: true` at type-specific endpoints.

## OneLake Security API Behaviors Discovered
- **Upsert-all pattern**: `PUT /workspaces/{ws}/items/{id}/dataAccessRoles` replaces ALL roles atomically. There is no individual role create/update endpoint.
- **Delete pattern**: GET all roles → filter out target role → PUT remaining roles back. Errors if role not found.
- **Show pattern**: GET all roles → find by name (client-side filter). No server-side individual GET.
- **Body format**: PUT body is the complete array of role definitions. Each role has `name` and members/permissions.
- **No individual role endpoints**: All CRUD operations go through the same PUT endpoint with the full role set.

## Managed Private Endpoint API Behaviors Discovered
- **Create body**: `{"name": "<endpoint_name>", "privateLinkResourceId": "<ARM_resource_id>", "groupId": "<subresource_type>", "requestMessage"?: "<approval_message>"}`.
- **Group ID values**: `blob`, `sqlServer`, `dfs`, `queue`, etc. (maps to Azure resource sub-resource types).
- **Create is LRO**: Returns 202, requires polling.
- **No update**: Endpoints are immutable after creation. Only create and delete.
- **Response status fields**: `provisioningState` and `connectionState` track endpoint lifecycle.
- **Requires Admin role**: All mutations require workspace Admin.

## Capacity API Behaviors Discovered
- **Read-only resource**: Only `list` and `show` operations. No create/update/delete via this endpoint.
- **Tenant-level scope**: `GET /capacities` (no workspace context). Individual: `GET /capacities/{id}`.
- **Response fields**: `displayName`, `id`, `sku`, `region`, `state`.
- **State values**: Includes `Active`, `Inactive`. Used to validate capacity availability before workspace assignment.

## Job Scheduler API Behaviors Discovered
- **Generic item-scoped**: All endpoints use `/workspaces/{ws}/items/{id}/jobs/...` pattern (works for any item type).
- **Job type required**: Most endpoints include `{job_type}` in path: `/jobs/{job_type}/schedules`.
- **Run on demand**: `POST /workspaces/{ws}/items/{id}/jobs/instances?jobType={job_type}` with optional body.
- **Cancel**: `POST /workspaces/{ws}/items/{id}/jobs/instances/{instance_id}/cancel`.
- **Schedule CRUD**: At `/workspaces/{ws}/items/{id}/jobs/{job_type}/schedules/{schedule_id}`.
- **Create schedule body**: Includes `enabled`, `configuration` with cron or interval settings.
- **Known job types**: Vary by item type — `RunNotebook`, `Pipeline`, `sparkjob`, `RefreshGraph`, `refreshMaterializedLakeViews`, `tableMaintenance`, etc.

## Copy Job API Behaviors Discovered
- **Definition file**: Part path is `CopyJobV1.json`.
- **Create is LRO**: `POST /workspaces/{ws}/copyJobs` with `poll: true`.
- **Get/Update definition are LRO**: Both use `poll: true`. Get Definition sends empty body `{}`.
- **Required roles**: Create/Delete require "Member"; Update/Definition require "Contributor".

## Dataflow API Behaviors Discovered
- **Definition file**: Part path is `dataflow.json`.
- **Create is LRO**: `POST /workspaces/{ws}/dataflows` with `poll: true`.
- **Get/Update definition are LRO**: Both use `poll: true`. Get Definition sends empty body `{}`.
- **Required roles**: Create/Delete require "Member"; Update/Definition require "Contributor".
- **Identical structure to Copy Job**: Same LRO patterns, same role requirements, different definition file name.

## SQL Database API Behaviors Discovered
- **Creation modes**: `New` (fresh database), `Restore` (point-in-time restore from existing), `RestoreDeletedDatabase` (restore from deleted). Each mode has different `creationPayload` fields.
- **Create body (New)**: `{"displayName": "...", "creationPayload": {"creationMode": "New", "backupRetentionDays": 7, "collation": "..."}}`.
- **Restore body**: Requires `restorePointInTime` (ISO 8601) and `sourceDatabaseReference` with `workspaceId` + `id`.
- **Hard delete**: `DELETE /workspaces/{ws}/sqlDatabases/{id}?hardDelete=true` permanently removes (vs soft delete for restore).
- **List deleted**: `GET /workspaces/{ws}/sqlDatabases/restorableDeletedDatabases` lists soft-deleted databases available for restore.
- **TDS connection resolution**: `GET /workspaces/{ws}/sqlDatabases/{id}` → extracts `properties.serverFqdn` (may include port as `host,1433`) and `properties.databaseName` (falls back to `displayName`).
- **SQL auth token**: Uses `client.require_sql_auth()` for SQL-scoped AAD token.
- **Connection string output**: `Server=tcp:{server},{port};Initial Catalog={database};Encrypt=True;TrustServerCertificate=False;Authentication=ActiveDirectoryDefault`.
- **Import type inference**: `Unknown` → first non-empty observation sets type → subsequent observations widen (Int→BigInt→Float→NVarChar, never narrows). JSON number with i32 fit → Int, else BigInt. Strings try parse order: Int→BigInt→Float→Bit→Date→NVarChar(len).
- **Import SQL generation**: `CREATE TABLE [dbo].[{name}] (...)` with nullable columns. Batched `INSERT INTO ... VALUES` (default batch_size=100, 120s timeout per batch). Optional `DROP TABLE IF EXISTS`.
- **NVarChar length calculation**: `clamp(observed_max_len * 2, 50, 4000)` — doubles observed length with floor/ceiling.
- **Mirroring support**: `POST .../startMirroring` and `POST .../stopMirroring` (same pattern as Mirrored Database).
- **Audit settings**: `GET/PATCH .../settings/sqlAudit`. Body: `{"state": "Enabled|Disabled", "retentionDays": N, "auditActionsAndGroups": [...], "predicateExpression": "..."}`.
- **Definition formats**: Supports `dacpac` and `sqlproj` via `?format={fmt}` query parameter.
- **Revalidate CMK**: `POST .../revalidateCMK` (LRO) — revalidates customer-managed key encryption.
- **F4+ capacity requirement**: SQL Database TDS connections require F4+ capacity. F2 fails with error 18456 State 240.

## KQL Dashboard API Behaviors Discovered
- **Definition file**: Part path is `RealTimeDashboard.json`.
- **Endpoint pattern**: Standard CRUD at `/workspaces/{ws}/kqlDashboards/{id}`.
- **Get/Update definition are LRO**: Both use `poll: true` at type-specific endpoints.
- **Create is LRO**: `POST /workspaces/{ws}/kqlDashboards` with `poll: true`.

## ML Model API Behaviors Discovered
- **CRUD only**: No definition support (no getDefinition/updateDefinition).
- **Endpoint pattern**: Standard at `/workspaces/{ws}/mlModels/{id}`.
- **Create body**: `{"displayName": "...", "description"?: "..."}`.
- **Create is LRO**: Returns 202, requires polling.

## ML Experiment API Behaviors Discovered
- **CRUD only**: No definition support (no getDefinition/updateDefinition).
- **Endpoint pattern**: Standard at `/workspaces/{ws}/mlExperiments/{id}`.
- **Create body**: `{"displayName": "...", "description"?: "..."}`.
- **Create is LRO**: Returns 202, requires polling.

## Common API Patterns Across All Command Groups
- **List pagination**: All list endpoints use `get_list()` with `"value"` key (except lakehouse tables which use `"data"`). Supports `--all` (fetches all pages), `--continuation-token` (resumes from token), `--limit` (client-side truncation).
- **Create responses**: Return the created object with at minimum `id`, `displayName`, `type`.
- **Delete responses**: Return `{"status": "deleted", "id": "<id>"}`.
- **Update validation**: All update commands require at least one field (`--name` or `--description`). Fail with `INVALID_INPUT` if neither provided.
- **LRO standard pattern**: POST returns 202 + `Location` header. Poll every 2s, max 120s. Terminal: `status == "Succeeded"` or `"Failed"`.
- **Error enrichment**: All commands use `enrich_forbidden()` to add required role hints on 403 errors. Not-found errors include `fabio <group> list` suggestions.
- **Dry-run guard**: All mutations support `--dry-run` which returns the planned request body without executing. Output: `{"status": "dry_run", "message": "Would <action>..."}`.
- **Definition operations pattern**: `POST .../getDefinition` (LRO, empty body `{}`) returns base64-encoded parts. `POST .../updateDefinition` (LRO) accepts `{"definition": {"parts": [{"path": "<file>", "payload": "<base64>", "payloadType": "InlineBase64"}]}}`.
- **Tenant-level vs workspace-scoped resources**:
  - Tenant-level (no workspace prefix): `/capacities`, `/connections`, `/deploymentPipelines`, `/admin/domains`, `/externalDataShares/invitations`
  - Workspace-scoped: All other resources at `/workspaces/{ws}/<resource>`

