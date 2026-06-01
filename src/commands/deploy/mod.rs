pub mod apply;
pub mod changeset;
pub mod export;
pub mod init_params;
pub mod ordering;
pub mod params;
pub mod plan;
pub mod platform;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::Subcommand;
use serde_json::json;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

use self::changeset::ChangeAction;
use self::plan::resolve_workspace;
use self::platform::get_git_metadata;

#[derive(Debug, Subcommand)]
pub enum DeployCommand {
    /// Preview what would be deployed (create/update/delete/skip)
    #[command(display_order = 1)]
    Plan {
        /// Source directory containing Fabric item definitions with .platform files
        #[arg(long)]
        source: PathBuf,

        /// Target workspace ID or name
        #[arg(short, long)]
        workspace: String,

        /// Only deploy specific item types (comma-separated)
        #[arg(long, value_delimiter = ',')]
        item_types: Option<Vec<String>>,

        /// Include delete actions for items not in source
        #[arg(long)]
        delete_orphans: bool,

        /// Don't error on unresolved logical ID references
        #[arg(long)]
        allow_unresolved: bool,

        /// Skip content-hash comparison, show all items as needing update
        #[arg(long)]
        force_all: bool,

        /// Save plan to a file for later apply
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,

        /// Parameter file for environment-aware substitution (JSON)
        #[arg(long, value_name = "FILE")]
        parameters: Option<PathBuf>,

        /// Target environment name for parameter substitution (e.g., "prod", "staging")
        #[arg(long, value_name = "NAME")]
        env: Option<String>,
    },

    /// Execute deployment (create/update/delete items)
    #[command(display_order = 2)]
    Apply {
        /// Source directory containing Fabric item definitions with .platform files
        #[arg(long, required_unless_present = "plan")]
        source: Option<PathBuf>,

        /// Target workspace ID or name
        #[arg(short, long, required_unless_present = "plan")]
        workspace: Option<String>,

        /// Apply a previously saved plan file
        #[arg(long, conflicts_with_all = ["source", "workspace"])]
        plan: Option<PathBuf>,

        /// Only deploy specific item types (comma-separated)
        #[arg(long, value_delimiter = ',')]
        item_types: Option<Vec<String>>,

        /// Delete items not in source
        #[arg(long)]
        delete_orphans: bool,

        /// Proceed despite unresolved references
        #[arg(long)]
        allow_unresolved: bool,

        /// Stop on first failure (default: continue remaining items)
        #[arg(long)]
        fail_fast: bool,

        /// Apply saved plan even if workspace state changed
        #[arg(long)]
        force: bool,

        /// Skip content-hash comparison, redeploy all items
        #[arg(long)]
        force_all: bool,

        /// Max parallel operations per type batch
        #[arg(long, default_value = "8")]
        concurrency: usize,

        /// Parameter file for environment-aware substitution (JSON)
        #[arg(long, value_name = "FILE")]
        parameters: Option<PathBuf>,

        /// Target environment name for parameter substitution (e.g., "prod", "staging")
        #[arg(long, value_name = "NAME")]
        env: Option<String>,
    },

    /// Export workspace item definitions to a local directory
    #[command(display_order = 3)]
    Export {
        /// Source workspace ID or name
        #[arg(short, long)]
        workspace: String,

        /// Output directory to write .platform items
        #[arg(long, value_name = "DIR")]
        dir: PathBuf,

        /// Only export specific item types (comma-separated)
        #[arg(long, value_delimiter = ',')]
        item_types: Option<Vec<String>>,

        /// Overwrite existing files in output directory
        #[arg(long)]
        overwrite: bool,
    },

    /// Generate a parameters.json scaffold by scanning or diffing exported definitions
    #[command(display_order = 4, name = "init-params")]
    InitParams {
        /// Source directory containing exported .platform items (e.g., dev workspace)
        #[arg(long)]
        source: PathBuf,

        /// Comparison directory to diff against (e.g., prod workspace export)
        #[arg(long, value_name = "DIR")]
        compare: Option<PathBuf>,

        /// Environment name for the source directory (used in diff mode)
        #[arg(long, default_value = "dev")]
        source_env: String,

        /// Environment name for the comparison directory (used in diff mode)
        #[arg(long, default_value = "prod")]
        compare_env: String,

        /// Output file path for generated parameters.json
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, cmd: &DeployCommand) -> Result<()> {
    match cmd {
        DeployCommand::Plan {
            source,
            workspace,
            item_types,
            delete_orphans,
            allow_unresolved,
            force_all,
            out,
            parameters,
            env,
        } => {
            execute_plan(
                cli,
                client,
                source,
                workspace,
                item_types.as_deref(),
                *delete_orphans,
                *allow_unresolved,
                *force_all,
                out.as_deref(),
                parameters.as_deref(),
                env.as_deref(),
            )
            .await
        }
        DeployCommand::Apply {
            source,
            workspace,
            plan,
            item_types,
            delete_orphans,
            allow_unresolved,
            fail_fast,
            force,
            force_all,
            concurrency,
            parameters,
            env,
        } => {
            execute_apply(
                cli,
                client,
                source.as_deref(),
                workspace.as_deref(),
                plan.as_deref(),
                item_types.as_deref(),
                *delete_orphans,
                *allow_unresolved,
                *fail_fast,
                *force,
                *force_all,
                *concurrency,
                parameters.as_deref(),
                env.as_deref(),
            )
            .await
        }
        DeployCommand::Export {
            workspace,
            dir,
            item_types,
            overwrite,
        } => {
            execute_export(
                cli,
                client,
                workspace,
                dir,
                item_types.as_deref(),
                *overwrite,
            )
            .await
        }
        DeployCommand::InitParams {
            source,
            compare,
            source_env,
            compare_env,
            out,
        } => execute_init_params(
            cli,
            source,
            compare.as_deref(),
            source_env,
            compare_env,
            out.as_deref(),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
async fn execute_plan(
    cli: &Cli,
    client: &FabricClient,
    source: &Path,
    workspace: &str,
    item_types: Option<&[String]>,
    delete_orphans: bool,
    allow_unresolved: bool,
    force_all: bool,
    out: Option<&std::path::Path>,
    parameters: Option<&std::path::Path>,
    env: Option<&str>,
) -> Result<()> {
    // Validate parameter flags
    if parameters.is_some() && env.is_none() {
        bail!("--env is required when --parameters is specified");
    }
    if env.is_some() && parameters.is_none() {
        bail!("--parameters is required when --env is specified");
    }

    // Resolve workspace
    let workspace_id = resolve_workspace(client, workspace).await?;

    // Parse source directory
    let mut source_workspace = platform::parse_source_directory(source)?;

    if source_workspace.items.is_empty() {
        bail!(
            "No items found in source directory: {}. Expected directories with .platform files.",
            source.display()
        );
    }

    // Apply parameter substitution if configured
    let param_warnings = if let (Some(param_path), Some(env_name)) = (parameters, env) {
        let parsed_params = params::parse_parameters(param_path)?;

        // Build substitution context (no deployed items yet during plan)
        let deployed_items = std::collections::HashMap::new();
        let ctx = params::SubstitutionContext {
            workspace_id: &workspace_id,
            workspace_name: None,
            deployed_items: &deployed_items,
        };

        params::apply_parameters(&mut source_workspace, &parsed_params, env_name, &ctx)?
    } else {
        Vec::new()
    };

    // Build changeset
    let deployed_items = plan::fetch_deployed_items(client, &workspace_id, item_types).await?;
    let workspace_fingerprint = plan::compute_workspace_fingerprint(&deployed_items);

    let changeset = plan::build_changeset(
        cli,
        client,
        &workspace_id,
        &source_workspace,
        &deployed_items,
        item_types,
        delete_orphans,
        force_all,
    )
    .await?;

    // Check for errors
    if changeset.has_errors() && !allow_unresolved {
        let error_msg = changeset.errors.join("\n  ");
        bail!(
            "Plan has unresolved errors:\n  {error_msg}\nUse --allow-unresolved to proceed anyway."
        );
    }

    // Save plan to file if requested
    if let Some(out_path) = out {
        let plan_json = json!({
            "version": 1,
            "workspace_id": workspace_id,
            "workspace_fingerprint": workspace_fingerprint,
            "source_path": source.display().to_string(),
            "source_git": get_git_metadata(source),
            "changeset": changeset,
            "parameters": parameters.map(|p| p.display().to_string()),
            "env": env,
        });
        let content = serde_json::to_string_pretty(&plan_json)?;
        std::fs::write(out_path, content)?;
    }

    // Render output
    let summary = changeset.summary();
    let git_meta = get_git_metadata(source);

    let mut all_warnings = changeset.warnings.clone();
    all_warnings.extend(param_warnings);

    let output_data = json!({
        "workspace_id": workspace_id,
        "source_git": git_meta,
        "changes": changeset.changes,
        "warnings": all_warnings,
        "errors": changeset.errors,
        "summary": summary,
        "parameters_applied": parameters.is_some(),
        "env": env,
    });

    output::render_object(cli, &output_data, "summary");

    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    clippy::too_many_lines
)]
async fn execute_apply(
    cli: &Cli,
    client: &FabricClient,
    source: Option<&std::path::Path>,
    workspace: Option<&str>,
    plan_file: Option<&std::path::Path>,
    item_types: Option<&[String]>,
    delete_orphans: bool,
    allow_unresolved: bool,
    fail_fast: bool,
    force: bool,
    force_all: bool,
    concurrency: usize,
    parameters: Option<&std::path::Path>,
    env: Option<&str>,
) -> Result<()> {
    // Validate parameter flags
    if parameters.is_some() && env.is_none() {
        bail!("--env is required when --parameters is specified");
    }
    if env.is_some() && parameters.is_none() {
        bail!("--parameters is required when --env is specified");
    }

    // Determine source and workspace from either direct args or plan file
    let (workspace_id, mut source_workspace, changeset) = if let Some(plan_path) = plan_file {
        // Load saved plan
        let content = std::fs::read_to_string(plan_path)?;
        let plan: serde_json::Value = serde_json::from_str(&content)?;

        let ws_id = plan
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid plan file: missing workspace_id"))?
            .to_owned();

        let cs: changeset::Changeset = serde_json::from_value(
            plan.get("changeset")
                .ok_or_else(|| anyhow::anyhow!("Invalid plan file: missing changeset"))?
                .clone(),
        )?;

        let source_path = plan
            .get("source_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid plan file: missing source_path"))?;

        let src = platform::parse_source_directory(std::path::Path::new(source_path))?;

        // Staleness check: compare workspace fingerprint from plan time vs now
        if let Some(saved_fingerprint) = plan.get("workspace_fingerprint").and_then(|v| v.as_str())
        {
            let current_items = plan::fetch_deployed_items(client, &ws_id, None).await?;
            let current_fingerprint = plan::compute_workspace_fingerprint(&current_items);

            if current_fingerprint != saved_fingerprint && !force {
                bail!(
                    "Workspace state has changed since plan was created.\n\
                     Plan fingerprint: {saved_fingerprint}\n\
                     Current fingerprint: {current_fingerprint}\n\
                     Use --force to apply anyway, or re-run `fabio deploy plan` to get a fresh plan."
                );
            }
        }

        (ws_id, src, cs)
    } else {
        // Build changeset from source + workspace
        let src_path =
            source.ok_or_else(|| anyhow::anyhow!("--source is required when not using --plan"))?;
        let ws = workspace
            .ok_or_else(|| anyhow::anyhow!("--workspace is required when not using --plan"))?;

        let workspace_id = resolve_workspace(client, ws).await?;
        let mut source_ws = platform::parse_source_directory(src_path)?;

        if source_ws.items.is_empty() {
            bail!("No items found in source directory: {}", src_path.display());
        }

        // Apply parameter substitution before building changeset
        if let (Some(param_path), Some(env_name)) = (parameters, env) {
            let parsed_params = params::parse_parameters(param_path)?;
            let deployed_items = std::collections::HashMap::new();
            let ctx = params::SubstitutionContext {
                workspace_id: &workspace_id,
                workspace_name: None,
                deployed_items: &deployed_items,
            };
            let _warnings =
                params::apply_parameters(&mut source_ws, &parsed_params, env_name, &ctx)?;
        }

        let deployed = plan::fetch_deployed_items(client, &workspace_id, item_types).await?;

        let cs = plan::build_changeset(
            cli,
            client,
            &workspace_id,
            &source_ws,
            &deployed,
            item_types,
            delete_orphans,
            force_all,
        )
        .await?;

        (workspace_id, source_ws, cs)
    };

    // Apply parameter substitution for plan-file path (re-apply to parsed source)
    if plan_file.is_some() {
        if let (Some(param_path), Some(env_name)) = (parameters, env) {
            let parsed_params = params::parse_parameters(param_path)?;
            let deployed_items = std::collections::HashMap::new();
            let ctx = params::SubstitutionContext {
                workspace_id: &workspace_id,
                workspace_name: None,
                deployed_items: &deployed_items,
            };
            let _warnings =
                params::apply_parameters(&mut source_workspace, &parsed_params, env_name, &ctx)?;
        }
    }

    // Check for errors
    if changeset.has_errors() && !allow_unresolved {
        let error_msg = changeset.errors.join("\n  ");
        bail!(
            "Deployment blocked by unresolved errors:\n  {error_msg}\nUse --allow-unresolved to proceed."
        );
    }

    // Check if there's anything to do
    if !changeset.has_changes() {
        let output_data = json!({
            "status": "no_changes",
            "message": "Workspace is already in sync with source.",
            "summary": changeset.summary(),
        });
        output::render_object(cli, &output_data, "status");
        return Ok(());
    }

    // Dry-run guard
    if cli.dry_run {
        let summary = changeset.summary();
        let output_data = json!({
            "status": "dry_run",
            "message": format!(
                "Would create {}, update {}, delete {}, skip {}",
                summary.create, summary.update, summary.delete, summary.skip
            ),
            "changes": changeset.changes.iter()
                .filter(|c| c.action != ChangeAction::Skip)
                .collect::<Vec<_>>(),
            "summary": summary,
        });
        output::render_object(cli, &output_data, "status");
        return Ok(());
    }

    // Execute the changeset
    let result = apply::execute_changeset(
        cli,
        client,
        &workspace_id,
        &changeset,
        &source_workspace,
        concurrency,
        fail_fast,
    )
    .await?;

    // Render result
    let output_data = json!({
        "status": if result.failed.is_empty() { "succeeded" } else { "partial_failure" },
        "succeeded": result.succeeded.len(),
        "failed": result.failed.len(),
        "skipped": result.skipped.len(),
        "duration_ms": result.duration_ms,
        "failures": result.failed,
    });

    output::render_object(cli, &output_data, "status");

    if !result.failed.is_empty() {
        bail!("Deployment completed with {} failures", result.failed.len());
    }

    Ok(())
}

async fn execute_export(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    output: &std::path::Path,
    item_types: Option<&[String]>,
    overwrite: bool,
) -> Result<()> {
    let workspace_id = resolve_workspace(client, workspace).await?;

    let result =
        export::export_workspace(cli, client, &workspace_id, output, item_types, overwrite).await?;

    let output_data = json!({
        "status": if cli.dry_run { "dry_run" } else { "exported" },
        "workspace_id": workspace_id,
        "output_dir": output.display().to_string(),
        "total_items": result.total_items,
        "exported": result.exported,
        "skipped": result.skipped,
    });

    output::render_object(cli, &output_data, "status");

    Ok(())
}

fn execute_init_params(
    cli: &Cli,
    source: &Path,
    compare: Option<&Path>,
    source_env: &str,
    compare_env: &str,
    out: Option<&Path>,
) -> Result<()> {
    let result = if let Some(compare_dir) = compare {
        init_params::diff_for_parameters(source, compare_dir, source_env, compare_env)?
    } else {
        init_params::scan_for_candidates(source)?
    };

    // Write to file if --out specified
    if let Some(out_path) = out {
        let content = serde_json::to_string_pretty(&result.parameters_json)?;
        std::fs::write(out_path, &content)?;
    }

    // Render output
    let output_data = json!({
        "status": "generated",
        "mode": result.summary.mode,
        "source_items": result.summary.source_items,
        "compare_items": result.summary.compare_items,
        "rules_generated": result.summary.rules_generated,
        "guids_found": result.summary.guids_found,
        "parameters": result.parameters_json,
        "output_file": out.map(|p| p.display().to_string()),
    });

    output::render_object(cli, &output_data, "status");

    Ok(())
}
