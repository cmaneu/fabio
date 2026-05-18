# Fabio CLI - Session Context

## Goal
- Design and implement an agent-first CLI (`fabio`) to manage Microsoft Fabric artifacts and data, inspired by AWS/gcloud/Azure CLI principles, with structured JSON output, composability via stdin/stdout, and machine-readable errors.

## Constraints & Preferences
- CLI designed for AI agents first (structured output, no interactive prompts, explicit params)
- JSON output by default with `--output json|table|plain` flag
- Composable: manage inputs and produce outputs for piping
- Machine-readable error codes in structured JSON envelope
- Python 3.10+, uses Click, azure-identity, requests, rich
- Linting: ruff (line-length 99), mypy strict, pytest
- venv at `.venv/`, run commands with `.venv/bin/python3`, `.venv/bin/pytest`, etc.

## Progress
### Done
- Core output system: JSON envelope (`{"data":..., "count":N}` or `{"error":{"code":...,"message":...}}`), table, plain formats
- Structured error system: `ErrorCode` enum (AUTH_REQUIRED, NOT_FOUND, RATE_LIMITED, CAPACITY_INACTIVE, etc.) + `FabioError` exception
- `FabioGroup` custom Click group catches `FabioError` and renders to stderr
- Global options: `--output/-o`, `--query/-q`, `--quiet`
- Client: `get/post/patch/delete`, `upload_onelake_file`, `download_onelake_file`, `load_table`, `get_item_definition`, `update_item_definition`
- **LRO polling**: `_poll_lro()` handles 202 responses by polling `Location` header until Succeeded/Failed; used by `post(path, body, poll=True)`
- **CAPACITY_INACTIVE** error code: detects `CapacityNotActive` in response body/errorCode, returns clear message
- **Device-code login fix**: `prompt_callback` prints code/URL to stderr
- **Empty response handling**: `_handle_response` handles 200 with empty body (e.g. assignToCapacity)
- Commands: `workspace list/show/create/delete/assign-capacity`, `item list/show/create/delete/copy`, `lakehouse tables/files/upload/download/load-table/copy-file/delete-file/move-file/delete-table/copy-table/move-table/create-shortcut/get-shortcut/delete-shortcut`, `notebook create/get-definition/run/status/stop/delete`, `auth login/logout/status`
- **Server-side file copy**: Uses OneLake Blob API (`PUT` with `x-ms-copy-source` header), async with polling
- **Server-side file move**: copy + DFS DELETE (OneLake rejects `x-ms-rename-source`)
- **Server-side table copy/move/delete**: Lists table files via root filesystem listing, copies each file via Blob API, deletes via recursive DFS DELETE
- **Shortcuts**: Create/get/delete OneLake, ADLS Gen2, and S3 shortcuts
- **load-table fixes**: PascalCase mode (`Overwrite`/`Append`), `formatOptions.format` key, LRO polling enabled
- **Notebook ipynb fix**: `source` field as list of strings per ipynb spec (Fabric rejects single string)
- **item copy + notebook create**: LRO-enabled (`poll=True`)
- 155 tests passing, ruff + format clean
- All pushed to main (latest commit `4251767`)
- **End-to-end workflow verified** against live Fabric tenant:
  - Created `fabio-demo-source` workspace → SalesLakehouse → uploaded CSV → loaded Delta table → created SalesAnalysis notebook
  - Created `fabio-demo-dest` workspace → SalesLakehouse → downloaded/uploaded CSV copy → loaded table → copied notebook via `item copy`
  - Server-side file copy, move, delete between workspaces
  - Table copy, move, delete between workspaces
  - Shortcut create/get/delete between workspaces
- Reassigned 5 workspaces from inactive capacity `64fd7fa6-...` to active `afdf5707-...` using `workspace assign-capacity`
- Research files saved: `~/repositories/research/fabio-cli-commands.md`, `~/repositories/research/fabric-api-unexpected-behaviors.md`
- **Notebook execution commands**: run/status/stop/delete all verified against live tenant
- 165 tests passing, pushed to main (commit `20a90d0`)

### Blocked
- (none)

## Key Decisions
- JSON envelope always wraps output: lists get `{"data":[...],"count":N}`, objects get `{"data":{...}}`
- Errors on stderr as `{"error":{"code":"...","message":"..."}}` with non-zero exit
- `--query` supports simple dot-notation field projection (not full JMESPath; users can pipe to `jq`)
- `FabioError` raised everywhere instead of `click.ClickException`
- OneLake upload uses DFS create+append+flush 3-step pattern
- Notebook creation builds minimal .ipynb JSON, base64-encodes for Fabric API; `source` must be list of strings
- Item copy fetches definition from source via LRO, posts to destination workspace via LRO
- LRO polling: 2s default interval, 120s max wait, handles `Location`/`x-ms-operation-id` headers
- `post()` accepts `poll=True` for LRO-aware operations; non-LRO callers use default `poll=False`
- Load-table requires PascalCase values (`"Overwrite"`, `"Csv"`) and `format` inside `formatOptions`
- **Server-side copy**: OneLake Blob API (`onelake.blob.fabric.microsoft.com`) supports `PUT` with `x-ms-copy-source`; returns 202 with `x-ms-copy-status: pending` for async copy. Poll via HEAD.
- **No native move/rename**: OneLake rejects `x-ms-rename-source` (`UnsupportedHeader`). Move = copy + delete.
- **Table file listing**: DFS listing with `directory='Tables/<name>'` shows virtual lakehouse-in-lakehouse view. Must list from root (no `directory` param) to get real paths prefixed with item ID.
- **Recursive delete**: DFS `DELETE /{ws}/{lh}/Tables/{name}?recursive=true` works for deleting table directories.

## Critical Context
- Fabric Shortcuts API: `POST /v1/workspaces/{workspaceId}/items/{itemId}/shortcuts` with target body specifying source workspace/lakehouse/path
- User's tenant: `f32b018c-68ee-40d8-9e1a-d7ab42193a10`, username `imejiauseche.local@cadata2607.onmicrosoft.com`
- Active capacity: `afdf5707-dde2-41ef-9d98-df65aa40eb7f` (small SKU, Spark concurrency limit ~1)
- Inactive capacity: `64fd7fa6-4b6e-4262-85c1-b70872798927` (paused)
- Source workspace: `1619af1e-c97a-43f8-8f1e-c1990b0b3914` (fabio-demo-source), lakehouse `d4f7211c-cc03-4a86-9f16-0bb2f2af3c59`
- Dest workspace: `c112b455-f02d-4c18-a0af-be75a82816d0` (fabio-demo-dest), lakehouse `36755b0f-b6af-4699-8945-df3aeb8717d6`
- Notebook in source: `38177352-dc1c-440b-a860-a83ec508e806` (SalesAnalysis)
- Notebook in dest: `0bff0250-5447-4853-acb8-eb478a6a7a72` (copied SalesAnalysis)
- Fabric REST base URL: `https://api.fabric.microsoft.com/v1`
- OneLake DFS base URL: `https://onelake.dfs.fabric.microsoft.com`
- OneLake Blob base URL: `https://onelake.blob.fabric.microsoft.com`
- Fabric scope: `https://analysis.windows.net/powerbi/api/.default`
- Storage scope: `https://storage.azure.com/.default`
- Entry point: `fabio = "fabio.cli:main"`
- Spark rate limit on small capacity: LRO reports 430 `TooManyRequestsForCapacity` (non-standard code, need to wait ~5min)

## Relevant Files
- `src/fabio/cli.py`: Main entry point, FabioGroup, global options, command registration
- `src/fabio/output.py`: Structured output system (render_json, render_table, render_plain, _apply_query, read_stdin_json)
- `src/fabio/errors.py`: ErrorCode enum (incl. CAPACITY_INACTIVE) + FabioError exception
- `src/fabio/client.py`: HTTP client with LRO polling, OneLake upload/download/copy/delete/move, table ops, Blob + DFS endpoints
- `src/fabio/commands/workspace.py`: list/show/create/delete/assign-capacity
- `src/fabio/commands/item.py`: list/show/create/delete/copy (LRO-enabled)
- `src/fabio/commands/lakehouse.py`: tables/files/upload/download/load-table/copy-file/delete-file/move-file/delete-table/copy-table/move-table/create-shortcut/get-shortcut/delete-shortcut
- `src/fabio/commands/notebook.py`: create (ipynb list-of-strings source, LRO)/get-definition/run/status/stop/delete
- `src/fabio/commands/auth.py`: login (browser + device-code with prompt_callback)/logout/status
- `src/fabio/auth_store.py`: Persistent auth state (~/.config/fabio/)
- `src/fabio/cache.py`: Token cache with libsecret detection
- `tests/test_client.py`: Tests for _handle_response, _poll_lro, require_auth, copy, delete, table ops
- `tests/test_workspace_crud.py`: Tests for workspace create/delete/assign-capacity
- `tests/test_lakehouse_ops.py`: Tests for upload/download/load-table/copy-file/delete-file/move-file/delete-table/copy-table/move-table
- `tests/test_shortcut.py`: Tests for shortcut create/get/delete
- `tests/test_notebook.py`: Tests for notebook create/get-definition/run/status/stop/delete
- `tests/test_item_copy.py`: Tests for item copy
- `~/repositories/research/fabio-cli-commands.md`: Full command reference
- `~/repositories/research/fabric-api-unexpected-behaviors.md`: 12 documented API gotchas
- `pyproject.toml`: Project config, dependencies, ruff/mypy settings

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
