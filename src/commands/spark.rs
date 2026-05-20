use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::enrich_forbidden;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum SparkCommand {
    /// Get workspace-level Spark settings (custom pools, starter pools, etc.)
    #[command(display_order = 1)]
    GetSettings {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Update workspace-level Spark settings
    #[command(display_order = 2)]
    UpdateSettings {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Settings as JSON (merges with existing settings).
        /// Example: '{"automaticLog":{"enabled":true},"highConcurrency":{"notebookInteractiveRunEnabled":true}}'
        #[arg(long)]
        settings: String,
    },

    // ── Custom Pools ─────────────────────────────────────────────────────
    /// List custom Spark pools in a workspace
    #[command(display_order = 10)]
    ListPools {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of a custom Spark pool
    #[command(display_order = 11)]
    GetPool {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Pool ID
        #[arg(long)]
        pool_id: String,
    },
    /// Create a custom Spark pool
    #[command(display_order = 12)]
    CreatePool {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Pool name
        #[arg(long)]
        name: String,

        /// Node family (e.g., `MemoryOptimized`)
        #[arg(long, default_value = "MemoryOptimized")]
        node_family: String,

        /// Node size (e.g., Small, Medium, Large, `XLarge`, `XXLarge`)
        #[arg(long, default_value = "Small", value_parser = ["Small", "Medium", "Large", "XLarge", "XXLarge"])]
        node_size: String,

        /// Enable auto-scale
        #[arg(long, default_value_t = true)]
        auto_scale_enabled: bool,

        /// Minimum node count (when auto-scale is enabled)
        #[arg(long, default_value_t = 1)]
        min_node_count: u32,

        /// Maximum node count
        #[arg(long, default_value_t = 3)]
        max_node_count: u32,

        /// Enable dynamic executor allocation
        #[arg(long, default_value_t = true)]
        dynamic_executor_enabled: bool,

        /// Minimum executors for dynamic allocation
        #[arg(long, default_value_t = 1)]
        min_executors: u32,

        /// Maximum executors for dynamic allocation
        #[arg(long, default_value_t = 2)]
        max_executors: u32,
    },
    /// Update a custom Spark pool
    #[command(display_order = 13)]
    UpdatePool {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Pool ID
        #[arg(long)]
        pool_id: String,

        /// Pool configuration as JSON (partial update)
        #[arg(long)]
        config: String,
    },
    /// Delete a custom Spark pool
    #[command(display_order = 14)]
    DeletePool {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Pool ID
        #[arg(long)]
        pool_id: String,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &SparkCommand) -> Result<()> {
    match command {
        SparkCommand::GetSettings { workspace } => get_settings(cli, client, workspace).await,
        SparkCommand::UpdateSettings {
            workspace,
            settings,
        } => update_settings(cli, client, workspace, settings).await,
        SparkCommand::ListPools { workspace } => list_pools(cli, client, workspace).await,
        SparkCommand::GetPool { workspace, pool_id } => {
            get_pool(cli, client, workspace, pool_id).await
        }
        SparkCommand::CreatePool {
            workspace,
            name,
            node_family,
            node_size,
            auto_scale_enabled,
            min_node_count,
            max_node_count,
            dynamic_executor_enabled,
            min_executors,
            max_executors,
        } => {
            create_pool(
                cli,
                client,
                workspace,
                name,
                node_family,
                node_size,
                *auto_scale_enabled,
                *min_node_count,
                *max_node_count,
                *dynamic_executor_enabled,
                *min_executors,
                *max_executors,
            )
            .await
        }
        SparkCommand::UpdatePool {
            workspace,
            pool_id,
            config,
        } => update_pool(cli, client, workspace, pool_id, config).await,
        SparkCommand::DeletePool { workspace, pool_id } => {
            delete_pool(cli, client, workspace, pool_id).await
        }
    }
}

// ─── Settings ────────────────────────────────────────────────────────────────

async fn get_settings(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/spark/settings"))
        .await?;
    output::render_object(cli, &data, "workspace");
    Ok(())
}

async fn update_settings(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    settings: &str,
) -> Result<()> {
    let body: Value = serde_json::from_str(settings)
        .map_err(|e| anyhow::anyhow!("Invalid --settings JSON: {e}"))?;

    if output::dry_run_guard(cli, "spark update-settings", &body) {
        return Ok(());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/spark/settings"), &body)
        .await
        .map_err(|e| enrich_forbidden(e, "spark update-settings", "Admin"))?;
    output::render_object(cli, &data, "workspace");
    Ok(())
}

// ─── Custom Pools ────────────────────────────────────────────────────────────

async fn list_pools(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/spark/pools"),
            "value",
            cli.all,
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["name", "id", "nodeFamily", "nodeSize"],
        &["NAME", "ID", "NODE FAMILY", "NODE SIZE"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn get_pool(cli: &Cli, client: &FabricClient, workspace: &str, pool_id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/spark/pools/{pool_id}"))
        .await?;
    output::render_object(cli, &data, "id");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create_pool(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    node_family: &str,
    node_size: &str,
    auto_scale_enabled: bool,
    min_node_count: u32,
    max_node_count: u32,
    dynamic_executor_enabled: bool,
    min_executors: u32,
    max_executors: u32,
) -> Result<()> {
    let body = serde_json::json!({
        "name": name,
        "nodeFamily": node_family,
        "nodeSize": node_size,
        "autoScale": {
            "enabled": auto_scale_enabled,
            "minNodeCount": min_node_count,
            "maxNodeCount": max_node_count
        },
        "dynamicExecutorAllocation": {
            "enabled": dynamic_executor_enabled,
            "minExecutors": min_executors,
            "maxExecutors": max_executors
        }
    });

    if output::dry_run_guard(cli, "spark create-pool", &body) {
        return Ok(());
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/spark/pools"),
            &body,
            false,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "spark create-pool", "Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update_pool(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    pool_id: &str,
    config: &str,
) -> Result<()> {
    let body: Value =
        serde_json::from_str(config).map_err(|e| anyhow::anyhow!("Invalid --config JSON: {e}"))?;

    if output::dry_run_guard(cli, "spark update-pool", &body) {
        return Ok(());
    }

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/spark/pools/{pool_id}"),
            &body,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "spark update-pool", "Admin"))?;
    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete_pool(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    pool_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "spark delete-pool",
        &serde_json::json!({ "workspace": workspace, "poolId": pool_id }),
    ) {
        return Ok(());
    }

    client
        .delete(&format!("/workspaces/{workspace}/spark/pools/{pool_id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "spark delete-pool", "Admin"))?;

    let obj = serde_json::json!({ "poolId": pool_id, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}
