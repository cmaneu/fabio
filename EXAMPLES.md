# Examples

## Output formats and filtering

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

## Workspace management

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

## Lakehouse -- files and tables

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
fabio lakehouse sync --source-workspace $WS --source-id $LH --source-path Files/data \
  --dest-workspace $WS2 --dest-id $LH2 --dest-path Files/data --delete

# Sync with rename detection (detects moved/renamed files via content matching)
# Uses Content-MD5 or unique file size to match — performs O(1) atomic rename
# at the destination instead of re-copying large files
fabio lakehouse sync --source-workspace $WS --source-id $LH --source-path Files/data \
  --dest-workspace $WS2 --dest-id $LH2 --dest-path Files/data --delete --checksum

# Sync only CSV and Parquet files, skip temp files
fabio lakehouse sync --source-workspace $WS --source-id $LH --source-path Files/data \
  --dest-workspace $WS2 --dest-id $LH2 --dest-path Files/data \
  --include "*.csv;*.parquet" --exclude "*.tmp;_delta_log/*"

# Sync with safety limit (abort deletions if more than 10 files would be deleted)
fabio lakehouse sync --source-workspace $WS --source-id $LH --source-path Files/data \
  --dest-workspace $WS2 --dest-id $LH2 --dest-path Files/data \
  --delete --max-delete 10

# Move files: sync and delete source files after successful transfer
fabio lakehouse sync --source-workspace $WS --source-id $LH --source-path Files/inbox \
  --dest-workspace $WS2 --dest-id $LH2 --dest-path Files/archive \
  --remove-source-files

# Sync only files that already exist at dest (refresh without creating new)
fabio lakehouse sync --source-workspace $WS --source-id $LH --source-path Files/data \
  --dest-workspace $WS2 --dest-id $LH2 --dest-path Files/data \
  --existing --force

# Sync local directory to lakehouse (incremental upload of new/changed files)
fabio lakehouse sync --local ./data/ \
  --dest-workspace $WS --dest-id $LH --dest-path Files/data

# Local sync with checksum (only upload if local MD5 differs from remote)
fabio lakehouse sync --local ./exports/ \
  --dest-workspace $WS --dest-id $LH --dest-path Files/exports --checksum

# Local sync with include filter and move semantics (delete local after upload)
fabio lakehouse sync --local ./inbox/ \
  --dest-workspace $WS --dest-id $LH --dest-path Files/archive \
  --include "*.csv;*.parquet" --remove-source-files

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

## Notebooks

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

## Warehouse and SQL

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

## SQL Database

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

## SQL Endpoint

```bash
# List SQL endpoints in a workspace (auto-created alongside lakehouses)
fabio sql-endpoint list --workspace $WS -o table

# Query a SQL endpoint directly (supports three-part naming for cross-lakehouse queries)
fabio sql-endpoint query --workspace $WS --id $SQLEP \
  --sql "SELECT TOP 10 * FROM dbo.sales ORDER BY order_date DESC"

# Query from a file
fabio sql-endpoint query --workspace $WS --id $SQLEP --sql @queries/analytics.sql

# Pipe SQL via stdin
echo "SELECT COUNT(*) as total FROM dbo.products" | fabio sql-endpoint query --workspace $WS --id $SQLEP

# Get connection string for external tools (SSMS, Azure Data Studio)
fabio sql-endpoint connection-string --workspace $WS --id $SQLEP
```

## Real-Time Intelligence (Eventhouse + EventStream)

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

## Semantic Models and Reports

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

## Data Pipelines

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

## Git integration (CI/CD)

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

## Deployment Pipelines

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

## Deploy (CI/CD Engine)

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

## For fabric-cicd users

If you're familiar with Microsoft's [fabric-cicd](https://github.com/microsoft/fabric-cicd) Python library, fabio is fully compatible with the same source directory format and parameter files. Here's how common workflows translate.

### Creating the source directory

In fabric-cicd, you typically export items from a git-connected workspace or build the `.platform` directory structure programmatically in Python. With fabio, there are three approaches:

**1. Export from an existing workspace (most common):**
```bash
# Export all items to a local directory — ready for version control
fabio deploy export --workspace "Development" --dir ./workspace/ --overwrite
```

**2. Let an AI coding agent create the source files:**

Since fabio's source format is just files on disk (`.platform` JSON + definition files), any coding agent (GitHub Copilot, Cursor, OpenCode, etc.) can generate them. For example:

> "Create a fabio deploy source directory with a Lakehouse called SalesLH, a Notebook that loads CSV data into a Delta table, and a DataPipeline that runs the notebook daily"

The agent creates the directory structure with `.platform` files and definition content — then you deploy with `fabio deploy apply`.

**3. Build incrementally with fabio commands, then export:**
```bash
# Create items interactively or via scripts
fabio lakehouse create --workspace $WS --name "SalesLH"
fabio notebook create --workspace $WS --name "ETL" --code "# transform logic"
fabio data-pipeline create --workspace $WS --name "Daily Load"

# Once satisfied, export to create your source of truth
fabio deploy export --workspace $WS --dir ./workspace/ --overwrite
```

All three approaches produce the same `.platform` directory structure that `fabio deploy apply` consumes.

### Basic deployment (equivalent workflows)

**fabric-cicd (Python):**
```python
from fabric_cicd import FabricWorkspace, publish_all_items
from azure.identity import DefaultAzureCredential

ws = FabricWorkspace(
    workspace_id="aaaabbbb-cccc-dddd-eeee-ffffffffffff",
    repository_directory="./workspace",
    environment="prod",
    item_type_in_scope=["Notebook", "DataPipeline"],
)
publish_all_items(ws)
```

**fabio (equivalent):**
```bash
fabio deploy apply \
  --source ./workspace \
  --workspace aaaabbbb-cccc-dddd-eeee-ffffffffffff \
  --item-types Notebook,DataPipeline
```

### Preview before deploying (fabio-only capability)

fabric-cicd has no dry-run or plan mode. fabio lets you preview:

```bash
# See what would change without touching anything
fabio deploy plan --source ./workspace --workspace "Production"

# Dry-run shows the plan and exits
fabio deploy apply --source ./workspace --workspace "Production" --dry-run
```

### Parameter substitution

Both tools use the same YAML format with the same four rule types. fabio reads `parameter.yml` directly — no conversion needed:

```yaml
# parameter.yml (works identically in both fabric-cicd and fabio)
find_replace:
  - find_value: "db52be81-c2b2-4261-84fa-840c67f4bbd0"
    replace_value:
      PPE: "81bbb339-8d0b-46e8-bfa6-289a159c0733"
      PROD: "$items.Lakehouse.SalesLH.id"
    item_type: "Notebook"
    file_path: "notebook-content.py"

key_value_replace:
  - find_key: $.variables[?(@.name=="Server")].value
    replace_value:
      PPE: "server-ppe.database.windows.net"
      PROD: "server-prod.database.windows.net"
    item_type: "VariableLibrary"

spark_pool:
  - instance_pool_id: "72c68dbc-0775-4d59-909d-a47896f4573b"
    replace_value:
      PPE:
        type: "Capacity"
        name: "Pool_Large_PPE"
      PROD:
        type: "Capacity"
        name: "Pool_Large_PROD"

semantic_model_binding:
  default:
    connection_id:
      PPE: "76e05dfe-9855-4e3d-a410-1dda048dbe99"
      PROD: "c4f8e2b1-3d2a-4f5b-9c6e-7a8b9c0d1e2f"
```

```bash
fabio deploy apply --source ./workspace --workspace $WS \
  --parameters parameter.yml --env PROD
```

### Dynamic variables

Both tools support the same dynamic variables (slight syntax difference):

| fabric-cicd | fabio | Resolves to |
|---|---|---|
| `$workspace.$id` | `$workspace.id` | Target workspace GUID |
| `$items.Type.Name.$id` | `$items.Type.Name.id` | Deployed item GUID |
| N/A | `$workspace.name` | Target workspace display name |
| N/A | `$ENV:VAR_NAME` | Environment variable value |
| N/A | `$items.Type.Name.sqlendpoint` | SQL endpoint connection string |
| N/A | `$items.Type.Name.queryserviceuri` | Eventhouse query URI |

### Config file (equivalent to fabric-cicd's `config.yml`)

**fabric-cicd `config.yml`:**
```yaml
core:
  workspace_id:
    PPE: "ws-ppe-guid"
    PROD: "ws-prod-guid"
  repository_directory: "."
  item_types_in_scope:
    - Notebook
    - DataPipeline
  parameter: "parameter.yml"
```

**fabio `deploy-config.yml`:**
```yaml
source: "."
parameters: "./parameters.json"

environments:
  PPE:
    workspace: "ws-ppe-guid"
  PROD:
    workspace: "ws-prod-guid"

filters:
  item_types:
    - Notebook
    - DataPipeline
```

```bash
fabio deploy apply --config deploy-config.yml --env PROD
```

### Git-diff selective deployment

**fabric-cicd:**
```python
from fabric_cicd import get_changed_items
changed = get_changed_items(ws.repository_directory, "HEAD~1")
publish_all_items(ws, items_to_include=changed)
```

**fabio (built-in):**
```bash
fabio deploy apply --source ./workspace --workspace $WS --git-diff HEAD~1
```

### Deleting orphaned items

**fabric-cicd:**
```python
# Requires feature flags for dangerous types
from fabric_cicd import append_feature_flag
append_feature_flag("enable_lakehouse_unpublish")
publish_all_items(ws)  # Unpublishes orphans automatically
```

**fabio:**
```bash
# Safe by default -- protected types need explicit opt-in
fabio deploy apply --source ./workspace --workspace $WS \
  --delete-orphans --allow-delete-types Lakehouse,Warehouse
```

### Folder management

Both tools auto-manage workspace folders from the source directory structure:

```
workspace/
├── ETL/                        ← becomes workspace folder "/ETL"
│   └── Transform.Notebook/
├── Reports/                    ← becomes workspace folder "/Reports"
│   └── Sales.Report/
└── SalesLH.Lakehouse/          ← root level (no folder)
```

fabric-cicd handles this automatically. fabio does too (disable with `--no-folders`).

### Additional capabilities in fabio

| Workflow | fabric-cicd | fabio |
|---|---|---|
| Check what will change | Not available | `fabio deploy plan` |
| Skip unchanged items | Re-uploads everything | SHA-256 hash comparison |
| Catch errors before deploy | Not available | `fabio deploy validate` |
| Scaffold parameter file | Manual | `fabio deploy init-params` |
| Export workspace to disk | Not available | `fabio deploy export` |
| Rename an item | Delete + re-create (new GUID) | Detects via logical ID (preserves GUID) |
| CI/CD runtime | Python 3.9+ with pip deps | Single binary (curl install) |

## Connections and Gateways

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

## Spark pools and jobs

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

## Apache Airflow

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

## Data Agents (AI-powered Q&A)

```bash
# Create a data agent
fabio data-agent create --workspace $WS --name "SalesAssistant"

# ─── Configuration ─────────────────────────────────────────────────
# Set AI instructions (inline or from file)
fabio data-agent update-config --workspace $WS --id $DA \
  --instructions "Answer questions about sales data. Use SQL for lakehouse tables."

# Load instructions from a file (useful for multi-line instructions)
fabio data-agent update-config --workspace $WS --id $DA \
  --instructions-file instructions.txt

# Enable preview runtime (agentic NL2SQL reasoning path)
fabio data-agent update-config --workspace $WS --id $DA --enable-preview-runtime

# Read current config
fabio data-agent get-config --workspace $WS --id $DA

# ─── Datasource Management ────────────────────────────────────────
# Add a lakehouse as data source (auto-detects type from artifact)
fabio data-agent add-datasource --workspace $WS --id $DA \
  --artifact "SalesLakehouse"

# Add with explicit type and instructions
fabio data-agent add-datasource --workspace $WS --id $DA \
  --artifact $LAKEHOUSE_ID --artifact-type Lakehouse \
  --instructions "Contains product catalog and order history"

# Add a warehouse from another workspace
fabio data-agent add-datasource --workspace $WS --id $DA \
  --artifact "AnalyticsWarehouse" --artifact-workspace $OTHER_WS

# List configured data sources
fabio data-agent list-datasources --workspace $WS --id $DA

# Show details of a specific data source
fabio data-agent show-datasource --workspace $WS --id $DA --datasource "SalesLakehouse"

# Remove a data source
fabio data-agent remove-datasource --workspace $WS --id $DA --datasource "SalesLakehouse"

# ─── Table Selection ──────────────────────────────────────────────
# Select specific tables
fabio data-agent select-tables --workspace $WS --id $DA \
  --datasource $LH_ID --tables "orders,products,customers"

# Select all tables
fabio data-agent select-tables --workspace $WS --id $DA \
  --datasource $LH_ID --all-tables

# Unselect specific tables
fabio data-agent select-tables --workspace $WS --id $DA \
  --datasource $LH_ID --tables "staging_raw" --unselect

# ─── Element Descriptions ─────────────────────────────────────────
# List all elements (tables/columns) with selection state
fabio data-agent list-elements --workspace $WS --id $DA --datasource $LH_ID

# Set a description on a table
fabio data-agent describe-element --workspace $WS --id $DA \
  --datasource $LH_ID --path "dbo.orders" \
  --description "Customer orders with amounts and shipping dates"

# Set a description on a column
fabio data-agent describe-element --workspace $WS --id $DA \
  --datasource $LH_ID --path "dbo.orders.total_amount" \
  --description "Total order value in USD including tax"

# Clear a description
fabio data-agent describe-element --workspace $WS --id $DA \
  --datasource $LH_ID --path "dbo.orders.total_amount"

# ─── Few-shot Examples ────────────────────────────────────────────
# Add a single example
fabio data-agent add-fewshot --workspace $WS --id $DA \
  --datasource $LH_ID \
  --question "Who is the top customer by revenue?" \
  --answer "SELECT TOP 1 customer_name, SUM(total_amount) as revenue FROM orders GROUP BY customer_name ORDER BY revenue DESC"

# Bulk upload from JSON file
# File format: [{"question":"...", "query":"..."}]
fabio data-agent upload-fewshots --workspace $WS --id $DA \
  --datasource $LH_ID --file fewshots.json

# Bulk upload from CSV file (columns: question, query)
fabio data-agent upload-fewshots --workspace $WS --id $DA \
  --datasource $LH_ID --file fewshots.csv

# List few-shot examples
fabio data-agent list-fewshots --workspace $WS --id $DA --datasource $LH_ID

# Remove a few-shot by ID
fabio data-agent remove-fewshot --workspace $WS --id $DA \
  --datasource $LH_ID --fewshot-id $FEWSHOT_ID

# ─── Publishing & Querying ────────────────────────────────────────
# Publish the agent (activates the chat endpoint — no portal needed)
fabio data-agent publish --workspace $WS --id $DA --description "v1.0 production"

# Publish to Microsoft 365 Copilot Agent Store
fabio data-agent publish --workspace $WS --id $DA --to-m365

# Query the published agent
fabio data-agent query --workspace $WS --id $DA \
  --prompt "Who is the top customer by revenue?"

# Query with explicit published URL
fabio data-agent query --workspace $WS --id $DA \
  --published-url "https://api.fabric.microsoft.com/v1/workspaces/$WS/dataagents/$DA/aiassistant/openai" \
  --prompt "What is the most expensive product?"

# Query the draft (sandbox) agent before publishing
fabio data-agent query --workspace $WS --id $DA \
  --stage sandbox --prompt "Test: how many orders?"

# Query with custom timeout and execution steps
fabio data-agent query --workspace $WS --id $DA \
  --prompt "Complex query..." --timeout 600 --show-steps

# Pipe questions from stdin
echo "How many orders were placed last month?" | \
  fabio data-agent query --workspace $WS --id $DA

# ─── Low-level Definition (advanced) ──────────────────────────────
# Get raw definition (all parts base64-encoded)
fabio data-agent get-definition --workspace $WS --id $DA

# Update with raw definition JSON
fabio data-agent update-definition --workspace $WS --id $DA --file definition.json
```

## ML Models

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

## Environments

```bash
# Create an environment with custom Spark settings
fabio environment create --workspace $WS --name "DataScience-Env"

# Add libraries to staging
fabio environment import-staging-libraries --workspace $WS --id $ENV \
  --content '{"libraries":[{"name":"scikit-learn","version":"1.4.0"}]}'

# Publish staged changes
fabio environment publish --workspace $WS --id $ENV
```

## Mirroring

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

## Graph Models and Ontologies

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

## Security and Governance

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

## Named profiles

```bash
# ── Creating profiles ──

# Save a dev profile with workspace and table output
fabio profile save --name dev --workspace $DEV_WS --default-output table

# Save a prod profile with workspace and JSON output
fabio profile save --name prod --workspace $PROD_WS --capacity $PROD_CAP --default-output json

# Save a profile for private link environments
fabio profile save --name private --workspace $WS --private-link-workspace $PRIVATE_WS_ID

# ── Switching contexts ──

# Activate the dev profile (all subsequent commands inherit its defaults)
fabio profile use --name dev

# Now these work without --workspace or --output flags
fabio lakehouse list
fabio item list
fabio notebook run --id $NB_ID

# One-off command against prod without switching active profile
fabio lakehouse list --profile prod

# Explicit flags always override profile defaults
fabio item list --workspace $OTHER_WS --output json

# ── Inspecting profiles ──

# List all profiles (shows which is active)
fabio profile list -o table

# Show full details of a specific profile
fabio profile show --name dev

# ── Cleanup ──

# Delete a profile
fabio profile delete --name old-env
```

## Composability and scripting

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

## GitHub Actions

Use fabio in CI/CD workflows to deploy Fabric artifacts automatically. No `fabio auth login` is needed -- fabio picks up credentials from the environment via `DefaultAzureCredential`.

### Option 1: OIDC federated credentials (secretless, recommended)

Uses GitHub's OIDC token exchange -- no long-lived secrets stored in your repo. Requires the `azure/login` action to broker the token exchange.

```yaml
name: Fabric Deploy
on: [push]

permissions:
  id-token: write   # Required for OIDC token exchange
  contents: read

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Azure Login (OIDC)
        uses: azure/login@v3
        with:
          client-id: ${{ secrets.AZURE_CLIENT_ID }}
          tenant-id: ${{ secrets.AZURE_TENANT_ID }}
          allow-no-subscriptions: true

      - name: Install fabio
        run: |
          ARCH=$(uname -m | sed 's/x86_64/x64/;s/aarch64/arm64/')
          curl -fsSL "https://github.com/iemejia/fabio/releases/latest/download/fabio-linux-${ARCH}.tar.gz" \
            | tar -xz -C /usr/local/bin

      - name: Deploy to Fabric
        run: |
          fabio deploy plan --source ./fabric-items/ --workspace "Production"
          fabio deploy apply --source ./fabric-items/ --workspace "Production"
```

**Setup:**

1. Create an Entra ID app registration (no client secret needed)
2. Add a federated credential for your GitHub repo (`repo:org/repo:ref:refs/heads/main`)
3. Grant the service principal Fabric workspace permissions (Contributor or Admin)
4. Store `AZURE_CLIENT_ID` and `AZURE_TENANT_ID` as GitHub repo secrets

### Option 2: Service principal with client secret (simplest)

No extra GitHub Actions required -- just set environment variables. Fabio authenticates directly.

```yaml
name: Fabric Deploy
on: [push]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Install fabio
        run: |
          ARCH=$(uname -m | sed 's/x86_64/x64/;s/aarch64/arm64/')
          curl -fsSL "https://github.com/iemejia/fabio/releases/latest/download/fabio-linux-${ARCH}.tar.gz" \
            | tar -xz -C /usr/local/bin

      - name: Deploy to Fabric
        env:
          AZURE_CLIENT_ID: ${{ secrets.AZURE_CLIENT_ID }}
          AZURE_TENANT_ID: ${{ secrets.AZURE_TENANT_ID }}
          AZURE_CLIENT_SECRET: ${{ secrets.AZURE_CLIENT_SECRET }}
        run: |
          fabio deploy plan --source ./fabric-items/ --workspace "Production"
          fabio deploy apply --source ./fabric-items/ --workspace "Production"
```

**Setup:**

1. Create an Entra ID app registration with a client secret
2. Grant the service principal Fabric workspace permissions (Contributor or Admin)
3. Store `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_CLIENT_SECRET` as GitHub repo secrets

### Which option to choose?

| | OIDC (federated) | Client secret |
|---|---|---|
| Security | No long-lived secrets | Secret stored in GitHub |
| Setup complexity | Higher (federated credential config) | Lower |
| Dependencies | `azure/login` action | None (env vars only) |
| Secret rotation | Automatic (token-based) | Manual (expiry management) |
| Recommended for | Production workloads | Quick setup, dev/test |

## Context extraction (agent memory)

```bash
# Extract a graph of all items and relationships from a workspace
fabio context extract --workspace $WS

# Scan multiple workspaces at once
fabio context extract --workspace $WS1 --workspace $WS2 --workspace $WS3

# Deep mode: fetch definitions to discover embedded references (slower)
fabio context extract --workspace $WS --deep

# Include connection objects as graph edges
fabio context extract --workspace $WS --include-connections

# Full extraction with all discovery layers
fabio context extract --workspace $WS --deep --include-connections

# Filter to specific item types
fabio context extract --workspace $WS --item-types "Notebook,Lakehouse,SemanticModel"

# Fast inventory-only mode (skip property fetching, just list items)
fabio context extract --workspace $WS --no-properties

# Increase concurrency for large workspaces
fabio context extract --workspace $WS --deep --concurrency 16

# Use workspace name instead of ID
fabio context extract --workspace "sales-analytics"

# Preview what would be scanned without making API calls
fabio context extract --workspace $WS --deep --dry-run

# ── Incremental context building ──

# Save graph to a file
fabio context extract --workspace $WS --deep --output-file context.json

# Later: add another workspace to the existing graph (merge)
fabio context extract --workspace $NEW_WS --deep \
  --merge context.json --output-file context.json

# Build up context workspace by workspace
fabio context extract --workspace $WS1 --output-file graph.json
fabio context extract --workspace $WS2 --merge graph.json --output-file graph.json
fabio context extract --workspace $WS3 --merge graph.json --output-file graph.json

# Quick inventory first, then deepen a specific workspace
fabio context extract --workspace $WS1 --workspace $WS2 --no-properties --output-file graph.json
fabio context extract --workspace $WS1 --deep --merge graph.json --output-file graph.json

# Pipe to jq for graph analysis
fabio context extract --workspace $WS --deep | jq '.data.summary'
fabio context extract --workspace $WS --deep | jq '.data.edges[] | select(.relationship == "default_lakehouse")'

# ── JSON-LD output (RDF-compatible) ──

# Export as JSON-LD for graph databases and SPARQL endpoints
fabio context extract --workspace $WS --deep --format jsonld

# Save JSON-LD to file for import into Neptune/Stardog/Jena
fabio context extract --workspace $WS --deep --format jsonld --output-file context.jsonld

# JSON-LD output has @context vocabulary + @graph with typed resources
# Each item becomes: {"@id": "urn:fabric:item:<uuid>", "@type": "fabric:Notebook", ...}
# Edges are inlined as typed properties: {"fabric:defaultLakehouse": {"@id": "urn:fabric:item:<uuid>"}}
```

## Self-update

```bash
# Check if a newer version is available (no install)
fabio upgrade --check

# Update to the latest release
fabio upgrade

# Preview the update without installing (dry-run)
fabio upgrade --dry-run

# Install a specific version
fabio upgrade --target-version 0.23.0

# Force reinstall even if already on the latest version
fabio upgrade --force

# Combine: check what version would be installed without doing it
fabio upgrade --dry-run --force
```
