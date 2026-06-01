use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Metadata from a `.platform` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMetadata {
    pub item_type: String,
    pub display_name: String,
    pub logical_id: Option<String>,
    pub description: Option<String>,
    /// Definition format required by the Fabric API (e.g., "ipynb" for Notebooks).
    pub definition_format: Option<String>,
}

/// A single definition part (file) belonging to an item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionPart {
    /// Relative path within the item directory (e.g., "notebook-content.py").
    pub path: String,
    /// Base64-encoded payload.
    pub payload: String,
    /// Payload type (always "`InlineBase64`" for Fabric API).
    pub payload_type: String,
}

/// A fully parsed item from a `.platform` source directory.
#[derive(Debug, Clone)]
pub struct SourceItem {
    /// Metadata from `.platform`.
    pub metadata: PlatformMetadata,
    /// Definition parts (files excluding `.platform` and `creationPayload.json`).
    pub parts: Vec<DefinitionPart>,
    /// SHA256 hash of all definition parts (for change detection).
    pub content_hash: String,
    /// Optional creation payload (from `creationPayload.json`).
    /// Included in the creation request body as the `creationPayload` field.
    pub creation_payload: Option<serde_json::Value>,
    /// Path to the item directory on disk.
    #[allow(dead_code)]
    pub source_path: PathBuf,
}

/// All items discovered from a source directory.
#[derive(Debug)]
pub struct SourceWorkspace {
    pub items: Vec<SourceItem>,
    /// Map from `logical_id` → index into items vec.
    #[allow(dead_code)]
    pub logical_id_index: HashMap<String, usize>,
    /// Map from (type, name) → index into items vec.
    #[allow(dead_code)]
    pub type_name_index: HashMap<(String, String), usize>,
}

/// Parse a `.platform` JSON file and extract metadata.
fn parse_platform_file(path: &Path) -> Result<PlatformMetadata> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read .platform file: {}", path.display()))?;

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Invalid JSON in .platform file: {}", path.display()))?;

    let metadata = parsed
        .get("metadata")
        .with_context(|| format!("Missing 'metadata' in .platform: {}", path.display()))?;

    let item_type = metadata
        .get("type")
        .and_then(|v| v.as_str())
        .with_context(|| format!("Missing 'metadata.type' in .platform: {}", path.display()))?
        .to_owned();

    let display_name = metadata
        .get("displayName")
        .and_then(|v| v.as_str())
        .with_context(|| {
            format!(
                "Missing 'metadata.displayName' in .platform: {}",
                path.display()
            )
        })?
        .to_owned();

    let description = metadata
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let logical_id = parsed
        .get("config")
        .and_then(|c| c.get("logicalId"))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let definition_format = parsed
        .get("config")
        .and_then(|c| c.get("definitionFormat"))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    Ok(PlatformMetadata {
        item_type,
        display_name,
        logical_id,
        description,
        definition_format,
    })
}

/// Compute a deterministic content hash over all definition parts.
///
/// The hash is computed over sorted (path, payload) pairs to ensure
/// consistency regardless of filesystem ordering.
fn compute_content_hash(parts: &[DefinitionPart]) -> String {
    let mut hasher = Sha256::new();

    // Sort by path for deterministic ordering
    let mut sorted: Vec<(&str, &str)> = parts
        .iter()
        .map(|p| (p.path.as_str(), p.payload.as_str()))
        .collect();
    sorted.sort_by_key(|(path, _)| *path);

    for (path, payload) in sorted {
        hasher.update(path.as_bytes());
        hasher.update(b"\x00"); // separator
        hasher.update(payload.as_bytes());
        hasher.update(b"\x00");
    }

    let hash = hasher.finalize();
    let hex = hash.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });
    format!("sha256:{hex}")
}

/// Parse a source directory containing Fabric item folders with `.platform` files.
///
/// Expected directory structure:
/// ```text
/// source_dir/
/// ├── MyNotebook.Notebook/
/// │   ├── .platform
/// │   └── notebook-content.py
/// ├── MyPipeline.DataPipeline/
/// │   ├── .platform
/// │   └── pipeline-content.json
/// ```
pub fn parse_source_directory(source_dir: &Path) -> Result<SourceWorkspace> {
    if !source_dir.is_dir() {
        bail!("Source directory does not exist: {}", source_dir.display());
    }

    let mut items = Vec::new();
    let mut logical_id_index = HashMap::new();
    let mut type_name_index = HashMap::new();

    // Walk top-level entries looking for directories with .platform files
    let entries = std::fs::read_dir(source_dir)
        .with_context(|| format!("Failed to read source directory: {}", source_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let platform_path = path.join(".platform");
        if !platform_path.exists() {
            continue;
        }

        let metadata = parse_platform_file(&platform_path)?;

        // Read all non-.platform, non-creationPayload files as definition parts
        let parts = read_definition_parts(&path)?;
        let content_hash = compute_content_hash(&parts);

        // Read optional creationPayload.json
        let creation_payload_path = path.join("creationPayload.json");
        let creation_payload = if creation_payload_path.exists() {
            let content = std::fs::read_to_string(&creation_payload_path).with_context(|| {
                format!(
                    "Failed to read creationPayload.json: {}",
                    creation_payload_path.display()
                )
            })?;
            let parsed: serde_json::Value = serde_json::from_str(&content).with_context(|| {
                format!(
                    "Invalid JSON in creationPayload.json: {}",
                    creation_payload_path.display()
                )
            })?;
            Some(parsed)
        } else {
            None
        };

        let idx = items.len();

        if let Some(ref lid) = metadata.logical_id {
            logical_id_index.insert(lid.clone(), idx);
        }

        type_name_index.insert(
            (metadata.item_type.clone(), metadata.display_name.clone()),
            idx,
        );

        items.push(SourceItem {
            metadata,
            parts,
            content_hash,
            creation_payload,
            source_path: path,
        });
    }

    Ok(SourceWorkspace {
        items,
        logical_id_index,
        type_name_index,
    })
}

/// Read all definition files from an item directory (excluding `.platform`).
fn read_definition_parts(item_dir: &Path) -> Result<Vec<DefinitionPart>> {
    let mut parts = Vec::new();
    read_parts_recursive(item_dir, item_dir, &mut parts)?;
    Ok(parts)
}

/// Recursively read files from an item directory, building definition parts.
fn read_parts_recursive(
    base_dir: &Path,
    current_dir: &Path,
    parts: &mut Vec<DefinitionPart>,
) -> Result<()> {
    let entries = std::fs::read_dir(current_dir)
        .with_context(|| format!("Failed to read item directory: {}", current_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            read_parts_recursive(base_dir, &path, parts)?;
        } else {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Skip .platform file and creationPayload.json (not definition parts)
            if file_name == ".platform" || file_name == "creationPayload.json" {
                continue;
            }

            // Compute relative path from base_dir
            let rel_path = path
                .strip_prefix(base_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                // Normalize to forward slashes for Fabric API
                .replace('\\', "/");

            let content = std::fs::read(&path)
                .with_context(|| format!("Failed to read file: {}", path.display()))?;

            let encoded = BASE64.encode(&content);

            parts.push(DefinitionPart {
                path: rel_path,
                payload: encoded,
                payload_type: "InlineBase64".to_owned(),
            });
        }
    }

    Ok(())
}

/// Write a source workspace to disk in the standard `.platform` directory format.
///
/// Used by `deploy export` to create a local copy of a workspace.
pub fn write_source_directory(
    output_dir: &Path,
    items: &[(PlatformMetadata, Vec<DefinitionPart>)],
    overwrite: bool,
) -> Result<usize> {
    if output_dir.exists() && !overwrite {
        // Check if it's non-empty
        let has_entries = std::fs::read_dir(output_dir).is_ok_and(|mut rd| rd.next().is_some());
        if has_entries {
            bail!(
                "Output directory is not empty: {}. Use --overwrite to replace.",
                output_dir.display()
            );
        }
    }

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    let mut count = 0;

    for (metadata, parts) in items {
        let dir_name = format!("{}.{}", metadata.display_name, metadata.item_type);
        let item_dir = output_dir.join(&dir_name);
        std::fs::create_dir_all(&item_dir)?;

        // Write .platform file
        let platform_content = build_platform_json(metadata);
        std::fs::write(item_dir.join(".platform"), platform_content)?;

        // Write definition parts
        for part in parts {
            let part_path = item_dir.join(Path::new(&part.path));

            // Create parent directories if needed
            if let Some(parent) = part_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let decoded = BASE64
                .decode(&part.payload)
                .with_context(|| format!("Failed to decode base64 for part: {}", part.path))?;

            std::fs::write(&part_path, decoded)?;
        }

        count += 1;
    }

    Ok(count)
}

/// Build the JSON content for a `.platform` file.
fn build_platform_json(metadata: &PlatformMetadata) -> String {
    let mut obj = serde_json::json!({
        "$schema": "https://developer.microsoft.com/json-schemas/fabric/gitIntegration/platformProperties/2.0.0/schema.json",
        "metadata": {
            "type": metadata.item_type,
            "displayName": metadata.display_name,
        },
        "config": {
            "version": "2.0",
        }
    });

    if let Some(ref desc) = metadata.description {
        obj["metadata"]["description"] = serde_json::Value::String(desc.clone());
    }

    if let Some(ref lid) = metadata.logical_id {
        obj["config"]["logicalId"] = serde_json::Value::String(lid.clone());
    }

    if let Some(ref fmt) = metadata.definition_format {
        obj["config"]["definitionFormat"] = serde_json::Value::String(fmt.clone());
    }

    serde_json::to_string_pretty(&obj).unwrap_or_default()
}

/// Get the git source metadata for the current directory (if in a git repo).
#[derive(Debug, Clone, Serialize)]
pub struct SourceGitMetadata {
    pub commit: Option<String>,
    pub branch: Option<String>,
    pub dirty: bool,
}

/// Try to extract git metadata from the source directory.
pub fn get_git_metadata(source_dir: &Path) -> Option<SourceGitMetadata> {
    use std::process::Command;

    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(source_dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());

    // If we can't even get a commit, this isn't a git repo
    commit.as_ref()?;

    let branch = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(source_dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|s| !s.is_empty());

    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(source_dir)
        .output()
        .ok()
        .is_some_and(|o| !o.stdout.is_empty());

    Some(SourceGitMetadata {
        commit,
        branch,
        dirty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_platform_file() {
        let dir = TempDir::new().unwrap();
        let platform = dir.path().join(".platform");
        fs::write(
            &platform,
            r#"{
                "$schema": "https://developer.microsoft.com/json-schemas/fabric/gitIntegration/platformProperties/2.0.0/schema.json",
                "metadata": {
                    "type": "Notebook",
                    "displayName": "Hello World",
                    "description": "A test notebook"
                },
                "config": {
                    "version": "2.0",
                    "logicalId": "99b570c5-0c79-9dc4-4c9b-fa16c621384c"
                }
            }"#,
        )
        .unwrap();

        let meta = parse_platform_file(&platform).unwrap();
        assert_eq!(meta.item_type, "Notebook");
        assert_eq!(meta.display_name, "Hello World");
        assert_eq!(meta.description.as_deref(), Some("A test notebook"));
        assert_eq!(
            meta.logical_id.as_deref(),
            Some("99b570c5-0c79-9dc4-4c9b-fa16c621384c")
        );
    }

    #[test]
    fn test_parse_source_directory() {
        let dir = TempDir::new().unwrap();

        // Create a notebook item
        let nb_dir = dir.path().join("MyNotebook.Notebook");
        fs::create_dir_all(&nb_dir).unwrap();
        fs::write(
            nb_dir.join(".platform"),
            r#"{"metadata":{"type":"Notebook","displayName":"MyNotebook"},"config":{"version":"2.0","logicalId":"aaa-bbb"}}"#,
        )
        .unwrap();
        fs::write(nb_dir.join("notebook-content.py"), "# Hello").unwrap();

        // Create a pipeline item
        let pl_dir = dir.path().join("ETL.DataPipeline");
        fs::create_dir_all(&pl_dir).unwrap();
        fs::write(
            pl_dir.join(".platform"),
            r#"{"metadata":{"type":"DataPipeline","displayName":"ETL"},"config":{"version":"2.0"}}"#,
        )
        .unwrap();
        fs::write(pl_dir.join("pipeline-content.json"), r#"{"activities":[]}"#).unwrap();

        // A non-item file at root (should be ignored)
        fs::write(dir.path().join("README.md"), "# Docs").unwrap();

        let workspace = parse_source_directory(dir.path()).unwrap();
        assert_eq!(workspace.items.len(), 2);
        assert!(workspace.logical_id_index.contains_key("aaa-bbb"));
        assert!(
            workspace
                .type_name_index
                .contains_key(&("Notebook".to_owned(), "MyNotebook".to_owned()))
        );
        assert!(
            workspace
                .type_name_index
                .contains_key(&("DataPipeline".to_owned(), "ETL".to_owned()))
        );
    }

    #[test]
    fn test_parse_source_directory_with_creation_payload() {
        let dir = TempDir::new().unwrap();

        // Create a KQL database item with creationPayload.json
        let kql_dir = dir.path().join("MyDB.KQLDatabase");
        fs::create_dir_all(&kql_dir).unwrap();
        fs::write(
            kql_dir.join(".platform"),
            r#"{"metadata":{"type":"KQLDatabase","displayName":"MyDB"},"config":{"version":"2.0"}}"#,
        )
        .unwrap();
        fs::write(
            kql_dir.join("creationPayload.json"),
            r#"{"databaseType":"ReadWrite","parentEventhouseItemId":"eh-123"}"#,
        )
        .unwrap();
        fs::write(kql_dir.join("DatabaseProperties.json"), r"{}").unwrap();

        let workspace = parse_source_directory(dir.path()).unwrap();
        assert_eq!(workspace.items.len(), 1);

        let item = &workspace.items[0];
        assert!(item.creation_payload.is_some());
        let payload = item.creation_payload.as_ref().unwrap();
        assert_eq!(payload["databaseType"], "ReadWrite");
        assert_eq!(payload["parentEventhouseItemId"], "eh-123");

        // creationPayload.json should NOT be in the definition parts
        assert_eq!(item.parts.len(), 1);
        assert_eq!(item.parts[0].path, "DatabaseProperties.json");
    }

    #[test]
    fn test_parse_source_directory_without_creation_payload() {
        let dir = TempDir::new().unwrap();

        let nb_dir = dir.path().join("Nb.Notebook");
        fs::create_dir_all(&nb_dir).unwrap();
        fs::write(
            nb_dir.join(".platform"),
            r#"{"metadata":{"type":"Notebook","displayName":"Nb"},"config":{"version":"2.0"}}"#,
        )
        .unwrap();
        fs::write(nb_dir.join("notebook-content.py"), "# code").unwrap();

        let workspace = parse_source_directory(dir.path()).unwrap();
        let item = &workspace.items[0];
        assert!(item.creation_payload.is_none());
        assert_eq!(item.parts.len(), 1);
    }

    #[test]
    fn test_content_hash_deterministic() {
        let parts = vec![
            DefinitionPart {
                path: "b.json".to_owned(),
                payload: BASE64.encode(b"content-b"),
                payload_type: "InlineBase64".to_owned(),
            },
            DefinitionPart {
                path: "a.json".to_owned(),
                payload: BASE64.encode(b"content-a"),
                payload_type: "InlineBase64".to_owned(),
            },
        ];

        // Same parts in different order should produce same hash
        let parts_reversed = vec![parts[1].clone(), parts[0].clone()];

        let hash1 = compute_content_hash(&parts);
        let hash2 = compute_content_hash(&parts_reversed);
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("sha256:"));
    }

    #[test]
    fn test_write_source_directory() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("export");

        let items = vec![(
            PlatformMetadata {
                item_type: "Notebook".to_owned(),
                display_name: "Test".to_owned(),
                logical_id: Some("lid-123".to_owned()),
                description: None,
                definition_format: None,
            },
            vec![DefinitionPart {
                path: "notebook-content.py".to_owned(),
                payload: BASE64.encode(b"# code"),
                payload_type: "InlineBase64".to_owned(),
            }],
        )];

        let count = write_source_directory(&output, &items, false).unwrap();
        assert_eq!(count, 1);

        let nb_dir = output.join("Test.Notebook");
        assert!(nb_dir.join(".platform").exists());
        assert!(nb_dir.join("notebook-content.py").exists());

        let content = fs::read_to_string(nb_dir.join("notebook-content.py")).unwrap();
        assert_eq!(content, "# code");
    }
}
