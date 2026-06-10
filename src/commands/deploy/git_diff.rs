//! Git diff-based selective deployment.
//!
//! Uses `git diff --name-status` to determine which items changed between refs,
//! enabling partial deployments that only include modified items.

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// Result of git diff analysis: sets of changed and deleted item identifiers.
#[derive(Debug, Clone)]
pub struct GitDiffResult {
    /// Items that were added or modified (as `(type, displayName)` tuples).
    pub changed: HashSet<(String, String)>,
    /// Items that were deleted (as `(type, displayName)` tuples).
    pub deleted: HashSet<(String, String)>,
}

/// Analyze git diff between the current state and a reference to find changed items.
///
/// Scans the diff for files that belong to item directories (contain `.platform`).
/// Returns sets of changed and deleted items by (type, name).
///
/// # Arguments
/// * `source_dir` - The source directory containing `.platform` item directories
/// * `git_ref` - The git reference to compare against (e.g., "HEAD~1", "main", a commit SHA)
pub fn get_changed_items(source_dir: &Path, git_ref: &str) -> Result<GitDiffResult> {
    let mut changed = HashSet::new();
    let mut deleted = HashSet::new();

    // Find git root
    let git_root = find_git_root(source_dir)?;

    // Get diff output
    let output = Command::new("git")
        .args(["diff", "--name-status", git_ref])
        .current_dir(&git_root)
        .output()
        .with_context(|| format!("Failed to run git diff against ref '{git_ref}'"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff failed: {stderr}");
    }

    let diff_output = String::from_utf8_lossy(&output.stdout);

    // Compute relative path from git root to source_dir
    let source_rel = source_dir
        .canonicalize()
        .unwrap_or_else(|_| source_dir.to_path_buf());
    let git_root_canon = git_root.canonicalize().unwrap_or_else(|_| git_root.clone());

    for line in diff_output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            continue;
        }

        let status = parts[0];
        // For renames (R###), use the new path
        let file_path = if status.starts_with('R') && parts.len() >= 3 {
            parts[2]
        } else {
            parts[1]
        };

        let abs_path = git_root_canon.join(file_path);

        // Check if this file is within our source directory
        if !abs_path.starts_with(&source_rel) {
            continue;
        }

        // Get relative path from source_dir
        let rel_to_source = match abs_path.strip_prefix(&source_rel) {
            Ok(p) => p.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        // Determine the item directory (first path component)
        let item_dir_name = match rel_to_source.split('/').next() {
            Some(name) if !name.is_empty() => name.to_owned(),
            _ => continue,
        };

        if status.starts_with('D') {
            // For deleted files, try to extract type from the directory name pattern
            // "Name.ItemType" → ("ItemType", "Name")
            if let Some((name, item_type)) = parse_item_dir_name(&item_dir_name) {
                // If the .platform itself was deleted, the whole item was deleted
                if rel_to_source.ends_with(".platform") {
                    deleted.insert((item_type, name));
                } else {
                    // A file within the item was deleted — item was modified
                    changed.insert((item_type, name));
                }
            }
        } else {
            // Added, Modified, or Renamed — item was changed
            if let Some((name, item_type)) = parse_item_dir_name(&item_dir_name) {
                changed.insert((item_type, name));
            }
        }
    }

    // Items in both changed and deleted should be treated as changed (rename case)
    for item in &changed {
        deleted.remove(item);
    }

    Ok(GitDiffResult { changed, deleted })
}

/// Parse an item directory name in "DisplayName.ItemType" format.
/// Returns `(display_name, item_type)` or None if the format is invalid.
fn parse_item_dir_name(dir_name: &str) -> Option<(String, String)> {
    let last_dot = dir_name.rfind('.')?;
    if last_dot == 0 || last_dot == dir_name.len() - 1 {
        return None;
    }
    let name = &dir_name[..last_dot];
    let item_type = &dir_name[last_dot + 1..];

    // Basic sanity: item_type should be PascalCase (starts with uppercase letter)
    if item_type
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
    {
        Some((name.to_owned(), item_type.to_owned()))
    } else {
        None
    }
}

/// Find the git root directory by walking up from the given path.
fn find_git_root(start: &Path) -> Result<std::path::PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(start)
        .output()
        .context("Failed to find git root (is this a git repository?)")?;

    if !output.status.success() {
        anyhow::bail!("Not a git repository: {}", start.display());
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok(std::path::PathBuf::from(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_item_dir_name_valid() {
        let result = parse_item_dir_name("MyNotebook.Notebook");
        assert_eq!(
            result,
            Some(("MyNotebook".to_owned(), "Notebook".to_owned()))
        );
    }

    #[test]
    fn test_parse_item_dir_name_with_dots_in_name() {
        let result = parse_item_dir_name("My.Complex.Name.DataPipeline");
        assert_eq!(
            result,
            Some(("My.Complex.Name".to_owned(), "DataPipeline".to_owned()))
        );
    }

    #[test]
    fn test_parse_item_dir_name_invalid_no_dot() {
        assert_eq!(parse_item_dir_name("NoDotHere"), None);
    }

    #[test]
    fn test_parse_item_dir_name_invalid_lowercase_type() {
        // item types must start with uppercase
        assert_eq!(parse_item_dir_name("name.lowercase"), None);
    }

    #[test]
    fn test_parse_item_dir_name_starts_with_dot() {
        assert_eq!(parse_item_dir_name(".hidden"), None);
    }
}
