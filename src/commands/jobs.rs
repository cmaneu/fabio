use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Subcommand;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cli::Cli;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum JobsCommand {
    /// List recent jobs from the local ledger
    List {
        /// Filter by status (e.g., running, completed, failed)
        #[arg(long)]
        status: Option<String>,
    },
    /// Get details of a specific job
    Get {
        /// Job ID
        #[arg(long)]
        id: String,
    },
    /// Remove completed/failed jobs from the ledger
    Prune {
        /// Remove all jobs including currently running ones
        #[arg(long)]
        include_running: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEntry {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub workspace: String,
    pub item_id: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl JobEntry {
    pub fn new(id: &str, kind: &str, workspace: &str, item_id: &str) -> Self {
        Self {
            id: id.to_string(),
            kind: kind.to_string(),
            status: "running".to_string(),
            workspace: workspace.to_string(),
            item_id: item_id.to_string(),
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
            duration_secs: None,
            error: None,
        }
    }
}

pub struct JobLedger;

impl JobLedger {
    fn ledger_path() -> PathBuf {
        let home = home::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".fabio").join("jobs.jsonl")
    }

    pub fn append(entry: &JobEntry) -> Result<()> {
        let path = Self::ledger_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
            }
        }
        // Open with restricted permissions atomically to avoid TOCTOU
        #[cfg(unix)]
        let mut file = {
            use std::os::unix::fs::OpenOptionsExt;
            OpenOptions::new()
                .create(true)
                .append(true)
                .mode(0o600)
                .open(&path)?
        };
        #[cfg(not(unix))]
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;

        // Advisory lock to prevent concurrent append corruption
        file.lock_exclusive()?;
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{line}")?;
        // Lock released on drop
        Ok(())
    }

    pub fn update(id: &str, status: &str, error: Option<&str>) -> Result<()> {
        let path = Self::ledger_path();
        // Acquire exclusive lock for the read-modify-write operation
        #[cfg(unix)]
        let lock_file = {
            use std::os::unix::fs::OpenOptionsExt;
            OpenOptions::new()
                .create(true)
                .append(true)
                .mode(0o600)
                .open(&path)?
        };
        #[cfg(not(unix))]
        let lock_file = OpenOptions::new().create(true).append(true).open(&path)?;
        lock_file.lock_exclusive()?;

        let entries = Self::read_all()?;
        let updated: Vec<JobEntry> = entries
            .into_iter()
            .map(|mut e| {
                if e.id == id {
                    e.status = status.to_string();
                    e.completed_at = Some(Utc::now().to_rfc3339());
                    if let (Ok(start), Some(end_str)) =
                        (DateTime::parse_from_rfc3339(&e.started_at), &e.completed_at)
                        && let Ok(end) = DateTime::parse_from_rfc3339(end_str)
                    {
                        e.duration_secs = Some(
                            end.signed_duration_since(start)
                                .num_seconds()
                                .unsigned_abs(),
                        );
                    }
                    if let Some(err) = error {
                        e.error = Some(err.to_string());
                    }
                }
                e
            })
            .collect();
        Self::write_all(&updated)?;

        // Lock released on drop
        drop(lock_file);
        Ok(())
    }

    fn read_all() -> Result<Vec<JobEntry>> {
        let path = Self::ledger_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let entries: Vec<JobEntry> = reader
            .lines()
            .map_while(Result::ok)
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();
        Ok(entries)
    }

    fn write_all(entries: &[JobEntry]) -> Result<()> {
        let path = Self::ledger_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
            }
        }
        // Create with restricted permissions atomically to avoid TOCTOU
        #[cfg(unix)]
        let mut file = {
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&path)?
        };
        #[cfg(not(unix))]
        let mut file = fs::File::create(&path)?;

        for entry in entries {
            let line = serde_json::to_string(entry)?;
            writeln!(file, "{line}")?;
        }
        Ok(())
    }
}

pub fn execute(cli: &Cli, command: &JobsCommand) -> Result<()> {
    match command {
        JobsCommand::List { status } => list(cli, status.as_deref()),
        JobsCommand::Get { id } => get(cli, id),
        JobsCommand::Prune { include_running } => prune(cli, *include_running),
    }
}

fn list(cli: &Cli, status_filter: Option<&str>) -> Result<()> {
    let entries = JobLedger::read_all()?;
    // Use global --limit with a default of 20 for jobs
    let limit = cli.limit.unwrap_or(20);
    let filtered: Vec<Value> = entries
        .iter()
        .rev()
        .filter(|e| status_filter.is_none_or(|s| e.status == s))
        .take(limit)
        .map(|e| serde_json::to_value(e).unwrap_or_default())
        .collect();

    output::render_list(
        cli,
        &filtered,
        &["id", "kind", "status", "started_at", "duration_secs"],
        &["JOB_ID", "KIND", "STATUS", "STARTED", "DURATION"],
        "id",
    );
    Ok(())
}

fn get(cli: &Cli, id: &str) -> Result<()> {
    let entries = JobLedger::read_all()?;
    let entry = entries.iter().find(|e| e.id == id).ok_or_else(|| {
        FabioError::with_hint(
            ErrorCode::NotFound,
            format!("Job '{id}' not found in local ledger"),
            "Use 'fabio jobs list' to see available jobs.".to_string(),
        )
    })?;

    let obj = serde_json::to_value(entry)?;
    output::render_object(cli, &obj, "id");
    Ok(())
}

fn prune(cli: &Cli, include_running: bool) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "jobs prune",
        &serde_json::json!({ "include_running": include_running }),
    ) {
        return Ok(());
    }

    let entries = JobLedger::read_all()?;
    let before_count = entries.len();
    let remaining: Vec<JobEntry> = if include_running {
        Vec::new()
    } else {
        entries
            .into_iter()
            .filter(|e| e.status == "running")
            .collect()
    };
    let pruned = before_count - remaining.len();
    JobLedger::write_all(&remaining)?;

    let obj = serde_json::json!({
        "pruned": pruned,
        "remaining": remaining.len(),
        "status": "pruned"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}
