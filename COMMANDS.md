<!-- Auto-generated from commands.json. Do not edit manually. -->
<!-- Regenerate with: cargo test generate_commands_md -- --ignored -->

# Commands

> **AI agents**: Instead of parsing this file, run `fabio context agent` to get a machine-readable command schema with flags, types, mutability, and examples. Run `fabio context list` to discover item schemas, workflow recipes, and best practices.

## Global Flags

All commands accept these global flags:

| Flag | Short | Description |
|------|-------|-------------|
| `--output` | `-o` | Output format: `json`, `table`, `plain`, `csv`, `tsv` |
| `--json` | | Shorthand for `--output json` |
| `--query` | `-q` | JMESPath expression (see [jmespath.org](https://jmespath.org/)) |
| `--quiet` | | Suppress all stdout output |
| `--verbose` | `-v` | HTTP/LRO/auth diagnostic tracing on stderr |
| `--dry-run` | | Preview mutations without executing |
| `--limit` | | Maximum items for list commands |
| `--all` | | Fetch all pages (auto-paginate) |
| `--continuation-token` | | Resume from a previous page |
| `--profile` | | Named profile for default settings |
| `--lro-timeout` | | LRO polling timeout in seconds (default: 120) |
| `--force` | | Skip confirmation prompts |

## Core

```
fabio auth login                         Log in to Microsoft Fabric via device code flow, service principal, or WAM broker
fabio auth logout                        Log out and clear cached credentials
fabio auth status                        Show current authentication status and credential source

fabio workspace list                     List all workspaces
fabio workspace show                     Show details of a workspace
fabio workspace url                      Get the Fabric portal URL for a workspace
fabio workspace create                   Create a new workspace
fabio workspace update                   Update workspace properties (name and/or description)
fabio workspace delete                   Delete a workspace
fabio workspace assign-capacity          Assign a workspace to a capacity
fabio workspace unassign-capacity        Unassign a workspace from its capacity
fabio workspace provision-identity       Provision a workspace identity (managed identity)
fabio workspace deprovision-identity     Deprovision a workspace identity
fabio workspace list-role-assignments    List workspace role assignments
fabio workspace add-role-assignment      Add a role assignment to a workspace
fabio workspace update-role-assignment   Update a workspace role assignment
fabio workspace delete-role-assignment   Delete a workspace role assignment
fabio workspace show-role-assignment     Show a specific workspace role assignment
fabio workspace list-folders             List workspace folders
fabio workspace create-folder            Create a folder in a workspace
fabio workspace show-folder              Show details of a workspace folder
fabio workspace update-folder            Update a workspace folder
fabio workspace delete-folder            Delete a workspace folder
fabio workspace move-folder              Move a folder to another parent (or root)
fabio workspace apply-tags               Apply tags to a workspace
fabio workspace unapply-tags             Remove tags from a workspace
fabio workspace assign-to-domain         Assign workspace to a domain
fabio workspace unassign-from-domain     Unassign workspace from its domain
fabio workspace get-onelake-settings     Get `OneLake` settings for a workspace
fabio workspace modify-default-tier      Modify `OneLake` default tier (Hot, Cool, or Cold)
fabio workspace modify-diagnostics       Modify `OneLake` diagnostics configuration
fabio workspace modify-immutability-policy Modify `OneLake` immutability policy
fabio workspace export-lifecycle-policy  Export `OneLake` lifecycle policy
fabio workspace import-lifecycle-policy  Import `OneLake` lifecycle policy
fabio workspace reset-shortcut-cache     Reset `OneLake` shortcut cache for a workspace
fabio workspace get-network-policy       Get workspace network communication policy
fabio workspace set-network-policy       Set workspace network communication policy
fabio workspace get-firewall-rules       Get workspace IP firewall rules
fabio workspace set-firewall-rules       Set workspace IP firewall rules (replaces all existing rules)
fabio workspace get-git-outbound-policy  Get workspace git outbound policy
fabio workspace set-git-outbound-policy  Set workspace git outbound policy (requires Outbound Access Protection enabled)
fabio workspace get-inbound-azure-resource-rules Get workspace inbound Azure resource instance rules
fabio workspace set-inbound-azure-resource-rules Set workspace inbound Azure resource instance rules
fabio workspace get-outbound-cloud-connection-rules Get workspace outbound cloud connection rules (requires OAP enabled)
fabio workspace set-outbound-cloud-connection-rules Set workspace outbound cloud connection rules (requires OAP enabled)
fabio workspace get-outbound-gateway-rules Get workspace outbound gateway rules (requires OAP enabled)
fabio workspace set-outbound-gateway-rules Set workspace outbound gateway rules (requires OAP enabled)
fabio workspace get-settings             Get workspace settings (properties including `automaticMetadataSync`)
fabio workspace update-settings          Update workspace settings (e.g. enable automatic metadata sync)
fabio workspace set-dataset-storage-format Set default dataset storage format (Small or Large) via Power BI API
fabio workspace get-dataset-storage-format Get default dataset storage format via Power BI API
fabio workspace get-encryption           Get workspace Customer-Managed Key (CMK) encryption settings (Preview)
fabio workspace assign-encryption        Assign a Customer-Managed Key (CMK) to a workspace, enabling or rotating encryption (Preview)
fabio workspace reset-encryption         Reset workspace encryption by removing the CMK configuration (reverts to Microsoft-managed keys) (Preview)

fabio item list                          List items in a workspace
fabio item show                          Show details of an item
fabio item get-definition                Get the definition (source code/content) of an item
fabio item list-connections              List connections used by an item
fabio item exists                        Check if an item exists (returns {"exists": true/false})
fabio item url                           Get the Fabric portal URL for an item
fabio item inspect                       Aggregated item view: metadata + definition + connections
fabio item create                        Create a new item
fabio item update                        Update item properties (name and/or description)
fabio item update-definition             Update (override) item definition from file(s)
fabio item delete                        Delete an item
fabio item copy                          Copy an item to another workspace
fabio item move                          Move an item to another workspace (copy + delete source)
fabio item move-to-folder                Move an item to a folder within the same workspace
fabio item apply-tags                    Apply tags to an item
fabio item unapply-tags                  Remove tags from an item
fabio item bulk-export-definitions       Bulk export item definitions (LRO)
fabio item bulk-import-definitions       Bulk import item definitions (LRO)
fabio item bulk-move                     Bulk move items to another workspace (LRO)
fabio item bulk-create                   Bulk create items in parallel (client-side concurrency)
fabio item bulk-delete                   Bulk delete items in parallel (client-side concurrency)
fabio item list-external-data-shares     List external data shares for an item
fabio item create-external-data-share    Create an external data share for an item
fabio item show-external-data-share      Show details of an external data share
fabio item revoke-external-data-share    Revoke an external data share
fabio item delete-external-data-share    Delete an external data share
fabio item assign-identity               Assign a managed identity to an item
fabio item get-invitation                Get an external data share invitation (platform-level)
fabio item accept-invitation             Accept an external data share invitation

fabio lakehouse list                     List lakehouses in a workspace
fabio lakehouse show                     Show details of a lakehouse
fabio lakehouse create                   Create a new lakehouse
fabio lakehouse update                   Update a lakehouse (rename/redescribe)
fabio lakehouse delete                   Delete a lakehouse
fabio lakehouse list-tables              List tables in a lakehouse
fabio lakehouse list-files               List files in a lakehouse
fabio lakehouse query                    Execute SQL against the lakehouse SQL endpoint
fabio lakehouse upload                   Upload files to a lakehouse (supports glob patterns for parallel upload)
fabio lakehouse download                 Download a file from a lakehouse
fabio lakehouse upload-table             Upload a local file and load it into a Delta table (upload + load-table in one step)
fabio lakehouse load-table               Load a file (already in the lakehouse) into a Delta table
fabio lakehouse copy-file                Copy files between lakehouses (supports glob patterns for parallel copy)
fabio lakehouse move-file                Move files between lakehouses (supports glob patterns for parallel move)
fabio lakehouse copy-table               Copy a table between lakehouses
fabio lakehouse move-table               Move a table between lakehouses (copy + delete source)
fabio lakehouse sync                     Sync files between lakehouses (parallel, copies new/modified files)
fabio lakehouse create-directory         Create a directory in a lakehouse (DFS)
fabio lakehouse delete-file              Delete a file from a lakehouse
fabio lakehouse delete-table             Delete a table from a lakehouse
fabio lakehouse create-shortcut          Create a shortcut
fabio lakehouse get-shortcut             Get shortcut details
fabio lakehouse delete-shortcut          Delete a shortcut
fabio lakehouse bulk-create-shortcuts    Bulk-create multiple shortcuts (LRO)
fabio lakehouse get-definition           Get the definition of a lakehouse
fabio lakehouse update-definition        Update the definition of a lakehouse
fabio lakehouse refresh-materialized-views Trigger a refresh of materialized lake views
fabio lakehouse create-materialized-views-schedule Create a schedule for materialized lake view refresh
fabio lakehouse update-materialized-views-schedule Update a schedule for materialized lake view refresh
fabio lakehouse delete-materialized-views-schedule Delete a schedule for materialized lake view refresh
fabio lakehouse run-table-maintenance    Run table maintenance on a lakehouse
fabio lakehouse optimize-table           Optimize a Delta table (V-Order compaction + optional Z-Order)
fabio lakehouse vacuum-table             Vacuum a Delta table (remove old files beyond retention period)
fabio lakehouse table-schema             Show Delta table schema (reads from `OneLake` `_delta_log` without Spark/SQL)
fabio lakehouse iceberg-config           Get Iceberg REST Catalog configuration for a lakehouse
fabio lakehouse iceberg-namespaces       List table namespaces (schemas) via the Iceberg REST Catalog
fabio lakehouse iceberg-namespace        Get metadata for a specific namespace via the Iceberg REST Catalog
fabio lakehouse iceberg-tables           List tables in a namespace via the Iceberg REST Catalog
fabio lakehouse iceberg-table            Get table definition (schema, partitions, properties) via the Iceberg REST Catalog
fabio lakehouse iceberg-table-exists     Check if a table exists via the Iceberg REST Catalog (lightweight HEAD)
fabio lakehouse iceberg-namespace-exists Check if a namespace exists via the Iceberg REST Catalog (lightweight HEAD)
fabio lakehouse iceberg-credentials      Load vended storage credentials scoped to a specific table
fabio lakehouse iceberg-stats            Show table statistics from the latest Iceberg snapshot (record/file counts, size)
fabio lakehouse iceberg-snapshots        Show snapshot history for a table via the Iceberg REST Catalog
fabio lakehouse list-livy-sessions       List Livy sessions for a lakehouse
fabio lakehouse get-livy-session         Get details of a Livy session for a lakehouse

fabio capacity list                      List capacities available to the caller (Fabric API)
fabio capacity show                      Show details of a specific capacity (Fabric API)
fabio capacity suspend                   Suspend (pause) a capacity (ARM API)
fabio capacity resume                    Resume a suspended capacity (ARM API)
fabio capacity create                    Create a new Fabric capacity (ARM API)
fabio capacity update                    Update an existing Fabric capacity (ARM API)
fabio capacity delete                    Delete a Fabric capacity (ARM API)
fabio capacity list-skus                 List available SKUs for Fabric capacities (ARM API)
fabio capacity check-name                Check if a capacity name is available (ARM API)

fabio catalog search                     Search the Fabric catalog

fabio context agent                      Machine-readable CLI schema for agent introspection (flags, types, mutability, examples)
fabio context describe                   Deep-dive on a single command: flags, examples, output shape, notes — everything to invoke it
fabio context schema                     Show the definition schema/template for a Fabric item type
fabio context workflow                   Show a multi-step workflow recipe
fabio context best-practices             Show best-practices guidance for a topic
fabio context examples                   Show example output for a command (response shape + `JMESPath` tips)
fabio context list                       List all available documentation topics (schemas, workflows, examples, best-practices)
fabio context find                       Search commands by keyword (matches descriptions, flag names, and notes)
fabio context tenant                     Scan your Fabric tenant — build a relationship graph from workspace(s)

```

## Data Engineering

```
fabio notebook list                      List notebooks in a workspace
fabio notebook show                      Show details of a notebook
fabio notebook create                    Create a new notebook
fabio notebook update                    Update notebook properties (name and/or description)
fabio notebook get-definition            Get the definition (source code) of a notebook
fabio notebook update-definition         Update the definition (source code) of a notebook
fabio notebook delete                    Delete a notebook
fabio notebook run                       Run a notebook
fabio notebook status                    Check the status of a notebook run
fabio notebook get-job-instance          Get details of a specific job instance
fabio notebook stop                      Stop a running notebook
fabio notebook list-livy-sessions        List Livy sessions for a notebook
fabio notebook get-livy-session          Get details of a Livy session

fabio environment list                   List environments in a workspace
fabio environment show                   Show details of an environment
fabio environment create                 Create a new environment
fabio environment update                 Update environment properties (name and/or description)
fabio environment delete                 Delete an environment
fabio environment publish                Publish staged changes to an environment
fabio environment cancel-publish         Cancel a pending publish operation
fabio environment get-spark-settings     Get the published Spark settings (compute/pool/driver/executor)
fabio environment get-staging-spark-settings Get the staging (draft) Spark settings
fabio environment get-definition         Get the definition of an environment
fabio environment update-definition      Update the definition of an environment
fabio environment list-libraries         List published libraries of an environment
fabio environment export-libraries       Export external libraries configuration (published)
fabio environment list-staging-libraries List staging libraries of an environment
fabio environment delete-staging-library Delete a staging library by name
fabio environment export-staging-libraries Export external libraries configuration (staging)
fabio environment import-staging-libraries Import external libraries configuration into staging
fabio environment remove-staging-library Remove an external library from staging
fabio environment upload-staging-library Upload a custom library file into staging
fabio environment update-staging-spark-compute Update staging Spark compute configuration

fabio spark get-settings                 Get workspace-level Spark settings (custom pools, starter pools, etc.)
fabio spark update-settings              Update workspace-level Spark settings
fabio spark list-pools                   List custom Spark pools in a workspace
fabio spark get-pool                     Show details of a custom Spark pool
fabio spark create-pool                  Create a custom Spark pool
fabio spark update-pool                  Update a custom Spark pool
fabio spark delete-pool                  Delete a custom Spark pool
fabio spark get-capacity-settings        Get capacity-level Spark settings
fabio spark update-capacity-settings     Update capacity-level Spark settings
fabio spark list-capacity-pools          List custom Spark pools in a capacity
fabio spark create-capacity-pool         Create a custom Spark pool in a capacity
fabio spark get-capacity-pool            Get details of a capacity Spark pool
fabio spark update-capacity-pool         Update a capacity Spark pool
fabio spark delete-capacity-pool         Delete a capacity Spark pool
fabio spark list-livy-sessions           List Livy sessions in a workspace
fabio spark get-livy-session             Get details of a Livy session

fabio spark-job-definition list          List Spark job definitions in a workspace
fabio spark-job-definition show          Show details of a Spark job definition
fabio spark-job-definition create        Create a new Spark job definition
fabio spark-job-definition update        Update Spark job definition properties (name and/or description)
fabio spark-job-definition delete        Delete a Spark job definition
fabio spark-job-definition get-definition Get the definition of a Spark job definition
fabio spark-job-definition update-definition Update the definition of a Spark job definition
fabio spark-job-definition run           Run a Spark job definition

fabio data-pipeline list                 List data pipelines in a workspace
fabio data-pipeline show                 Show details of a data pipeline
fabio data-pipeline create               Create a new data pipeline
fabio data-pipeline update               Update data pipeline properties (name and/or description)
fabio data-pipeline delete               Delete a data pipeline
fabio data-pipeline run                  Run a data pipeline
fabio data-pipeline get-definition       Get the definition of a data pipeline
fabio data-pipeline update-definition    Update the definition of a data pipeline
fabio data-pipeline create-schedule      Create a schedule for a data pipeline
fabio data-pipeline list-schedules       List execute schedules for a data pipeline
fabio data-pipeline get-schedule         Get a specific execute schedule for a data pipeline
fabio data-pipeline update-schedule      Update an execute schedule for a data pipeline
fabio data-pipeline delete-schedule      Delete an execute schedule for a data pipeline
fabio data-pipeline list-instances       List execute job instances for a data pipeline
fabio data-pipeline get-instance         Get a specific execute job instance for a data pipeline

fabio apache-airflow-job list            List Apache Airflow jobs in a workspace
fabio apache-airflow-job show            Show details of an Apache Airflow job
fabio apache-airflow-job create          Create a new Apache Airflow job
fabio apache-airflow-job update          Update Apache Airflow job properties (name and/or description)
fabio apache-airflow-job delete          Delete an Apache Airflow job
fabio apache-airflow-job get-definition  Get the definition of an Apache Airflow job
fabio apache-airflow-job update-definition Update the definition of an Apache Airflow job
fabio apache-airflow-job start-environment Start the Airflow environment
fabio apache-airflow-job stop-environment Stop the Airflow environment
fabio apache-airflow-job get-environment Get environment status
fabio apache-airflow-job list-libraries  List installed libraries in the environment
fabio apache-airflow-job deploy-requirements Deploy requirements.txt to the environment
fabio apache-airflow-job get-settings    Get environment settings
fabio apache-airflow-job update-settings Update environment settings
fabio apache-airflow-job get-compute     Get environment compute information
fabio apache-airflow-job update-compute  Update the compute configuration for the Airflow job environment (pool template)
fabio apache-airflow-job list-files      List files (DAGs) in the Airflow job
fabio apache-airflow-job get-file        Get (download) a file from the Airflow job
fabio apache-airflow-job upload-file     Upload a file to the Airflow job
fabio apache-airflow-job delete-file     Delete a file from the Airflow job
fabio apache-airflow-job get-workspace-settings Get workspace-level Airflow settings
fabio apache-airflow-job update-workspace-settings Update workspace-level Airflow settings
fabio apache-airflow-job list-pool-templates List pool templates
fabio apache-airflow-job create-pool-template Create a pool template
fabio apache-airflow-job get-pool-template Get a pool template
fabio apache-airflow-job delete-pool-template Delete a pool template

fabio data-build-tool-job list           List data build tool jobs in a workspace [preview]
fabio data-build-tool-job show           Show details of a data build tool job [preview]
fabio data-build-tool-job create         Create a new data build tool job [preview]
fabio data-build-tool-job update         Update data build tool job properties (name and/or description) [preview]
fabio data-build-tool-job delete         Delete a data build tool job [preview]
fabio data-build-tool-job get-definition Get the definition of a data build tool job [preview]
fabio data-build-tool-job update-definition Update the definition of a data build tool job [preview]
fabio data-build-tool-job run            Run a data build tool job on-demand [preview]

```

## Data Warehousing & SQL

```
fabio warehouse list                     List warehouses in a workspace
fabio warehouse show                     Show details of a warehouse
fabio warehouse create                   Create a new warehouse
fabio warehouse update                   Update warehouse properties (name and/or description)
fabio warehouse delete                   Delete a warehouse
fabio warehouse query                    Execute a SQL query against a warehouse or SQL endpoint
fabio warehouse connection-string        Get the connection string for a warehouse
fabio warehouse get-sql-pools-config     Get SQL pools configuration for a workspace
fabio warehouse update-sql-pools-config  Update SQL pools configuration for a workspace
fabio warehouse get-audit-settings       Get SQL audit settings for a warehouse
fabio warehouse update-audit-settings    Update SQL audit settings for a warehouse
fabio warehouse set-audit-actions        Set audit actions and groups for a warehouse
fabio warehouse list-restore-points      List restore points for a warehouse
fabio warehouse create-restore-point     Create a restore point for a warehouse
fabio warehouse show-restore-point       Show details of a restore point
fabio warehouse update-restore-point     Update a restore point
fabio warehouse delete-restore-point     Delete a restore point
fabio warehouse restore-to-point         Restore a warehouse to a restore point

fabio warehouse-snapshot list            List warehouse snapshots in a workspace
fabio warehouse-snapshot show            Show details of a warehouse snapshot
fabio warehouse-snapshot create          Create a new warehouse snapshot
fabio warehouse-snapshot update          Update warehouse snapshot properties (name and/or description)
fabio warehouse-snapshot delete          Delete a warehouse snapshot

fabio sql-database list                  List SQL databases in a workspace
fabio sql-database show                  Show details of a SQL database
fabio sql-database create                Create a new SQL database
fabio sql-database update                Update SQL database properties
fabio sql-database delete                Delete a SQL database
fabio sql-database get-definition        Get the definition of a SQL database (dacpac or sqlproj format)
fabio sql-database update-definition     Update the definition of a SQL database
fabio sql-database start-mirroring       Start mirroring for the SQL database
fabio sql-database stop-mirroring        Stop mirroring for the SQL database
fabio sql-database revalidate-cmk        Revalidate Customer-Managed Key (CMK) for the SQL database
fabio sql-database get-audit-settings    Get SQL audit settings for the database
fabio sql-database update-audit-settings Update SQL audit settings for the database
fabio sql-database list-deleted          List restorable deleted SQL databases in a workspace
fabio sql-database query                 Execute a SQL query against a SQL database via TDS
fabio sql-database connection-string     Show the TDS connection string for a SQL database
fabio sql-database import                Import data from a CSV or JSON file into a SQL database table

fabio sql-endpoint list                  List SQL endpoints in a workspace
fabio sql-endpoint show                  Show details of a SQL endpoint
fabio sql-endpoint connection-string     Get the SQL connection string for a SQL endpoint
fabio sql-endpoint query                 Execute a SQL query against a SQL endpoint
fabio sql-endpoint refresh-metadata      Refresh metadata for all tables in a SQL endpoint (LRO)
fabio sql-endpoint get-audit-settings    Get SQL audit settings for the endpoint
fabio sql-endpoint update-audit-settings Update SQL audit settings for the endpoint
fabio sql-endpoint set-audit-actions     Set audit actions and groups for the endpoint

```

## Real-Time Intelligence

```
fabio eventhouse list                    List eventhouses in a workspace
fabio eventhouse show                    Show details of an eventhouse
fabio eventhouse create                  Create a new eventhouse
fabio eventhouse update                  Update eventhouse properties (name and/or description)
fabio eventhouse delete                  Delete an eventhouse
fabio eventhouse get-definition          Get the definition of an eventhouse
fabio eventhouse update-definition       Update the definition of an eventhouse

fabio kql-database list                  List KQL databases in a workspace
fabio kql-database show                  Show details of a KQL database
fabio kql-database create                Create a new KQL database
fabio kql-database update                Update KQL database properties (name and/or description)
fabio kql-database delete                Delete a KQL database
fabio kql-database query                 Execute a KQL query against a KQL database
fabio kql-database list-entities         List entities (tables, materialized views, external tables, functions) in a database
fabio kql-database describe              Get schema for all entities in a database
fabio kql-database describe-entity       Get detailed schema for a specific entity (table, view, function)
fabio kql-database sample                Sample rows from a table, materialized view, external table, or function
fabio kql-database ingest                Ingest inline data into a KQL table
fabio kql-database show-queryplan        Show execution plan for a KQL query without running it
fabio kql-database diagnostics           Run cluster diagnostics (capacity, health, ingestion failures)
fabio kql-database deeplink              Generate a deeplink URL for a KQL query in Fabric portal or ADX Web Explorer
fabio kql-database get-definition        Get the definition of a KQL database (KQL script)
fabio kql-database update-definition     Update the definition of a KQL database
fabio kql-database list-shortcuts        List shortcuts in a KQL database
fabio kql-database create-shortcut       Create a shortcut in a KQL database
fabio kql-database get-shortcut          Get a shortcut in a KQL database
fabio kql-database delete-shortcut       Delete a shortcut in a KQL database
fabio kql-database bulk-create-shortcuts Bulk-create multiple shortcuts (LRO)

fabio kql-queryset list                  List KQL querysets in a workspace
fabio kql-queryset show                  Show details of a KQL queryset
fabio kql-queryset create                Create a new KQL queryset
fabio kql-queryset update                Update KQL queryset properties (name and/or description)
fabio kql-queryset delete                Delete a KQL queryset
fabio kql-queryset get-definition        Get the definition of a KQL queryset
fabio kql-queryset update-definition     Update the definition of a KQL queryset
fabio kql-queryset run                   Run a saved query tab from the queryset against its configured data source

fabio kql-dashboard list                 List KQL dashboards in a workspace
fabio kql-dashboard show                 Show details of a KQL dashboard
fabio kql-dashboard create               Create a new KQL dashboard
fabio kql-dashboard update               Update KQL dashboard properties (name and/or description)
fabio kql-dashboard delete               Delete a KQL dashboard
fabio kql-dashboard get-definition       Get the definition of a KQL dashboard
fabio kql-dashboard update-definition    Update the definition of a KQL dashboard

fabio eventstream list                   List eventstreams in a workspace
fabio eventstream show                   Show details of an eventstream
fabio eventstream create                 Create a new eventstream
fabio eventstream update                 Update eventstream properties (name and/or description)
fabio eventstream delete                 Delete an eventstream
fabio eventstream get-definition         Get the definition of an eventstream
fabio eventstream update-definition      Update the definition of an eventstream
fabio eventstream get-topology           Get the topology of an eventstream
fabio eventstream pause                  Pause the entire eventstream
fabio eventstream resume                 Resume the entire eventstream
fabio eventstream get-destination        Get details of a destination
fabio eventstream get-destination-connection Get the connection of a destination
fabio eventstream pause-destination      Pause a destination
fabio eventstream resume-destination     Resume a destination
fabio eventstream get-source             Get details of a source
fabio eventstream get-source-connection  Get the connection of a source
fabio eventstream pause-source           Pause a source
fabio eventstream resume-source          Resume a source
fabio eventstream add-source             Add a source to an eventstream (fetches current definition, merges, and updates)
fabio eventstream add-destination        Add a destination to an eventstream (fetches current definition, merges, and updates)
fabio eventstream add-sample-source      Add a sample data source to an eventstream (high-level helper)
fabio eventstream add-derived-stream     Add a derived stream (filtered/transformed) between existing nodes
fabio eventstream validate               Validate an eventstream definition (client-side checks, no API call)
fabio eventstream list-components        List available eventstream component types (sources, destinations, operators)

fabio rti nl-to-kql                      Convert natural language to a KQL query (beta)

```

## Data Integration

```
fabio copy-job list                      List copy jobs in a workspace
fabio copy-job show                      Show details of a copy job
fabio copy-job create                    Create a new copy job
fabio copy-job update                    Update copy job properties (name and/or description)
fabio copy-job delete                    Delete a copy job
fabio copy-job get-definition            Get the definition of a copy job
fabio copy-job reset                     Reset a copy job (all entities or selected entities)
fabio copy-job update-definition         Update the definition of a copy job

fabio dataflow list                      List dataflows in a workspace
fabio dataflow show                      Show details of a dataflow
fabio dataflow create                    Create a new dataflow
fabio dataflow update                    Update dataflow properties (name and/or description)
fabio dataflow delete                    Delete a dataflow
fabio dataflow get-definition            Get the definition of a dataflow
fabio dataflow update-definition         Update the definition of a dataflow
fabio dataflow run                       Run a dataflow on demand
fabio dataflow discover-parameters       Discover parameters of a dataflow
fabio dataflow execute-query             Execute a query against a dataflow (returns Apache Arrow IPC)

fabio mirrored-database list             List mirrored databases in a workspace
fabio mirrored-database show             Show details of a mirrored database
fabio mirrored-database create           Create a new mirrored database
fabio mirrored-database update           Update mirrored database properties (name and/or description)
fabio mirrored-database delete           Delete a mirrored database
fabio mirrored-database get-definition   Get the definition of a mirrored database
fabio mirrored-database update-definition Update the definition of a mirrored database
fabio mirrored-database start            Start mirroring
fabio mirrored-database stop             Stop mirroring
fabio mirrored-database status           Get mirroring status
fabio mirrored-database table-status     Get tables mirroring status

fabio mirrored-catalog list              List mirrored catalogs in a workspace
fabio mirrored-catalog show              Show details of a mirrored catalog
fabio mirrored-catalog create            Create a new mirrored catalog
fabio mirrored-catalog update            Update mirrored catalog properties (name and/or description)
fabio mirrored-catalog delete            Delete a mirrored catalog
fabio mirrored-catalog get-definition    Get the definition of a mirrored catalog
fabio mirrored-catalog update-definition Update the definition of a mirrored catalog
fabio mirrored-catalog refresh-metadata  Refresh catalog metadata
fabio mirrored-catalog list-scopes       List catalog mirroring scopes (workspace-level)
fabio mirrored-catalog list-tables       List catalog mirroring tables (workspace-level)
fabio mirrored-catalog mirroring-status  Get mirroring status
fabio mirrored-catalog tables-mirroring-status Get tables mirroring status

fabio mirrored-databricks-catalog list   List mirrored Azure Databricks catalogs in a workspace
fabio mirrored-databricks-catalog show   Show details of a mirrored Azure Databricks catalog
fabio mirrored-databricks-catalog create Create a new mirrored Azure Databricks catalog
fabio mirrored-databricks-catalog update Update mirrored Databricks catalog properties (name and/or description)
fabio mirrored-databricks-catalog delete Delete a mirrored Azure Databricks catalog
fabio mirrored-databricks-catalog get-definition Get the definition of a mirrored Databricks catalog
fabio mirrored-databricks-catalog update-definition Update the definition of a mirrored Databricks catalog
fabio mirrored-databricks-catalog refresh-metadata Refresh catalog metadata
fabio mirrored-databricks-catalog discover-catalogs Discover available Databricks catalogs (workspace-level)
fabio mirrored-databricks-catalog discover-schemas Discover schemas in a Databricks catalog
fabio mirrored-databricks-catalog discover-tables Discover tables in a Databricks catalog schema

fabio mirrored-warehouse list            List mirrored warehouses in a workspace

fabio mounted-data-factory list          List Mounted Data Factorys in a workspace
fabio mounted-data-factory show          Show details of a Mounted Data Factory
fabio mounted-data-factory create        Create a new Mounted Data Factory
fabio mounted-data-factory update        Update Mounted Data Factory properties
fabio mounted-data-factory delete        Delete a Mounted Data Factory
fabio mounted-data-factory get-definition Get the definition of a Mounted Data Factory
fabio mounted-data-factory update-definition Update the definition of a Mounted Data Factory

```

## Analytics & Visualization

```
fabio semantic-model list                List semantic models in a workspace
fabio semantic-model show                Show details of a semantic model
fabio semantic-model create              Create a new semantic model from a definition file (model.bim)
fabio semantic-model update              Update semantic model properties (name and/or description)
fabio semantic-model delete              Delete a semantic model
fabio semantic-model get-definition      Get the definition of a semantic model
fabio semantic-model update-definition   Update the definition of a semantic model from a file
fabio semantic-model query               Execute a DAX query against a semantic model
fabio semantic-model bind-connection     Bind a semantic model to a connection
fabio semantic-model unbind-connection   Unbind a connection from a semantic model
fabio semantic-model refresh             Refresh a semantic model (required to frame Direct Lake models after creation)
fabio semantic-model takeover            Take over a semantic model (converts definition-managed to service-managed for portal editing)
fabio semantic-model list-parameters     List parameters of a semantic model
fabio semantic-model update-parameters   Update parameters of a semantic model
fabio semantic-model list-datasources    List datasources of a semantic model
fabio semantic-model update-datasources  Update datasources of a semantic model
fabio semantic-model list-users          List users (permissions) of a semantic model
fabio semantic-model add-user            Add a user to a semantic model
fabio semantic-model delete-user         Remove a user from a semantic model
fabio semantic-model refresh-status      Get refresh history and status for a semantic model
fabio semantic-model list-upstream       List upstream (lineage) datasets that this semantic model depends on
fabio semantic-model clone               Clone a semantic model to the same or different workspace
fabio semantic-model export-pbix         Export a semantic model as a .pbix file
fabio semantic-model import-pbix         Import a .pbix file as a new semantic model

fabio report list                        List reports in a workspace
fabio report show                        Show details of a report
fabio report create                      Create a new report from a definition file
fabio report update                      Update report properties (name and/or description)
fabio report delete                      Delete a report
fabio report get-definition              Get the definition of a report
fabio report update-definition           Update the definition of a report
fabio report publish-to-web              Publish a report to the web (generates a publicly accessible embed URL)

fabio paginated-report list              List paginated reports in a workspace
fabio paginated-report show              Show details of a paginated report
fabio paginated-report create            Create a paginated report in the specified workspace (requires an RDL definition file)
fabio paginated-report update            Update paginated report properties (name and/or description)
fabio paginated-report delete            Delete a paginated report
fabio paginated-report get-definition    Get the public definition of a paginated report (returns the .rdl file encoded in base64)
fabio paginated-report update-definition Update the definition of a paginated report

fabio dashboard list                     List dashboards in a workspace

fabio datamart list                      List datamarts in a workspace

fabio map list                           List maps in a workspace
fabio map show                           Show details of a map
fabio map create                         Create a new map
fabio map update                         Update map properties
fabio map delete                         Delete a map
fabio map get-definition                 Get the definition of a map
fabio map update-definition              Update the definition of a map

```

## AI & Machine Learning

```
fabio data-agent list                    List data agents in a workspace
fabio data-agent show                    Show details of a data agent
fabio data-agent create                  Create a new data agent
fabio data-agent update                  Update a data agent (name and/or description)
fabio data-agent delete                  Delete a data agent
fabio data-agent query                   Query (chat with) a published data agent using natural language
fabio data-agent get-config              Get the configuration of a data agent (instructions, data sources, preview runtime)
fabio data-agent update-config           Update the configuration of a data agent (instructions, preview runtime)
fabio data-agent list-datasources        List configured data sources for a data agent
fabio data-agent show-datasource         Show details of a configured data source
fabio data-agent add-datasource          Add a data source to the agent (auto-discovers schema from artifact)
fabio data-agent remove-datasource       Remove a data source from the agent
fabio data-agent update-datasource       Update a data source's metadata (instructions, description)
fabio data-agent list-fewshots           List few-shot examples for a data source
fabio data-agent show-fewshot            Show a specific few-shot example by ID
fabio data-agent add-fewshot             Add a few-shot example (question/query pair) to a data source
fabio data-agent update-fewshot          Update an existing few-shot example (question and/or query)
fabio data-agent remove-fewshot          Remove a few-shot example by ID
fabio data-agent clear-fewshots          Delete all few-shot examples for a data source
fabio data-agent upload-fewshots         Bulk upload few-shot examples from a JSON or CSV file
fabio data-agent select-tables           Select or unselect tables in a data source
fabio data-agent list-elements           List elements (tables, columns) in a data source with selection state and descriptions
fabio data-agent describe-element        Set or clear a description on a table or column in a data source
fabio data-agent delete-element          Delete a stale schema element (only elements no longer in the live schema)
fabio data-agent get-definition          Get the definition of a data agent (configuration, data sources, etc.)
fabio data-agent update-definition       Update the definition of a data agent (configure data sources, instructions, etc.)
fabio data-agent publish                 Publish a data agent (promotes draft configuration to published state)
fabio data-agent reset                   Reset staging (discard all draft changes, revert to published state)

fabio ml-model list                      List ML models in a workspace
fabio ml-model show                      Show details of an ML model
fabio ml-model create                    Create a new ML model
fabio ml-model update                    Update ML model properties (name and/or description)
fabio ml-model delete                    Delete an ML model
fabio ml-model get-endpoint              Get the ML model serving endpoint configuration
fabio ml-model update-endpoint           Update the ML model serving endpoint configuration
fabio ml-model score                     Score against the ML model endpoint
fabio ml-model list-versions             List endpoint versions
fabio ml-model get-version               Get a specific endpoint version
fabio ml-model update-version            Update a specific endpoint version
fabio ml-model activate-version          Activate a specific endpoint version
fabio ml-model deactivate-version        Deactivate a specific endpoint version
fabio ml-model score-version             Score against a specific endpoint version
fabio ml-model deactivate-all-versions   Deactivate all endpoint versions

fabio ml-experiment list                 List ML experiments in a workspace
fabio ml-experiment show                 Show details of an ML experiment
fabio ml-experiment create               Create a new ML experiment
fabio ml-experiment update               Update ML experiment properties (name and/or description)
fabio ml-experiment delete               Delete an ML experiment

fabio operations-agent list              List operations agents in a workspace
fabio operations-agent show              Show details of a operations agent
fabio operations-agent create            Create a new operations agent
fabio operations-agent update            Update operations agent properties
fabio operations-agent delete            Delete a operations agent
fabio operations-agent get-definition    Get the definition of a operations agent
fabio operations-agent update-definition Update the definition of a operations agent

fabio anomaly-detector list              List anomaly detectors in a workspace
fabio anomaly-detector show              Show details of an anomaly detector
fabio anomaly-detector create            Create a new anomaly detector
fabio anomaly-detector update            Update anomaly detector properties (name and/or description)
fabio anomaly-detector delete            Delete an anomaly detector
fabio anomaly-detector get-definition    Get the definition of an anomaly detector
fabio anomaly-detector update-definition Update the definition of an anomaly detector

```

## Graph & Ontology

```
fabio ontology list                      List ontologies in a workspace
fabio ontology show                      Show details of an ontology
fabio ontology create                    Create an ontology
fabio ontology update                    Update ontology properties (name and/or description)
fabio ontology delete                    Delete an ontology
fabio ontology get-definition            Get the ontology definition (entity types, bindings)
fabio ontology update-definition         Update the ontology definition (replaces current definition)
fabio ontology import                    Import an OWL ontology (RDF/XML or JSON-LD) and convert to Fabric format
fabio ontology export                    Export a Fabric Ontology to OWL format (RDF/XML or JSON-LD)

fabio graph-model list                   List graph models in a workspace
fabio graph-model show                   Show details of a graph model
fabio graph-model create                 Create a new graph model
fabio graph-model update                 Update graph model properties (name and/or description)
fabio graph-model delete                 Delete a graph model
fabio graph-model get-definition         Get the definition of a graph model
fabio graph-model update-definition      Update the definition of a graph model
fabio graph-model refresh-graph          Trigger a graph refresh job
fabio graph-model execute-query          Execute a graph query
fabio graph-model get-queryable-graph-type Get the queryable graph type
fabio graph-model initialize             Initialize a graph model for querying (portal-only operation)

fabio graph-query-set list               List graph query sets in a workspace
fabio graph-query-set show               Show details of a graph query set
fabio graph-query-set create             Create a new graph query set
fabio graph-query-set update             Update graph query set properties
fabio graph-query-set delete             Delete a graph query set
fabio graph-query-set get-definition     Get the definition of a graph query set
fabio graph-query-set update-definition  Update the definition of a graph query set

fabio digital-twin-builder list          List Digital Twin Builders in a workspace
fabio digital-twin-builder show          Show details of a Digital Twin Builder
fabio digital-twin-builder create        Create a new Digital Twin Builder
fabio digital-twin-builder update        Update Digital Twin Builder properties
fabio digital-twin-builder delete        Delete a Digital Twin Builder
fabio digital-twin-builder get-definition Get the definition of a Digital Twin Builder
fabio digital-twin-builder update-definition Update the definition of a Digital Twin Builder

fabio digital-twin-builder-flow list     List Digital Twin Builder flows in a workspace
fabio digital-twin-builder-flow show     Show details of a Digital Twin Builder flow
fabio digital-twin-builder-flow create   Create a new Digital Twin Builder flow
fabio digital-twin-builder-flow update   Update Digital Twin Builder flow properties
fabio digital-twin-builder-flow delete   Delete a Digital Twin Builder flow
fabio digital-twin-builder-flow get-definition Get the definition of a Digital Twin Builder flow
fabio digital-twin-builder-flow update-definition Update the definition of a Digital Twin Builder flow

```

## Connectors & APIs

```
fabio graphql-api list                   List GraphQL APIs in a workspace
fabio graphql-api show                   Show details of a GraphQL API
fabio graphql-api create                 Create a new GraphQL API
fabio graphql-api update                 Update GraphQL API properties (name and/or description)
fabio graphql-api delete                 Delete a GraphQL API
fabio graphql-api get-definition         Get the definition of a GraphQL API
fabio graphql-api update-definition      Update the definition of a GraphQL API
fabio graphql-api query                  Execute a GraphQL query against a GraphQL API

fabio connection list                    List all connections you have permission to access
fabio connection show                    Show details of a specific connection
fabio connection create                  Create a new connection
fabio connection update                  Update a connection's name, credentials, or privacy level
fabio connection delete                  Delete a connection
fabio connection list-supported-types    List supported connection types (gateway types catalog)
fabio connection list-role-assignments   List role assignments for a connection
fabio connection add-role-assignment     Add a role assignment to a connection
fabio connection show-role-assignment    Show a specific role assignment for a connection
fabio connection update-role-assignment  Update a role assignment for a connection
fabio connection delete-role-assignment  Delete a role assignment from a connection
fabio connection test-connection         Test a connection

fabio cosmos-db-database list            List Cosmos DB databases in a workspace
fabio cosmos-db-database show            Show details of a Cosmos DB database
fabio cosmos-db-database create          Create a new Cosmos DB database
fabio cosmos-db-database update          Update Cosmos DB database properties
fabio cosmos-db-database delete          Delete a Cosmos DB database
fabio cosmos-db-database get-definition  Get the definition of a Cosmos DB database
fabio cosmos-db-database update-definition Update the definition of a Cosmos DB database

fabio snowflake-database list            List Snowflake databases in a workspace
fabio snowflake-database show            Show details of a Snowflake database
fabio snowflake-database create          Create a new Snowflake database
fabio snowflake-database update          Update Snowflake database properties
fabio snowflake-database delete          Delete a Snowflake database
fabio snowflake-database get-definition  Get the definition of a Snowflake database
fabio snowflake-database update-definition Update the definition of a Snowflake database

fabio azure-databricks-storage list      List Azure Databricks storage items in a workspace
fabio azure-databricks-storage show      Show details of an Azure Databricks storage item
fabio azure-databricks-storage create    Create a new Azure Databricks storage item
fabio azure-databricks-storage update    Update Azure Databricks storage item properties
fabio azure-databricks-storage delete    Delete an Azure Databricks storage item
fabio azure-databricks-storage get-definition Get the definition of an Azure Databricks storage item
fabio azure-databricks-storage update-definition Update the definition of an Azure Databricks storage item

```

## Reactive & Events

```
fabio reflex list                        List reflexes in a workspace
fabio reflex show                        Show details of a reflex
fabio reflex create                      Create a new reflex
fabio reflex update                      Update reflex properties (name and/or description)
fabio reflex delete                      Delete a reflex
fabio reflex get-definition              Get the definition of a reflex
fabio reflex update-definition           Update the definition of a reflex
fabio reflex create-trigger              Create a trigger with auto-generated Reflex definition (KQL source + email/Teams alert)
fabio reflex configure-kql-source        Configure a KQL data source (portal-only operation)

fabio event-schema-set list              List event schema sets in a workspace
fabio event-schema-set show              Show details of a event schema set
fabio event-schema-set create            Create a new event schema set
fabio event-schema-set update            Update event schema set properties
fabio event-schema-set delete            Delete a event schema set
fabio event-schema-set get-definition    Get the definition of a event schema set
fabio event-schema-set update-definition Update the definition of a event schema set

fabio user-data-function list            List user data functions in a workspace
fabio user-data-function show            Show details of a user data function
fabio user-data-function create          Create a new user data function
fabio user-data-function update          Update user data function properties
fabio user-data-function delete          Delete a user data function
fabio user-data-function get-definition  Get the definition of a user data function
fabio user-data-function update-definition Update the definition of a user data function

fabio variable-library list              List variable librarys in a workspace
fabio variable-library show              Show details of a variable library
fabio variable-library create            Create a new variable library
fabio variable-library update            Update variable library properties
fabio variable-library delete            Delete a variable library
fabio variable-library get-definition    Get the definition of a variable library
fabio variable-library update-definition Update the definition of a variable library

```

## Governance & Administration

```
fabio domain list                        List domains in the tenant
fabio domain show                        Show details of a domain
fabio domain create                      Create a new domain
fabio domain update                      Update domain properties
fabio domain delete                      Delete a domain
fabio domain list-workspaces             List workspaces assigned to a domain
fabio domain assign-workspaces           Assign workspaces to a domain
fabio domain unassign-workspaces         Unassign workspaces from a domain
fabio domain assign-by-capacity          Bulk-assign all workspaces by capacity to a domain
fabio domain assign-by-principal         Bulk-assign all workspaces by principal to a domain

fabio deployment-pipeline list           List deployment pipelines
fabio deployment-pipeline show           Show details of a deployment pipeline
fabio deployment-pipeline create         Create a new deployment pipeline
fabio deployment-pipeline update         Update a deployment pipeline
fabio deployment-pipeline delete         Delete a deployment pipeline
fabio deployment-pipeline list-stages    List stages in a deployment pipeline
fabio deployment-pipeline list-stage-items List items in a deployment pipeline stage
fabio deployment-pipeline assign-workspace Assign a workspace to a deployment pipeline stage
fabio deployment-pipeline unassign-workspace Unassign the workspace from a deployment pipeline stage
fabio deployment-pipeline list-operations List deploy operations for a deployment pipeline
fabio deployment-pipeline show-operation Show details of a deploy operation
fabio deployment-pipeline list-role-assignments List role assignments for a deployment pipeline
fabio deployment-pipeline add-role-assignment Add a role assignment to a deployment pipeline
fabio deployment-pipeline delete-role-assignment Delete a role assignment from a deployment pipeline
fabio deployment-pipeline show-stage     Show details of a deployment pipeline stage
fabio deployment-pipeline update-stage   Update a deployment pipeline stage configuration
fabio deployment-pipeline deploy         Deploy items from one stage to another

fabio gateway list                       List all gateways
fabio gateway show                       Show details of a gateway
fabio gateway create                     Create a new gateway (`VirtualNetwork` type)
fabio gateway update                     Update gateway properties
fabio gateway delete                     Delete a gateway
fabio gateway list-members               List members of a gateway
fabio gateway update-member              Update a gateway member
fabio gateway delete-member              Delete a gateway member
fabio gateway list-role-assignments      List role assignments for a gateway
fabio gateway add-role-assignment        Add a role assignment to a gateway
fabio gateway show-role-assignment       Show a specific role assignment
fabio gateway update-role-assignment     Update a role assignment
fabio gateway delete-role-assignment     Delete a role assignment
fabio gateway check-status               Check the status of a gateway (`VNet` only)
fabio gateway check-member-status        Check the status of a gateway member (on-premises only)
fabio gateway restart                    Restart a gateway (`VNet` only, LRO)
fabio gateway shutdown                   Shut down a gateway (`VNet` only, LRO)

fabio managed-private-endpoint list      List managed private endpoints in a workspace
fabio managed-private-endpoint show      Show details of a managed private endpoint
fabio managed-private-endpoint create    Create a managed private endpoint
fabio managed-private-endpoint delete    Delete a managed private endpoint

fabio onelake-security list              List data access roles for an item
fabio onelake-security show              Show details of a data access role
fabio onelake-security create            Create or update a single data access role
fabio onelake-security upsert            Replace all data access roles for an item (atomic PUT)
fabio onelake-security delete            Delete a data access role

fabio admin list-tenant-settings         List all tenant settings
fabio admin update-tenant-setting        Update a tenant setting
fabio admin list-capacities-tenant-overrides List all capacities' delegated tenant setting overrides
fabio admin list-capacity-tenant-overrides List delegated tenant setting overrides for a capacity
fabio admin delete-capacity-tenant-override Delete a capacity delegated tenant setting override
fabio admin update-capacity-tenant-override Update a capacity delegated tenant setting override
fabio admin list-domains-tenant-overrides List all domains' delegated tenant setting overrides
fabio admin list-workspaces-tenant-overrides List all workspaces' delegated tenant setting overrides
fabio admin list-tags                    List tags
fabio admin create-tags                  Bulk-create tags
fabio admin update-tag                   Update a tag
fabio admin delete-tag                   Delete a tag
fabio admin list-workloads               List workloads
fabio admin list-workload-assignments    List workload assignments
fabio admin create-workload-assignment   Create a workload assignment
fabio admin delete-workload-assignment   Delete a workload assignment
fabio admin list-workspaces              List workspaces (admin view)
fabio admin show-workspace               Show workspace details (admin view)
fabio admin list-workspace-users         List users in a workspace (admin view)
fabio admin list-git-connections         List git connections across workspaces
fabio admin grant-admin-access           Grant temporary admin access to a workspace
fabio admin remove-admin-access          Remove temporary admin access from a workspace
fabio admin restore-workspace            Restore a deleted workspace
fabio admin list-network-policies        List network communication policies
fabio admin list-items                   List items (admin view)
fabio admin show-item                    Show item details (admin view)
fabio admin list-item-users              List users with access to an item (admin view)
fabio admin bulk-set-labels              Bulk-set sensitivity labels on items
fabio admin bulk-remove-labels           Bulk-remove sensitivity labels from items
fabio admin list-external-data-shares    List external data shares
fabio admin revoke-external-data-share   Revoke an external data share
fabio admin remove-all-sharing-links     Remove all sharing links for specified items
fabio admin bulk-remove-sharing-links    Bulk-remove sharing links
fabio admin list-domains                 List domains (admin view)
fabio admin create-domain                Create a domain
fabio admin show-domain                  Show domain details
fabio admin update-domain                Update a domain
fabio admin delete-domain                Delete a domain
fabio admin list-domain-workspaces       List workspaces in a domain
fabio admin assign-domain-workspaces     Assign workspaces to a domain
fabio admin unassign-domain-workspaces   Unassign workspaces from a domain
fabio admin unassign-all-domain-workspaces Unassign all workspaces from a domain
fabio admin list-domain-role-assignments List role assignments for a domain
fabio admin bulk-assign-domain-roles     Bulk-assign roles to a domain
fabio admin bulk-unassign-domain-roles   Bulk-unassign roles from a domain
fabio admin sync-domain-roles-to-subdomains Sync domain role assignments to subdomains
fabio admin assign-domain-workspaces-by-capacities Assign workspaces to a domain by capacities
fabio admin assign-domain-workspaces-by-principals Assign workspaces to a domain by principals
fabio admin list-user-access             List access details for a user

fabio deploy plan                        Preview what would be deployed (create/update/delete/skip)
fabio deploy apply                       Execute deployment (create/update/delete items)
fabio deploy export                      Export workspace item definitions to a local directory
fabio deploy init-params                 Generate a parameters.json scaffold by scanning or diffing exported definitions
fabio deploy validate                    Validate source directory locally (no API calls). Checks .platform files, item types, duplicate names/logical IDs, cross-references, and parameters

```

## Applications

```
fabio app-backend list                   List app backends in a workspace
fabio app-backend show                   Show details of an app backend
fabio app-backend create                 Create a new app backend
fabio app-backend update                 Update app backend properties (name and/or description)
fabio app-backend delete                 Delete an app backend

fabio org-app list                       List org apps in a workspace
fabio org-app show                       Show details of an org app
fabio org-app create                     Create a new org app
fabio org-app update                     Update org app properties (name and/or description)
fabio org-app delete                     Delete an org app
fabio org-app get-definition             Get the definition of an org app
fabio org-app update-definition          Update the definition of an org app

fabio org-app-audience list              List org app audiences in a workspace
fabio org-app-audience show              Show details of an org app audience
fabio org-app-audience create            Create a new org app audience
fabio org-app-audience update            Update org app audience properties (name and/or description)
fabio org-app-audience delete            Delete an org app audience
fabio org-app-audience get-definition    Get the definition of an org app audience
fabio org-app-audience update-definition Update the definition of an org app audience

```

## Configuration & Tooling

```
fabio rest call                          Send a raw REST request to the Fabric or Power BI API

fabio profile save                       Save a named profile with default settings
fabio profile use                        Set the active profile
fabio profile list                       List all saved profiles
fabio profile show                       Show details of a profile
fabio profile delete                     Delete a profile

fabio jobs list                          List recent jobs from the local ledger
fabio jobs get                           Get details of a specific job
fabio jobs prune                         Remove completed/failed jobs from the ledger

fabio feedback send                      Record feedback about CLI friction or issues
fabio feedback list                      List recorded feedback entries

fabio operation get-state                Get the state of a long-running operation
fabio operation get-result               Get the result of a completed long-running operation

fabio job-scheduler list-instances       List job instances for an item
fabio job-scheduler get-instance         Show details of a job instance
fabio job-scheduler run-on-demand        Run an on-demand job for an item
fabio job-scheduler cancel-instance      Cancel a running job instance
fabio job-scheduler list-schedules       List schedules for an item job type
fabio job-scheduler get-schedule         Show details of a specific schedule
fabio job-scheduler create-schedule      Create a schedule for an item job type
fabio job-scheduler update-schedule      Update an existing schedule
fabio job-scheduler delete-schedule      Delete a schedule

fabio upgrade                            Upgrade fabio to the latest release from GitHub

fabio mcp serve                          Start the MCP server (JSON-RPC 2.0 over stdin/stdout)

```

## Git Integration

```
fabio git status                         Show workspace Git status (changes, conflicts)
fabio git commit                         Commit workspace changes to the connected remote branch
fabio git pull                           Pull remote changes into the workspace (update from Git)
fabio git connect                        Connect a workspace to a Git repository
fabio git disconnect                     Disconnect a workspace from Git
fabio git init                           Initialize a workspace Git connection (required after connect)
fabio git checkout                       Switch to a different branch (disconnect + connect + init)
fabio git connection                     Show or manage Git connection and credentials
fabio git credentials                    Manage Git credentials
fabio git show-tracked                   Show tracked items and their Git sync status

```

## Other

```

```

---

*76 command groups, 810 subcommands total. Auto-generated from `commands.json`.*
