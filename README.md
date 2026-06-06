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

See [COMMANDS.md](COMMANDS.md) for the full list of 70 command groups and 771 subcommands.

If you are an AI agent, run `fabio agent-context` to get a machine-readable command schema with flags, types, mutability, and examples.

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

Generate tab-completion scripts for your shell. Completions cover all 70 command groups, 771 subcommands, and their flags.

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

### GitHub Actions

Use fabio in CI/CD workflows to deploy Fabric artifacts automatically. No `fabio auth login` is needed — fabio picks up credentials from the environment via `DefaultAzureCredential`.

#### Option 1: OIDC federated credentials (secretless, recommended)

Uses GitHub's OIDC token exchange — no long-lived secrets stored in your repo. Requires the `azure/login` action to broker the token exchange.

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

#### Option 2: Service principal with client secret (simplest)

No extra GitHub Actions required — just set environment variables. Fabio authenticates directly.

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

#### Which option to choose?

| | OIDC (federated) | Client secret |
|---|---|---|
| Security | No long-lived secrets | Secret stored in GitHub |
| Setup complexity | Higher (federated credential config) | Lower |
| Dependencies | `azure/login` action | None (env vars only) |
| Secret rotation | Automatic (token-based) | Manual (expiry management) |
| Recommended for | Production workloads | Quick setup, dev/test |

## Development

```bash
git clone https://github.com/iemejia/fabio.git && cd fabio

# Build
cargo build

# Run tests (unit + offline integration -- 634 tests)
cargo test

# Run E2E tests (requires live Fabric tenant -- 667 tests)
cargo test -- --ignored

# Lint (pedantic + nursery, zero warnings required)
cargo clippy --tests -- -D warnings

# Format
cargo fmt
```

### CI/CD

- GitHub Actions CI runs on 6 targets: x64 + arm64 for Linux, macOS, and Windows
- Release workflow: tag-triggered, builds 5 binaries with SHA256 checksums (Linux x64/arm64, macOS arm64, Windows x64/arm64)
- `cargo-deny` checks for security advisories and license compliance (permissive-only policy)
- Dependabot auto-merge for passing dependency updates
- CodeQL and Secret Scanning enabled

### Project Stats

- **70 command groups** with **771 subcommands**
- **1301 tests** (496 unit + 138 offline integration + 667 E2E requiring live tenant)
- **~16 MB** release binary (stripped, full LTO, panic=abort)
- Zero clippy warnings, zero unsafe code

## License

MIT
