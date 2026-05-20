use std::path::Path;

use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum LakehouseCommand {
    /// List tables in a lakehouse
    Tables {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,
    },
    /// List files in a lakehouse
    Files {
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
    /// Upload files to a lakehouse (supports glob patterns for parallel upload)
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
    /// Load a file into a Delta table
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

        /// File format: Csv, Parquet, Json
        #[arg(short, long, default_value = "Csv")]
        format: String,
    },
    /// Copy files between lakehouses (supports glob patterns for parallel copy)
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
    /// Delete a file from a lakehouse
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
    /// Move files between lakehouses (supports glob patterns for parallel move)
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
    /// Delete a table from a lakehouse
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
    /// Copy a table between lakehouses
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
    /// Create a shortcut
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
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &LakehouseCommand) -> Result<()> {
    match command {
        LakehouseCommand::Tables { workspace, id } => tables(cli, client, workspace, id).await,
        LakehouseCommand::Files {
            workspace,
            id,
            path,
        } => files(cli, client, workspace, id, path.as_deref()).await,
        LakehouseCommand::Upload {
            workspace,
            id,
            source_path,
            dest_path,
        } => upload(cli, client, workspace, id, source_path, dest_path).await,
        LakehouseCommand::Download {
            workspace,
            id,
            source_path,
            dest_path,
        } => download(cli, client, workspace, id, source_path, dest_path).await,
        LakehouseCommand::LoadTable {
            workspace,
            id,
            source_path,
            table,
            mode,
            format,
        } => load_table(cli, client, workspace, id, source_path, table, mode, format).await,
        LakehouseCommand::CopyFile {
            source_workspace,
            source_id,
            source_path,
            dest_workspace,
            dest_id,
            dest_path,
        } => {
            copy_file(
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
        }
        LakehouseCommand::DeleteFile {
            workspace,
            id,
            path,
        } => delete_file(cli, client, workspace, id, path).await,
        LakehouseCommand::MoveFile {
            source_workspace,
            source_id,
            source_path,
            dest_workspace,
            dest_id,
            dest_path,
        } => {
            move_file(
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
        }
        LakehouseCommand::DeleteTable {
            workspace,
            id,
            table,
        } => delete_table(cli, client, workspace, id, table).await,
        LakehouseCommand::CopyTable {
            source_workspace,
            source_id,
            source_table,
            dest_workspace,
            dest_id,
            dest_table,
        } => {
            copy_table(
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
        }
        LakehouseCommand::MoveTable {
            source_workspace,
            source_id,
            source_table,
            dest_workspace,
            dest_id,
            dest_table,
        } => {
            move_table(
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
        }
        LakehouseCommand::Sync {
            source_workspace,
            source_id,
            source_path,
            dest_workspace,
            dest_id,
            dest_path,
            delete,
            checksum,
        } => {
            sync_files(
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
        }
        LakehouseCommand::CreateShortcut {
            workspace,
            id,
            name,
            path,
            target_type,
            target,
        } => create_shortcut(cli, client, workspace, id, name, path, target_type, target).await,
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
        } => delete_shortcut(cli, client, workspace, id, name, path).await,
    }
}

async fn tables(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/lakehouses/{id}/tables"),
            "data",
            cli.all,
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
    const VALID_FORMATS: &[&str] = &["Csv", "Parquet", "Json"];

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

    let body = serde_json::json!({
        "relativePath": source_path,
        "pathType": "File",
        "mode": mode,
        "formatOptions": {
            "format": format,
            "header": true,
            "delimiter": ","
        }
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
    let regex = fnmatch_regex::glob_to_regex(pattern)
        .map_err(|e| crate::errors::FabioError::invalid_input(format!("Invalid pattern: {e}")))?;

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
            if regex.is_match(logical_path) {
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
        )
        .await?;

    let regex = fnmatch_regex::glob_to_regex(pattern)
        .map_err(|e| crate::errors::FabioError::invalid_input(format!("Invalid pattern: {e}")))?;

    let matched: Vec<String> = resp
        .items
        .iter()
        .filter_map(|t| {
            let name = t.get("name").and_then(Value::as_str)?;
            if regex.is_match(name) {
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
