use anyhow::{Result, bail};
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum GitCommand {
    /// Show workspace Git status (changes, conflicts)
    Status {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// Commit workspace changes to the connected remote branch
    Commit {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Commit message (max 300 characters)
        #[arg(short, long)]
        message: Option<String>,

        /// Commit all pending changes
        #[arg(long, conflicts_with = "items")]
        all: bool,

        /// Selective commit: comma-separated item object IDs
        #[arg(long, value_delimiter = ',', conflicts_with = "all")]
        items: Option<Vec<String>>,

        /// Override workspace head (auto-fetched from status if omitted)
        #[arg(long, hide = true)]
        workspace_head: Option<String>,

        /// Wait for the operation to complete
        #[arg(long)]
        wait: bool,

        /// Timeout in seconds when --wait is used (default: 120)
        #[arg(long, default_value = "120")]
        timeout: u64,
    },
    /// Pull remote changes into the workspace (update from Git)
    Pull {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Conflict resolution policy
        #[arg(long, value_parser = ["prefer-remote", "prefer-workspace"])]
        conflict_resolution: Option<String>,

        /// Allow overriding workspace items with incoming changes
        #[arg(long)]
        allow_override: bool,

        /// Override workspace head (auto-fetched from status if omitted)
        #[arg(long, hide = true)]
        workspace_head: Option<String>,

        /// Override remote commit hash (auto-fetched from status if omitted)
        #[arg(long, hide = true)]
        remote_commit_hash: Option<String>,

        /// Wait for the operation to complete
        #[arg(long)]
        wait: bool,

        /// Timeout in seconds when --wait is used (default: 120)
        #[arg(long, default_value = "120")]
        timeout: u64,
    },
    /// Connect a workspace to a Git repository
    Connect {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Git provider type
        #[arg(long, value_parser = ["azure-devops", "github"])]
        provider: String,

        /// Repository name
        #[arg(long)]
        repo: String,

        /// Branch name
        #[arg(long)]
        branch: String,

        /// Organization name (Azure DevOps only)
        #[arg(long)]
        org: Option<String>,

        /// Project name (Azure DevOps only)
        #[arg(long)]
        project: Option<String>,

        /// Owner name (GitHub only)
        #[arg(long)]
        owner: Option<String>,

        /// Relative directory path within the repo
        #[arg(long)]
        directory: Option<String>,

        /// Custom domain for GitHub Enterprise (ghe.com)
        #[arg(long)]
        custom_domain: Option<String>,

        /// Connection ID for configured credentials
        #[arg(long)]
        connection_id: Option<String>,
    },
    /// Disconnect a workspace from Git
    Disconnect {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// Initialize a workspace Git connection (required after connect)
    Init {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Initialization strategy when both sides have content
        #[arg(long, value_parser = ["none", "prefer-remote", "prefer-workspace"])]
        strategy: Option<String>,

        /// Wait for the operation to complete
        #[arg(long)]
        wait: bool,

        /// Timeout in seconds when --wait is used (default: 120)
        #[arg(long, default_value = "120")]
        timeout: u64,
    },
    /// Switch to a different branch (disconnect + connect + init)
    #[command(visible_alias = "switch")]
    Checkout {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Target branch name
        #[arg(long)]
        branch: String,

        /// Initialization strategy when both sides have content
        #[arg(long, value_parser = ["none", "prefer-remote", "prefer-workspace"])]
        strategy: Option<String>,

        /// Wait for initialization to complete
        #[arg(long)]
        wait: bool,

        /// Timeout in seconds when --wait is used (default: 120)
        #[arg(long, default_value = "120")]
        timeout: u64,
    },
    /// Show or manage Git connection and credentials
    #[command(subcommand)]
    Connection(ConnectionCommand),
    /// Manage Git credentials
    #[command(subcommand)]
    Credentials(CredentialsCommand),
}

#[derive(Debug, Subcommand)]
pub enum ConnectionCommand {
    /// Show Git connection details for the workspace
    Show {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum CredentialsCommand {
    /// Show your Git credentials configuration
    Show {
        /// Workspace ID
        #[arg(long)]
        workspace: String,
    },
    /// Update your Git credentials configuration
    Update {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Credentials source
        #[arg(long, value_parser = ["automatic", "configured-connection", "none"])]
        source: String,

        /// Connection ID (required when source is configured-connection)
        #[arg(long)]
        connection_id: Option<String>,
    },
}

#[allow(clippy::too_many_lines)]
pub async fn execute(cli: &Cli, client: &FabricClient, command: &GitCommand) -> Result<()> {
    match command {
        GitCommand::Status { workspace } => status(cli, client, workspace).await,
        GitCommand::Commit {
            workspace,
            message,
            all,
            items,
            workspace_head,
            wait,
            timeout,
        } => {
            commit(
                cli,
                client,
                workspace,
                message.as_deref(),
                *all,
                items.as_deref(),
                workspace_head.as_deref(),
                *wait,
                *timeout,
            )
            .await
        }
        GitCommand::Pull {
            workspace,
            conflict_resolution,
            allow_override,
            workspace_head,
            remote_commit_hash,
            wait,
            timeout,
        } => {
            pull(
                cli,
                client,
                workspace,
                conflict_resolution.as_deref(),
                *allow_override,
                workspace_head.as_deref(),
                remote_commit_hash.as_deref(),
                *wait,
                *timeout,
            )
            .await
        }
        GitCommand::Connect {
            workspace,
            provider,
            repo,
            branch,
            org,
            project,
            owner,
            directory,
            custom_domain,
            connection_id,
        } => {
            connect(
                cli,
                client,
                workspace,
                provider,
                repo,
                branch,
                org.as_deref(),
                project.as_deref(),
                owner.as_deref(),
                directory.as_deref(),
                custom_domain.as_deref(),
                connection_id.as_deref(),
            )
            .await
        }
        GitCommand::Disconnect { workspace } => disconnect(cli, client, workspace).await,
        GitCommand::Init {
            workspace,
            strategy,
            wait,
            timeout,
        } => init(cli, client, workspace, strategy.as_deref(), *wait, *timeout).await,
        GitCommand::Checkout {
            workspace,
            branch,
            strategy,
            wait,
            timeout,
        } => {
            checkout(
                cli,
                client,
                workspace,
                branch,
                strategy.as_deref(),
                *wait,
                *timeout,
            )
            .await
        }
        GitCommand::Connection(sub) => match sub {
            ConnectionCommand::Show { workspace } => connection_show(cli, client, workspace).await,
        },
        GitCommand::Credentials(sub) => match sub {
            CredentialsCommand::Show { workspace } => {
                credentials_show(cli, client, workspace).await
            }
            CredentialsCommand::Update {
                workspace,
                source,
                connection_id,
            } => credentials_update(cli, client, workspace, source, connection_id.as_deref()).await,
        },
    }
}

async fn status(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get_with_lro(&format!("/workspaces/{workspace}/git/status"))
        .await?;

    let changes = data
        .get("changes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if changes.is_empty() {
        output::render_object(cli, &data, "status");
    } else {
        output::render_list(
            cli,
            &changes,
            &[
                "itemMetadata.displayName",
                "itemMetadata.itemType",
                "workspaceChange",
                "remoteChange",
                "conflictType",
            ],
            &["NAME", "TYPE", "WORKSPACE", "REMOTE", "CONFLICT"],
            "itemMetadata.displayName",
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn commit(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    message: Option<&str>,
    all: bool,
    items: Option<&[String]>,
    workspace_head: Option<&str>,
    wait: bool,
    timeout: u64,
) -> Result<()> {
    if !all && items.is_none() {
        bail!("Specify --all to commit all changes, or --items for selective commit");
    }

    // Auto-fetch workspace head if not provided
    let head = if let Some(h) = workspace_head {
        h.to_string()
    } else {
        let status = client
            .get_with_lro(&format!("/workspaces/{workspace}/git/status"))
            .await?;
        status
            .get("workspaceHead")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Could not determine workspaceHead from status"))?
            .to_string()
    };

    let mode = if all { "All" } else { "Selective" };
    let mut body = serde_json::json!({
        "mode": mode,
        "workspaceHead": head,
    });

    if let Some(msg) = message {
        body["comment"] = Value::String(msg.to_string());
    }

    if let Some(item_ids) = items {
        let item_objs: Vec<Value> = item_ids
            .iter()
            .map(|id| serde_json::json!({"objectId": id}))
            .collect();
        body["items"] = Value::Array(item_objs);
    }

    let data = client
        .post_with_timeout(
            &format!("/workspaces/{workspace}/git/commitToGit"),
            &body,
            wait,
            timeout,
        )
        .await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn pull(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    conflict_resolution: Option<&str>,
    allow_override: bool,
    workspace_head: Option<&str>,
    remote_commit_hash: Option<&str>,
    wait: bool,
    timeout: u64,
) -> Result<()> {
    // Auto-fetch hashes from status if not provided
    let (head, remote_hash) = if let (Some(h), Some(r)) = (workspace_head, remote_commit_hash) {
        (h.to_string(), r.to_string())
    } else {
        let status = client
            .get_with_lro(&format!("/workspaces/{workspace}/git/status"))
            .await?;
        let h = workspace_head
            .map(String::from)
            .or_else(|| {
                status
                    .get("workspaceHead")
                    .and_then(Value::as_str)
                    .map(String::from)
            })
            .ok_or_else(|| anyhow::anyhow!("Could not determine workspaceHead from status"))?;
        let r = remote_commit_hash
            .map(String::from)
            .or_else(|| {
                status
                    .get("remoteCommitHash")
                    .and_then(Value::as_str)
                    .map(String::from)
            })
            .ok_or_else(|| anyhow::anyhow!("Could not determine remoteCommitHash from status"))?;
        (h, r)
    };

    let mut body = serde_json::json!({
        "remoteCommitHash": remote_hash,
        "workspaceHead": head,
    });

    if let Some(policy) = conflict_resolution {
        let api_policy = match policy {
            "prefer-remote" => "PreferRemote",
            "prefer-workspace" => "PreferWorkspace",
            _ => policy,
        };
        body["conflictResolution"] = serde_json::json!({
            "conflictResolutionType": "Workspace",
            "conflictResolutionPolicy": api_policy,
        });
    }

    if allow_override {
        body["options"] = serde_json::json!({
            "allowOverrideItems": true,
        });
    }

    let data = client
        .post_with_timeout(
            &format!("/workspaces/{workspace}/git/updateFromGit"),
            &body,
            wait,
            timeout,
        )
        .await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn connect(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    provider: &str,
    repo: &str,
    branch: &str,
    org: Option<&str>,
    project: Option<&str>,
    owner: Option<&str>,
    directory: Option<&str>,
    custom_domain: Option<&str>,
    connection_id: Option<&str>,
) -> Result<()> {
    let git_provider_details = match provider {
        "azure-devops" => {
            let org_name =
                org.ok_or_else(|| anyhow::anyhow!("--org is required for Azure DevOps provider"))?;
            let project_name = project.ok_or_else(|| {
                anyhow::anyhow!("--project is required for Azure DevOps provider")
            })?;
            let mut details = serde_json::json!({
                "gitProviderType": "AzureDevOps",
                "organizationName": org_name,
                "projectName": project_name,
                "repositoryName": repo,
                "branchName": branch,
            });
            if let Some(dir) = directory {
                details["directoryName"] = Value::String(dir.to_string());
            }
            details
        }
        "github" => {
            let owner_name =
                owner.ok_or_else(|| anyhow::anyhow!("--owner is required for GitHub provider"))?;
            let mut details = serde_json::json!({
                "gitProviderType": "GitHub",
                "ownerName": owner_name,
                "repositoryName": repo,
                "branchName": branch,
            });
            if let Some(dir) = directory {
                details["directoryName"] = Value::String(dir.to_string());
            }
            if let Some(domain) = custom_domain {
                details["customDomainName"] = Value::String(domain.to_string());
            }
            details
        }
        _ => bail!("Unsupported provider: {provider}. Use 'azure-devops' or 'github'."),
    };

    let mut body = serde_json::json!({
        "gitProviderDetails": git_provider_details,
    });

    if let Some(conn_id) = connection_id {
        body["myGitCredentials"] = serde_json::json!({
            "source": "ConfiguredConnection",
            "connectionId": conn_id,
        });
    }

    let data = client
        .post(
            &format!("/workspaces/{workspace}/git/connect"),
            &body,
            false,
        )
        .await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

async fn disconnect(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/git/disconnect"),
            &serde_json::json!({}),
            false,
        )
        .await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

async fn init(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    strategy: Option<&str>,
    wait: bool,
    timeout: u64,
) -> Result<()> {
    let body = strategy.map_or_else(
        || serde_json::json!({}),
        |s| {
            let api_strategy = match s {
                "prefer-remote" => "PreferRemote",
                "prefer-workspace" => "PreferWorkspace",
                "none" => "None",
                _ => s,
            };
            serde_json::json!({"initializationStrategy": api_strategy})
        },
    );

    let data = client
        .post_with_timeout(
            &format!("/workspaces/{workspace}/git/initializeConnection"),
            &body,
            wait,
            timeout,
        )
        .await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn checkout(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    branch: &str,
    strategy: Option<&str>,
    wait: bool,
    timeout: u64,
) -> Result<()> {
    // Step 1: Get current connection details to preserve provider config
    let connection = client
        .get(&format!("/workspaces/{workspace}/git/connection"))
        .await?;

    let provider_details = connection
        .get("gitProviderDetails")
        .ok_or_else(|| anyhow::anyhow!("Workspace is not connected to Git"))?;

    // Step 2: Disconnect from current branch
    client
        .post(
            &format!("/workspaces/{workspace}/git/disconnect"),
            &serde_json::json!({}),
            false,
        )
        .await?;

    // Step 3: Reconnect with the new branch
    let mut new_provider_details = provider_details.clone();
    new_provider_details["branchName"] = Value::String(branch.to_string());

    let connect_body = serde_json::json!({
        "gitProviderDetails": new_provider_details,
    });

    client
        .post(
            &format!("/workspaces/{workspace}/git/connect"),
            &connect_body,
            false,
        )
        .await?;

    // Step 4: Initialize the connection
    let init_body = strategy.map_or_else(
        || serde_json::json!({}),
        |s| {
            let api_strategy = match s {
                "prefer-remote" => "PreferRemote",
                "prefer-workspace" => "PreferWorkspace",
                "none" => "None",
                _ => s,
            };
            serde_json::json!({"initializationStrategy": api_strategy})
        },
    );

    let data = client
        .post_with_timeout(
            &format!("/workspaces/{workspace}/git/initializeConnection"),
            &init_body,
            wait,
            timeout,
        )
        .await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

async fn connection_show(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/git/connection"))
        .await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

async fn credentials_show(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/git/myGitCredentials"))
        .await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

async fn credentials_update(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    source: &str,
    connection_id: Option<&str>,
) -> Result<()> {
    let body = match source {
        "automatic" => serde_json::json!({"source": "Automatic"}),
        "none" => serde_json::json!({"source": "None"}),
        "configured-connection" => {
            let conn_id = connection_id.ok_or_else(|| {
                anyhow::anyhow!(
                    "--connection-id is required when source is 'configured-connection'"
                )
            })?;
            serde_json::json!({
                "source": "ConfiguredConnection",
                "connectionId": conn_id,
            })
        }
        _ => bail!(
            "Unsupported source: {source}. Use 'automatic', 'configured-connection', or 'none'."
        ),
    };

    let data = client
        .patch(
            &format!("/workspaces/{workspace}/git/myGitCredentials"),
            &body,
        )
        .await?;

    output::render_object(cli, &data, "status");
    Ok(())
}
