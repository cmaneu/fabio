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
    /// Upload a file to a lakehouse
    Upload {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Lakehouse ID
        #[arg(long)]
        id: String,

        /// Local source path
        #[arg(short = 's', long = "source-path")]
        source_path: String,

        /// Remote destination path
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
    /// Copy a file between lakehouses (server-side)
    CopyFile {
        /// Source workspace ID
        #[arg(long, visible_alias = "sw")]
        source_workspace: String,

        /// Source lakehouse ID
        #[arg(long, visible_alias = "si")]
        source_id: String,

        /// Source file path
        #[arg(short = 's', long = "source-path")]
        source_path: String,

        /// Destination workspace ID
        #[arg(long, visible_alias = "dw")]
        dest_workspace: String,

        /// Destination lakehouse ID
        #[arg(long, visible_alias = "di")]
        dest_id: String,

        /// Destination file path
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
    /// Move a file between lakehouses (copy + delete source)
    MoveFile {
        /// Source workspace ID
        #[arg(long, visible_alias = "sw")]
        source_workspace: String,

        /// Source lakehouse ID
        #[arg(long, visible_alias = "si")]
        source_id: String,

        /// Source file path
        #[arg(short = 's', long = "source-path")]
        source_path: String,

        /// Destination workspace ID
        #[arg(long, visible_alias = "dw")]
        dest_workspace: String,

        /// Destination lakehouse ID
        #[arg(long, visible_alias = "di")]
        dest_id: String,

        /// Destination file path
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
    let data = client
        .get(&format!("/workspaces/{workspace}/lakehouses/{id}/tables"))
        .await?;
    let items = data
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| data.get("value").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default();

    output::render_list(
        cli,
        &items,
        &["name", "type", "format"],
        &["NAME", "TYPE", "FORMAT"],
        "name",
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
    let path = Path::new(source_path);
    let data = std::fs::read(path).map_err(|e| {
        crate::errors::FabioError::invalid_input(format!("Cannot read file {source_path}: {e}"))
    })?;

    let result = client
        .upload_onelake_file(workspace, id, dest_path, &data)
        .await?;
    output::render_object(cli, &result, "status");
    Ok(())
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
    let result = client
        .copy_onelake_file(src_ws, src_id, src_path, dst_ws, dst_id, dst_path)
        .await?;
    output::render_object(cli, &result, "status");
    Ok(())
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
    // Copy then delete source
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
    Ok(())
}

async fn delete_table(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    table: &str,
) -> Result<()> {
    let path = format!("Tables/{table}");
    let result = client
        .delete_onelake_directory(workspace, id, &path)
        .await?;

    let obj = serde_json::json!({
        "table": table,
        "status": "deleted"
    });
    output::render_object(
        cli,
        &obj,
        result
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("status"),
    );
    Ok(())
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
    let dest_name = dst_table.unwrap_or(src_table);

    // List all files from root (no directory param) and filter for this table
    let files = client.list_onelake_files(src_ws, src_id, None).await?;
    let prefix = format!("{src_id}/Tables/{src_table}/");

    let mut copied = 0;
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
                client
                    .copy_onelake_file(src_ws, src_id, &src_path, dst_ws, dst_id, &dst_path)
                    .await?;
                copied += 1;
            }
        }
    }

    let obj = serde_json::json!({
        "sourceTable": src_table,
        "destTable": dest_name,
        "filesCopied": copied,
        "status": "copied"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
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
    let dest_name = dst_table.unwrap_or(src_table);

    // Copy table
    copy_table(
        cli,
        client,
        src_ws,
        src_id,
        src_table,
        dst_ws,
        dst_id,
        Some(dest_name),
    )
    .await?;

    // Delete source table
    let path = format!("Tables/{src_table}");
    client
        .delete_onelake_directory(src_ws, src_id, &path)
        .await?;

    let obj = serde_json::json!({
        "sourceTable": src_table,
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
