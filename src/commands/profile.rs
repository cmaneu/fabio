use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cli::Cli;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum ProfileCommand {
    /// Save a named profile with default settings
    Save {
        /// Profile name
        #[arg(long)]
        name: String,

        /// Default workspace ID
        #[arg(short, long)]
        workspace: Option<String>,

        /// Default capacity ID
        #[arg(short, long)]
        capacity: Option<String>,

        /// Default output format for this profile
        #[arg(long = "default-output")]
        default_output: Option<String>,
    },
    /// Set the active profile
    Use {
        /// Profile name to activate
        #[arg(long)]
        name: String,
    },
    /// List all saved profiles
    List,
    /// Show details of a profile
    Show {
        /// Profile name
        #[arg(long)]
        name: String,
    },
    /// Delete a profile
    Delete {
        /// Profile name
        #[arg(long)]
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub workspace: Option<String>,
    pub capacity: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileStore {
    pub active: Option<String>,
    pub profiles: HashMap<String, Profile>,
}

impl ProfileStore {
    fn config_path() -> PathBuf {
        let home = dirs_or_home();
        home.join(".fabio").join("profiles.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;
        Ok(())
    }
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_or_else(|_| PathBuf::from("."), PathBuf::from)
}

pub fn execute(cli: &Cli, command: &ProfileCommand) -> Result<()> {
    match command {
        ProfileCommand::Save {
            name,
            workspace,
            capacity,
            default_output,
        } => save(
            cli,
            name,
            workspace.as_deref(),
            capacity.as_deref(),
            default_output.as_deref(),
        ),
        ProfileCommand::Use { name } => use_profile(cli, name),
        ProfileCommand::List => list(cli),
        ProfileCommand::Show { name } => show(cli, name),
        ProfileCommand::Delete { name } => delete(cli, name),
    }
}

fn save(
    cli: &Cli,
    name: &str,
    workspace: Option<&str>,
    capacity: Option<&str>,
    output_fmt: Option<&str>,
) -> Result<()> {
    let mut store = ProfileStore::load();
    let profile = Profile {
        workspace: workspace.map(String::from),
        capacity: capacity.map(String::from),
        output: output_fmt.map(String::from),
    };
    store.profiles.insert(name.to_string(), profile);
    store.save()?;

    let obj = serde_json::json!({
        "name": name,
        "status": "saved"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

fn use_profile(cli: &Cli, name: &str) -> Result<()> {
    let mut store = ProfileStore::load();
    if !store.profiles.contains_key(name) {
        let valid: Vec<&String> = store.profiles.keys().collect();
        return Err(FabioError::with_hint(
            ErrorCode::NotFound,
            format!("Profile '{name}' not found"),
            format!(
                "Available profiles: {}. Use 'fabio profile save --name {name}' to create it.",
                if valid.is_empty() {
                    "(none)".to_string()
                } else {
                    valid
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ),
        )
        .into());
    }
    store.active = Some(name.to_string());
    store.save()?;

    let obj = serde_json::json!({
        "name": name,
        "status": "active"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn list(cli: &Cli) -> Result<()> {
    let store = ProfileStore::load();
    let items: Vec<Value> = store
        .profiles
        .iter()
        .map(|(name, profile)| {
            serde_json::json!({
                "name": name,
                "active": store.active.as_deref() == Some(name.as_str()),
                "workspace": profile.workspace,
                "capacity": profile.capacity,
                "output": profile.output,
            })
        })
        .collect();

    output::render_list(
        cli,
        &items,
        &["name", "active", "workspace"],
        &["NAME", "ACTIVE", "WORKSPACE"],
        "name",
    );
    Ok(())
}

fn show(cli: &Cli, name: &str) -> Result<()> {
    let store = ProfileStore::load();
    let profile = store.profiles.get(name).ok_or_else(|| {
        FabioError::with_hint(
            ErrorCode::NotFound,
            format!("Profile '{name}' not found"),
            "Use 'fabio profile list' to see available profiles.".to_string(),
        )
    })?;

    let obj = serde_json::json!({
        "name": name,
        "active": store.active.as_deref() == Some(name),
        "workspace": profile.workspace,
        "capacity": profile.capacity,
        "output": profile.output,
    });
    output::render_object(cli, &obj, "name");
    Ok(())
}

fn delete(cli: &Cli, name: &str) -> Result<()> {
    if output::dry_run_guard(cli, "profile delete", &serde_json::json!({ "name": name })) {
        return Ok(());
    }

    let mut store = ProfileStore::load();
    if store.profiles.remove(name).is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::NotFound,
            format!("Profile '{name}' not found"),
            "Use 'fabio profile list' to see available profiles.".to_string(),
        )
        .into());
    }
    if store.active.as_deref() == Some(name) {
        store.active = None;
    }
    store.save()?;

    let obj = serde_json::json!({
        "name": name,
        "status": "deleted"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}
