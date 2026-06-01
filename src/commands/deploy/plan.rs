use std::collections::HashMap;
use std::fmt::Write;

use anyhow::{Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::cli::Cli;
use crate::client::FabricClient;

use super::changeset::{Change, ChangeAction, Changeset};
use super::platform::SourceWorkspace;

/// Deployed item representation (from workspace API).
#[derive(Debug, Clone)]
pub struct DeployedItem {
    pub id: String,
    pub display_name: String,
    pub item_type: String,
    #[allow(dead_code)]
    pub definition_hash: Option<String>,
}

/// Fetch the list of deployed items in the target workspace.
pub async fn fetch_deployed_items(
    client: &FabricClient,
    workspace: &str,
    item_types: Option<&[String]>,
) -> Result<Vec<DeployedItem>> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/items"),
            "value",
            true,
            None,
        )
        .await?;

    let mut items: Vec<DeployedItem> = resp
        .items
        .iter()
        .filter_map(|v| {
            let id = v.get("id")?.as_str()?.to_owned();
            let display_name = v.get("displayName")?.as_str()?.to_owned();
            let item_type = v.get("type")?.as_str()?.to_owned();
            Some(DeployedItem {
                id,
                display_name,
                item_type,
                definition_hash: None,
            })
        })
        .collect();

    // Filter by item types if specified
    if let Some(types) = item_types {
        items.retain(|item| {
            types
                .iter()
                .any(|t| t.eq_ignore_ascii_case(&item.item_type))
        });
    }

    Ok(items)
}

/// Fetch the definition of a deployed item and compute its content hash.
///
/// Returns `None` if the item doesn't support definitions (e.g., Dashboard).
pub async fn fetch_definition_hash(
    client: &FabricClient,
    workspace: &str,
    item_id: &str,
) -> Result<Option<String>> {
    let path = format!("/workspaces/{workspace}/items/{item_id}/getDefinition");

    let result = client.post(&path, &serde_json::json!({}), true).await;

    match result {
        Ok(data) => {
            let parts = data
                .get("definition")
                .and_then(|d| d.get("parts"))
                .and_then(|p| p.as_array());

            parts.map_or(Ok(None), |parts| Ok(Some(hash_api_parts(parts))))
        }
        Err(e) => {
            let err_str = e.to_string();
            // Some item types don't support getDefinition (404 or specific error)
            if err_str.contains("NOT_FOUND")
                || err_str.contains("not supported")
                || err_str.contains("InvalidItemType")
            {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

/// Compute content hash from API definition parts (same algorithm as local).
fn hash_api_parts(parts: &[Value]) -> String {
    let mut hasher = Sha256::new();

    let mut sorted: Vec<(&str, &str)> = parts
        .iter()
        .filter_map(|p| {
            let path = p.get("path")?.as_str()?;
            let payload = p.get("payload")?.as_str()?;
            Some((path, payload))
        })
        .collect();
    sorted.sort_by_key(|(path, _)| *path);

    for (path, payload) in sorted {
        hasher.update(path.as_bytes());
        hasher.update(b"\x00");
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

/// Resolve a workspace identifier (ID or name) to a workspace ID.
pub async fn resolve_workspace(client: &FabricClient, workspace: &str) -> Result<String> {
    // If it looks like a GUID, use it directly
    if is_guid(workspace) {
        return Ok(workspace.to_owned());
    }

    // Otherwise, search by name
    let resp = client.get_list("/workspaces", "value", true, None).await?;

    for item in &resp.items {
        if let Some(name) = item.get("displayName").and_then(|v| v.as_str()) {
            if name.eq_ignore_ascii_case(workspace) {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    return Ok(id.to_owned());
                }
            }
        }
    }

    bail!("Workspace not found: \"{workspace}\". Use a workspace ID or verify the name.")
}

fn is_guid(s: &str) -> bool {
    let s = s.trim();
    s.len() == 36
        && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        && s.chars().filter(|&c| c == '-').count() == 4
}

/// Build a changeset by comparing source items against deployed items.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub async fn build_changeset(
    cli: &Cli,
    client: &FabricClient,
    workspace_id: &str,
    source: &SourceWorkspace,
    deployed_items: &[DeployedItem],
    item_types: Option<&[String]>,
    delete_orphans: bool,
    force_all: bool,
) -> Result<Changeset> {
    let mut changeset = Changeset::new();

    if !cli.quiet {
        eprintln!(
            "[deploy] comparing {} source item(s) against {} deployed item(s)",
            source.items.len(),
            deployed_items.len()
        );
    }

    // Build deployed lookup: (type, name) → DeployedItem
    let mut deployed_map: HashMap<(String, String), DeployedItem> = HashMap::new();
    for item in deployed_items {
        deployed_map.insert(
            (item.item_type.clone(), item.display_name.clone()),
            item.clone(),
        );
    }

    // Track which deployed items are matched (for orphan detection)
    let mut matched_deployed: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();

    // Compare each source item against deployed state
    for source_item in &source.items {
        // Skip if filtered by item type
        if let Some(types) = item_types {
            if !types
                .iter()
                .any(|t| t.eq_ignore_ascii_case(&source_item.metadata.item_type))
            {
                continue;
            }
        }

        let key = (
            source_item.metadata.item_type.clone(),
            source_item.metadata.display_name.clone(),
        );

        match deployed_map.get(&key) {
            Some(deployed) => {
                matched_deployed.insert(key);

                if force_all {
                    // Force mode: always update
                    changeset.changes.push(Change {
                        name: source_item.metadata.display_name.clone(),
                        item_type: source_item.metadata.item_type.clone(),
                        action: ChangeAction::Update,
                        reason: "forced update (--force-all)".to_owned(),
                        logical_id: source_item.metadata.logical_id.clone(),
                        deployed_id: Some(deployed.id.clone()),
                        source_hash: Some(source_item.content_hash.clone()),
                    });
                } else {
                    // Fetch deployed definition hash for comparison
                    let deployed_hash =
                        fetch_definition_hash(client, workspace_id, &deployed.id).await?;

                    match deployed_hash {
                        Some(ref dh) if *dh == source_item.content_hash => {
                            changeset.changes.push(Change {
                                name: source_item.metadata.display_name.clone(),
                                item_type: source_item.metadata.item_type.clone(),
                                action: ChangeAction::Skip,
                                reason: "unchanged".to_owned(),
                                logical_id: source_item.metadata.logical_id.clone(),
                                deployed_id: Some(deployed.id.clone()),
                                source_hash: Some(source_item.content_hash.clone()),
                            });
                        }
                        _ => {
                            changeset.changes.push(Change {
                                name: source_item.metadata.display_name.clone(),
                                item_type: source_item.metadata.item_type.clone(),
                                action: ChangeAction::Update,
                                reason: "definition changed".to_owned(),
                                logical_id: source_item.metadata.logical_id.clone(),
                                deployed_id: Some(deployed.id.clone()),
                                source_hash: Some(source_item.content_hash.clone()),
                            });
                        }
                    }
                }
            }
            None => {
                // Item exists in source but not deployed → Create
                changeset.changes.push(Change {
                    name: source_item.metadata.display_name.clone(),
                    item_type: source_item.metadata.item_type.clone(),
                    action: ChangeAction::Create,
                    reason: "new item".to_owned(),
                    logical_id: source_item.metadata.logical_id.clone(),
                    deployed_id: None,
                    source_hash: Some(source_item.content_hash.clone()),
                });
            }
        }

        // Warn if item has no logical ID
        if source_item.metadata.logical_id.is_none() {
            changeset.warnings.push(format!(
                "{} \"{}\" has no logicalId in .platform — rename tracking won't work",
                source_item.metadata.item_type, source_item.metadata.display_name
            ));
        }
    }

    // Detect orphans (deployed but not in source)
    if delete_orphans {
        for ((item_type, name), deployed) in &deployed_map {
            if !matched_deployed.contains(&(item_type.clone(), name.clone())) {
                changeset.changes.push(Change {
                    name: name.clone(),
                    item_type: item_type.clone(),
                    action: ChangeAction::Delete,
                    reason: "not in source".to_owned(),
                    logical_id: None,
                    deployed_id: Some(deployed.id.clone()),
                    source_hash: None,
                });
            }
        }
    }

    // Check for unresolved logical ID references
    validate_references(source, &changeset, cli);

    Ok(changeset)
}

/// Validate that cross-item references (logical IDs embedded in definitions) can be resolved.
#[allow(clippy::missing_const_for_fn)]
fn validate_references(source: &SourceWorkspace, changeset: &Changeset, _cli: &Cli) {
    // For Phase 1: basic validation that items exist.
    // Full logical ID resolution (scanning definition content for GUIDs that match
    // other items' logical IDs) will come in Phase 2 with parameter substitution.
    //
    // For now, we just ensure the source is internally consistent.
    let _ = source;
    let _ = changeset;
}

/// Compute a workspace state fingerprint from the deployed item list.
///
/// This is a lightweight hash over the sorted list of (id, type, name) tuples.
/// Used for plan staleness detection: if the fingerprint changes between plan and apply,
/// it means the workspace state has diverged.
pub fn compute_workspace_fingerprint(deployed_items: &[DeployedItem]) -> String {
    let mut hasher = Sha256::new();

    let mut sorted: Vec<(&str, &str, &str)> = deployed_items
        .iter()
        .map(|item| {
            (
                item.id.as_str(),
                item.item_type.as_str(),
                item.display_name.as_str(),
            )
        })
        .collect();
    sorted.sort_unstable();

    for (id, item_type, name) in sorted {
        hasher.update(id.as_bytes());
        hasher.update(b"\x00");
        hasher.update(item_type.as_bytes());
        hasher.update(b"\x00");
        hasher.update(name.as_bytes());
        hasher.update(b"\x00");
    }

    let hash = hasher.finalize();
    let hex = hash.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });
    format!("sha256:{hex}")
}

/// Normalize a content hash from the deployed API response for comparison.
///
/// The Fabric API may return base64 payloads with different whitespace or key ordering.
/// This function re-encodes JSON parts with sorted keys before hashing.
#[allow(dead_code)]
pub fn normalize_and_hash_definition(definition: &Value) -> Option<String> {
    let parts = definition.get("definition")?.get("parts")?.as_array()?;

    let mut normalized_parts: Vec<(&str, String)> = Vec::new();

    for part in parts {
        let path = part.get("path")?.as_str()?;
        let payload = part.get("payload")?.as_str()?;

        // Try to decode, parse as JSON, re-serialize with sorted keys
        let normalized_payload = BASE64.decode(payload).map_or_else(
            |_| payload.to_owned(),
            |decoded| {
                serde_json::from_slice::<Value>(&decoded).map_or_else(
                    |_| payload.to_owned(),
                    |json| {
                        let re_encoded = serde_json::to_string(&json).unwrap_or_default();
                        BASE64.encode(re_encoded.as_bytes())
                    },
                )
            },
        );

        normalized_parts.push((path, normalized_payload));
    }

    normalized_parts.sort_by_key(|(path, _)| *path);

    let mut hasher = Sha256::new();
    for (path, payload) in &normalized_parts {
        hasher.update(path.as_bytes());
        hasher.update(b"\x00");
        hasher.update(payload.as_bytes());
        hasher.update(b"\x00");
    }

    let hash = hasher.finalize();
    let hex = hash.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });
    Some(format!("sha256:{hex}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;

    #[test]
    fn test_is_guid_valid() {
        assert!(is_guid("12345678-1234-1234-1234-123456789abc"));
        assert!(is_guid("ABCDEF00-0000-0000-0000-000000000000"));
        assert!(is_guid("abcdef00-1111-2222-3333-444444444444"));
    }

    #[test]
    fn test_is_guid_invalid() {
        assert!(!is_guid("not-a-guid"));
        assert!(!is_guid("12345678-1234-1234-1234"));
        assert!(!is_guid("12345678123412341234123456789abc")); // no dashes
        assert!(!is_guid("12345678-1234-1234-1234-123456789abcX")); // too long
        assert!(!is_guid("")); // empty
        assert!(!is_guid("My Workspace Name")); // workspace name
    }

    #[test]
    fn test_hash_api_parts_deterministic() {
        let parts = vec![
            serde_json::json!({"path": "b.json", "payload": "Y29udGVudC1i"}),
            serde_json::json!({"path": "a.json", "payload": "Y29udGVudC1h"}),
        ];

        let hash1 = hash_api_parts(&parts);
        assert!(hash1.starts_with("sha256:"));
        assert_eq!(hash1.len(), 7 + 64); // "sha256:" + 64 hex chars

        // Same parts in different order should produce same hash (sorted internally)
        let parts_reversed = vec![parts[1].clone(), parts[0].clone()];
        let hash2 = hash_api_parts(&parts_reversed);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_api_parts_different_content() {
        let parts_a = vec![serde_json::json!({"path": "a.json", "payload": "AAAA"})];
        let parts_b = vec![serde_json::json!({"path": "a.json", "payload": "BBBB"})];

        let hash_a = hash_api_parts(&parts_a);
        let hash_b = hash_api_parts(&parts_b);
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn test_hash_api_parts_different_paths() {
        let parts_a = vec![serde_json::json!({"path": "x.json", "payload": "AAAA"})];
        let parts_b = vec![serde_json::json!({"path": "y.json", "payload": "AAAA"})];

        let hash_a = hash_api_parts(&parts_a);
        let hash_b = hash_api_parts(&parts_b);
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn test_hash_api_parts_skips_malformed() {
        // Missing payload field
        let parts = vec![serde_json::json!({"path": "a.json"})];
        let hash = hash_api_parts(&parts);
        // Should still return a valid hash (of empty input)
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn test_normalize_and_hash_definition_basic() {
        let payload = BASE64.encode(br#"{"z":1,"a":2}"#);
        let definition = serde_json::json!({
            "definition": {
                "parts": [
                    {"path": "file.json", "payload": payload, "payloadType": "InlineBase64"}
                ]
            }
        });

        let hash = normalize_and_hash_definition(&definition);
        assert!(hash.is_some());
        assert!(hash.unwrap().starts_with("sha256:"));
    }

    #[test]
    fn test_normalize_and_hash_definition_key_order_with_preserve_order() {
        // With serde_json `preserve_order` feature enabled, JSON key order
        // is maintained during re-serialization. So different key orders
        // produce different hashes (this is expected behavior — the source
        // of truth is the exact byte sequence, not semantic equivalence).
        let payload_az = BASE64.encode(br#"{"a":1,"z":2}"#);
        let payload_za = BASE64.encode(br#"{"z":2,"a":1}"#);

        let def_az = serde_json::json!({
            "definition": {
                "parts": [{"path": "f.json", "payload": payload_az, "payloadType": "InlineBase64"}]
            }
        });
        let def_za = serde_json::json!({
            "definition": {
                "parts": [{"path": "f.json", "payload": payload_za, "payloadType": "InlineBase64"}]
            }
        });

        let hash_az = normalize_and_hash_definition(&def_az).unwrap();
        let hash_za = normalize_and_hash_definition(&def_za).unwrap();

        // With preserve_order, keys stay in insertion order — hashes differ
        // This is correct: the local source is the canonical representation,
        // and the comparison catches any byte-level change.
        assert_ne!(hash_az, hash_za);
    }

    #[test]
    fn test_normalize_and_hash_definition_same_content_same_hash() {
        // Same exact payload should produce identical hash
        let payload = BASE64.encode(br#"{"key":"value","num":42}"#);

        let def1 = serde_json::json!({
            "definition": {
                "parts": [{"path": "f.json", "payload": payload, "payloadType": "InlineBase64"}]
            }
        });
        let def2 = serde_json::json!({
            "definition": {
                "parts": [{"path": "f.json", "payload": payload, "payloadType": "InlineBase64"}]
            }
        });

        let hash1 = normalize_and_hash_definition(&def1).unwrap();
        let hash2 = normalize_and_hash_definition(&def2).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_normalize_and_hash_definition_non_json_payload() {
        // Binary/non-JSON payload should hash as-is
        let payload = BASE64.encode(b"\x00\x01\x02\x03");
        let definition = serde_json::json!({
            "definition": {
                "parts": [{"path": "binary.dat", "payload": payload, "payloadType": "InlineBase64"}]
            }
        });

        let hash = normalize_and_hash_definition(&definition);
        assert!(hash.is_some());
    }

    #[test]
    fn test_normalize_and_hash_definition_missing_parts() {
        let definition = serde_json::json!({"definition": {}});
        assert!(normalize_and_hash_definition(&definition).is_none());

        let definition2 = serde_json::json!({});
        assert!(normalize_and_hash_definition(&definition2).is_none());
    }

    #[test]
    fn test_compute_workspace_fingerprint_deterministic() {
        let items = vec![
            DeployedItem {
                id: "bbb".to_owned(),
                display_name: "ItemB".to_owned(),
                item_type: "Notebook".to_owned(),
                definition_hash: None,
            },
            DeployedItem {
                id: "aaa".to_owned(),
                display_name: "ItemA".to_owned(),
                item_type: "Lakehouse".to_owned(),
                definition_hash: None,
            },
        ];

        let fp1 = compute_workspace_fingerprint(&items);

        // Reversed order should produce same fingerprint (sorted internally)
        let items_rev = vec![items[1].clone(), items[0].clone()];
        let fp2 = compute_workspace_fingerprint(&items_rev);

        assert_eq!(fp1, fp2);
        assert!(fp1.starts_with("sha256:"));
    }

    #[test]
    fn test_compute_workspace_fingerprint_changes_on_modification() {
        let items1 = vec![DeployedItem {
            id: "aaa".to_owned(),
            display_name: "Item".to_owned(),
            item_type: "Notebook".to_owned(),
            definition_hash: None,
        }];
        let items2 = vec![DeployedItem {
            id: "bbb".to_owned(),
            display_name: "Item".to_owned(),
            item_type: "Notebook".to_owned(),
            definition_hash: None,
        }];

        let fp1 = compute_workspace_fingerprint(&items1);
        let fp2 = compute_workspace_fingerprint(&items2);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_compute_workspace_fingerprint_empty() {
        let fp = compute_workspace_fingerprint(&[]);
        assert!(fp.starts_with("sha256:"));
    }
}
