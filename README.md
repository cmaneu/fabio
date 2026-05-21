# fabio

Agent-first CLI for managing Microsoft Fabric artifacts and data.

Designed for composability: structured JSON output by default, consistent error codes, and stdin/stdout piping between commands. Follows the [10 principles for agent-native CLIs](https://trevinsays.com/p/10-principles-for-agent-native-clis).

## Installation

From source (requires Rust 1.85+):

```bash
cargo install --git https://github.com/iemejia/fabio.git
```

Or download pre-built binaries from the [releases page](https://github.com/iemejia/fabio/releases).

## Design Principles

- **JSON by default** -- All commands output structured JSON for machine consumption
- **Composable** -- Pipe output between commands via stdout/stdin
- **Structured errors** -- Machine-readable error codes with hints and valid enum values
- **Non-interactive** -- No prompts; all parameters via flags/env/files
- **Safe mutations** -- `--dry-run` for destructive operations; idempotent where possible
- **Bounded responses** -- `--limit` for list commands; concise default output
- **Async-aware** -- `--wait` for long-running operations; local job ledger
- **Discoverable** -- `fabio agent-context` provides machine-readable command schema

## Quick Start

```bash
# Authenticate (uses az login credentials)
fabio auth status

# List workspaces (JSON output)
fabio workspace list

# Table format for humans
fabio workspace list -o table

# Field projection
fabio workspace list --query displayName

# List items in a workspace
fabio item list --workspace <workspace-id>

# List lakehouse tables
fabio lakehouse tables --workspace <id> --id <lakehouse-id>

# Upload files with glob patterns (parallel)
fabio lakehouse upload --workspace <id> --id <lakehouse-id> --source "data/*.csv" --dest Files/

# Sync files between lakehouses (copies new/modified only)
fabio lakehouse sync --workspace <src-ws> --id <src-lh> --dest-workspace <dst-ws> --dest-id <dst-lh>

# Run a notebook and wait for completion
fabio notebook run --workspace <id> --id <notebook-id> --wait

# Query a data agent
fabio data-agent query --workspace <id> --id <agent-id> --prompt "What were total sales last quarter?"

# Check Git status for a workspace
fabio git status --workspace <id>

# Machine-readable command schema for AI agents
fabio agent-context
```

## Output Formats

```bash
# JSON (default) - structured envelope for agents
fabio workspace list
# {"data":[...],"count":2}

# Table - human-readable columns
fabio workspace list -o table

# Plain - one value per line for shell scripting
fabio workspace list -o plain
```

## Error Handling

All errors are structured JSON on stderr with machine-readable codes:

```json
{"error":{"code":"AUTH_REQUIRED","message":"Not authenticated. Run 'az login' first."}}
```

Error codes: `AUTH_REQUIRED`, `FORBIDDEN`, `NOT_FOUND`, `CONFLICT`, `RATE_LIMITED`, `CAPACITY_INACTIVE`, `INVALID_INPUT`, `API_ERROR`, `TIMEOUT`, `NETWORK_ERROR`

## Global Options

| Flag | Description |
|------|-------------|
| `-o`, `--output` | Output format: `json` (default), `table`, `plain` |
| `--json` | Shorthand for `--output json` |
| `-q`, `--query` | Field projection (dot-notation extraction) |
| `--quiet` | Suppress all stdout output |
| `--force` | Skip confirmation prompts for destructive operations |
| `--dry-run` | Preview mutations without executing |
| `--limit` | Limit number of results for list commands |
| `--all` | Fetch all pages (auto-paginate) |
| `--continuation-token` | Resume pagination from a previous token |
| `--profile` | Use a named profile for default settings |

## Commands

### Core

```
fabio auth status            Show authentication status
fabio auth login             Sign in to Microsoft Fabric
fabio auth logout            Sign out and clear credentials

fabio workspace list         List all accessible workspaces
fabio workspace show         Show workspace details
fabio workspace create       Create a new workspace
fabio workspace delete       Delete a workspace
fabio workspace update       Update workspace name/description
fabio workspace assign-capacity    Assign a capacity
fabio workspace unassign-capacity  Remove capacity assignment
fabio workspace provision-identity Provision workspace identity
fabio workspace deprovision-identity Remove workspace identity
fabio workspace list-role-assignments  List role assignments
fabio workspace add-role-assignment    Add a role assignment
fabio workspace update-role-assignment Update a role assignment
fabio workspace delete-role-assignment Delete a role assignment

fabio item list              List items in a workspace (--type filter)
fabio item show              Show item details
fabio item create            Create a new item
fabio item update            Update item name/description
fabio item delete            Delete an item
fabio item copy              Copy an item between workspaces
fabio item move              Move an item between workspaces
fabio item get-definition    Get item definition (source code)
fabio item update-definition Update item definition
fabio item list-connections  List connections used by an item

fabio lakehouse list         List lakehouses in a workspace
fabio lakehouse show         Show lakehouse details
fabio lakehouse create       Create a new lakehouse
fabio lakehouse update       Update lakehouse name/description
fabio lakehouse delete       Delete a lakehouse
fabio lakehouse list-tables  List tables in a lakehouse
fabio lakehouse list-files   List files in a lakehouse
fabio lakehouse upload       Upload files (supports glob patterns, parallel)
fabio lakehouse download     Download a file from a lakehouse
fabio lakehouse load-table   Load a file into a Delta table
fabio lakehouse copy-file    Copy files (supports glob patterns, parallel)
fabio lakehouse move-file    Move files (supports glob patterns, parallel)
fabio lakehouse delete-file  Delete a file
fabio lakehouse copy-table   Copy a table between lakehouses
fabio lakehouse move-table   Move a table (copy + delete source)
fabio lakehouse delete-table Delete a table
fabio lakehouse sync         Sync files between lakehouses (ETag/MD5)
fabio lakehouse create-shortcut  Create a shortcut (OneLake/ADLS/S3)
fabio lakehouse get-shortcut     Get shortcut details
fabio lakehouse delete-shortcut  Delete a shortcut

fabio capacity list          List available capacities
fabio capacity show          Show capacity details
```

### Data & Compute

```
fabio notebook list          List notebooks
fabio notebook show          Show notebook details
fabio notebook create        Create a new notebook
fabio notebook update        Update notebook name/description
fabio notebook delete        Delete a notebook
fabio notebook get-definition  Get notebook source code
fabio notebook update-definition Update notebook source
fabio notebook run           Run a notebook (--wait to block until done)
fabio notebook status        Check run status
fabio notebook stop          Stop a running notebook

fabio warehouse list         List warehouses in a workspace
fabio warehouse show         Show warehouse details
fabio warehouse create       Create a warehouse
fabio warehouse update       Update warehouse name/description
fabio warehouse delete       Delete a warehouse
fabio warehouse query        Execute SQL (--sql, @file, or stdin)

fabio data-agent list        List data agents
fabio data-agent create      Create a new data agent
fabio data-agent show        Show data agent details
fabio data-agent update      Update name/description
fabio data-agent delete      Delete a data agent
fabio data-agent query       Chat with a published data agent

fabio ontology list          List ontologies
fabio ontology show          Show ontology details
fabio ontology create        Create an ontology (with RDF support)
fabio ontology update        Update ontology properties
fabio ontology delete        Delete an ontology
fabio ontology get-definition   Get ontology definition
fabio ontology update-definition Update ontology definition

fabio environment list       List environments
fabio environment show       Show environment details
fabio environment create     Create an environment
fabio environment update     Update environment properties
fabio environment delete     Delete an environment
fabio environment publish    Publish staged changes
fabio environment cancel-publish Cancel a pending publish
fabio environment get-spark-settings Get published Spark settings
fabio environment get-staging-spark-settings Get staging settings

fabio data-pipeline list     List data pipelines
fabio data-pipeline show     Show pipeline details
fabio data-pipeline create   Create a data pipeline
fabio data-pipeline update   Update pipeline properties
fabio data-pipeline delete   Delete a data pipeline
fabio data-pipeline run      Run a data pipeline

fabio copy-job list          List copy jobs
fabio copy-job show          Show copy job details
fabio copy-job create        Create a copy job
fabio copy-job update        Update copy job properties
fabio copy-job delete        Delete a copy job
fabio copy-job get-definition   Get copy job definition
fabio copy-job update-definition Update copy job definition

fabio dataflow list          List dataflows
fabio dataflow show          Show dataflow details
fabio dataflow create        Create a dataflow
fabio dataflow update        Update dataflow properties
fabio dataflow delete        Delete a dataflow
fabio dataflow get-definition   Get dataflow definition
fabio dataflow update-definition Update dataflow definition

fabio report list            List reports
fabio report show            Show report details
fabio report create          Create a report (from definition file)
fabio report update          Update report properties
fabio report delete          Delete a report
fabio report get-definition  Get report definition
fabio report update-definition Update report definition

fabio semantic-model list    List semantic models
fabio semantic-model show    Show semantic model details
fabio semantic-model create  Create from definition file (model.bim)
fabio semantic-model update  Update properties
fabio semantic-model delete  Delete a semantic model
fabio semantic-model get-definition  Get definition
fabio semantic-model update-definition Update definition

fabio eventhouse list        List eventhouses
fabio eventhouse show        Show eventhouse details
fabio eventhouse create      Create an eventhouse
fabio eventhouse update      Update eventhouse properties
fabio eventhouse delete      Delete an eventhouse

fabio eventstream list       List eventstreams
fabio eventstream show       Show eventstream details
fabio eventstream create     Create an eventstream
fabio eventstream update     Update eventstream properties
fabio eventstream delete     Delete an eventstream
fabio eventstream get-definition   Get definition
fabio eventstream update-definition Update definition

fabio kql-database list      List KQL databases
fabio kql-database show      Show KQL database details
fabio kql-database create    Create a KQL database
fabio kql-database update    Update KQL database properties
fabio kql-database delete    Delete a KQL database
fabio kql-database get-definition   Get definition
fabio kql-database update-definition Update definition

fabio kql-queryset list      List KQL querysets
fabio kql-queryset show      Show KQL queryset details
fabio kql-queryset create    Create a KQL queryset
fabio kql-queryset update    Update KQL queryset properties
fabio kql-queryset delete    Delete a KQL queryset
fabio kql-queryset get-definition   Get definition
fabio kql-queryset update-definition Update definition

fabio kql-dashboard list     List KQL dashboards
fabio kql-dashboard show     Show KQL dashboard details
fabio kql-dashboard create   Create a KQL dashboard
fabio kql-dashboard update   Update KQL dashboard properties
fabio kql-dashboard delete   Delete a KQL dashboard
fabio kql-dashboard get-definition   Get definition
fabio kql-dashboard update-definition Update definition

fabio mirrored-database list   List mirrored databases
fabio mirrored-database show   Show mirrored database details
fabio mirrored-database create Create a mirrored database
fabio mirrored-database update Update mirrored database properties
fabio mirrored-database delete Delete a mirrored database
fabio mirrored-database get-definition   Get definition
fabio mirrored-database update-definition Update definition
fabio mirrored-database start  Start mirroring
fabio mirrored-database stop   Stop mirroring
fabio mirrored-database status Get mirroring status
fabio mirrored-database table-status Get table mirroring status

fabio reflex list            List Reflex items
fabio reflex show            Show Reflex details
fabio reflex create          Create a Reflex
fabio reflex update          Update Reflex properties
fabio reflex delete          Delete a Reflex
fabio reflex get-definition  Get definition (ReflexEntities.json)
fabio reflex update-definition Update definition

fabio ml-model list          List ML models
fabio ml-model show          Show ML model details
fabio ml-model create        Create an ML model
fabio ml-model update        Update ML model properties
fabio ml-model delete        Delete an ML model

fabio ml-experiment list     List ML experiments
fabio ml-experiment show     Show ML experiment details
fabio ml-experiment create   Create an ML experiment
fabio ml-experiment update   Update ML experiment properties
fabio ml-experiment delete   Delete an ML experiment

fabio spark get-settings     Get workspace Spark settings
fabio spark update-settings  Update workspace Spark settings
fabio spark list-pools       List custom Spark pools
fabio spark get-pool         Get pool details
fabio spark create-pool      Create a custom Spark pool
fabio spark update-pool      Update a custom pool
fabio spark delete-pool      Delete a custom pool

fabio spark-job-definition list   List Spark job definitions
fabio spark-job-definition show   Show details
fabio spark-job-definition create Create a Spark job definition
fabio spark-job-definition update Update properties
fabio spark-job-definition delete Delete a Spark job definition
fabio spark-job-definition get-definition   Get definition
fabio spark-job-definition update-definition Update definition
fabio spark-job-definition run    Run a Spark job

fabio graphql-api list       List GraphQL APIs
fabio graphql-api show       Show GraphQL API details
fabio graphql-api create     Create a GraphQL API
fabio graphql-api update     Update GraphQL API properties
fabio graphql-api delete     Delete a GraphQL API
fabio graphql-api get-definition   Get definition (schema.graphql)
fabio graphql-api update-definition Update definition
```

### Integration

```
fabio git status             Show workspace Git status
fabio git commit             Commit workspace changes to remote
fabio git pull               Pull remote changes into workspace
fabio git connect            Connect a workspace to a Git repo
fabio git disconnect         Disconnect a workspace from Git
fabio git initialize         Initialize Git connection after connect
fabio git switch             Switch to a different branch
fabio git connection show    Show Git connection details
fabio git credentials show   Show Git credentials configuration
fabio git credentials update Update Git credentials

fabio connection list        List all connections
fabio connection show        Show connection details
fabio connection create      Create a new connection
fabio connection update      Update a connection
fabio connection delete      Delete a connection
fabio connection list-supported-types List supported connection types

fabio deployment-pipeline list   List deployment pipelines
fabio deployment-pipeline show   Show pipeline details
fabio deployment-pipeline create Create a pipeline
fabio deployment-pipeline update Update pipeline properties
fabio deployment-pipeline delete Delete a pipeline
fabio deployment-pipeline list-stages List stages
fabio deployment-pipeline list-stage-items List items in a stage
fabio deployment-pipeline assign-workspace Assign workspace to stage
fabio deployment-pipeline unassign-workspace Unassign workspace
fabio deployment-pipeline deploy Deploy items between stages

fabio domain list            List domains in the tenant
fabio domain show            Show domain details
fabio domain create          Create a domain
fabio domain update          Update domain properties
fabio domain delete          Delete a domain
fabio domain list-workspaces List workspaces in a domain
fabio domain assign-workspaces Assign workspaces to a domain
fabio domain unassign-workspaces Unassign workspaces
fabio domain assign-by-capacity Bulk-assign workspaces by capacity
fabio domain assign-by-principal Bulk-assign by principal

fabio job-scheduler list-instances List job instances for an item
fabio job-scheduler get-instance Get job instance details
fabio job-scheduler run-on-demand Run an on-demand job
fabio job-scheduler cancel-instance Cancel a running instance
fabio job-scheduler list-schedules List schedules
fabio job-scheduler get-schedule Get schedule details
fabio job-scheduler create-schedule Create a schedule
fabio job-scheduler update-schedule Update a schedule
fabio job-scheduler delete-schedule Delete a schedule
```

### Security & Governance

```
fabio onelake-security list  List data access roles
fabio onelake-security show  Show a data access role
fabio onelake-security upsert Create or replace all roles
fabio onelake-security delete Delete a data access role

fabio managed-private-endpoint list   List managed private endpoints
fabio managed-private-endpoint show   Show endpoint details
fabio managed-private-endpoint create Create a managed private endpoint
fabio managed-private-endpoint delete Delete a managed private endpoint
```

### Configuration

```
fabio profile save           Save a named profile
fabio profile use            Set the active profile
fabio profile list           List saved profiles
fabio profile show           Show profile details
fabio profile delete         Delete a profile

fabio jobs list              List recent jobs from local ledger
fabio jobs get               Get details of a specific job
fabio jobs prune             Remove completed/failed jobs

fabio feedback send          Record feedback about CLI friction
fabio feedback list          List recorded feedback entries

fabio agent-context          Machine-readable command schema for AI agents
```

## Authentication

Fabio uses the Azure credential chain (`az login` or environment credentials). No built-in token storage -- it delegates to the Azure CLI credential cache.

```bash
# Login via Azure CLI first
az login

# Verify fabio can authenticate
fabio auth status
```

## Development

```bash
git clone https://github.com/iemejia/fabio.git && cd fabio

# Build
cargo build

# Run tests (unit tests)
cargo test

# Run E2E tests (requires live Fabric tenant)
cargo test -- --ignored

# Lint
cargo clippy --tests -- -D warnings

# Format
cargo fmt
```

## License

MIT
