use std::path::Path;

use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum LakehouseCommand {
    // ── CRUD ─────────────────────────────────────────────────────────────
    /// List lakehouses in a workspace
    #[command(display_order = 0)]
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a lakehouse
    #[command(display_order = 0)]
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,
    },
    /// Create a new lakehouse
    #[command(display_order = 0)]
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse display name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Enable schemas (multi-schema lakehouse)
        #[arg(long)]
        enable_schemas: bool,
    },
    /// Update a lakehouse (rename/redescribe)
    #[command(display_order = 0)]
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a lakehouse
    #[command(display_order = 0)]
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,
    },

    // ── List ─────────────────────────────────────────────────────────────
    /// List tables in a lakehouse
    #[command(visible_alias = "tables", display_order = 1)]
    ListTables {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,
    },
    /// List files in a lakehouse
    #[command(visible_alias = "files", display_order = 2)]
    ListFiles {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Directory path to list (default: root)
        #[arg(short, long)]
        path: Option<String>,
    },

    // ── Read/Write ───────────────────────────────────────────────────────
    /// Upload files to a lakehouse (supports glob patterns for parallel upload)
    #[command(display_order = 10)]
    Upload {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Local source path (supports glob patterns, e.g. ./data/*.csv)
        #[arg(short = 's', long = "source-path")]
        source_path: String,

        /// Remote destination path (directory when uploading multiple files)
        #[arg(short = 'd', long = "dest-path")]
        dest_path: String,
    },
    /// Download a file from a lakehouse
    #[command(display_order = 11)]
    Download {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Remote source path
        #[arg(short = 's', long = "source-path")]
        source_path: String,

        /// Local destination path
        #[arg(short = 'd', long = "dest-path")]
        dest_path: String,
    },
    /// Upload a local file and load it into a Delta table (upload + load-table in one step)
    #[command(display_order = 12)]
    UploadTable {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Local source file path (e.g., ./data.csv)
        #[arg(short = 's', long = "source-path")]
        source_path: String,

        /// Table name
        #[arg(short = 't', long)]
        table: String,

        /// Load mode: Overwrite or Append
        #[arg(short, long, default_value = "Overwrite")]
        mode: String,

        /// File format: Csv, Parquet (auto-detected from extension if omitted)
        #[arg(short, long)]
        format: Option<String>,
    },
    /// Load a file (already in the lakehouse) into a Delta table
    #[command(display_order = 13)]
    LoadTable {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Relative path to the source file (e.g., Files/data.csv)
        #[arg(short = 's', long = "source-path")]
        source_path: String,

        /// Table name
        #[arg(short = 't', long)]
        table: String,

        /// Load mode: Overwrite or Append
        #[arg(short, long, default_value = "Overwrite")]
        mode: String,

        /// File format: Csv, Parquet
        #[arg(short, long, default_value = "Csv")]
        format: String,
    },

    // ── Copy/Move/Sync ───────────────────────────────────────────────────
    /// Copy files between lakehouses (supports glob patterns for parallel copy)
    #[command(display_order = 20)]
    CopyFile {
        /// Source workspace ID
        #[arg(long, visible_alias = "sw")]
        source_workspace: String,

        /// Source lakehouse ID
        #[arg(long, visible_alias = "si")]
        source_id: String,

        /// Source file path (supports glob patterns, e.g. Files/data/*.csv)
        #[arg(short = 's', long = "source-path")]
        source_path: String,

        /// Destination workspace ID
        #[arg(long, visible_alias = "dw")]
        dest_workspace: String,

        /// Destination lakehouse ID
        #[arg(long, visible_alias = "di")]
        dest_id: String,

        /// Destination path (directory when copying multiple files)
        #[arg(short = 'd', long = "dest-path")]
        dest_path: String,
    },
    /// Move files between lakehouses (supports glob patterns for parallel move)
    #[command(display_order = 21)]
    MoveFile {
        /// Source workspace ID
        #[arg(long, visible_alias = "sw")]
        source_workspace: String,

        /// Source lakehouse ID
        #[arg(long, visible_alias = "si")]
        source_id: String,

        /// Source file path (supports glob patterns, e.g. Files/data/*.csv)
        #[arg(short = 's', long = "source-path")]
        source_path: String,

        /// Destination workspace ID
        #[arg(long, visible_alias = "dw")]
        dest_workspace: String,

        /// Destination lakehouse ID
        #[arg(long, visible_alias = "di")]
        dest_id: String,

        /// Destination path (directory when moving multiple files)
        #[arg(short = 'd', long = "dest-path")]
        dest_path: String,
    },
    /// Copy a table between lakehouses
    #[command(display_order = 22)]
    CopyTable {
        /// Source workspace ID
        #[arg(long, visible_alias = "sw")]
        source_workspace: String,

        /// Source lakehouse ID
        #[arg(long, visible_alias = "si")]
        source_id: String,

        /// Source table name (supports glob patterns)
        #[arg(short = 's', long = "source-table")]
        source_table: String,

        /// Destination workspace ID
        #[arg(long, visible_alias = "dw")]
        dest_workspace: String,

        /// Destination lakehouse ID
        #[arg(long, visible_alias = "di")]
        dest_id: String,

        /// Destination table name (ignored for glob patterns)
        #[arg(short = 'd', long = "dest-table")]
        dest_table: Option<String>,
    },
    /// Move a table between lakehouses (copy + delete source)
    #[command(display_order = 23)]
    MoveTable {
        /// Source workspace ID
        #[arg(long, visible_alias = "sw")]
        source_workspace: String,

        /// Source lakehouse ID
        #[arg(long, visible_alias = "si")]
        source_id: String,

        /// Source table name (supports glob patterns)
        #[arg(short = 's', long = "source-table")]
        source_table: String,

        /// Destination workspace ID
        #[arg(long, visible_alias = "dw")]
        dest_workspace: String,

        /// Destination lakehouse ID
        #[arg(long, visible_alias = "di")]
        dest_id: String,

        /// Destination table name (ignored for glob patterns)
        #[arg(short = 'd', long = "dest-table")]
        dest_table: Option<String>,
    },
    /// Sync files between lakehouses (parallel, copies new/modified files)
    #[command(display_order = 24)]
    Sync {
        /// Source workspace ID
        #[arg(long, visible_alias = "sw")]
        source_workspace: String,

        /// Source lakehouse ID
        #[arg(long, visible_alias = "si")]
        source_id: String,

        /// Source path (e.g. Files/data or Tables/mytable)
        #[arg(short = 's', long = "source-path")]
        source_path: String,

        /// Destination workspace ID
        #[arg(long, visible_alias = "dw")]
        dest_workspace: String,

        /// Destination lakehouse ID
        #[arg(long, visible_alias = "di")]
        dest_id: String,

        /// Destination path
        #[arg(short = 'd', long = "dest-path")]
        dest_path: String,

        /// Delete files at destination that don't exist in source
        #[arg(long)]
        delete: bool,

        /// Use Content-MD5 checksums for comparison (slower, requires HEAD per file)
        #[arg(long)]
        checksum: bool,
    },

    // ── Delete ───────────────────────────────────────────────────────────
    /// Delete a file from a lakehouse
    #[command(display_order = 30)]
    DeleteFile {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// File path to delete
        #[arg(short, long)]
        path: String,
    },
    /// Delete a table from a lakehouse
    #[command(display_order = 31)]
    DeleteTable {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Table name (supports glob patterns)
        #[arg(short = 't', long = "table")]
        table: String,
    },

    // ── Shortcuts ────────────────────────────────────────────────────────
    /// Create a shortcut
    #[command(display_order = 40)]
    CreateShortcut {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Shortcut name
        #[arg(long)]
        name: String,

        /// Shortcut path (e.g., Tables or Files)
        #[arg(short, long)]
        path: String,

        /// Target type: `OneLake`, `AdlsGen2`, S3
        #[arg(long = "target-type")]
        target_type: String,

        /// Target body as JSON string
        #[arg(long = "target")]
        target: String,
    },
    /// Get shortcut details
    #[command(display_order = 41)]
    GetShortcut {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Shortcut name
        #[arg(long)]
        name: String,

        /// Shortcut path
        #[arg(short, long)]
        path: String,
    },
    /// Delete a shortcut
    #[command(display_order = 42)]
    DeleteShortcut {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Shortcut name
        #[arg(long)]
        name: String,

        /// Shortcut path
        #[arg(short, long)]
        path: String,
    },

    /// Bulk-create multiple shortcuts (LRO)
    #[command(display_order = 43)]
    BulkCreateShortcuts {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Path to JSON file with array of shortcut requests
        #[arg(long, group = "input")]
        file: Option<String>,

        /// Inline JSON with array of shortcut requests
        #[arg(long, group = "input")]
        content: Option<String>,

        /// Conflict policy: `Abort`, `GenerateUniqueName`, `CreateOrOverwrite`, `OverwriteOnly`
        #[arg(long = "conflict-policy")]
        conflict_policy: Option<String>,
    },

    // ── Definitions ──────────────────────────────────────────────────────
    /// Get the definition of a lakehouse
    #[command(display_order = 50)]
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,
    },
    /// Update the definition of a lakehouse
    #[command(display_order = 51)]
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Definition file path (reads file content)
        #[arg(long)]
        file: Option<String>,

        /// Definition content (inline JSON)
        #[arg(long)]
        content: Option<String>,
    },

    // ── Materialized Lake Views ──────────────────────────────────────────
    /// Trigger a refresh of materialized lake views
    #[command(display_order = 60)]
    RefreshMaterializedViews {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,
    },
    /// Create a schedule for materialized lake view refresh
    #[command(display_order = 61)]
    CreateMaterializedViewsSchedule {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Schedule definition file path (JSON)
        #[arg(long)]
        file: Option<String>,

        /// Schedule definition content (inline JSON)
        #[arg(long)]
        content: Option<String>,
    },
    /// Update a schedule for materialized lake view refresh
    #[command(display_order = 62)]
    UpdateMaterializedViewsSchedule {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Schedule ID
        #[arg(long)]
        schedule_id: String,

        /// Schedule definition file path (JSON)
        #[arg(long)]
        file: Option<String>,

        /// Schedule definition content (inline JSON)
        #[arg(long)]
        content: Option<String>,
    },
    /// Delete a schedule for materialized lake view refresh
    #[command(display_order = 63)]
    DeleteMaterializedViewsSchedule {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Schedule ID
        #[arg(long)]
        schedule_id: String,
    },

    // ── Table Maintenance ────────────────────────────────────────────────
    /// Run table maintenance on a lakehouse
    #[command(display_order = 70)]
    RunTableMaintenance {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Configuration file path (optional JSON)
        #[arg(long)]
        file: Option<String>,

        /// Configuration content (optional inline JSON)
        #[arg(long)]
        content: Option<String>,
    },

    // ── Livy Sessions ────────────────────────────────────────────────────
    /// List Livy sessions for a lakehouse
    #[command(display_order = 80)]
    ListLivySessions {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,
    },
    /// Get details of a Livy session for a lakehouse
    #[command(display_order = 81)]
    GetLivySession {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Livy session ID
        #[arg(long)]
        livy_id: String,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &LakehouseCommand) -> Result<()> {
    match command {
        LakehouseCommand::List { workspace } => list_lakehouses(cli, client, workspace).await,
        LakehouseCommand::Show { workspace, id } => {
            show_lakehouse(cli, client, workspace, id).await
        }
        LakehouseCommand::Create {
            workspace,
            name,
            description,
            enable_schemas,
        } => {
            create_lakehouse(
                cli,
                client,
                workspace,
                name,
                description.as_deref(),
                *enable_schemas,
            )
            .await
        }
        LakehouseCommand::Update {
            workspace,
            id,
            name,
            description,
        } => {
            update_lakehouse(
                cli,
                client,
                workspace,
                id,
                name.as_deref(),
                description.as_deref(),
            )
            .await
        }
        LakehouseCommand::Delete { workspace, id } => {
            delete_lakehouse(cli, client, workspace, id).await
        }
        LakehouseCommand::ListTables { workspace, id } => tables(cli, client, workspace, id).await,
        LakehouseCommand::ListFiles {
            workspace,
            id,
            path,
        } => files(cli, client, workspace, id, path.as_deref()).await,
        LakehouseCommand::Upload {
            workspace,
            id,
            source_path,
            dest_path,
        } => upload(cli, client, workspace, id, source_path, dest_path)
            .await
            .map_err(|e| enrich_forbidden(e, "lakehouse upload", "Contributor")),
        LakehouseCommand::Download {
            workspace,
            id,
            source_path,
            dest_path,
        } => download(cli, client, workspace, id, source_path, dest_path).await,
        LakehouseCommand::UploadTable {
            workspace,
            id,
            source_path,
            table,
            mode,
            format,
        } => upload_table(
            cli,
            client,
            workspace,
            id,
            source_path,
            table,
            mode,
            format.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse upload-table", "Contributor")),
        LakehouseCommand::LoadTable {
            workspace,
            id,
            source_path,
            table,
            mode,
            format,
        } => load_table(cli, client, workspace, id, source_path, table, mode, format)
            .await
            .map_err(|e| enrich_forbidden(e, "lakehouse load-table", "Contributor")),
        LakehouseCommand::CopyFile {
            source_workspace,
            source_id,
            source_path,
            dest_workspace,
            dest_id,
            dest_path,
        } => copy_file(
            cli,
            client,
            source_workspace,
            source_id,
            source_path,
            dest_workspace,
            dest_id,
            dest_path,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse copy-file", "Contributor")),
        LakehouseCommand::DeleteFile {
            workspace,
            id,
            path,
        } => delete_file(cli, client, workspace, id, path)
            .await
            .map_err(|e| enrich_forbidden(e, "lakehouse delete-file", "Contributor")),
        LakehouseCommand::MoveFile {
            source_workspace,
            source_id,
            source_path,
            dest_workspace,
            dest_id,
            dest_path,
        } => move_file(
            cli,
            client,
            source_workspace,
            source_id,
            source_path,
            dest_workspace,
            dest_id,
            dest_path,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse move-file", "Contributor")),
        LakehouseCommand::DeleteTable {
            workspace,
            id,
            table,
        } => delete_table(cli, client, workspace, id, table)
            .await
            .map_err(|e| enrich_forbidden(e, "lakehouse delete-table", "Contributor")),
        LakehouseCommand::CopyTable {
            source_workspace,
            source_id,
            source_table,
            dest_workspace,
            dest_id,
            dest_table,
        } => copy_table(
            cli,
            client,
            source_workspace,
            source_id,
            source_table,
            dest_workspace,
            dest_id,
            dest_table.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse copy-table", "Contributor")),
        LakehouseCommand::MoveTable {
            source_workspace,
            source_id,
            source_table,
            dest_workspace,
            dest_id,
            dest_table,
        } => move_table(
            cli,
            client,
            source_workspace,
            source_id,
            source_table,
            dest_workspace,
            dest_id,
            dest_table.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse move-table", "Contributor")),
        LakehouseCommand::Sync {
            source_workspace,
            source_id,
            source_path,
            dest_workspace,
            dest_id,
            dest_path,
            delete,
            checksum,
        } => sync_files(
            cli,
            client,
            source_workspace,
            source_id,
            source_path,
            dest_workspace,
            dest_id,
            dest_path,
            *delete,
            *checksum,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse sync", "Contributor")),
        LakehouseCommand::CreateShortcut {
            workspace,
            id,
            name,
            path,
            target_type,
            target,
        } => create_shortcut(cli, client, workspace, id, name, path, target_type, target)
            .await
            .map_err(|e| enrich_forbidden(e, "lakehouse create-shortcut", "Contributor")),
        LakehouseCommand::GetShortcut {
            workspace,
            id,
            name,
            path,
        } => get_shortcut(cli, client, workspace, id, name, path).await,
        LakehouseCommand::DeleteShortcut {
            workspace,
            id,
            name,
            path,
        } => delete_shortcut(cli, client, workspace, id, name, path)
            .await
            .map_err(|e| enrich_forbidden(e, "lakehouse delete-shortcut", "Contributor")),
        LakehouseCommand::BulkCreateShortcuts {
            workspace,
            id,
            file,
            content,
            conflict_policy,
        } => bulk_create_shortcuts(
            cli,
            client,
            workspace,
            id,
            file.as_deref(),
            content.as_deref(),
            conflict_policy.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse bulk-create-shortcuts", "Contributor")),
        LakehouseCommand::GetDefinition { workspace, id } => {
            get_definition(cli, client, workspace, id).await
        }
        LakehouseCommand::UpdateDefinition {
            workspace,
            id,
            file,
            content,
        } => {
            update_definition(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        LakehouseCommand::RefreshMaterializedViews { workspace, id } => {
            refresh_materialized_views(cli, client, workspace, id).await
        }
        LakehouseCommand::CreateMaterializedViewsSchedule {
            workspace,
            id,
            file,
            content,
        } => {
            create_materialized_views_schedule(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        LakehouseCommand::UpdateMaterializedViewsSchedule {
            workspace,
            id,
            schedule_id,
            file,
            content,
        } => {
            update_materialized_views_schedule(
                cli,
                client,
                workspace,
                id,
                schedule_id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        LakehouseCommand::DeleteMaterializedViewsSchedule {
            workspace,
            id,
            schedule_id,
        } => delete_materialized_views_schedule(cli, client, workspace, id, schedule_id).await,
        LakehouseCommand::RunTableMaintenance {
            workspace,
            id,
            file,
            content,
        } => {
            run_table_maintenance(
                cli,
                client,
                workspace,
                id,
                file.as_deref(),
                content.as_deref(),
            )
            .await
        }
        LakehouseCommand::ListLivySessions { workspace, id } => {
            list_livy_sessions(cli, client, workspace, id).await
        }
        LakehouseCommand::GetLivySession {
            workspace,
            id,
            livy_id,
        } => get_livy_session(cli, client, workspace, id, livy_id).await,
    }
}

// ─── CRUD Operations ─────────────────────────────────────────────────────────

async fn list_lakehouses(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/lakehouses"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id", "description"],
        &["NAME", "ID", "DESCRIPTION"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn show_lakehouse(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/lakehouses/{id}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn create_lakehouse(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    description: Option<&str>,
    enable_schemas: bool,
) -> Result<()> {
    let mut body = serde_json::json!({
        "displayName": name,
    });
    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }
    if enable_schemas {
        body["creationPayload"] = serde_json::json!({
            "enableSchemas": true
        });
    }

    if output::dry_run_guard(
        cli,
        "lakehouse create",
        &serde_json::json!({
            "workspace": workspace,
            "displayName": name,
            "description": description,
            "enableSchemas": enable_schemas
        }),
    ) {
        return Ok(());
    }

    let data = client
        .post(&format!("/workspaces/{workspace}/lakehouses"), &body, true)
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse create", "Member"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update_lakehouse(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "At least one of --name or --description must be provided".to_string(),
            "Example: fabio lakehouse update --workspace <WS> --id <ID> --name \"New Name\""
                .to_string(),
        )
        .into());
    }

    let mut body = serde_json::json!({});
    if let Some(n) = name {
        body["displayName"] = Value::String(n.to_string());
    }
    if let Some(d) = description {
        body["description"] = Value::String(d.to_string());
    }

    if output::dry_run_guard(cli, "lakehouse update", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/lakehouses/{id}"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse update", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete_lakehouse(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "lakehouse delete",
        &serde_json::json!({
            "workspace": workspace,
            "id": id
        }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/lakehouses/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse delete", "Member"))?;

    let obj = serde_json::json!({ "id": id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Data Operations ─────────────────────────────────────────────────────────

async fn tables(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/lakehouses/{id}/tables"),
            "data",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["name", "type", "format"],
        &["NAME", "TYPE", "FORMAT"],
        "name",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn files(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    path: Option<&str>,
) -> Result<()> {
    let items = client.list_onelake_files(workspace, id, path).await?;
    output::render_list(
        cli,
        &items,
        &["name", "contentLength", "lastModified"],
        &["NAME", "SIZE", "MODIFIED"],
        "name",
    );
    Ok(())
}

async fn upload(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    source_path: &str,
    dest_path: &str,
) -> Result<()> {
    use crate::parallel::{self, BatchSummary};

    // Expand glob patterns for local files
    let local_files = expand_local_glob(source_path)?;

    if local_files.len() == 1 {
        // Single file: upload directly
        let data = std::fs::read(&local_files[0]).map_err(|e| {
            crate::errors::FabioError::invalid_input(format!(
                "Cannot read file {}: {e}",
                local_files[0]
            ))
        })?;
        let result = client
            .upload_onelake_file(workspace, id, dest_path, &data)
            .await?;
        output::render_object(cli, &result, "status");
        return Ok(());
    }

    // Multiple files: upload in parallel to dest_path as directory
    let concurrency = parallel::default_concurrency();
    eprintln!(
        "  Uploading {} files with concurrency={concurrency}...",
        local_files.len()
    );

    let item_names: Vec<String> = local_files.clone();
    let upload_tasks: Vec<(String, String)> = local_files
        .into_iter()
        .map(|local| {
            let filename = Path::new(&local)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let remote = format!("{dest_path}/{filename}");
            (local, remote)
        })
        .collect();

    let workspace = workspace.to_string();
    let id = id.to_string();
    let client = client.clone();

    let results = parallel::execute_parallel(upload_tasks, concurrency, move |(local, remote)| {
        let client = client.clone();
        let workspace = workspace.clone();
        let id = id.clone();
        async move {
            let data = tokio::fs::read(&local).await.map_err(|e| {
                anyhow::anyhow!(
                    "{}",
                    crate::errors::FabioError::invalid_input(format!(
                        "Cannot read file {local}: {e}"
                    ))
                )
            })?;
            client
                .upload_onelake_file(&workspace, &id, &remote, &data)
                .await?;
            Ok(())
        }
    })
    .await;

    let summary = BatchSummary::from_results(&results, &item_names);
    render_batch_result(cli, &summary, "uploaded")
}

async fn download(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    source_path: &str,
    dest_path: &str,
) -> Result<()> {
    let data = client
        .download_onelake_file(workspace, id, source_path)
        .await?;

    std::fs::write(dest_path, &data).map_err(|e| {
        crate::errors::FabioError::invalid_input(format!("Cannot write to {dest_path}: {e}"))
    })?;

    let obj = serde_json::json!({
        "source": source_path,
        "destination": dest_path,
        "size": data.len(),
        "status": "downloaded"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn load_table(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    source_path: &str,
    table: &str,
    mode: &str,
    format: &str,
) -> Result<()> {
    const VALID_MODES: &[&str] = &["Overwrite", "Append"];
    const VALID_FORMATS: &[&str] = &["Csv", "Parquet"];

    if !VALID_MODES.contains(&mode) {
        return Err(crate::errors::FabioError::with_hint(
            crate::errors::ErrorCode::InvalidInput,
            format!("Invalid load mode: '{mode}'"),
            format!(
                "--mode must be one of: {} (got: '{mode}')",
                VALID_MODES.join(", ")
            ),
        )
        .into());
    }
    if !VALID_FORMATS.contains(&format) {
        return Err(crate::errors::FabioError::with_hint(
            crate::errors::ErrorCode::InvalidInput,
            format!("Invalid format: '{format}'"),
            format!(
                "--format must be one of: {} (got: '{format}')",
                VALID_FORMATS.join(", ")
            ),
        )
        .into());
    }

    if output::dry_run_guard(
        cli,
        "lakehouse load-table",
        &serde_json::json!({
            "workspace": workspace,
            "lakehouse": id,
            "source_path": source_path,
            "table": table,
            "mode": mode,
            "format": format
        }),
    ) {
        return Ok(());
    }

    let format_options = match format {
        "Csv" => serde_json::json!({
            "format": format,
            "header": true,
            "delimiter": ","
        }),
        _ => serde_json::json!({
            "format": format
        }),
    };

    let body = serde_json::json!({
        "relativePath": source_path,
        "pathType": "File",
        "mode": mode,
        "formatOptions": format_options
    });

    let data = client
        .post(
            &format!("/workspaces/{workspace}/lakehouses/{id}/tables/{table}/load"),
            &body,
            true,
        )
        .await?;

    let obj = if data.is_null() {
        serde_json::json!({
            "table": table,
            "source": source_path,
            "mode": mode,
            "status": "loaded"
        })
    } else {
        data
    };

    output::render_object(cli, &obj, "status");
    Ok(())
}

/// Upload a local file to the lakehouse and load it into a Delta table in one step.
/// Auto-detects format from file extension if `--format` is not provided.
#[allow(clippy::too_many_arguments)]
async fn upload_table(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    source_path: &str,
    table: &str,
    mode: &str,
    format: Option<&str>,
) -> Result<()> {
    const VALID_MODES: &[&str] = &["Overwrite", "Append"];
    const VALID_FORMATS: &[&str] = &["Csv", "Parquet"];

    if !VALID_MODES.contains(&mode) {
        return Err(crate::errors::FabioError::with_hint(
            crate::errors::ErrorCode::InvalidInput,
            format!("Invalid load mode: '{mode}'"),
            format!(
                "--mode must be one of: {} (got: '{mode}')",
                VALID_MODES.join(", ")
            ),
        )
        .into());
    }

    // Auto-detect format from file extension if not explicitly provided
    let detected_format = match format {
        Some(f) => f.to_string(),
        None => detect_format_from_extension(source_path)?,
    };

    if !VALID_FORMATS.contains(&detected_format.as_str()) {
        return Err(crate::errors::FabioError::with_hint(
            crate::errors::ErrorCode::InvalidInput,
            format!("Invalid format: '{detected_format}'"),
            format!(
                "--format must be one of: {} (got: '{detected_format}')",
                VALID_FORMATS.join(", ")
            ),
        )
        .into());
    }

    // Derive a staging path in the lakehouse Files area
    let filename = Path::new(source_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let staging_path = format!("Files/.staging/{filename}");

    if output::dry_run_guard(
        cli,
        "lakehouse upload-table",
        &serde_json::json!({
            "workspace": workspace,
            "lakehouse": id,
            "source_path": source_path,
            "staging_path": staging_path,
            "table": table,
            "mode": mode,
            "format": detected_format
        }),
    ) {
        return Ok(());
    }

    // Step 1: Upload the local file to Files/.staging/<filename>
    let data = std::fs::read(source_path).map_err(|e| {
        crate::errors::FabioError::invalid_input(format!("Cannot read file {source_path}: {e}"))
    })?;

    eprintln!("  Uploading {source_path} to {staging_path}...");
    client
        .upload_onelake_file(workspace, id, &staging_path, &data)
        .await?;

    // Step 2: Load the uploaded file into the Delta table
    eprintln!("  Loading into table '{table}' (mode={mode}, format={detected_format})...");
    let format_options = match detected_format.as_str() {
        "Csv" => serde_json::json!({
            "format": detected_format,
            "header": true,
            "delimiter": ","
        }),
        _ => serde_json::json!({
            "format": detected_format
        }),
    };
    let body = serde_json::json!({
        "relativePath": staging_path,
        "pathType": "File",
        "mode": mode,
        "formatOptions": format_options
    });

    let load_result = client
        .post(
            &format!("/workspaces/{workspace}/lakehouses/{id}/tables/{table}/load"),
            &body,
            true,
        )
        .await;

    // Step 3: Clean up the staging file (best-effort)
    let _ = client
        .delete_onelake_file(workspace, id, &staging_path)
        .await;

    // Handle the load result
    load_result?;

    let obj = serde_json::json!({
        "table": table,
        "source": source_path,
        "mode": mode,
        "format": detected_format,
        "status": "loaded"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

/// Detect the file format (Csv, Parquet) from a file extension. JSON is not supported by the API.
fn detect_format_from_extension(path: &str) -> Result<String> {
    let ext = Path::new(path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "csv" | "tsv" => Ok("Csv".to_string()),
        "parquet" | "pq" => Ok("Parquet".to_string()),
        "json" | "jsonl" | "ndjson" => Err(crate::errors::FabioError::with_hint(
            crate::errors::ErrorCode::InvalidInput,
            "JSON format is not supported by the Fabric load-table API".to_string(),
            "Convert to CSV or Parquet first. The load-table API only supports Csv and Parquet formats.".to_string(),
        )
        .into()),
        _ => Err(crate::errors::FabioError::with_hint(
            crate::errors::ErrorCode::InvalidInput,
            format!("Cannot detect format from extension '.{ext}'"),
            "Use --format to specify one of: Csv, Parquet".to_string(),
        )
        .into()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn copy_file(
    cli: &Cli,
    client: &FabricClient,
    src_ws: &str,
    src_id: &str,
    src_path: &str,
    dst_ws: &str,
    dst_id: &str,
    dst_path: &str,
) -> Result<()> {
    use crate::parallel::{self, BatchSummary};

    // Check if source path contains a glob pattern
    let matched_files = expand_remote_glob(client, src_ws, src_id, src_path).await?;

    if matched_files.len() == 1 && matched_files[0] == src_path {
        // Single file: copy directly
        let result = client
            .copy_onelake_file(src_ws, src_id, src_path, dst_ws, dst_id, dst_path)
            .await?;
        output::render_object(cli, &result, "status");
        return Ok(());
    }

    // Multiple files: copy in parallel, dest_path is a directory
    let concurrency = parallel::default_concurrency();
    eprintln!(
        "  Copying {} files with concurrency={concurrency}...",
        matched_files.len()
    );

    let item_names: Vec<String> = matched_files.clone();
    let copy_tasks: Vec<(String, String)> = matched_files
        .into_iter()
        .map(|src| {
            let filename = src.rsplit('/').next().unwrap_or(&src).to_string();
            let dest = format!("{dst_path}/{filename}");
            (src, dest)
        })
        .collect();

    let src_ws = src_ws.to_string();
    let src_id = src_id.to_string();
    let dst_ws = dst_ws.to_string();
    let dst_id = dst_id.to_string();
    let client = client.clone();

    let results = parallel::execute_parallel(copy_tasks, concurrency, move |(src, dest)| {
        let client = client.clone();
        let src_ws = src_ws.clone();
        let src_id = src_id.clone();
        let dst_ws = dst_ws.clone();
        let dst_id = dst_id.clone();
        async move {
            client
                .copy_onelake_file(&src_ws, &src_id, &src, &dst_ws, &dst_id, &dest)
                .await?;
            Ok(())
        }
    })
    .await;

    let summary = BatchSummary::from_results(&results, &item_names);
    render_batch_result(cli, &summary, "copied")
}

async fn delete_file(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    path: &str,
) -> Result<()> {
    let result = client.delete_onelake_file(workspace, id, path).await?;
    output::render_object(cli, &result, "status");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn move_file(
    cli: &Cli,
    client: &FabricClient,
    src_ws: &str,
    src_id: &str,
    src_path: &str,
    dst_ws: &str,
    dst_id: &str,
    dst_path: &str,
) -> Result<()> {
    use crate::parallel::{self, BatchSummary};

    // Check if source path contains a glob pattern
    let matched_files = expand_remote_glob(client, src_ws, src_id, src_path).await?;

    if matched_files.len() == 1 && matched_files[0] == src_path {
        // Single file: copy then delete
        client
            .copy_onelake_file(src_ws, src_id, src_path, dst_ws, dst_id, dst_path)
            .await?;
        client.delete_onelake_file(src_ws, src_id, src_path).await?;

        let obj = serde_json::json!({
            "source": src_path,
            "destination": dst_path,
            "status": "moved"
        });
        output::render_object(cli, &obj, "status");
        return Ok(());
    }

    // Multiple files: copy in parallel, then delete sources on success
    let concurrency = parallel::default_concurrency();
    eprintln!(
        "  Moving {} files with concurrency={concurrency}...",
        matched_files.len()
    );

    let item_names: Vec<String> = matched_files.clone();
    let copy_tasks: Vec<(String, String)> = matched_files
        .iter()
        .map(|src| {
            let filename = src.rsplit('/').next().unwrap_or(src).to_string();
            let dest = format!("{dst_path}/{filename}");
            (src.clone(), dest)
        })
        .collect();

    let src_ws_owned = src_ws.to_string();
    let src_id_owned = src_id.to_string();
    let dst_ws_owned = dst_ws.to_string();
    let dst_id_owned = dst_id.to_string();
    let client_clone = client.clone();

    let results = parallel::execute_parallel(copy_tasks, concurrency, move |(src, dest)| {
        let client = client_clone.clone();
        let sw = src_ws_owned.clone();
        let si = src_id_owned.clone();
        let dw = dst_ws_owned.clone();
        let di = dst_id_owned.clone();
        async move {
            client
                .copy_onelake_file(&sw, &si, &src, &dw, &di, &dest)
                .await?;
            Ok(())
        }
    })
    .await;

    let summary = BatchSummary::from_results(&results, &item_names);

    if !summary.all_succeeded() {
        return render_batch_result(cli, &summary, "move_failed");
    }

    // All copies succeeded — now delete sources in parallel
    let src_ws_owned = src_ws.to_string();
    let src_id_owned = src_id.to_string();
    let client_clone = client.clone();

    let delete_results =
        parallel::execute_parallel(matched_files.clone(), concurrency, move |src| {
            let client = client_clone.clone();
            let sw = src_ws_owned.clone();
            let si = src_id_owned.clone();
            async move {
                client.delete_onelake_file(&sw, &si, &src).await?;
                Ok(())
            }
        })
        .await;

    let delete_summary = BatchSummary::from_results(&delete_results, &item_names);
    if !delete_summary.all_succeeded() {
        eprintln!(
            "  Warning: {} source files could not be deleted after successful copy",
            delete_summary.failed
        );
    }

    render_batch_result(cli, &summary, "moved")
}

/// Check if a path contains glob metacharacters.
fn is_glob_pattern(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

/// Expand a local file glob pattern into a list of matching file paths.
fn expand_local_glob(pattern: &str) -> Result<Vec<String>> {
    if !is_glob_pattern(pattern) {
        // Not a glob — treat as a single file or directory
        let path = Path::new(pattern);
        if path.is_dir() {
            // Upload all files in the directory
            let mut files = Vec::new();
            for entry in std::fs::read_dir(path).map_err(|e| {
                crate::errors::FabioError::invalid_input(format!(
                    "Cannot read directory {pattern}: {e}"
                ))
            })? {
                let entry = entry.map_err(|e| {
                    crate::errors::FabioError::invalid_input(format!("Directory read error: {e}"))
                })?;
                if entry.file_type().is_ok_and(|ft| ft.is_file()) {
                    files.push(entry.path().to_string_lossy().to_string());
                }
            }
            if files.is_empty() {
                return Err(crate::errors::FabioError::invalid_input(format!(
                    "No files found in directory: {pattern}"
                ))
                .into());
            }
            files.sort();
            return Ok(files);
        }
        return Ok(vec![pattern.to_string()]);
    }

    let matches: Vec<String> = glob::glob(pattern)
        .map_err(|e| {
            crate::errors::FabioError::invalid_input(format!("Invalid glob pattern: {e}"))
        })?
        .filter_map(Result::ok)
        .filter(|p| p.is_file())
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    if matches.is_empty() {
        return Err(crate::errors::FabioError::invalid_input(format!(
            "No files matched pattern: {pattern}"
        ))
        .into());
    }

    Ok(matches)
}

/// Expand a remote glob pattern by listing files and filtering with fnmatch.
async fn expand_remote_glob(
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    pattern: &str,
) -> Result<Vec<String>> {
    if !is_glob_pattern(pattern) {
        return Ok(vec![pattern.to_string()]);
    }

    // Extract directory prefix for listing (everything before the first glob char)
    let dir_prefix = pattern
        .find(['*', '?', '['])
        .and_then(|pos| pattern[..pos].rfind('/'))
        .map(|pos| &pattern[..pos]);

    let files = client
        .list_onelake_files(workspace, item_id, dir_prefix)
        .await?;

    let prefix_with_id = format!("{item_id}/");
    let glob_pattern = glob::Pattern::new(pattern)
        .map_err(|e| crate::errors::FabioError::invalid_input(format!("Invalid pattern: {e}")))?;
    let match_opts = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let matched: Vec<String> = files
        .iter()
        .filter_map(|f| {
            let name = f.get("name").and_then(Value::as_str)?;
            let is_dir = f
                .get("isDirectory")
                .and_then(Value::as_str)
                .unwrap_or("false")
                == "true";
            if is_dir {
                return None;
            }
            // Strip item ID prefix to get the logical path
            let logical_path = name.strip_prefix(&prefix_with_id).unwrap_or(name);
            if glob_pattern.matches_with(logical_path, match_opts) {
                Some(logical_path.to_string())
            } else {
                None
            }
        })
        .collect();

    if matched.is_empty() {
        return Err(crate::errors::FabioError::invalid_input(format!(
            "No remote files matched pattern: {pattern}"
        ))
        .into());
    }

    Ok(matched)
}

/// Expand a table name glob pattern against the lakehouse table list.
async fn expand_table_glob(
    client: &FabricClient,
    workspace: &str,
    lakehouse_id: &str,
    pattern: &str,
) -> Result<Vec<String>> {
    if !is_glob_pattern(pattern) {
        return Ok(vec![pattern.to_string()]);
    }

    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/lakehouses/{lakehouse_id}/tables"),
            "data",
            true, // Always paginate for glob expansion
            None,
        )
        .await?;

    let glob_pattern = glob::Pattern::new(pattern)
        .map_err(|e| crate::errors::FabioError::invalid_input(format!("Invalid pattern: {e}")))?;
    let match_opts = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let matched: Vec<String> = resp
        .items
        .iter()
        .filter_map(|t| {
            let name = t.get("name").and_then(Value::as_str)?;
            if glob_pattern.matches_with(name, match_opts) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();

    if matched.is_empty() {
        return Err(crate::errors::FabioError::invalid_input(format!(
            "No tables matched pattern: {pattern}"
        ))
        .into());
    }

    Ok(matched)
}

/// Render a batch operation result (success or partial failure).
fn render_batch_result(
    cli: &Cli,
    summary: &crate::parallel::BatchSummary,
    status_verb: &str,
) -> Result<()> {
    if summary.all_succeeded() {
        let obj = serde_json::json!({
            "filesProcessed": summary.succeeded,
            "status": status_verb
        });
        output::render_object(cli, &obj, "status");
        Ok(())
    } else {
        let obj = serde_json::json!({
            "filesProcessed": summary.succeeded,
            "filesFailed": summary.failed,
            "failures": summary.failures,
            "status": "partial_failure"
        });
        output::render_object(cli, &obj, "status");
        Err(crate::errors::FabioError::new(
            crate::errors::ErrorCode::ApiError,
            format!(
                "Operation partially failed: {}/{} files {status_verb}",
                summary.succeeded, summary.total
            ),
        )
        .into())
    }
}

/// Sync files between source and destination paths in `OneLake`.
/// By default, compares files using `ETag` (from listing, zero extra API calls).
/// With `--checksum`, uses `Content-MD5` via HEAD requests for content-level verification.
/// Optionally deletes files at dest that don't exist in source (`--delete`).
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn sync_files(
    cli: &Cli,
    client: &FabricClient,
    src_ws: &str,
    src_id: &str,
    src_path: &str,
    dst_ws: &str,
    dst_id: &str,
    dst_path: &str,
    delete_extra: bool,
    checksum: bool,
) -> Result<()> {
    use crate::parallel::{self, BatchSummary};

    let concurrency = parallel::default_concurrency();

    // Build file maps for source and destination
    let src_map = build_file_map(client, src_ws, src_id, src_path).await?;
    let dst_map = build_file_map(client, dst_ws, dst_id, dst_path).await?;

    // Determine files to copy based on comparison strategy
    let to_copy = if checksum {
        // MD5-based: need HEAD requests for files that exist in both
        eprintln!("  Using Content-MD5 checksums (HEAD per file)...");
        compute_checksum_diff(
            client,
            &src_map,
            &dst_map,
            src_ws,
            src_id,
            src_path,
            dst_ws,
            dst_id,
            dst_path,
            concurrency,
        )
        .await?
    } else {
        // ETag-based (default): compare ETags from listing (free)
        src_map
            .keys()
            .filter(|rel| {
                dst_map.get(*rel).is_none_or(|dst_info| {
                    let src_info = &src_map[*rel];
                    src_info.etag != dst_info.etag
                })
            })
            .cloned()
            .collect()
    };

    // Determine files to delete (at dest but not in source)
    let to_delete: Vec<String> = if delete_extra {
        dst_map
            .keys()
            .filter(|rel| !src_map.contains_key(*rel))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    let strategy = if checksum { "Content-MD5" } else { "ETag" };
    eprintln!(
        "  Sync ({strategy}): {} to copy, {} to delete, concurrency={concurrency}",
        to_copy.len(),
        to_delete.len()
    );

    // Copy new/modified files in parallel
    let (copied, copy_failed) = if to_copy.is_empty() {
        (0, 0)
    } else {
        let copy_tasks: Vec<(String, String)> = to_copy
            .iter()
            .map(|rel| (format!("{src_path}/{rel}"), format!("{dst_path}/{rel}")))
            .collect();
        let item_names: Vec<String> = to_copy.clone();
        let sw = src_ws.to_string();
        let si = src_id.to_string();
        let dw = dst_ws.to_string();
        let di = dst_id.to_string();
        let cc = client.clone();

        let results = parallel::execute_parallel(copy_tasks, concurrency, move |(src, dst)| {
            let c = cc.clone();
            let sw = sw.clone();
            let si = si.clone();
            let dw = dw.clone();
            let di = di.clone();
            async move {
                c.copy_onelake_file(&sw, &si, &src, &dw, &di, &dst).await?;
                Ok(())
            }
        })
        .await;
        let summary = BatchSummary::from_results(&results, &item_names);
        (summary.succeeded, summary.failed)
    };

    // Delete extra files in parallel
    let (deleted, delete_failed) = if to_delete.is_empty() {
        (0, 0)
    } else {
        let delete_tasks: Vec<String> = to_delete
            .iter()
            .map(|rel| format!("{dst_path}/{rel}"))
            .collect();
        let item_names: Vec<String> = to_delete.clone();
        let dw = dst_ws.to_string();
        let di = dst_id.to_string();
        let cc = client.clone();

        let results = parallel::execute_parallel(delete_tasks, concurrency, move |path| {
            let c = cc.clone();
            let dw = dw.clone();
            let di = di.clone();
            async move {
                c.delete_onelake_file(&dw, &di, &path).await?;
                Ok(())
            }
        })
        .await;
        let summary = BatchSummary::from_results(&results, &item_names);
        (summary.succeeded, summary.failed)
    };

    let total_failed = copy_failed + delete_failed;
    let status = if total_failed == 0 {
        "synced"
    } else {
        "partial_failure"
    };
    let obj = serde_json::json!({
        "sourceFiles": src_map.len(),
        "destFiles": dst_map.len(),
        "copied": copied,
        "deleted": deleted,
        "unchanged": src_map.len() - to_copy.len(),
        "failed": total_failed,
        "strategy": strategy,
        "status": status
    });
    output::render_object(cli, &obj, "status");

    if total_failed > 0 {
        return Err(crate::errors::FabioError::new(
            crate::errors::ErrorCode::ApiError,
            format!("Sync partially failed: {total_failed} operations failed"),
        )
        .into());
    }

    Ok(())
}

/// File metadata extracted from DFS listing.
struct FileInfo {
    size: u64,
    etag: String,
}

/// Build a file map (`relative_path` -> `FileInfo`) from a remote listing.
/// Lists from root (no directory param) to avoid the DFS virtual view doubling.
async fn build_file_map(
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
    path: &str,
) -> Result<std::collections::HashMap<String, FileInfo>> {
    let files = client.list_onelake_files(workspace, item_id, None).await?;
    let prefix = format!("{item_id}/{path}/");

    let mut map = std::collections::HashMap::new();
    for file in &files {
        let Some(name) = file.get("name").and_then(Value::as_str) else {
            continue;
        };
        let is_dir = file
            .get("isDirectory")
            .and_then(Value::as_str)
            .unwrap_or("false")
            == "true";
        if is_dir {
            continue;
        }
        if let Some(relative) = name.strip_prefix(&prefix) {
            let size = file
                .get("contentLength")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let etag = file
                .get("eTag")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            map.insert(relative.to_string(), FileInfo { size, etag });
        }
    }
    Ok(map)
}

/// Compute diff using `Content-MD5` checksums (parallel HEAD requests).
/// Returns list of relative paths that need copying.
#[allow(clippy::too_many_arguments)]
async fn compute_checksum_diff(
    client: &FabricClient,
    src_map: &std::collections::HashMap<String, FileInfo>,
    dst_map: &std::collections::HashMap<String, FileInfo>,
    src_ws: &str,
    src_id: &str,
    src_path: &str,
    dst_ws: &str,
    dst_id: &str,
    dst_path: &str,
    concurrency: usize,
) -> Result<Vec<String>> {
    use crate::parallel;

    // Files only in source — always copy
    let mut to_copy: Vec<String> = src_map
        .keys()
        .filter(|rel| !dst_map.contains_key(*rel))
        .cloned()
        .collect();

    // Files in both — compare MD5 via HEAD
    let common: Vec<String> = src_map
        .keys()
        .filter(|rel| dst_map.contains_key(*rel))
        .cloned()
        .collect();

    if common.is_empty() {
        return Ok(to_copy);
    }

    eprintln!("  Checking MD5 for {} files...", common.len());

    // Get MD5 for source files
    let src_tasks: Vec<String> = common
        .iter()
        .map(|rel| format!("{src_path}/{rel}"))
        .collect();
    let sw = src_ws.to_string();
    let si = src_id.to_string();
    let cc = client.clone();
    let src_results = parallel::execute_parallel(src_tasks, concurrency, move |path| {
        let c = cc.clone();
        let sw = sw.clone();
        let si = si.clone();
        async move {
            let props = c.get_file_properties(&sw, &si, &path).await?;
            Ok(props)
        }
    })
    .await;

    // Get MD5 for dest files
    let dst_tasks: Vec<String> = common
        .iter()
        .map(|rel| format!("{dst_path}/{rel}"))
        .collect();
    let dw = dst_ws.to_string();
    let di = dst_id.to_string();
    let cc = client.clone();
    let dst_results = parallel::execute_parallel(dst_tasks, concurrency, move |path| {
        let c = cc.clone();
        let dw = dw.clone();
        let di = di.clone();
        async move {
            let props = c.get_file_properties(&dw, &di, &path).await?;
            Ok(props)
        }
    })
    .await;

    // Compare MD5s
    for (i, rel) in common.iter().enumerate() {
        let src_md5 = src_results
            .get(i)
            .and_then(|r| r.result.as_ref().ok())
            .and_then(|v| v.get("contentMD5"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let dst_md5 = dst_results
            .get(i)
            .and_then(|r| r.result.as_ref().ok())
            .and_then(|v| v.get("contentMD5"))
            .and_then(Value::as_str)
            .unwrap_or("");

        // If either MD5 is empty (not provided by API), fall back to size comparison
        if src_md5.is_empty() || dst_md5.is_empty() {
            let src_info = &src_map[rel];
            let dst_info = &dst_map[rel];
            if src_info.size != dst_info.size {
                to_copy.push(rel.clone());
            }
        } else if src_md5 != dst_md5 {
            to_copy.push(rel.clone());
        }
    }

    Ok(to_copy)
}

async fn delete_table(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    table: &str,
) -> Result<()> {
    use crate::parallel::{self, BatchSummary};

    let tables = expand_table_glob(client, workspace, id, table).await?;

    if tables.len() == 1 {
        let path = format!("Tables/{}", tables[0]);
        client
            .delete_onelake_directory(workspace, id, &path)
            .await?;
        let obj = serde_json::json!({
            "table": tables[0],
            "status": "deleted"
        });
        output::render_object(cli, &obj, "status");
        return Ok(());
    }

    // Multiple tables matched — delete in parallel
    let concurrency = parallel::default_concurrency();
    eprintln!(
        "  Deleting {} tables with concurrency={concurrency}...",
        tables.len()
    );

    let item_names = tables.clone();
    let workspace = workspace.to_string();
    let id = id.to_string();
    let client = client.clone();

    let results = parallel::execute_parallel(tables, concurrency, move |tbl| {
        let client = client.clone();
        let workspace = workspace.clone();
        let id = id.clone();
        async move {
            let path = format!("Tables/{tbl}");
            client
                .delete_onelake_directory(&workspace, &id, &path)
                .await?;
            Ok(())
        }
    })
    .await;

    let summary = BatchSummary::from_results(&results, &item_names);
    render_batch_result(cli, &summary, "deleted")
}

#[allow(clippy::too_many_arguments)]
async fn copy_table(
    cli: &Cli,
    client: &FabricClient,
    src_ws: &str,
    src_id: &str,
    src_table: &str,
    dst_ws: &str,
    dst_id: &str,
    dst_table: Option<&str>,
) -> Result<()> {
    let tables = expand_table_glob(client, src_ws, src_id, src_table).await?;

    if tables.len() > 1 {
        // Multiple tables — process each (dest_table is ignored for globs)
        eprintln!("  Copying {} tables...", tables.len());
        for tbl in &tables {
            copy_single_table(cli, client, src_ws, src_id, tbl, dst_ws, dst_id, tbl, true).await?;
        }
        return Ok(());
    }

    let table_name = &tables[0];
    let dest_name = dst_table.unwrap_or(table_name);
    copy_single_table(
        cli, client, src_ws, src_id, table_name, dst_ws, dst_id, dest_name, true,
    )
    .await
}

/// Copy a single table's files in parallel.
/// When `render` is true, outputs result to stdout. When false, stays silent (for `move_table`).
#[allow(clippy::too_many_arguments)]
async fn copy_single_table(
    cli: &Cli,
    client: &FabricClient,
    src_ws: &str,
    src_id: &str,
    src_table: &str,
    dst_ws: &str,
    dst_id: &str,
    dest_name: &str,
    render: bool,
) -> Result<()> {
    use crate::parallel::{self, BatchSummary};

    let concurrency = parallel::default_concurrency();

    // List all files from root (no directory param) and filter for this table
    let files = client.list_onelake_files(src_ws, src_id, None).await?;
    let prefix = format!("{src_id}/Tables/{src_table}/");

    // Collect file copy tasks
    let mut copy_tasks: Vec<(String, String)> = Vec::new();
    for file in &files {
        if let Some(name) = file.get("name").and_then(Value::as_str) {
            let is_dir = file
                .get("isDirectory")
                .and_then(Value::as_str)
                .unwrap_or("false")
                == "true";
            if is_dir {
                continue;
            }
            if let Some(relative) = name.strip_prefix(&prefix) {
                let src_path = format!("Tables/{src_table}/{relative}");
                let dst_path = format!("Tables/{dest_name}/{relative}");
                copy_tasks.push((src_path, dst_path));
            }
        }
    }

    let total_files = copy_tasks.len();
    if total_files == 0 {
        if render {
            let obj = serde_json::json!({
                "sourceTable": src_table,
                "destTable": dest_name,
                "filesCopied": 0,
                "status": "copied"
            });
            output::render_object(cli, &obj, "status");
        }
        return Ok(());
    }

    eprintln!(
        "  Copying {total_files} files for table '{src_table}' with concurrency={concurrency}..."
    );

    let item_names: Vec<String> = copy_tasks.iter().map(|(s, _)| s.clone()).collect();

    let src_ws = src_ws.to_string();
    let src_id = src_id.to_string();
    let dst_ws = dst_ws.to_string();
    let dst_id = dst_id.to_string();
    let client = client.clone();

    let results =
        parallel::execute_parallel(copy_tasks, concurrency, move |(src_path, dst_path)| {
            let client = client.clone();
            let src_ws = src_ws.clone();
            let src_id = src_id.clone();
            let dst_ws = dst_ws.clone();
            let dst_id = dst_id.clone();
            async move {
                client
                    .copy_onelake_file(&src_ws, &src_id, &src_path, &dst_ws, &dst_id, &dst_path)
                    .await?;
                Ok(())
            }
        })
        .await;

    let summary = BatchSummary::from_results(&results, &item_names);

    if summary.all_succeeded() {
        if render {
            let obj = serde_json::json!({
                "sourceTable": src_table,
                "destTable": dest_name,
                "filesCopied": summary.succeeded,
                "status": "copied"
            });
            output::render_object(cli, &obj, "status");
        }
        Ok(())
    } else {
        if render {
            let obj = serde_json::json!({
                "sourceTable": src_table,
                "destTable": dest_name,
                "filesCopied": summary.succeeded,
                "filesFailed": summary.failed,
                "failures": summary.failures,
                "status": "partial_failure"
            });
            output::render_object(cli, &obj, "status");
        }
        Err(crate::errors::FabioError::new(
            crate::errors::ErrorCode::ApiError,
            format!(
                "Table copy partially failed: {}/{} files copied",
                summary.succeeded, summary.total
            ),
        )
        .into())
    }
}

#[allow(clippy::too_many_arguments)]
async fn move_table(
    cli: &Cli,
    client: &FabricClient,
    src_ws: &str,
    src_id: &str,
    src_table: &str,
    dst_ws: &str,
    dst_id: &str,
    dst_table: Option<&str>,
) -> Result<()> {
    let tables = expand_table_glob(client, src_ws, src_id, src_table).await?;

    if tables.len() > 1 {
        // Multiple tables — move each (dest_table is ignored for globs)
        eprintln!("  Moving {} tables...", tables.len());
        for tbl in &tables {
            copy_single_table(cli, client, src_ws, src_id, tbl, dst_ws, dst_id, tbl, false).await?;
            let path = format!("Tables/{tbl}");
            client
                .delete_onelake_directory(src_ws, src_id, &path)
                .await?;
            eprintln!("  ✓ moved table '{tbl}'");
        }
        return Ok(());
    }

    let table_name = &tables[0];
    let dest_name = dst_table.unwrap_or(table_name);

    // Copy table (parallel) — errors will propagate if any file fails
    copy_single_table(
        cli, client, src_ws, src_id, table_name, dst_ws, dst_id, dest_name, false,
    )
    .await?;

    // Delete source table only after ALL copies succeeded
    let path = format!("Tables/{table_name}");
    client
        .delete_onelake_directory(src_ws, src_id, &path)
        .await?;

    let obj = serde_json::json!({
        "sourceTable": table_name,
        "destTable": dest_name,
        "status": "moved"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create_shortcut(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: &str,
    path: &str,
    target_type: &str,
    target: &str,
) -> Result<()> {
    let target_body: Value = serde_json::from_str(target).map_err(|e| {
        crate::errors::FabioError::invalid_input(format!("Invalid target JSON: {e}"))
    })?;

    let body = serde_json::json!({
        "name": name,
        "path": path,
        "target": {
            target_type: target_body
        }
    });

    let data = client
        .post(
            &format!("/workspaces/{workspace}/items/{id}/shortcuts"),
            &body,
            false,
        )
        .await?;
    output::render_object(cli, &data, "name");
    Ok(())
}

async fn get_shortcut(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: &str,
    path: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/items/{id}/shortcuts/{path}/{name}"
        ))
        .await?;
    output::render_object(cli, &data, "name");
    Ok(())
}

async fn delete_shortcut(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: &str,
    path: &str,
) -> Result<()> {
    client
        .delete(&format!(
            "/workspaces/{workspace}/items/{id}/shortcuts/{path}/{name}"
        ))
        .await?;

    let obj = serde_json::json!({
        "name": name,
        "path": path,
        "status": "deleted"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn bulk_create_shortcuts(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
    conflict_policy: Option<&str>,
) -> Result<()> {
    let input = read_shortcut_json_input(file, content)?;

    // Wrap in the API envelope if user provided a raw array
    let body = if input.is_array() {
        serde_json::json!({ "createShortcutRequests": input })
    } else {
        input
    };

    if output::dry_run_guard(cli, "lakehouse bulk-create-shortcuts", &body) {
        return Ok(());
    }

    let mut url = format!("/workspaces/{workspace}/items/{id}/shortcuts/bulkCreate");
    if let Some(policy) = conflict_policy {
        use std::fmt::Write;
        let _ = write!(url, "?shortcutConflictPolicy={policy}");
    }

    let data = client.post(&url, &body, true).await?;
    output::render_object(cli, &data, "value");
    Ok(())
}

fn read_shortcut_json_input(file: Option<&str>, content: Option<&str>) -> Result<Value> {
    if let Some(c) = content {
        serde_json::from_str(c).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in --content: {e}"),
                "Provide a valid JSON array of shortcut requests.".to_string(),
            )
            .into()
        })
    } else if let Some(f) = file {
        let data = std::fs::read_to_string(f).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Failed to read file '{f}': {e}"),
                "Provide a valid file path.".to_string(),
            )
        })?;
        serde_json::from_str(&data).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in file '{f}': {e}"),
                "Provide a valid JSON array of shortcut requests.".to_string(),
            )
            .into()
        })
    } else {
        Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Either --file or --content must be provided".to_string(),
            "Example: fabio lakehouse bulk-create-shortcuts --workspace <WS> --id <ID> --file shortcuts.json".to_string(),
        )
        .into())
    }
}

// ─── Definitions ─────────────────────────────────────────────────────────────

async fn get_definition(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/lakehouses/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse get-definition", "Contributor"))?;
    output::render_object(cli, &data, "definition");
    Ok(())
}

async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body_str = match (file, content) {
        (Some(path), _) => std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))?,
        (_, Some(c)) => c.to_string(),
        (None, None) => {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                "Either --file or --content must be provided".to_string(),
                "Example: fabio lakehouse update-definition --workspace <WS> --id <ID> --file definition.json".to_string(),
            ).into());
        }
    };

    let body: Value = serde_json::from_str(&body_str)?;

    if output::dry_run_guard(
        cli,
        "lakehouse update-definition",
        &serde_json::json!({ "workspace": workspace, "id": id }),
    ) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/lakehouses/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse update-definition", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "definition_updated" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Materialized Lake Views ─────────────────────────────────────────────────

async fn refresh_materialized_views(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let body = serde_json::json!({});

    if output::dry_run_guard(cli, "lakehouse refresh-materialized-views", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/lakehouses/{id}/jobs/refreshMaterializedLakeViews/instances"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse refresh-materialized-views", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "refresh_triggered" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

async fn create_materialized_views_schedule(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "create-materialized-views-schedule")?;

    if output::dry_run_guard(cli, "lakehouse create-materialized-views-schedule", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/lakehouses/{id}/jobs/refreshMaterializedLakeViews/schedules"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse create-materialized-views-schedule", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn update_materialized_views_schedule(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    schedule_id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = read_json_body(file, content, "update-materialized-views-schedule")?;

    if output::dry_run_guard(cli, "lakehouse update-materialized-views-schedule", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/lakehouses/{id}/jobs/refreshMaterializedLakeViews/schedules/{schedule_id}"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse update-materialized-views-schedule", "Contributor"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete_materialized_views_schedule(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    schedule_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "lakehouse delete-materialized-views-schedule",
        &serde_json::json!({ "workspace": workspace, "id": id, "scheduleId": schedule_id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!(
            "/workspaces/{workspace}/lakehouses/{id}/jobs/refreshMaterializedLakeViews/schedules/{schedule_id}"
        ))
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse delete-materialized-views-schedule", "Contributor"))?;

    let obj = serde_json::json!({ "scheduleId": schedule_id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

// ─── Table Maintenance ───────────────────────────────────────────────────────

async fn run_table_maintenance(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    file: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
    let body = match (file, content) {
        (Some(f), _) => {
            let text = std::fs::read_to_string(f)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{f}': {e}"))?;
            serde_json::from_str(&text)?
        }
        (_, Some(c)) => serde_json::from_str(c)?,
        (None, None) => serde_json::json!({}),
    };

    if output::dry_run_guard(cli, "lakehouse run-table-maintenance", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/lakehouses/{id}/jobs/tableMaintenance/instances"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse run-table-maintenance", "Contributor"))?;

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({ "id": id, "status": "maintenance_triggered" });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Livy Sessions ───────────────────────────────────────────────────────────

async fn list_livy_sessions(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/lakehouses/{id}/livySessions"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "name", "state", "kind"],
        &["ID", "NAME", "STATE", "KIND"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn get_livy_session(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    livy_id: &str,
) -> Result<()> {
    let data = client
        .get(&format!(
            "/workspaces/{workspace}/lakehouses/{id}/livySessions/{livy_id}"
        ))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn read_json_body(file: Option<&str>, content: Option<&str>, command: &str) -> Result<Value> {
    match (file, content) {
        (Some(f), _) => {
            let text = std::fs::read_to_string(f)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{f}': {e}"))?;
            Ok(serde_json::from_str(&text)?)
        }
        (_, Some(c)) => Ok(serde_json::from_str(c)?),
        _ => Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Either --file or --content must be provided".to_string(),
            format!(
                "Example: fabio lakehouse {command} --workspace <WS> --id <ID> --file config.json"
            ),
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── detect_format_from_extension ────────────────────────────────────

    #[test]
    fn detect_format_csv() {
        assert_eq!(detect_format_from_extension("data.csv").unwrap(), "Csv");
    }

    #[test]
    fn detect_format_tsv() {
        assert_eq!(detect_format_from_extension("data.tsv").unwrap(), "Csv");
    }

    #[test]
    fn detect_format_parquet() {
        assert_eq!(
            detect_format_from_extension("sales.parquet").unwrap(),
            "Parquet"
        );
    }

    #[test]
    fn detect_format_pq_shorthand() {
        assert_eq!(
            detect_format_from_extension("events.pq").unwrap(),
            "Parquet"
        );
    }

    #[test]
    fn detect_format_json_errors() {
        let err = detect_format_from_extension("logs.json").unwrap_err();
        let fabio_err = err.downcast_ref::<FabioError>().unwrap();
        assert_eq!(fabio_err.code, ErrorCode::InvalidInput);
        assert!(fabio_err.message.contains("JSON format is not supported"));
    }

    #[test]
    fn detect_format_jsonl_errors() {
        let err = detect_format_from_extension("stream.jsonl").unwrap_err();
        let fabio_err = err.downcast_ref::<FabioError>().unwrap();
        assert_eq!(fabio_err.code, ErrorCode::InvalidInput);
        assert!(fabio_err.message.contains("JSON format is not supported"));
    }

    #[test]
    fn detect_format_ndjson_errors() {
        let err = detect_format_from_extension("events.ndjson").unwrap_err();
        let fabio_err = err.downcast_ref::<FabioError>().unwrap();
        assert_eq!(fabio_err.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn detect_format_case_insensitive() {
        assert_eq!(detect_format_from_extension("DATA.CSV").unwrap(), "Csv");
        assert_eq!(
            detect_format_from_extension("big.PARQUET").unwrap(),
            "Parquet"
        );
        // JSON should error regardless of case
        assert!(detect_format_from_extension("log.JSON").is_err());
    }

    #[test]
    fn detect_format_mixed_case() {
        assert_eq!(detect_format_from_extension("file.Csv").unwrap(), "Csv");
        assert_eq!(
            detect_format_from_extension("file.Parquet").unwrap(),
            "Parquet"
        );
        // JSON should error regardless of case
        assert!(detect_format_from_extension("file.JsonL").is_err());
    }

    #[test]
    fn detect_format_with_path_components() {
        assert_eq!(
            detect_format_from_extension("/tmp/dir/file.csv").unwrap(),
            "Csv"
        );
        assert_eq!(
            detect_format_from_extension("./relative/path.parquet").unwrap(),
            "Parquet"
        );
        // JSON should error even with path components
        assert!(detect_format_from_extension("C:\\Users\\data\\file.json").is_err());
    }

    #[test]
    fn detect_format_with_dots_in_path() {
        assert_eq!(
            detect_format_from_extension("/tmp/my.dir/v1.2/data.csv").unwrap(),
            "Csv"
        );
    }

    #[test]
    fn detect_format_unknown_extension_errors() {
        let err = detect_format_from_extension("data.xlsx").unwrap_err();
        let fabio_err = err.downcast_ref::<FabioError>().unwrap();
        assert_eq!(fabio_err.code, ErrorCode::InvalidInput);
        assert!(
            fabio_err.message.contains("xlsx"),
            "error message should mention the extension"
        );
        let hint = fabio_err.hint.as_ref().unwrap();
        assert!(hint.contains("--format"), "hint should suggest --format");
        assert!(hint.contains("Csv"), "hint should list valid formats");
    }

    #[test]
    fn detect_format_no_extension_errors() {
        let err = detect_format_from_extension("Makefile").unwrap_err();
        let fabio_err = err.downcast_ref::<FabioError>().unwrap();
        assert_eq!(fabio_err.code, ErrorCode::InvalidInput);
        let hint = fabio_err.hint.as_ref().unwrap();
        assert!(hint.contains("--format"), "hint should suggest --format");
    }

    #[test]
    fn detect_format_hidden_file_no_extension_errors() {
        let err = detect_format_from_extension(".gitignore").unwrap_err();
        let fabio_err = err.downcast_ref::<FabioError>().unwrap();
        assert_eq!(fabio_err.code, ErrorCode::InvalidInput);
    }

    // ─── upload_table staging path derivation ────────────────────────────

    #[test]
    fn staging_path_uses_filename_only() {
        // Verify the staging path logic (same as in upload_table)
        let source = "/home/user/data/sales_2024.csv";
        let filename = Path::new(source)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let staging = format!("Files/.staging/{filename}");
        assert_eq!(staging, "Files/.staging/sales_2024.csv");
    }

    #[test]
    fn staging_path_handles_deep_nesting() {
        let source = "/home/user/projects/etl/output/2024/01/report.parquet";
        let filename = Path::new(source)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let staging = format!("Files/.staging/{filename}");
        assert_eq!(staging, "Files/.staging/report.parquet");
    }

    #[test]
    fn staging_path_handles_relative_paths() {
        let source = "./data/events.json";
        let filename = Path::new(source)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let staging = format!("Files/.staging/{filename}");
        assert_eq!(staging, "Files/.staging/events.json");
    }

    // ─── upload_table mode/format validation (inline logic) ──────────────

    #[test]
    fn valid_modes_accepted() {
        const VALID_MODES: &[&str] = &["Overwrite", "Append"];
        assert!(VALID_MODES.contains(&"Overwrite"));
        assert!(VALID_MODES.contains(&"Append"));
        assert!(!VALID_MODES.contains(&"overwrite"));
        assert!(!VALID_MODES.contains(&"OVERWRITE"));
        assert!(!VALID_MODES.contains(&"Upsert"));
        assert!(!VALID_MODES.contains(&"Replace"));
    }

    #[test]
    fn valid_formats_accepted() {
    const VALID_FORMATS: &[&str] = &["Csv", "Parquet"];
        assert!(VALID_FORMATS.contains(&"Csv"));
        assert!(VALID_FORMATS.contains(&"Parquet"));
        assert!(!VALID_FORMATS.contains(&"Json"));
        assert!(!VALID_FORMATS.contains(&"csv"));
        assert!(!VALID_FORMATS.contains(&"Avro"));
        assert!(!VALID_FORMATS.contains(&"XML"));
    }

    // ─── is_glob_pattern ─────────────────────────────────────────────────

    #[test]
    fn glob_pattern_detection() {
        assert!(is_glob_pattern("Files/*.csv"));
        assert!(is_glob_pattern("Tables/[a-z]*"));
        assert!(is_glob_pattern("data?.parquet"));
        assert!(!is_glob_pattern("Files/data.csv"));
        assert!(!is_glob_pattern("/plain/path/file.txt"));
    }

    #[test]
    fn glob_pattern_in_directory() {
        assert!(is_glob_pattern("Files/subdir/*.parquet"));
        assert!(is_glob_pattern("Tables/sales_*"));
        assert!(!is_glob_pattern("Tables/sales_2024"));
    }
}
