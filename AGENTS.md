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
- **Full Rust implementation** (130+ commands across 25 groups): auth, workspace, item, lakehouse, capacity, notebook, warehouse, data-agent, ontology, environment, data-pipeline, eventhouse, kql-database, mirrored-database, spark, git, connection, deployment-pipeline, domain, job-scheduler, onelake-security, managed-private-endpoint, profile, jobs, feedback + agent-context
- Core output system: JSON envelope (`{"data":..., "count":N}` or `{"error":{"code":...,"message":...}}`), table, plain formats
- Structured error system: `ErrorCode` enum (AUTH_REQUIRED, NOT_FOUND, RATE_LIMITED, CAPACITY_INACTIVE, API_ERROR, TIMEOUT, etc.) + `FabioError`
- Global options fully wired: `--output/-o`, `--query/-q` (dot-notation field extraction), `--quiet` (suppresses stdout), `--profile`, `--dry-run`, `--limit`
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
- **Git integration**: status, commit, pull, connect, disconnect, initialize, switch (branch), connection/credentials management
- **Ontology management**: list, show, create, update, delete, get-definition, update-definition (RDF file support)
- **Environment**: list, show, create, update, delete, publish, cancel-publish, get-spark-settings, get-staging-spark-settings
- **Data Pipeline**: list, show, create, update, delete, run (triggers Pipeline job)
- **Eventhouse**: list, show, create, update, delete
- **KQL Database**: list, show, create, update, delete, get-definition, update-definition (ReadWrite/ReadOnlyFollowing)
- **Mirrored Database**: list, show, create, update, delete, get/update-definition, start, stop, status, table-status
- **Capacity**: list, show (inspect available capacities)
- **Connection**: list, show, create, update, delete, list-supported-types
- **Deployment Pipeline**: list, show, create, update, delete, list-stages, list-stage-items, assign-workspace, unassign-workspace, deploy
- **Domain**: list, show, create, update, delete, list-workspaces, assign-workspaces, unassign-workspaces, assign-by-capacity, assign-by-principal
- **Job Scheduler**: list-instances, get-instance, run-on-demand, cancel-instance, list-schedules, get-schedule, create-schedule, update-schedule, delete-schedule
- **Spark**: get-settings, update-settings, list-pools, get-pool, create-pool, update-pool, delete-pool
- **OneLake Security**: list, show, upsert, delete (data access roles for row/column-level security)
- **Managed Private Endpoint**: list, show, create, delete (workspace private networking)
- **Agent-native compliance** (all 10 principles implemented):
  - Principle 1: Non-interactive by default
  - Principle 2: Structured parseable output
  - Principle 3: Errors that teach and enumerate
  - Principle 4: Safe retries (`--dry-run`)
  - Principle 5: Bounded responses (`--limit`)
  - Principle 6: Consistent vocabulary (list/show/create/delete/copy/move)
  - Principle 7: `fabio agent-context` machine-readable schema
  - Principle 8: Async-aware (`--wait`, jobs ledger)
  - Principle 9: Named profiles (`fabio profile save/use/list/show/delete`)
  - Principle 10: Two-way I/O (`fabio feedback send/list`)
- **310 Rust tests** (54 unit + 256 E2E integration), zero clippy warnings, rustfmt clean
- **CI/CD**: GitHub Actions (6-target matrix: x64+arm64 for linux/macos/windows), Dependabot auto-merge, CodeQL, Secret Scanning
- **Release workflow**: Triggered on tags, builds 6 binaries, publishes GitHub Release with SHA256 checksums
- Release binary: ~5 MB, stripped, LTO-optimized

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
- **Server-side copy**: OneLake Blob API supports `PUT` with `x-ms-copy-source`; returns 202 with pending status. Poll via HEAD.
- **No native move/rename**: OneLake rejects `x-ms-rename-source`. Move = copy + delete.
- **Table file listing**: Must list from root (no `directory` param) to get real paths prefixed with item ID.
- **Recursive delete**: DFS `DELETE /{ws}/{lh}/Tables/{name}?recursive=true` works for directories.
- All destructive actions use consistent verb `delete` (not `remove`)
- Cross-workspace ops use `--source-workspace`/`--dest-workspace` with `visible_alias` short forms
- Auth relies on `DefaultAzureCredential` chain (az login, environment, managed identity)
- `azure_identity`/`azure_core` with `default-features = false` (no OpenSSL dependency)
- `unsafe_code = "forbid"` in lints

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

## Relevant Files
- `Cargo.toml`: Project config, dependencies, clippy/lints config, release profile (LTO+strip)
- `rust-toolchain.toml`: stable channel, rustfmt+clippy components
- `src/main.rs`: Entry point, tokio async main, error handling dispatch
- `src/cli.rs`: Clap derive CLI definition, OutputFormat enum, Command enum with 25 subcommand groups
- `src/errors.rs`: ErrorCode enum + FabioError struct with thiserror
- `src/output.rs`: render_list, render_object, render_error (respects --quiet/--query), apply_query, unit tests
- `src/client.rs`: FabricClient with async HTTP (get/post/put/patch/delete), LRO polling, OneLake DFS/Blob ops, run_notebook
- `src/commands/mod.rs`: Command dispatch
- `src/commands/auth.rs`: login/logout/status (DefaultAzureCredential chain)
- `src/commands/workspace.rs`: list/show/create/delete/assign-capacity
- `src/commands/item.rs`: list/show/create/delete/copy/move
- `src/commands/lakehouse.rs`: 15 subcommands (tables, files, upload, download, load-table, copy-file, delete-file, move-file, delete-table, copy-table, move-table, sync, create-shortcut, get-shortcut, delete-shortcut)
- `src/commands/notebook.rs`: create/get-definition/run (with --wait/--timeout)/status/stop/delete
- `src/commands/warehouse.rs`: list/show/create/update/delete/query (endpoint resolved, stdin/file/flag SQL input)
- `src/commands/dataagent.rs`: list/show/create/update/delete/query
- `src/commands/git.rs`: status/commit/pull/connect/disconnect/initialize/switch/connection/credentials
- `src/commands/ontology.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/environment.rs`: list/show/create/update/delete/publish/cancel-publish/get-spark-settings/get-staging-spark-settings
- `src/commands/data_pipeline.rs`: list/show/create/update/delete/run
- `src/commands/eventhouse.rs`: list/show/create/update/delete
- `src/commands/kql_database.rs`: list/show/create/update/delete/get-definition/update-definition
- `src/commands/mirrored_database.rs`: list/show/create/update/delete/get-definition/update-definition/start/stop/status/table-status
- `src/commands/capacity.rs`: list/show
- `src/commands/connection.rs`: list/show/create/update/delete/list-supported-types
- `src/commands/deployment_pipeline.rs`: list/show/create/update/delete/list-stages/list-stage-items/assign-workspace/unassign-workspace/deploy
- `src/commands/domain.rs`: list/show/create/update/delete/list-workspaces/assign-workspaces/unassign-workspaces/assign-by-capacity/assign-by-principal
- `src/commands/job_scheduler.rs`: list-instances/get-instance/run-on-demand/cancel-instance/list-schedules/get-schedule/create-schedule/update-schedule/delete-schedule
- `src/commands/spark.rs`: get-settings/update-settings/list-pools/get-pool/create-pool/update-pool/delete-pool
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
- `tests/e2e_dataagent.rs`: Data agent tests
- `tests/e2e_git.rs`: Git command group tests
- `tests/e2e_ontology.rs`: Ontology CRUD + definition tests
- `tests/e2e_agent_native.rs`: Agent-native compliance tests (principles 1-10)
- `tests/e2e_sync.rs`: Lakehouse sync tests
- `tests/e2e_connection.rs`: Connection CRUD + list-supported-types tests
- `tests/e2e_environment.rs`: Environment CRUD tests
- `tests/e2e_data_pipeline.rs`: Data pipeline CRUD + run tests
- `tests/e2e_eventhouse.rs`: Eventhouse CRUD tests
- `tests/e2e_kql_database.rs`: KQL database tests
- `tests/e2e_mirrored_database.rs`: Mirrored database tests
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

## Next Steps
- Add ODBC support to warehouse query (`odbc-api` crate)
- Add `warehouse create`/`warehouse delete` commands
- Add `notebook update` command (update definition)
- Add pagination support for list commands (continuationToken)
- Consider adding `--filter` flag for list commands
