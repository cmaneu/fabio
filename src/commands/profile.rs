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

        /// Workspace ID for private link URL routing
        #[arg(long)]
        private_link_workspace: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_link_workspace: Option<String>,
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
            // Restrict directory permissions on Unix to prevent other users from listing contents
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
            }
        }
        let json = serde_json::to_string_pretty(self)?;

        // Write atomically with restricted permissions to avoid TOCTOU window
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&path)?;
            file.write_all(json.as_bytes())?;
        }
        #[cfg(not(unix))]
        {
            fs::write(&path, json)?;
        }
        Ok(())
    }
}

fn dirs_or_home() -> PathBuf {
    home::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn execute(cli: &Cli, command: &ProfileCommand) -> Result<()> {
    match command {
        ProfileCommand::Save {
            name,
            workspace,
            capacity,
            default_output,
            private_link_workspace,
        } => save(
            cli,
            name,
            workspace.as_deref(),
            capacity.as_deref(),
            default_output.as_deref(),
            private_link_workspace.as_deref(),
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
    private_link_workspace: Option<&str>,
) -> Result<()> {
    let mut store = ProfileStore::load();

    // Merge with existing profile: only override fields that were explicitly provided.
    let existing = store.profiles.get(name).cloned().unwrap_or(Profile {
        workspace: None,
        capacity: None,
        output: None,
        private_link_workspace: None,
    });

    let profile = Profile {
        workspace: workspace.map(String::from).or(existing.workspace),
        capacity: capacity.map(String::from).or(existing.capacity),
        output: output_fmt.map(String::from).or(existing.output),
        private_link_workspace: private_link_workspace
            .map(String::from)
            .or(existing.private_link_workspace),
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
                "private_link_workspace": profile.private_link_workspace,
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
        "private_link_workspace": profile.private_link_workspace,
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
