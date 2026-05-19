use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cli::Cli;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum FeedbackCommand {
    /// Record feedback about CLI friction or issues
    Send {
        /// Feedback message
        message: String,
    },
    /// List recorded feedback entries
    List,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeedbackEntry {
    timestamp: String,
    message: String,
}

fn feedback_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_or_else(|_| PathBuf::from("."), PathBuf::from);
    home.join(".fabio").join("feedback.jsonl")
}

pub fn execute(cli: &Cli, command: &FeedbackCommand) -> Result<()> {
    match command {
        FeedbackCommand::Send { message } => send(cli, message),
        FeedbackCommand::List => list(cli),
    }
}

fn send(cli: &Cli, message: &str) -> Result<()> {
    let path = feedback_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let entry = FeedbackEntry {
        timestamp: Utc::now().to_rfc3339(),
        message: message.to_string(),
    };

    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    let line = serde_json::to_string(&entry)?;
    writeln!(file, "{line}")?;

    // Optionally send upstream if endpoint configured
    let upstream_sent = std::env::var("FABIO_FEEDBACK_ENDPOINT").ok();

    let obj = serde_json::json!({
        "status": "recorded",
        "upstream": upstream_sent,
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn list(cli: &Cli) -> Result<()> {
    let path = feedback_path();
    let entries: Vec<Value> = if path.exists() {
        fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    } else {
        Vec::new()
    };

    output::render_list(
        cli,
        &entries,
        &["timestamp", "message"],
        &["TIMESTAMP", "MESSAGE"],
        "message",
    );
    Ok(())
}
