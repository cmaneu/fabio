# fabio

Agent-first CLI for managing Microsoft Fabric artifacts and data.

Designed for composability: structured JSON output by default, consistent error codes, and stdin/stdout piping between commands. Follows the [10 principles for agent-native CLIs](https://trevinsays.com/p/10-principles-for-agent-native-clis).

## Installation

From source (requires Rust 1.85+):

```bash
cargo install --git https://github.com/iemejia/fabio.git
```

Or download pre-built binaries from the [releases page](https://github.com/iemejia/fabio/releases) (Linux, macOS, Windows — x64 and arm64).

## Design Principles

- **JSON by default** -- All commands output structured JSON for machine consumption
- **Composable** -- Pipe output between commands via stdout/stdin
- **Structured errors** -- Machine-readable error codes with hints and valid enum values
- **Non-interactive** -- No prompts; all parameters via flags/env/files
- **Safe mutations** -- `--dry-run` for destructive operations; idempotent where possible
- **Bounded responses** -- `--limit` for list commands; concise default output
- **Async-aware** -- `--wait` for long-running operations; local job ledger
- **Discoverable** -- `fabio agent-context` provides machine-readable command schema
- **Throttling-aware** -- Bulk/batch APIs preferred; parallel execution with rate-limit retry

## Quick Start

```bash
# 1. Sign in
fabio auth login

# 2. Create a workspace and assign compute capacity
fabio workspace create --name "sales-analytics" -o table
fabio workspace assign-capacity --id <workspace-id> --capacity <capacity-id>

# 3. Create a lakehouse for your data
fabio lakehouse create --workspace <workspace-id> --name "SalesLakehouse" -o table

# 4. Upload local CSV files (glob patterns, parallel upload)
fabio lakehouse upload --workspace <ws> --id <lh> --source "data/*.csv" --dest Files/raw/

# 5. Load a CSV into a managed Delta table
fabio lakehouse load-table --workspace <ws> --id <lh> \
  --path Files/raw/orders.csv --table orders --mode Overwrite --format Csv

# 6. Check your tables
fabio lakehouse list-tables --workspace <ws> --id <lh> -o table

# 7. Query the data via SQL
fabio warehouse query --workspace <ws> --id <warehouse-id> \
  --sql "SELECT country, SUM(revenue) as total FROM dbo.orders GROUP BY country"
```

That's it -- from sign-in to queryable Delta tables in 7 commands.

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
{"error":{"code":"AUTH_REQUIRED","message":"Not authenticated. Run 'az login' first.","hint":"Run: az login"}}
```

Error codes: `AUTH_REQUIRED`, `FORBIDDEN`, `NOT_FOUND`, `CONFLICT`, `RATE_LIMITED`, `CAPACITY_INACTIVE`, `INVALID_INPUT`, `API_ERROR`, `TIMEOUT`, `NETWORK_ERROR`

## Global Options

| Flag | Description |
|------|-------------|
| `-o`, `--output` | Output format: `json` (default), `table`, `plain`, `csv`, `tsv` |
| `--json` | Shorthand for `--output json` |
| `-q`, `--query` | Field projection (dot-notation extraction) |
| `--quiet` | Suppress all stdout output |
| `--force` | Skip confirmation prompts for destructive operations |
| `--dry-run` | Preview mutations without executing |
| `--limit` | Limit number of results for list commands |
| `--all` | Fetch all pages (auto-paginate) |
| `--continuation-token` | Resume pagination from a previous token |
| `--profile` | Use a named profile for default settings |
| `--lro-timeout` | Override default LRO polling timeout (seconds) |
| `--hard-delete` | Permanently delete (skip recycle bin) -- on item deletes |

## Commands

### Core

```
fabio auth login             Log in to Microsoft Fabric (validates credentials)
fabio auth logout            Log out and clear cached credentials
fabio auth status            Show current authentication status and credential source

fabio workspace list         List all workspaces
fabio workspace show         Show details of a workspace
fabio workspace create       Create a new workspace
fabio workspace update       Update workspace properties (name/description)
fabio workspace delete       Delete a workspace
fabio workspace assign-capacity      Assign a workspace to a capacity
fabio workspace unassign-capacity    Unassign a workspace from its capacity
fabio workspace provision-identity   Provision workspace managed identity
fabio workspace deprovision-identity Deprovision workspace managed identity
fabio workspace list-role-assignments  List workspace role assignments
fabio workspace show-role-assignment   Show a specific role assignment
fabio workspace add-role-assignment    Add a role assignment
fabio workspace update-role-assignment Update a role assignment
fabio workspace delete-role-assignment Delete a role assignment
fabio workspace list-folders       List workspace folders
fabio workspace create-folder      Create a folder in a workspace
fabio workspace show-folder        Show folder details
fabio workspace update-folder      Update a folder
fabio workspace delete-folder      Delete a folder
fabio workspace move-folder        Move a folder to another parent (or root)
fabio workspace apply-tags         Apply tags to a workspace
fabio workspace unapply-tags       Remove tags from a workspace
fabio workspace assign-to-domain   Assign workspace to a domain
fabio workspace unassign-from-domain Unassign workspace from its domain
fabio workspace get-onelake-settings Get OneLake settings
fabio workspace modify-default-tier  Modify OneLake default tier (Hot/Cold)
fabio workspace modify-diagnostics   Modify OneLake diagnostics configuration
fabio workspace modify-immutability-policy Modify OneLake immutability policy
fabio workspace export-lifecycle-policy Export OneLake lifecycle policy
fabio workspace import-lifecycle-policy Import OneLake lifecycle policy
fabio workspace reset-shortcut-cache   Reset OneLake shortcut cache
fabio workspace get-network-policy     Get network communication policy
fabio workspace set-network-policy     Set network communication policy

fabio item list              List items in a workspace (--type, --folder, --recursive)
fabio item show              Show item details
fabio item create            Create a new item
fabio item update            Update item properties (name/description)
fabio item delete            Delete an item (--hard-delete for permanent)
fabio item copy              Copy an item to another workspace
fabio item move              Move an item to another workspace (copy + delete)
fabio item move-to-folder    Move an item to a folder (or root)
fabio item get-definition    Get item definition (source code/content)
fabio item update-definition Update item definition from file(s)
fabio item list-connections  List connections used by an item
fabio item exists            Check if an item exists (returns {exists: true/false})
fabio item url               Get Fabric portal URL for an item
fabio item inspect           Get metadata + definition + connections in one call
fabio item apply-tags        Apply tags to an item
fabio item unapply-tags      Remove tags from an item
fabio item bulk-create       Create multiple items in parallel
fabio item bulk-delete       Delete multiple items in parallel
fabio item bulk-export-definitions Bulk export item definitions (LRO)
fabio item bulk-import-definitions Bulk import item definitions (LRO)
fabio item bulk-move         Bulk move items to another workspace (LRO)
fabio item list-external-data-shares   List external data shares for an item
fabio item create-external-data-share  Create an external data share
fabio item show-external-data-share    Show external data share details
fabio item revoke-external-data-share  Revoke an external data share
fabio item delete-external-data-share  Delete an external data share
fabio item assign-identity   Assign a managed identity to an item
fabio item get-invitation    Get an external data share invitation
fabio item accept-invitation Accept an external data share invitation

fabio lakehouse list         List lakehouses in a workspace
fabio lakehouse show         Show lakehouse details
fabio lakehouse create       Create a new lakehouse
fabio lakehouse update       Update a lakehouse (rename/redescribe)
fabio lakehouse delete       Delete a lakehouse
fabio lakehouse list-tables  List tables in a lakehouse
fabio lakehouse list-files   List files in a lakehouse
fabio lakehouse upload       Upload files (supports glob patterns, parallel)
fabio lakehouse download     Download a file from a lakehouse
fabio lakehouse upload-table Upload a file and load it into a Delta table (one step)
fabio lakehouse load-table   Load an existing file into a Delta table (--schema)
fabio lakehouse query        Execute T-SQL via SQL analytics endpoint
fabio lakehouse table-schema Read Delta table schema from OneLake (no Spark)
fabio lakehouse optimize-table Run V-Order + Z-Order optimization
fabio lakehouse vacuum-table Remove old files (retention period)
fabio lakehouse copy-file    Copy files between lakehouses (glob, parallel)
fabio lakehouse move-file    Move files between lakehouses (glob, parallel)
fabio lakehouse delete-file  Delete a file
fabio lakehouse copy-table   Copy a table between lakehouses
fabio lakehouse move-table   Move a table (copy + delete source)
fabio lakehouse delete-table Delete a table
fabio lakehouse sync         Sync files between lakehouses (ETag/MD5 comparison)
fabio lakehouse create-shortcut      Create a shortcut (OneLake/ADLS/S3, --conflict-policy)
fabio lakehouse get-shortcut         Get shortcut details
fabio lakehouse delete-shortcut      Delete a shortcut
fabio lakehouse bulk-create-shortcuts Bulk-create multiple shortcuts (LRO)
fabio lakehouse get-definition       Get lakehouse definition
fabio lakehouse update-definition    Update lakehouse definition
fabio lakehouse refresh-materialized-views Trigger materialized view refresh
fabio lakehouse create-materialized-views-schedule Create refresh schedule
fabio lakehouse update-materialized-views-schedule Update refresh schedule
fabio lakehouse delete-materialized-views-schedule Delete refresh schedule
fabio lakehouse run-table-maintenance Run table maintenance job
fabio lakehouse list-livy-sessions   List Livy sessions
fabio lakehouse get-livy-session     Get Livy session details

fabio capacity list          List available capacities
fabio capacity show          Show capacity details
fabio capacity suspend       Suspend (pause) a capacity
fabio capacity resume        Resume a suspended capacity
fabio capacity create        Create a new capacity (ARM)
fabio capacity update        Update capacity properties (ARM)
fabio capacity delete        Delete a capacity (ARM)
fabio capacity list-skus     List available SKUs and regions
fabio capacity check-name    Check capacity name availability

fabio catalog search         Search items across the tenant
```

### Data & Compute

```
fabio notebook list          List notebooks in a workspace
fabio notebook show          Show notebook details
fabio notebook create        Create a new notebook (--lakehouse for binding)
fabio notebook update        Update notebook properties (name/description)
fabio notebook delete        Delete a notebook
fabio notebook get-definition   Get notebook source code (--strip-output)
fabio notebook update-definition Update notebook source
fabio notebook run           Run a notebook (--wait, --timeout, --parameters)
fabio notebook status        Check run status
fabio notebook get-job-instance Get details of a specific job instance
fabio notebook stop          Stop a running notebook
fabio notebook list-livy-sessions List Livy sessions for a notebook
fabio notebook get-livy-session   Get Livy session details

fabio warehouse list         List warehouses in a workspace
fabio warehouse show         Show warehouse details
fabio warehouse create       Create a warehouse
fabio warehouse update       Update warehouse properties (name/description)
fabio warehouse delete       Delete a warehouse
fabio warehouse query        Execute SQL (--sql, @file, or stdin)
fabio warehouse connection-string Get TDS connection string
fabio warehouse get-sql-pools-config Get SQL pools configuration
fabio warehouse update-sql-pools-config Update SQL pools configuration
fabio warehouse get-audit-settings Get SQL audit settings
fabio warehouse update-audit-settings Update SQL audit settings
fabio warehouse set-audit-actions Set audit actions and groups
fabio warehouse list-restore-points List restore points
fabio warehouse create-restore-point Create a restore point
fabio warehouse show-restore-point Show restore point details
fabio warehouse update-restore-point Update a restore point
fabio warehouse delete-restore-point Delete a restore point
fabio warehouse restore-to-point Restore a warehouse to a point

fabio warehouse-snapshot list   List warehouse snapshots
fabio warehouse-snapshot show   Show snapshot details
fabio warehouse-snapshot create Create a snapshot (--warehouse-id)
fabio warehouse-snapshot update Update snapshot properties
fabio warehouse-snapshot delete Delete a snapshot

fabio sql-database list      List SQL databases in a workspace
fabio sql-database show      Show SQL database details
fabio sql-database create    Create a SQL database
fabio sql-database update    Update SQL database properties
fabio sql-database delete    Delete a SQL database
fabio sql-database query     Execute SQL (--sql, @file, or stdin) via TDS
fabio sql-database connection-string Get TDS connection string
fabio sql-database import    Import CSV/JSON into a table (type inference)
fabio sql-database get-definition   Get definition (dacpac/sqlproj format)
fabio sql-database update-definition Update definition
fabio sql-database start-mirroring Start mirroring
fabio sql-database stop-mirroring  Stop mirroring
fabio sql-database revalidate-cmk  Revalidate Customer-Managed Key
fabio sql-database get-audit-settings Get SQL audit settings
fabio sql-database update-audit-settings Update SQL audit settings
fabio sql-database list-deleted List restorable deleted databases

fabio sql-endpoint list      List SQL analytics endpoints
fabio sql-endpoint show      Show endpoint details
fabio sql-endpoint connection-string Get TDS connection string
fabio sql-endpoint refresh-metadata  Refresh table sync metadata (LRO)
fabio sql-endpoint get-audit-settings  Get audit configuration
fabio sql-endpoint update-audit-settings Update audit settings
fabio sql-endpoint set-audit-actions    Set audit action groups

fabio data-agent list        List data agents
fabio data-agent show        Show data agent details
fabio data-agent create      Create a new data agent
fabio data-agent update      Update name/description
fabio data-agent delete      Delete a data agent
fabio data-agent query       Chat with a published data agent
fabio data-agent get-definition   Get definition (configuration, data sources)
fabio data-agent update-definition Update definition (instructions, data sources)
fabio data-agent publish     Publish a data agent (promotes draft to published)

fabio ontology list          List ontologies
fabio ontology show          Show ontology details
fabio ontology create        Create an ontology
fabio ontology update        Update ontology properties
fabio ontology delete        Delete an ontology
fabio ontology get-definition   Get definition (--decode for readable output)
fabio ontology update-definition Update definition (--dir for folder format)

fabio environment list       List environments in a workspace
fabio environment show       Show environment details
fabio environment create     Create an environment
fabio environment update     Update environment properties
fabio environment delete     Delete an environment
fabio environment publish    Publish staged changes
fabio environment cancel-publish Cancel a pending publish
fabio environment get-spark-settings Get published Spark settings
fabio environment get-staging-spark-settings Get staging (draft) settings
fabio environment upload-staging-library Upload a library to staging (.whl/.jar/.tar.gz)
fabio environment get-definition   Get environment definition
fabio environment update-definition Update environment definition
fabio environment list-libraries   List published libraries
fabio environment export-libraries Export external libraries config (published)
fabio environment list-staging-libraries List staging libraries
fabio environment delete-staging-library Delete a staging library
fabio environment export-staging-libraries Export external libraries (staging)
fabio environment import-staging-libraries Import external libraries into staging
fabio environment remove-staging-library Remove external library from staging
fabio environment update-staging-spark-compute Update staging Spark config

fabio data-pipeline list     List data pipelines
fabio data-pipeline show     Show pipeline details
fabio data-pipeline create   Create a data pipeline
fabio data-pipeline update   Update pipeline properties
fabio data-pipeline delete   Delete a data pipeline
fabio data-pipeline run      Run a data pipeline
fabio data-pipeline get-definition   Get pipeline definition
fabio data-pipeline update-definition Update pipeline definition
fabio data-pipeline create-schedule  Create a pipeline schedule

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
fabio dataflow discover-parameters Discover M parameters
fabio dataflow run           Run a dataflow (--wait, --job-type execute|apply-changes)
fabio dataflow execute-query Execute a named query (returns Arrow IPC binary)
```

### Analytics & Reporting

```
fabio report list            List reports
fabio report show            Show report details
fabio report create          Create a report (--dataset to bind semantic model)
fabio report update          Update report properties
fabio report delete          Delete a report
fabio report get-definition  Get report definition
fabio report update-definition Update report definition
fabio report publish-to-web  Publish report to the web (public embed URL)

fabio semantic-model list    List semantic models
fabio semantic-model show    Show semantic model details
fabio semantic-model create  Create from TMDL or model.bim
fabio semantic-model update  Update properties
fabio semantic-model delete  Delete a semantic model
fabio semantic-model get-definition    Get definition
fabio semantic-model update-definition Update definition
fabio semantic-model query   Execute a DAX query
fabio semantic-model bind-connection Bind to a connection
fabio semantic-model unbind-connection Unbind from a connection
fabio semantic-model refresh Refresh (frame Direct Lake models)
fabio semantic-model takeover Convert definition-managed to service-managed
fabio semantic-model list-parameters   List M parameters (Power BI API)
fabio semantic-model update-parameters Update M parameters
fabio semantic-model list-datasources  List data sources
fabio semantic-model update-datasources Update data sources
fabio semantic-model list-users        List dataset permissions
fabio semantic-model add-user          Add a user/principal
fabio semantic-model delete-user       Remove a user/principal
fabio semantic-model refresh-status    View refresh history
fabio semantic-model list-upstream     Show upstream dependencies
fabio semantic-model clone             Clone a dataset (same/cross-workspace)
fabio semantic-model export-pbix       Download as .pbix binary
fabio semantic-model import-pbix       Upload .pbix file

fabio paginated-report list  List paginated reports
fabio paginated-report update Update paginated report properties

fabio dashboard list         List dashboards

fabio datamart list          List datamarts
```

### Real-Time Intelligence

```
fabio eventhouse list        List eventhouses
fabio eventhouse show        Show eventhouse details
fabio eventhouse create      Create an eventhouse
fabio eventhouse update      Update eventhouse properties
fabio eventhouse delete      Delete an eventhouse
fabio eventhouse get-definition   Get definition
fabio eventhouse update-definition Update definition

fabio eventstream list       List eventstreams
fabio eventstream show       Show eventstream details
fabio eventstream create     Create an eventstream
fabio eventstream update     Update eventstream properties
fabio eventstream delete     Delete an eventstream
fabio eventstream get-definition   Get definition
fabio eventstream update-definition Update definition
fabio eventstream get-topology     Get eventstream topology
fabio eventstream pause      Pause the entire eventstream
fabio eventstream resume     Resume the entire eventstream
fabio eventstream get-source Get source details
fabio eventstream get-source-connection Get source connection info
fabio eventstream pause-source   Pause a source
fabio eventstream resume-source  Resume a source
fabio eventstream get-destination Get destination details
fabio eventstream get-destination-connection Get destination connection info
fabio eventstream pause-destination Pause a destination
fabio eventstream resume-destination Resume a destination
fabio eventstream add-source Add a source (fetches definition, merges, updates)
fabio eventstream add-destination Add a destination (same pattern)

fabio kql-database list      List KQL databases
fabio kql-database show      Show KQL database details
fabio kql-database create    Create a KQL database (--eventhouse-id)
fabio kql-database update    Update KQL database properties
fabio kql-database delete    Delete a KQL database
fabio kql-database query     Execute KQL queries (--kql)
fabio kql-database get-definition   Get definition
fabio kql-database update-definition Update definition
fabio kql-database list-shortcuts   List shortcuts in a KQL database
fabio kql-database create-shortcut  Create a shortcut
fabio kql-database get-shortcut     Get shortcut details
fabio kql-database delete-shortcut  Delete a shortcut
fabio kql-database bulk-create-shortcuts Bulk-create shortcuts (LRO)

fabio kql-queryset list      List KQL querysets
fabio kql-queryset show      Show KQL queryset details
fabio kql-queryset create    Create a KQL queryset
fabio kql-queryset update    Update KQL queryset properties
fabio kql-queryset delete    Delete a KQL queryset
fabio kql-queryset get-definition   Get definition
fabio kql-queryset update-definition Update definition
fabio kql-queryset run       Run a saved query tab against its data source

fabio kql-dashboard list     List KQL dashboards
fabio kql-dashboard show     Show KQL dashboard details
fabio kql-dashboard create   Create a KQL dashboard
fabio kql-dashboard update   Update KQL dashboard properties
fabio kql-dashboard delete   Delete a KQL dashboard
fabio kql-dashboard get-definition   Get definition
fabio kql-dashboard update-definition Update definition

fabio reflex list            List reflexes (Data Activator)
fabio reflex show            Show reflex details
fabio reflex create          Create a reflex
fabio reflex update          Update reflex properties
fabio reflex delete          Delete a reflex
fabio reflex get-definition  Get definition (ReflexEntities.json)
fabio reflex update-definition Update definition
fabio reflex configure-kql-source Configure a KQL data source

fabio anomaly-detector list  List anomaly detectors
fabio anomaly-detector show  Show anomaly detector details
fabio anomaly-detector create Create an anomaly detector
fabio anomaly-detector update Update properties
fabio anomaly-detector delete Delete an anomaly detector
fabio anomaly-detector get-definition   Get definition
fabio anomaly-detector update-definition Update definition

fabio event-schema-set list  List event schema sets
fabio event-schema-set show  Show event schema set details
fabio event-schema-set create Create an event schema set
fabio event-schema-set update Update properties
fabio event-schema-set delete Delete an event schema set
fabio event-schema-set get-definition   Get definition
fabio event-schema-set update-definition Update definition

fabio rti nl-to-kql          Translate natural language to KQL (AI-powered)
```

### Data Science & AI

```
fabio ml-model list          List ML models
fabio ml-model show          Show ML model details
fabio ml-model create        Create an ML model
fabio ml-model update        Update ML model properties
fabio ml-model delete        Delete an ML model
fabio ml-model get-endpoint  Get model serving endpoint configuration
fabio ml-model update-endpoint Update model serving endpoint
fabio ml-model score         Score (invoke) a deployed model
fabio ml-model list-versions List endpoint versions
fabio ml-model get-version   Get version details
fabio ml-model update-version Update a version
fabio ml-model activate-version Activate a version
fabio ml-model deactivate-version Deactivate a version
fabio ml-model score-version Score a specific version
fabio ml-model deactivate-all-versions Deactivate all versions

fabio ml-experiment list     List ML experiments
fabio ml-experiment show     Show ML experiment details
fabio ml-experiment create   Create an ML experiment
fabio ml-experiment update   Update ML experiment properties
fabio ml-experiment delete   Delete an ML experiment

fabio operations-agent list  List operations agents
fabio operations-agent show  Show operations agent details
fabio operations-agent create Create an operations agent
fabio operations-agent update Update properties
fabio operations-agent delete Delete an operations agent
fabio operations-agent get-definition   Get definition (Configurations.json)
fabio operations-agent update-definition Update definition
```

### Spark

```
fabio spark get-settings     Get workspace-level Spark settings
fabio spark update-settings  Update workspace-level Spark settings
fabio spark list-pools       List custom Spark pools in a workspace
fabio spark get-pool         Get pool details
fabio spark create-pool      Create a custom Spark pool
fabio spark update-pool      Update a custom pool
fabio spark delete-pool      Delete a custom pool
fabio spark get-capacity-settings Get capacity-level Spark settings
fabio spark update-capacity-settings Update capacity-level Spark settings
fabio spark list-capacity-pools List custom Spark pools in a capacity
fabio spark create-capacity-pool Create a capacity Spark pool
fabio spark get-capacity-pool Get capacity pool details
fabio spark update-capacity-pool Update a capacity pool
fabio spark delete-capacity-pool Delete a capacity pool
fabio spark list-livy-sessions List Livy sessions in a workspace
fabio spark get-livy-session Get Livy session details

fabio spark-job-definition list   List Spark job definitions
fabio spark-job-definition show   Show details
fabio spark-job-definition create Create a Spark job definition
fabio spark-job-definition update Update properties
fabio spark-job-definition delete Delete a Spark job definition
fabio spark-job-definition get-definition   Get definition
fabio spark-job-definition update-definition Update definition
fabio spark-job-definition run    Run a Spark job

fabio apache-airflow-job list         List Airflow jobs
fabio apache-airflow-job show         Show Airflow job details
fabio apache-airflow-job create       Create an Airflow job
fabio apache-airflow-job update       Update Airflow job properties
fabio apache-airflow-job delete       Delete an Airflow job
fabio apache-airflow-job get-definition    Get definition
fabio apache-airflow-job update-definition Update definition
fabio apache-airflow-job start-environment Start Airflow runtime
fabio apache-airflow-job stop-environment  Stop Airflow runtime
fabio apache-airflow-job get-environment   Get environment status
fabio apache-airflow-job list-libraries    List installed libraries
fabio apache-airflow-job deploy-requirements Deploy pip requirements
fabio apache-airflow-job get-settings      Get environment settings
fabio apache-airflow-job update-settings   Update environment settings
fabio apache-airflow-job get-compute       Get compute configuration
fabio apache-airflow-job list-files        List DAG files
fabio apache-airflow-job get-file          Download a file
fabio apache-airflow-job upload-file       Upload a file
fabio apache-airflow-job delete-file       Delete a file
fabio apache-airflow-job get-workspace-settings Get workspace Airflow settings
fabio apache-airflow-job update-workspace-settings Update workspace settings
fabio apache-airflow-job list-pool-templates List pool templates
fabio apache-airflow-job create-pool-template Create a pool template
fabio apache-airflow-job get-pool-template   Get a pool template
fabio apache-airflow-job delete-pool-template Delete a pool template
```

### Graph & Digital Twins

```
fabio graphql-api list       List GraphQL APIs
fabio graphql-api show       Show GraphQL API details
fabio graphql-api create     Create a GraphQL API
fabio graphql-api update     Update GraphQL API properties
fabio graphql-api delete     Delete a GraphQL API
fabio graphql-api get-definition   Get definition (schema.graphql)
fabio graphql-api update-definition Update definition
fabio graphql-api query      Execute a GraphQL query

fabio graph-model list       List graph models
fabio graph-model show       Show graph model details
fabio graph-model create     Create a graph model (--ontology)
fabio graph-model update     Update graph model properties
fabio graph-model delete     Delete a graph model
fabio graph-model get-definition   Get definition
fabio graph-model update-definition Update definition
fabio graph-model refresh    Trigger a graph refresh job
fabio graph-model execute-query Run a graph query (KQL)
fabio graph-model get-queryable-graph-type Get queryable type
fabio graph-model initialize Initialize a graph model for querying

fabio graph-query-set list   List graph query sets
fabio graph-query-set show   Show graph query set details
fabio graph-query-set create Create a graph query set
fabio graph-query-set update Update properties
fabio graph-query-set delete Delete a graph query set
fabio graph-query-set get-definition   Get definition
fabio graph-query-set update-definition Update definition

fabio digital-twin-builder list   List digital twin builders
fabio digital-twin-builder show   Show details
fabio digital-twin-builder create Create a digital twin builder
fabio digital-twin-builder update Update properties
fabio digital-twin-builder delete Delete a digital twin builder
fabio digital-twin-builder get-definition   Get definition
fabio digital-twin-builder update-definition Update definition

fabio digital-twin-builder-flow list   List DTB flows
fabio digital-twin-builder-flow show   Show flow details
fabio digital-twin-builder-flow create Create a flow (--dtb-id)
fabio digital-twin-builder-flow update Update flow properties
fabio digital-twin-builder-flow delete Delete a flow
fabio digital-twin-builder-flow get-definition   Get definition
fabio digital-twin-builder-flow update-definition Update definition

fabio map list               List maps (geospatial visualization)
fabio map show               Show map details
fabio map create             Create a map
fabio map update             Update map properties
fabio map delete             Delete a map
fabio map get-definition     Get definition (map.json)
fabio map update-definition  Update definition
```

### Mirroring & External Data

```
fabio mirrored-database list   List mirrored databases
fabio mirrored-database show   Show mirrored database details
fabio mirrored-database create Create a mirrored database
fabio mirrored-database update Update properties
fabio mirrored-database delete Delete a mirrored database
fabio mirrored-database get-definition   Get definition
fabio mirrored-database update-definition Update definition
fabio mirrored-database start  Start mirroring
fabio mirrored-database stop   Stop mirroring
fabio mirrored-database status Get mirroring status
fabio mirrored-database table-status Get table mirroring status

fabio mirrored-catalog list  List mirrored catalogs
fabio mirrored-catalog show  Show mirrored catalog details
fabio mirrored-catalog create Create a mirrored catalog
fabio mirrored-catalog update Update properties
fabio mirrored-catalog delete Delete a mirrored catalog
fabio mirrored-catalog get-definition   Get definition
fabio mirrored-catalog update-definition Update definition
fabio mirrored-catalog refresh-metadata Refresh catalog metadata
fabio mirrored-catalog list-scopes      List catalog mirroring scopes
fabio mirrored-catalog list-tables      List catalog mirroring tables
fabio mirrored-catalog mirroring-status Get mirroring status
fabio mirrored-catalog tables-mirroring-status Get tables mirroring status

fabio mirrored-databricks-catalog list   List Databricks catalogs
fabio mirrored-databricks-catalog show   Show catalog details
fabio mirrored-databricks-catalog create Create a Databricks catalog
fabio mirrored-databricks-catalog update Update properties
fabio mirrored-databricks-catalog delete Delete a catalog
fabio mirrored-databricks-catalog get-definition   Get definition
fabio mirrored-databricks-catalog update-definition Update definition
fabio mirrored-databricks-catalog refresh-metadata Refresh catalog metadata
fabio mirrored-databricks-catalog discover-catalogs Discover available catalogs
fabio mirrored-databricks-catalog discover-schemas  Discover schemas in a catalog
fabio mirrored-databricks-catalog discover-tables   Discover tables in a schema

fabio mirrored-warehouse list  List mirrored warehouses

fabio cosmos-db-database list  List Cosmos DB databases
fabio cosmos-db-database show  Show Cosmos DB database details
fabio cosmos-db-database create Create a Cosmos DB database
fabio cosmos-db-database update Update properties
fabio cosmos-db-database delete Delete a Cosmos DB database
fabio cosmos-db-database get-definition   Get definition
fabio cosmos-db-database update-definition Update definition

fabio snowflake-database list  List Snowflake databases
fabio snowflake-database show  Show Snowflake database details
fabio snowflake-database create Create a Snowflake database
fabio snowflake-database update Update properties
fabio snowflake-database delete Delete a Snowflake database
fabio snowflake-database get-definition   Get definition
fabio snowflake-database update-definition Update definition

fabio mounted-data-factory list  List mounted data factories
fabio mounted-data-factory show  Show details
fabio mounted-data-factory create Create (--adf-resource-id)
fabio mounted-data-factory update Update properties
fabio mounted-data-factory delete Delete a mounted data factory
fabio mounted-data-factory get-definition   Get definition
fabio mounted-data-factory update-definition Update definition

fabio variable-library list  List variable libraries
fabio variable-library show  Show variable library details
fabio variable-library create Create a variable library
fabio variable-library update Update properties
fabio variable-library delete Delete a variable library
fabio variable-library get-definition   Get definition
fabio variable-library update-definition Update definition

fabio user-data-function list  List user data functions
fabio user-data-function show  Show function details
fabio user-data-function create Create a user data function
fabio user-data-function update Update properties
fabio user-data-function delete Delete a function
fabio user-data-function get-definition   Get definition
fabio user-data-function update-definition Update definition
```

### Integration & DevOps

```
fabio git status             Show workspace Git status (changes, conflicts)
fabio git commit             Commit workspace changes to remote
fabio git pull               Pull remote changes into workspace
fabio git connect            Connect a workspace to a Git repo
fabio git disconnect         Disconnect a workspace from Git
fabio git init               Initialize Git connection (required after connect)
fabio git checkout           Switch to a different branch (disconnect + connect + init)
fabio git connection show    Show Git connection details
fabio git credentials show   Show Git credentials configuration
fabio git credentials update Update Git credentials configuration
fabio git show-tracked       Show tracked items and Git sync status

fabio connection list        List all connections
fabio connection show        Show connection details
fabio connection create      Create a new connection
fabio connection update      Update connection (name, credentials, privacy)
fabio connection delete      Delete a connection
fabio connection list-supported-types List supported connection types
fabio connection test-connection Test a connection
fabio connection list-role-assignments List role assignments
fabio connection add-role-assignment Add a role assignment
fabio connection show-role-assignment Show a role assignment
fabio connection update-role-assignment Update a role assignment
fabio connection delete-role-assignment Delete a role assignment

fabio deployment-pipeline list   List deployment pipelines
fabio deployment-pipeline show   Show pipeline details
fabio deployment-pipeline create Create a pipeline
fabio deployment-pipeline update Update pipeline properties
fabio deployment-pipeline delete Delete a pipeline
fabio deployment-pipeline list-stages List stages
fabio deployment-pipeline show-stage  Show stage details
fabio deployment-pipeline update-stage Update stage configuration
fabio deployment-pipeline list-stage-items List items in a stage
fabio deployment-pipeline assign-workspace   Assign workspace to a stage
fabio deployment-pipeline unassign-workspace Unassign workspace
fabio deployment-pipeline list-operations   List deploy operation history
fabio deployment-pipeline show-operation    Show deploy operation details
fabio deployment-pipeline list-role-assignments List role assignments
fabio deployment-pipeline add-role-assignment   Add a role assignment
fabio deployment-pipeline delete-role-assignment Delete a role assignment
fabio deployment-pipeline deploy Deploy items between stages

fabio domain list            List domains in the tenant
fabio domain show            Show domain details
fabio domain create          Create a domain
fabio domain update          Update domain properties
fabio domain delete          Delete a domain
fabio domain list-workspaces List workspaces in a domain
fabio domain assign-workspaces   Assign workspaces to a domain
fabio domain unassign-workspaces Unassign workspaces
fabio domain assign-by-capacity  Bulk-assign workspaces by capacity
fabio domain assign-by-principal Bulk-assign workspaces by principal

fabio job-scheduler list-instances List job instances for an item
fabio job-scheduler get-instance   Get job instance details
fabio job-scheduler run-on-demand  Run an on-demand job
fabio job-scheduler cancel-instance Cancel a running instance
fabio job-scheduler list-schedules List schedules for an item
fabio job-scheduler get-schedule   Get schedule details
fabio job-scheduler create-schedule Create a schedule
fabio job-scheduler update-schedule Update a schedule
fabio job-scheduler delete-schedule Delete a schedule

fabio deploy plan            Plan deployment (diff source directory vs live workspace)
fabio deploy apply           Apply deployment (create/update/rename/delete items)
fabio deploy export          Export a workspace to a source directory
fabio deploy init-params     Generate parameter file from GUIDs/diffs
fabio deploy validate        Validate source directory offline (no API calls)
```

### Security & Governance

```
fabio onelake-security list  List data access roles
fabio onelake-security show  Show a data access role
fabio onelake-security create Create a data access role (--conflict-policy)
fabio onelake-security upsert Create or replace all roles
fabio onelake-security delete Delete a data access role

fabio managed-private-endpoint list   List managed private endpoints
fabio managed-private-endpoint show   Show endpoint details
fabio managed-private-endpoint create Create a managed private endpoint
fabio managed-private-endpoint delete Delete a managed private endpoint

fabio gateway list           List gateways
fabio gateway show           Show gateway details
fabio gateway create         Create a VNet gateway
fabio gateway update         Update gateway properties
fabio gateway delete         Delete a gateway
fabio gateway list-members   List gateway members
fabio gateway update-member  Update a gateway member
fabio gateway delete-member  Delete a gateway member
fabio gateway list-role-assignments   List role assignments
fabio gateway add-role-assignment     Add a role assignment
fabio gateway show-role-assignment    Show a role assignment
fabio gateway update-role-assignment  Update a role assignment
fabio gateway delete-role-assignment  Delete a role assignment
```

### Administration

```
fabio admin list-tenant-settings List all tenant settings
fabio admin update-tenant-setting Update a tenant setting
fabio admin list-capacities-tenant-overrides List all capacity overrides
fabio admin list-capacity-tenant-overrides List overrides for a capacity
fabio admin delete-capacity-tenant-override Delete a capacity override
fabio admin update-capacity-tenant-override Update a capacity override
fabio admin list-domains-tenant-overrides List all domain overrides
fabio admin list-workspaces-tenant-overrides List all workspace overrides
fabio admin list-tags        List tags
fabio admin create-tags      Bulk-create tags
fabio admin update-tag       Update a tag
fabio admin delete-tag       Delete a tag
fabio admin list-workloads   List workloads
fabio admin list-workload-assignments List workload assignments
fabio admin create-workload-assignment Create a workload assignment
fabio admin delete-workload-assignment Delete a workload assignment
fabio admin list-workspaces  List workspaces (admin view)
fabio admin show-workspace   Show workspace details (admin)
fabio admin list-workspace-users List users in a workspace (admin)
fabio admin list-git-connections List git connections across workspaces
fabio admin grant-admin-access Grant temporary admin access
fabio admin remove-admin-access Remove temporary admin access
fabio admin restore-workspace Restore a deleted workspace
fabio admin list-network-policies List network policies
fabio admin list-items       List items (admin view)
fabio admin show-item        Show item details (admin)
fabio admin list-item-users  List users with access to an item (admin)
fabio admin bulk-set-labels  Bulk-set sensitivity labels on items
fabio admin bulk-remove-labels Bulk-remove sensitivity labels
fabio admin list-external-data-shares List external data shares
fabio admin revoke-external-data-share Revoke an external data share
fabio admin remove-all-sharing-links Remove all sharing links for items
fabio admin bulk-remove-sharing-links Bulk-remove sharing links
fabio admin list-domains     List domains (admin view)
fabio admin create-domain    Create a domain
fabio admin show-domain      Show domain details
fabio admin update-domain    Update a domain
fabio admin delete-domain    Delete a domain
fabio admin list-domain-workspaces List workspaces in a domain
fabio admin assign-domain-workspaces Assign workspaces to a domain
fabio admin unassign-domain-workspaces Unassign workspaces from domain
fabio admin unassign-all-domain-workspaces Unassign all from domain
fabio admin list-domain-role-assignments List domain role assignments
fabio admin bulk-assign-domain-roles Bulk-assign domain roles
fabio admin bulk-unassign-domain-roles Bulk-unassign domain roles
fabio admin sync-domain-roles-to-subdomains Sync roles to subdomains
fabio admin assign-domain-workspaces-by-capacities Assign by capacities
fabio admin assign-domain-workspaces-by-principals Assign by principals
fabio admin list-user-access List access details for a user
```

### Configuration & Tooling

```
fabio rest call              Raw REST API passthrough (Fabric or Power BI)

fabio profile save           Save a named profile with default settings
fabio profile use            Set the active profile
fabio profile list           List all saved profiles
fabio profile show           Show profile details
fabio profile delete         Delete a profile

fabio jobs list              List recent jobs from local ledger
fabio jobs get               Get details of a specific job
fabio jobs prune             Remove completed/failed jobs

fabio feedback send          Record feedback about CLI friction
fabio feedback list          List recorded feedback entries

fabio operation get-state    Get state of a long-running operation
fabio operation get-result   Get result of a completed operation

fabio agent-context          Machine-readable command schema for AI agents

fabio completions <shell>    Generate shell completion scripts (bash/zsh/fish/powershell/elvish)
```

## Authentication

Fabio authenticates with its own dedicated Entra ID application ("Fabio CLI"). It also supports the Azure credential chain as a fallback.

```bash
# Login with Fabio CLI identity (device code flow, works on any machine)
fabio auth login

# Verify authentication
fabio auth status
```

Supported credential sources (via `DefaultAzureCredential` fallback):
- Fabio CLI identity (`fabio auth login` -- recommended)
- Azure CLI (`az login`)
- Environment variables (`AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_CLIENT_SECRET`)
- Managed Identity (when running on Azure)

## Shell Completions

Generate tab-completion scripts for your shell. Completions cover all 69 command groups, 766 subcommands, and their flags.

### Bash

```bash
# Add to ~/.bashrc
eval "$(fabio completions bash)"
```

### Zsh

```bash
# Add to ~/.zshrc
eval "$(fabio completions zsh)"
```

Or, for faster shell startup (generates a static file):

```bash
fabio completions zsh > ~/.zfunc/_fabio
# Ensure ~/.zfunc is in your fpath (add to ~/.zshrc before compinit):
# fpath=(~/.zfunc $fpath)
```

### Fish

```bash
fabio completions fish > ~/.config/fish/completions/fabio.fish
```

### PowerShell

```powershell
# Add to your $PROFILE
fabio completions powershell | Out-String | Invoke-Expression
```

Or, for persistent completions:

```powershell
# Generate and save to profile directory
fabio completions powershell > "$HOME\Documents\PowerShell\Completions\fabio.ps1"
# Source in $PROFILE:
# . "$HOME\Documents\PowerShell\Completions\fabio.ps1"
```

### Elvish

```bash
fabio completions elvish >> ~/.config/elvish/rc.elv
```

After setting up completions, restart your shell or source the configuration file. Then use `Tab` to complete commands, subcommands, and flags:

```
fabio lak<Tab>         → fabio lakehouse
fabio lakehouse <Tab>  → list  show  create  upload  ...
fabio lakehouse list --out<Tab> → --output
```

## Examples

### Output formats and filtering

```bash
# JSON (default) -- structured envelope for agents
fabio workspace list
# {"data":[...],"count":5}

# Table -- human-readable columns
fabio workspace list -o table

# Plain -- one value per line, great for shell scripting
fabio workspace list -o plain

# Field projection (extract a single field from each item)
fabio workspace list --query displayName

# Limit results
fabio workspace list --limit 3

# Fetch all pages automatically
fabio item list --workspace $WS --all

# Resume from a previous pagination token
fabio item list --workspace $WS --continuation-token "eyJ..."

# Suppress output (useful in scripts -- errors still go to stderr)
fabio lakehouse delete --workspace $WS --id $LH --quiet

# Dry-run a mutation to preview what would happen
fabio workspace delete --id $WS --dry-run
```

### Workspace management

```bash
# Create a workspace and assign a capacity
fabio workspace create --name "my-project"
fabio workspace assign-capacity --id $WS --capacity $CAP_ID

# Provision a managed identity for the workspace
fabio workspace provision-identity --id $WS

# Add a contributor
fabio workspace add-role-assignment --id $WS \
  --principal $USER_ID --principal-type User --role Contributor

# Organize with folders
fabio workspace create-folder --id $WS --name "Production"
fabio workspace create-folder --id $WS --name "Staging"

# Tag workspaces for governance
fabio workspace apply-tags --id $WS --tag-ids '["tag-uuid-1"]'

# Assign to a domain
fabio workspace assign-to-domain --id $WS --domain $DOMAIN_ID

# Configure OneLake storage tier
fabio workspace modify-default-tier --id $WS --tier Cold
```

### Lakehouse -- files and tables

```bash
# Create a lakehouse
fabio lakehouse create --workspace $WS --name "DataLake"

# Upload files (glob patterns, parallel)
fabio lakehouse upload --workspace $WS --id $LH --source "data/*.parquet" --dest Files/raw/

# Upload a single file and immediately load it as a Delta table
fabio lakehouse upload-table --workspace $WS --id $LH \
  --source sales.csv --table sales --mode Overwrite --format Csv

# Load an already-uploaded file into a Delta table
fabio lakehouse load-table --workspace $WS --id $LH \
  --path Files/raw/products.parquet --table products --mode Append --format Parquet

# List tables
fabio lakehouse list-tables --workspace $WS --id $LH -o table

# List files in a directory
fabio lakehouse list-files --workspace $WS --id $LH --path Files/raw/

# Download a file
fabio lakehouse download --workspace $WS --id $LH --path Files/raw/report.csv --dest ./local/

# Copy files between lakehouses (parallel, glob)
fabio lakehouse copy-file --workspace $WS --id $LH \
  --source "Files/raw/*.csv" --dest-workspace $WS2 --dest-id $LH2 --dest Files/imported/

# Move files (copy + delete source)
fabio lakehouse move-file --workspace $WS --id $LH \
  --source "Files/staging/*" --dest Files/archive/

# Copy a Delta table between lakehouses
fabio lakehouse copy-table --workspace $WS --id $LH --table sales \
  --dest-workspace $WS2 --dest-id $LH2

# Sync files between lakehouses (only copies new/modified based on ETag)
fabio lakehouse sync --workspace $WS --id $LH \
  --dest-workspace $WS2 --dest-id $LH2 --delete

# Create a shortcut to an ADLS Gen2 container
fabio lakehouse create-shortcut --workspace $WS --id $LH \
  --name "external-data" --path Files/ \
  --target-type adls --location "https://storageacct.dfs.core.windows.net/container" \
  --subpath "data/2024/"

# Run table maintenance (optimize, vacuum)
fabio lakehouse run-table-maintenance --workspace $WS --id $LH

# Trigger materialized view refresh
fabio lakehouse refresh-materialized-views --workspace $WS --id $LH
```

### Notebooks

```bash
# Create a notebook bound to a lakehouse
fabio notebook create --workspace $WS --name "ETL-Pipeline" \
  --lakehouse $LH --source etl_notebook.py

# Run and wait for completion (default timeout 600s)
fabio notebook run --workspace $WS --id $NB --wait

# Run with parameters
fabio notebook run --workspace $WS --id $NB --wait \
  --parameters '[{"name":"start_date","value":"2024-01-01","type":"Text"}]'

# Run with a custom timeout
fabio notebook run --workspace $WS --id $NB --wait --timeout 1800

# Check status of a running notebook
fabio notebook status --workspace $WS --id $NB

# Stop a long-running notebook
fabio notebook stop --workspace $WS --id $NB

# Get notebook source code (strip outputs for clean diffs)
fabio notebook get-definition --workspace $WS --id $NB --strip-output

# Update notebook code from file
fabio notebook update-definition --workspace $WS --id $NB --source updated_etl.py
```

### Warehouse and SQL

```bash
# Create a warehouse
fabio warehouse create --workspace $WS --name "SalesWarehouse"

# Run a SQL query (inline)
fabio warehouse query --workspace $WS --id $WH \
  --sql "SELECT TOP 100 * FROM dbo.fact_sales ORDER BY order_date DESC"

# Run SQL from a file
fabio warehouse query --workspace $WS --id $WH --sql @queries/monthly_report.sql

# Pipe SQL via stdin
echo "SELECT COUNT(*) as total FROM dbo.customers" | fabio warehouse query --workspace $WS --id $WH

# Get connection string for external tools
fabio warehouse connection-string --workspace $WS --id $WH

# Create and manage restore points
fabio warehouse create-restore-point --workspace $WS --id $WH --name "pre-migration"
fabio warehouse list-restore-points --workspace $WS --id $WH -o table
```

### SQL Database

```bash
# Create a SQL database
fabio sql-database create --workspace $WS --name "OrdersDB"

# Import a CSV file (auto-creates table with inferred types)
fabio sql-database import --workspace $WS --id $DB \
  --file orders.csv --table orders --drop-if-exists

# Import JSON data
fabio sql-database import --workspace $WS --id $DB \
  --file events.json --table events --batch-size 500

# Query the database
fabio sql-database query --workspace $WS --id $DB \
  --sql "SELECT status, COUNT(*) as cnt FROM orders GROUP BY status"

# Get TDS connection string
fabio sql-database connection-string --workspace $WS --id $DB
```

### Real-Time Intelligence (Eventhouse + EventStream)

```bash
# Create the RTI stack
fabio eventhouse create --workspace $WS --name "TelemetryHub"
fabio kql-database create --workspace $WS --name "SensorDB" --eventhouse-id $EH

# Create a table and ingestion mapping
fabio kql-database query --workspace $WS --id $KDB \
  --kql ".create table SensorEvents (DeviceId: string, Temperature: real, Timestamp: datetime)"
fabio kql-database query --workspace $WS --id $KDB \
  --kql ".create table SensorEvents ingestion json mapping 'JsonMapping' '[{\"column\":\"DeviceId\",\"path\":\"$.deviceId\"},{\"column\":\"Temperature\",\"path\":\"$.temperature\"},{\"column\":\"Timestamp\",\"path\":\"$.timestamp\"}]'"

# Create an eventstream with a custom endpoint source
fabio eventstream create --workspace $WS --name "SensorIngestion"
fabio eventstream add-source --workspace $WS --id $ES \
  --name "app-source" --source-type CustomEndpoint

# Add an Eventhouse destination (DirectIngestion with pre-created mapping)
fabio eventstream add-destination --workspace $WS --id $ES \
  --name "kql-sink" --destination-type Eventhouse --input-node "app-source-stream" \
  --properties '{"dataIngestionMode":"DirectIngestion","workspaceId":"'$WS'","itemId":"'$KDB'","tableName":"SensorEvents","connectionName":"es-conn-1","mappingRuleName":"JsonMapping"}'

# Get Event Hub connection info for sending events
fabio eventstream get-source-connection --workspace $WS --id $ES --source-id $SRC

# Query the ingested data
fabio kql-database query --workspace $WS --id $KDB \
  --kql "SensorEvents | where Timestamp > ago(1h) | summarize avg(Temperature) by DeviceId"

# Run a saved queryset tab
fabio kql-queryset run --workspace $WS --id $QS --tab "Hourly Summary"
```

### Semantic Models and Reports

```bash
# Create a Direct Lake semantic model from TMDL files
fabio semantic-model create --workspace $WS --name "SalesModel" \
  --file model.tmdl --connection $SQL_ENDPOINT_ID

# Refresh (frame) the Direct Lake model
fabio semantic-model refresh --workspace $WS --id $SM

# Take over so it's editable in the portal
fabio semantic-model takeover --workspace $WS --id $SM

# Execute a DAX query
fabio semantic-model query --workspace $WS --id $SM \
  --dax "EVALUATE SUMMARIZECOLUMNS('Sales'[Country], \"Revenue\", SUM('Sales'[Amount]))"

# Create a report bound to the semantic model
fabio report create --workspace $WS --name "Sales Dashboard" --dataset $SM

# Update report visuals from definition files
fabio report update-definition --workspace $WS --id $RPT \
  --file definition.pbir --report-json report.json
```

### Data Pipelines

```bash
# Create and run a data pipeline
fabio data-pipeline create --workspace $WS --name "Daily-ETL"
fabio data-pipeline run --workspace $WS --id $DP

# Schedule a pipeline
fabio data-pipeline create-schedule --workspace $WS --id $DP \
  --content '{"enabled":true,"configuration":{"type":"Daily","startTime":"06:00"}}'

# Check pipeline run status
fabio job-scheduler list-instances --workspace $WS --id $DP --job-type Pipeline -o table
```

### Git integration (CI/CD)

```bash
# Connect a workspace to a Git repo
fabio git connect --workspace $WS \
  --provider github --owner myorg --repo fabric-project --branch main \
  --directory "/" --connection-id $GIT_CONN

# Initialize (first-time sync)
fabio git init --workspace $WS --strategy prefer-workspace

# Check what changed
fabio git status --workspace $WS -o table

# Commit workspace changes
fabio git commit --workspace $WS --message "feat: add sales pipeline" --wait

# Pull remote changes
fabio git pull --workspace $WS --strategy prefer-remote --wait

# Switch branch
fabio git checkout --workspace $WS --branch feature/new-model --strategy prefer-remote
```

### Deployment Pipelines

```bash
# Create a deployment pipeline (Dev -> Test -> Prod)
fabio deployment-pipeline create --name "Analytics Pipeline"

# Assign workspaces to stages
fabio deployment-pipeline assign-workspace --id $DP --stage-id $DEV_STAGE --workspace $DEV_WS
fabio deployment-pipeline assign-workspace --id $DP --stage-id $PROD_STAGE --workspace $PROD_WS

# Deploy from Dev to Prod
fabio deployment-pipeline deploy --id $DP \
  --source-stage $DEV_STAGE --target-stage $PROD_STAGE --wait

# Deploy specific items only
fabio deployment-pipeline deploy --id $DP \
  --source-stage $DEV_STAGE --items '[{"itemId":"abc","itemType":"Notebook"}]'
```

### Deploy (CI/CD Engine)

The `deploy` command group provides stateless, content-hash-based convergence for Fabric workspaces -- similar to Terraform but without a state file. It always diffs against the live workspace.

```bash
# Export a workspace to a local directory (source of truth)
fabio deploy export --workspace "Production" --dir ./fabric-items/ --overwrite

# Plan changes (diff source directory against live workspace)
fabio deploy plan --source ./fabric-items/ --workspace "Staging"

# Apply changes (create/update/rename/delete items in dependency order)
fabio deploy apply --source ./fabric-items/ --workspace "Staging"

# Preview without executing
fabio deploy apply --source ./fabric-items/ --workspace "Staging" --dry-run

# Deploy with environment-specific parameters
fabio deploy apply --source ./fabric-items/ --workspace "Production" \
  --parameters params.json --env prod

# Delete items in workspace that aren't in source (opt-in)
fabio deploy apply --source ./fabric-items/ --workspace "Staging" --delete-orphans

# Save a plan for later (staleness-protected)
fabio deploy plan --source ./fabric-items/ --workspace "Staging" --out plan.json
fabio deploy apply --plan plan.json

# Generate a parameter file by scanning for GUIDs
fabio deploy init-params --source ./fabric-items/ --out params.json

# Generate parameters by diffing two environments
fabio deploy init-params --source ./dev-items/ --compare ./prod-items/ \
  --source-env dev --compare-env prod --out params.json
```

Deploy handles 42 item types in dependency order, supports parallel execution (default 8 concurrent), rename detection via logical IDs, and automatic post-deploy hooks (semantic model refresh, environment publish).

### Connections and Gateways

```bash
# List available connection types
fabio connection list-supported-types -o table

# Create a connection to Azure SQL
fabio connection create --name "AzureSQL-Prod" \
  --connectivity-type ShareableCloud --type Sql \
  --parameters '{"server":"myserver.database.windows.net","database":"mydb"}' \
  --credential-type Basic --credentials '{"username":"admin","password":"***"}'

# Test the connection
fabio connection test-connection --id $CONN

# Create a VNet gateway for private connectivity
fabio gateway create --name "data-gateway" --capacity $CAP \
  --subscription $SUB --resource-group $RG \
  --vnet myVNet --subnet data-subnet
```

### Spark pools and jobs

```bash
# Get workspace Spark settings
fabio spark get-settings --workspace $WS

# Create a custom Spark pool
fabio spark create-pool --workspace $WS \
  --content '{"name":"HighMem","nodeFamily":"MemoryOptimized","nodeSize":"Large","autoScale":{"enabled":true,"minNodeCount":1,"maxNodeCount":10}}'

# Create and run a Spark job definition
fabio spark-job-definition create --workspace $WS --name "DailyAgg" --file spark_job.json
fabio spark-job-definition run --workspace $WS --id $SJD --wait
```

### Apache Airflow

```bash
# Create an Airflow job
fabio apache-airflow-job create --workspace $WS --name "DataOrchestrator"

# Start the Airflow environment
fabio apache-airflow-job start-environment --workspace $WS --id $AJ

# Upload a DAG file
fabio apache-airflow-job upload-file --workspace $WS --id $AJ \
  --path dags/daily_etl.py --source ./dags/daily_etl.py

# Deploy pip requirements
fabio apache-airflow-job deploy-requirements --workspace $WS --id $AJ \
  --content "pandas>=2.0\npyarrow>=14.0"

# Check environment status
fabio apache-airflow-job get-environment --workspace $WS --id $AJ
```

### Data Agents (AI-powered Q&A)

```bash
# Create a data agent
fabio data-agent create --workspace $WS --name "SalesAssistant"

# Configure its definition (instructions + data sources)
fabio data-agent update-definition --workspace $WS --id $DA --file agent_config/

# Query the agent
fabio data-agent query --workspace $WS --id $DA \
  --prompt "Show me the top 5 customers by revenue this quarter"
```

### ML Models

```bash
# Create and manage ML models
fabio ml-model create --workspace $WS --name "ChurnPredictor"

# Score against a deployed model endpoint
fabio ml-model score --workspace $WS --id $MODEL \
  --content '{"input_data":{"columns":["age","tenure","monthly_charges"],"data":[[35,24,79.95]]}}'

# Manage model versions
fabio ml-model list-versions --workspace $WS --id $MODEL -o table
fabio ml-model activate-version --workspace $WS --id $MODEL --version-id $V
```

### Environments

```bash
# Create an environment with custom Spark settings
fabio environment create --workspace $WS --name "DataScience-Env"

# Add libraries to staging
fabio environment import-staging-libraries --workspace $WS --id $ENV \
  --content '{"libraries":[{"name":"scikit-learn","version":"1.4.0"}]}'

# Publish staged changes
fabio environment publish --workspace $WS --id $ENV
```

### Mirroring

```bash
# Create a mirrored database
fabio mirrored-database create --workspace $WS --name "MirroredSalesDB"

# Start and monitor mirroring
fabio mirrored-database start --workspace $WS --id $MDB
fabio mirrored-database status --workspace $WS --id $MDB
fabio mirrored-database table-status --workspace $WS --id $MDB -o table

# Cosmos DB mirroring
fabio cosmos-db-database create --workspace $WS --name "CosmosOrders"
```

### Graph Models and Ontologies

```bash
# Create an ontology with entity types
fabio ontology create --workspace $WS --name "ManufacturingOntology"
fabio ontology update-definition --workspace $WS --id $ONT --dir ./ontology/

# Create a graph model linked to the ontology
fabio graph-model create --workspace $WS --name "FactoryGraph" --ontology $ONT

# After loading, query the graph
fabio graph-model execute-query --workspace $WS --id $GM \
  --query "nodes('Equipment') | where status == 'Offline'"
```

### Security and Governance

```bash
# Set OneLake data access roles (row/column-level security)
fabio onelake-security upsert --workspace $WS --id $LH \
  --content '[{"name":"AnalystRole","members":[{"principalId":"...","principalType":"User"}],"decisionRules":[{"effect":"Permit","paths":["Tables/sales"]}]}]'

# Create a managed private endpoint
fabio managed-private-endpoint create --workspace $WS \
  --name "sql-private" --resource-id "/subscriptions/.../Microsoft.Sql/servers/myserver" \
  --group-id sqlServer

# Domain management
fabio domain create --name "Finance"
fabio domain assign-workspaces --id $DOM --workspace-ids '["ws1","ws2"]'
```

### Named profiles

```bash
# Save profiles for different environments
fabio profile save --name dev --workspace $DEV_WS
fabio profile save --name prod --workspace $PROD_WS

# Use a profile (sets defaults for --workspace, etc.)
fabio profile use --name dev

# Override profile with explicit flags
fabio lakehouse list-tables --id $LH --profile prod

# List profiles
fabio profile list -o table
```

### Composability and scripting

```bash
# Pipe workspace IDs into another command
fabio workspace list --query id -o plain | while read ws; do
  fabio item list --workspace "$ws" --type Lakehouse --limit 1
done

# Use jq for complex JSON processing
fabio lakehouse list-tables --workspace $WS --id $LH | jq '.data[].name'

# Combine with standard tools
fabio item list --workspace $WS -o plain --query displayName | sort | uniq -c | sort -rn

# Machine-readable schema for building AI agents on top of fabio
fabio agent-context | jq '.commands[] | select(.group == "lakehouse")'
```

## Development

```bash
git clone https://github.com/iemejia/fabio.git && cd fabio

# Build
cargo build

# Run tests (unit + offline integration -- 624 tests)
cargo test

# Run E2E tests (requires live Fabric tenant -- 662 tests)
cargo test -- --ignored

# Lint (pedantic + nursery, zero warnings required)
cargo clippy --tests -- -D warnings

# Format
cargo fmt
```

### CI/CD

- GitHub Actions CI runs on 6 targets: x64 + arm64 for Linux, macOS, and Windows
- Release workflow: tag-triggered, builds 5 binaries with SHA256 checksums (Linux x64/arm64, macOS arm64, Windows x64/arm64)
- Dependabot auto-merge for passing dependency updates
- CodeQL and Secret Scanning enabled

### Project Stats

- **69 command groups** with **766 subcommands**
- **1286 tests** (487 unit + 137 offline integration + 662 E2E requiring live tenant)
- **~16 MB** release binary (stripped, full LTO, panic=abort)
- Zero clippy warnings, zero unsafe code

## License

MIT
