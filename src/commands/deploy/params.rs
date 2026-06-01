//! Parameter substitution for environment-aware deployments.
//!
//! Supports a JSON parameter file (`parameters.json`) with:
//! - `find_replace`: Literal or regex-based string replacement in definition payloads
//! - `key_value_replace`: JSONPath-based value replacement at specific JSON keys
//! - `spark_pool`: Spark pool instance ID to environment-specific pool configuration mapping
//! - `semantic_model_binding`: Semantic model connection ID promotion across environments
//! - Dynamic variables: `$workspace.id`, `$workspace.name`, `$items.Type.Name.id`, `$ENV:VAR`
//!
//! The parameter file format is a superset of fabric-cicd's YAML `parameter.yml`,
//! expressed in JSON for agent-native tooling consistency.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use jsonpath_rust::JsonPath;
use jsonpath_rust::query::queryable::Queryable;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::platform::{DefinitionPart, SourceWorkspace};

/// Parsed parameter file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    /// String find-and-replace rules.
    #[serde(default)]
    pub find_replace: Vec<FindReplaceRule>,

    /// JSONPath-based key-value replacement rules.
    #[serde(default)]
    pub key_value_replace: Vec<KeyValueReplaceRule>,

    /// Spark pool instance mapping rules.
    #[serde(default)]
    pub spark_pool: Vec<SparkPoolRule>,

    /// Semantic model connection binding rules.
    #[serde(default)]
    pub semantic_model_binding: Option<SemanticModelBinding>,
}

/// A single find-and-replace rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindReplaceRule {
    /// The string or regex pattern to find.
    pub find_value: String,

    /// Environment-keyed replacement values.
    /// Key is the environment name (e.g., "dev", "prod") or "_ALL_" for all environments.
    pub replace_value: HashMap<String, String>,

    /// Enable regex mode. In regex mode, only capture group 1 is replaced.
    #[serde(default)]
    pub is_regex: bool,

    /// Restrict to specific item type(s).
    #[serde(default)]
    pub item_type: Option<StringOrVec>,

    /// Restrict to specific item name(s).
    #[serde(default)]
    pub item_name: Option<StringOrVec>,

    /// Restrict to specific file path(s) within definitions.
    #[serde(default)]
    pub file_path: Option<StringOrVec>,
}

/// A value that can be either a single string or a list of strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

impl StringOrVec {
    pub fn contains(&self, value: &str) -> bool {
        match self {
            Self::Single(s) => s.eq_ignore_ascii_case(value),
            Self::Multiple(v) => v.iter().any(|s| s.eq_ignore_ascii_case(value)),
        }
    }
}

/// A JSONPath-based key-value replacement rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValueReplaceRule {
    /// `JSONPath` expression identifying the key(s) whose value should be replaced.
    pub find_key: String,

    /// Environment-keyed replacement values (can be any JSON value type).
    pub replace_value: HashMap<String, serde_json::Value>,

    /// Restrict to specific item type(s).
    #[serde(default)]
    pub item_type: Option<StringOrVec>,

    /// Restrict to specific item name(s).
    #[serde(default)]
    pub item_name: Option<StringOrVec>,

    /// Restrict to specific file path(s) within definitions.
    #[serde(default)]
    pub file_path: Option<StringOrVec>,
}

/// A Spark pool instance mapping rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparkPoolRule {
    /// The source Spark pool instance ID to match.
    pub instance_pool_id: String,

    /// Environment-keyed pool configuration objects.
    pub replace_value: HashMap<String, SparkPoolConfig>,

    /// Restrict to specific item name(s).
    #[serde(default)]
    pub item_name: Option<StringOrVec>,
}

/// Spark pool configuration for a target environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparkPoolConfig {
    /// Pool type: "Capacity" or "Workspace".
    #[serde(rename = "type")]
    pub pool_type: String,

    /// Pool display name.
    pub name: String,
}

/// Semantic model connection binding configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticModelBinding {
    /// Default connection binding applied to all semantic models.
    #[serde(default)]
    pub default: Option<ConnectionBinding>,

    /// Per-model connection binding overrides.
    #[serde(default)]
    pub models: Vec<ModelBinding>,
}

/// A connection binding with environment-keyed connection IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionBinding {
    /// Environment-keyed connection GUID values.
    pub connection_id: HashMap<String, String>,
}

/// A per-model connection binding override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBinding {
    /// Semantic model name(s) this binding applies to.
    pub semantic_model_name: StringOrVec,

    /// Environment-keyed connection GUID values.
    pub connection_id: HashMap<String, String>,
}

/// Context for resolving dynamic variables during substitution.
pub struct SubstitutionContext<'a> {
    /// Target workspace ID.
    pub workspace_id: &'a str,
    /// Target workspace name (if resolved).
    pub workspace_name: Option<&'a str>,
    /// Map of (Type, Name) → deployed GUID for `$items.Type.Name.id` resolution.
    pub deployed_items: &'a HashMap<(String, String), String>,
}

/// Parse a parameter file from disk.
pub fn parse_parameters(path: &Path) -> Result<Parameters> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read parameters file: {}", path.display()))?;

    // Support both JSON and simplified key=value, but primarily JSON
    let params: Parameters = serde_json::from_str(&content)
        .with_context(|| format!("Invalid JSON in parameters file: {}", path.display()))?;

    // Validate rules
    for (i, rule) in params.find_replace.iter().enumerate() {
        if rule.find_value.is_empty() {
            bail!(
                "parameters file rule #{}: find_value cannot be empty",
                i + 1
            );
        }
        if rule.replace_value.is_empty() {
            bail!(
                "parameters file rule #{}: replace_value must have at least one environment entry",
                i + 1
            );
        }
        if rule.is_regex {
            Regex::new(&rule.find_value).with_context(|| {
                format!(
                    "parameters file rule #{}: invalid regex pattern: {}",
                    i + 1,
                    rule.find_value
                )
            })?;
        }
    }

    // Validate key_value_replace rules
    for (i, rule) in params.key_value_replace.iter().enumerate() {
        if rule.find_key.is_empty() {
            bail!(
                "key_value_replace rule #{}: find_key cannot be empty",
                i + 1
            );
        }
        if rule.replace_value.is_empty() {
            bail!(
                "key_value_replace rule #{}: replace_value must have at least one environment entry",
                i + 1
            );
        }
        // Validate JSONPath syntax
        jsonpath_rust::parser::parse_json_path(&rule.find_key).with_context(|| {
            format!(
                "key_value_replace rule #{}: invalid JSONPath expression: {}",
                i + 1,
                rule.find_key
            )
        })?;
    }

    // Validate spark_pool rules
    for (i, rule) in params.spark_pool.iter().enumerate() {
        if rule.instance_pool_id.is_empty() {
            bail!(
                "spark_pool rule #{}: instance_pool_id cannot be empty",
                i + 1
            );
        }
        if rule.replace_value.is_empty() {
            bail!(
                "spark_pool rule #{}: replace_value must have at least one environment entry",
                i + 1
            );
        }
    }

    Ok(params)
}

/// Resolve a replacement value, expanding dynamic variables.
///
/// Dynamic variables:
/// - `$workspace.id` → target workspace GUID
/// - `$workspace.name` → target workspace display name
/// - `$items.Type.Name.id` → deployed GUID of the named item
/// - `$ENV:VAR_NAME` → value of environment variable
fn resolve_value(raw: &str, ctx: &SubstitutionContext<'_>) -> Result<String> {
    if !raw.starts_with('$') {
        return Ok(raw.to_owned());
    }

    if raw == "$workspace.id" {
        return Ok(ctx.workspace_id.to_owned());
    }

    if raw == "$workspace.name" {
        return ctx.workspace_name.map(str::to_owned).ok_or_else(|| {
            anyhow::anyhow!("$workspace.name not available (workspace resolved by ID, not name)")
        });
    }

    if let Some(var_name) = raw.strip_prefix("$ENV:") {
        return std::env::var(var_name).with_context(|| {
            format!("Environment variable '{var_name}' referenced in parameters is not set")
        });
    }

    if let Some(item_ref) = raw.strip_prefix("$items.") {
        // Format: $items.Type.Name.id
        let parts: Vec<&str> = item_ref.splitn(3, '.').collect();
        if parts.len() == 3 && parts[2] == "id" {
            let item_type = parts[0];
            let item_name = parts[1];

            return ctx
                .deployed_items
                .get(&(item_type.to_owned(), item_name.to_owned()))
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Cannot resolve $items.{item_type}.{item_name}.id: item not found in deployed workspace or source"
                    )
                });
        }
        bail!("Invalid $items reference: '{raw}'. Expected format: $items.Type.Name.id");
    }

    // Unknown variable reference — return as-is with a warning? No, error.
    bail!(
        "Unknown dynamic variable: '{raw}'. Supported: $workspace.id, $workspace.name, $items.Type.Name.id, $ENV:VAR"
    );
}

/// Get the replacement value for a specific environment from a rule.
fn get_env_value<'a>(rule: &'a FindReplaceRule, env: &str) -> Option<&'a str> {
    // Check for exact environment match (case-insensitive)
    for (key, value) in &rule.replace_value {
        if key.eq_ignore_ascii_case(env) {
            return Some(value.as_str());
        }
    }
    // Check for _ALL_ wildcard
    for (key, value) in &rule.replace_value {
        if key.eq_ignore_ascii_case("_ALL_") {
            return Some(value.as_str());
        }
    }
    None
}

/// Apply parameter substitution to a source workspace's definition parts.
///
/// This modifies the definition payloads in-place (decoded from base64, substituted,
/// re-encoded to base64). The content hashes are recomputed after substitution.
pub fn apply_parameters(
    source: &mut SourceWorkspace,
    params: &Parameters,
    env: &str,
    ctx: &SubstitutionContext<'_>,
) -> Result<Vec<String>> {
    let mut warnings: Vec<String> = Vec::new();

    // Apply find_replace rules
    if !params.find_replace.is_empty() {
        apply_find_replace(source, &params.find_replace, env, ctx, &mut warnings)?;
    }

    // Apply key_value_replace rules
    if !params.key_value_replace.is_empty() {
        apply_key_value_replace(source, &params.key_value_replace, env, ctx, &mut warnings)?;
    }

    // Apply spark_pool rules
    if !params.spark_pool.is_empty() {
        apply_spark_pool_rules(source, &params.spark_pool, env, &mut warnings)?;
    }

    // Apply semantic_model_binding
    if let Some(ref binding) = params.semantic_model_binding {
        apply_semantic_model_binding(source, binding, env, &mut warnings)?;
    }

    Ok(warnings)
}

/// Apply `find_replace` rules to a source workspace.
fn apply_find_replace(
    source: &mut SourceWorkspace,
    rules: &[FindReplaceRule],
    env: &str,
    ctx: &SubstitutionContext<'_>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    // Compile regex patterns once
    let compiled_rules: Vec<CompiledRule<'_>> = rules
        .iter()
        .filter_map(|rule| {
            let raw_value = get_env_value(rule, env)?;
            let resolved = resolve_value(raw_value, ctx);
            match resolved {
                Ok(replacement) => {
                    let pattern = if rule.is_regex {
                        match Regex::new(&rule.find_value) {
                            Ok(re) => RulePattern::Regex(re),
                            Err(e) => {
                                warnings.push(format!(
                                    "Skipping rule (invalid regex '{}'): {e}",
                                    rule.find_value
                                ));
                                return None;
                            }
                        }
                    } else {
                        RulePattern::Literal(rule.find_value.clone())
                    };

                    Some(CompiledRule {
                        pattern,
                        replacement,
                        rule,
                    })
                }
                Err(e) => {
                    warnings.push(format!("Skipping rule (cannot resolve '{raw_value}'): {e}"));
                    None
                }
            }
        })
        .collect();

    if compiled_rules.is_empty() {
        return Ok(());
    }

    // Apply rules to each item's definition parts
    for item in &mut source.items {
        for part in &mut item.parts {
            let should_process = compiled_rules.iter().any(|cr| {
                rule_applies_to_item(
                    cr.rule,
                    &item.metadata.item_type,
                    &item.metadata.display_name,
                    &part.path,
                )
            });

            if !should_process {
                continue;
            }

            // Decode the payload
            let decoded = BASE64
                .decode(&part.payload)
                .with_context(|| format!("Failed to decode base64 payload for {}", part.path))?;

            let mut content = String::from_utf8(decoded).with_context(|| {
                format!(
                    "Non-UTF8 content in {} of {} (cannot apply text substitution)",
                    part.path, item.metadata.display_name
                )
            })?;

            let mut modified = false;

            // Apply each matching rule
            for cr in &compiled_rules {
                if !rule_applies_to_item(
                    cr.rule,
                    &item.metadata.item_type,
                    &item.metadata.display_name,
                    &part.path,
                ) {
                    continue;
                }

                match &cr.pattern {
                    RulePattern::Literal(find) => {
                        if content.contains(find.as_str()) {
                            content = content.replace(find.as_str(), &cr.replacement);
                            modified = true;
                        }
                    }
                    RulePattern::Regex(re) => {
                        // In regex mode, replace the content of capture group 1
                        let new_content = replace_capture_group(re, &content, &cr.replacement);
                        if new_content != content {
                            content = new_content;
                            modified = true;
                        }
                    }
                }
            }

            if modified {
                part.payload = BASE64.encode(content.as_bytes());
            }
        }

        // Recompute content hash after substitution
        if item.parts.iter().any(|_| true) {
            item.content_hash = compute_content_hash(&item.parts);
        }
    }

    Ok(())
}

/// Check if a `find_replace` rule applies to a specific item and file path.
fn rule_applies_to_item(
    rule: &FindReplaceRule,
    item_type: &str,
    item_name: &str,
    file_path: &str,
) -> bool {
    kv_rule_applies_to_item(
        rule.item_type.as_ref(),
        rule.item_name.as_ref(),
        rule.file_path.as_ref(),
        item_type,
        item_name,
        file_path,
    )
}

/// Check if a rule applies to a specific item and file path (generic version for `KeyValueReplaceRule`).
fn kv_rule_applies_to_item(
    item_type_filter: Option<&StringOrVec>,
    item_name_filter: Option<&StringOrVec>,
    file_path_filter: Option<&StringOrVec>,
    item_type: &str,
    item_name: &str,
    file_path: &str,
) -> bool {
    if let Some(types) = item_type_filter {
        if !types.contains(item_type) {
            return false;
        }
    }
    if let Some(names) = item_name_filter {
        if !names.contains(item_name) {
            return false;
        }
    }
    if let Some(paths) = file_path_filter {
        if !paths.contains(file_path) {
            return false;
        }
    }
    true
}

/// Apply `key_value_replace` rules to a source workspace.
fn apply_key_value_replace(
    source: &mut SourceWorkspace,
    rules: &[KeyValueReplaceRule],
    env: &str,
    ctx: &SubstitutionContext<'_>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    for rule in rules {
        // Get replacement value for this environment
        let replacement = get_env_value_json(&rule.replace_value, env);
        let Some(replacement) = replacement else {
            warnings.push(format!(
                "key_value_replace: no value for env '{env}' in rule with find_key '{}'",
                rule.find_key
            ));
            continue;
        };

        // Resolve dynamic variables if the value is a string
        let resolved_replacement = if let serde_json::Value::String(s) = replacement {
            match resolve_value(s, ctx) {
                Ok(resolved) => serde_json::Value::String(resolved),
                Err(e) => {
                    warnings.push(format!(
                        "key_value_replace: cannot resolve '{}': {e}",
                        rule.find_key
                    ));
                    continue;
                }
            }
        } else {
            replacement.clone()
        };

        // Validate JSONPath syntax upfront (already validated in parse_parameters, but guard here)
        if jsonpath_rust::parser::parse_json_path(&rule.find_key).is_err() {
            warnings.push(format!(
                "key_value_replace: invalid JSONPath '{}': parse error",
                rule.find_key
            ));
            continue;
        }

        for item in &mut source.items {
            for part in &mut item.parts {
                if !kv_rule_applies_to_item(
                    rule.item_type.as_ref(),
                    rule.item_name.as_ref(),
                    rule.file_path.as_ref(),
                    &item.metadata.item_type,
                    &item.metadata.display_name,
                    &part.path,
                ) {
                    continue;
                }

                // Decode payload
                let Ok(decoded) = BASE64.decode(&part.payload) else {
                    continue;
                };
                let Ok(content) = String::from_utf8(decoded) else {
                    continue;
                };

                // Parse as JSON
                let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(&content) else {
                    continue; // Not JSON, skip
                };

                // Apply JSONPath replacement
                let modified =
                    apply_jsonpath_replace(&mut json_value, &rule.find_key, &resolved_replacement);

                if modified {
                    let new_content = serde_json::to_string(&json_value)
                        .context("Failed to serialize JSON after key_value_replace")?;
                    part.payload = BASE64.encode(new_content.as_bytes());
                }
            }

            // Recompute hash
            item.content_hash = compute_content_hash(&item.parts);
        }
    }

    Ok(())
}

/// Apply a `JSONPath` replacement to a JSON value. Returns true if modified.
fn apply_jsonpath_replace(
    value: &mut serde_json::Value,
    path_expr: &str,
    replacement: &serde_json::Value,
) -> bool {
    // Get all matching paths
    let Ok(paths) = value.query_only_path(path_expr) else {
        return false;
    };

    if paths.is_empty() {
        return false;
    }

    let mut modified = false;
    for path in paths {
        if let Some(target) = value.reference_mut(path) {
            *target = replacement.clone();
            modified = true;
        }
    }
    modified
}

/// Apply `spark_pool` rules to a source workspace.
fn apply_spark_pool_rules(
    source: &mut SourceWorkspace,
    rules: &[SparkPoolRule],
    env: &str,
    warnings: &mut Vec<String>,
) -> Result<()> {
    for rule in rules {
        // Get pool config for this environment
        let config = get_spark_pool_config(&rule.replace_value, env);
        let Some(config) = config else {
            warnings.push(format!(
                "spark_pool: no value for env '{env}' in rule with instance_pool_id '{}'",
                rule.instance_pool_id
            ));
            continue;
        };

        for item in &mut source.items {
            // Spark pool rules typically apply to Environment items
            if let Some(ref names) = rule.item_name {
                if !names.contains(&item.metadata.display_name) {
                    continue;
                }
            }

            for part in &mut item.parts {
                // Decode payload
                let Ok(decoded) = BASE64.decode(&part.payload) else {
                    continue;
                };
                let Ok(content) = String::from_utf8(decoded) else {
                    continue;
                };

                // Check if this payload contains the instance_pool_id
                if !content.contains(&rule.instance_pool_id) {
                    continue;
                }

                // Parse as JSON and find/replace the pool configuration
                let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(&content) else {
                    continue;
                };

                let modified =
                    replace_spark_pool_in_json(&mut json_value, &rule.instance_pool_id, config);

                if modified {
                    let new_content = serde_json::to_string(&json_value)
                        .context("Failed to serialize JSON after spark_pool replace")?;
                    part.payload = BASE64.encode(new_content.as_bytes());
                }
            }

            item.content_hash = compute_content_hash(&item.parts);
        }
    }

    Ok(())
}

/// Replace Spark pool configuration in a JSON value tree.
/// Searches for objects containing `instancePoolId` matching the target,
/// then replaces the pool type and name fields.
fn replace_spark_pool_in_json(
    value: &mut serde_json::Value,
    instance_pool_id: &str,
    config: &SparkPoolConfig,
) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            // Check if this object has a matching instancePoolId
            let has_match = map
                .get("instancePoolId")
                .or_else(|| map.get("instance_pool_id"))
                .and_then(|v| v.as_str())
                .is_some_and(|v| v == instance_pool_id);

            if has_match {
                // Replace the pool type and name
                if let Some(t) = map.get_mut("type") {
                    *t = serde_json::Value::String(config.pool_type.clone());
                }
                if let Some(n) = map.get_mut("name") {
                    *n = serde_json::Value::String(config.name.clone());
                }
                return true;
            }

            // Recurse into child values
            let mut modified = false;
            for v in map.values_mut() {
                if replace_spark_pool_in_json(v, instance_pool_id, config) {
                    modified = true;
                }
            }
            modified
        }
        serde_json::Value::Array(arr) => {
            let mut modified = false;
            for v in arr {
                if replace_spark_pool_in_json(v, instance_pool_id, config) {
                    modified = true;
                }
            }
            modified
        }
        _ => false,
    }
}

/// Apply `semantic_model_binding` rules to a source workspace.
fn apply_semantic_model_binding(
    source: &mut SourceWorkspace,
    binding: &SemanticModelBinding,
    env: &str,
    warnings: &mut Vec<String>,
) -> Result<()> {
    for item in &mut source.items {
        // Only apply to SemanticModel items
        if !item
            .metadata
            .item_type
            .eq_ignore_ascii_case("SemanticModel")
        {
            continue;
        }

        // Find the connection ID for this model
        let connection_id = find_model_connection_id(binding, &item.metadata.display_name, env);

        let Some(connection_id) = connection_id else {
            continue; // No binding for this model + env
        };

        // Find and replace in definition.pbism or connection-related parts
        for part in &mut item.parts {
            let Ok(decoded) = BASE64.decode(&part.payload) else {
                continue;
            };
            let Ok(content) = String::from_utf8(decoded) else {
                continue;
            };

            let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(&content) else {
                continue;
            };

            let modified = replace_connection_id_in_json(&mut json_value, &connection_id);

            if modified {
                let new_content = serde_json::to_string(&json_value)
                    .context("Failed to serialize JSON after semantic_model_binding")?;
                part.payload = BASE64.encode(new_content.as_bytes());
            }
        }

        item.content_hash = compute_content_hash(&item.parts);
    }

    if binding.default.is_none() && binding.models.is_empty() {
        warnings.push("semantic_model_binding: no default or models defined".to_owned());
    }

    Ok(())
}

/// Find the connection ID for a specific model in the binding config.
fn find_model_connection_id(
    binding: &SemanticModelBinding,
    model_name: &str,
    env: &str,
) -> Option<String> {
    // Check model-specific overrides first
    for model in &binding.models {
        if model.semantic_model_name.contains(model_name) {
            return get_env_value_str(&model.connection_id, env).map(str::to_owned);
        }
    }

    // Fall back to default
    if let Some(ref default) = binding.default {
        return get_env_value_str(&default.connection_id, env).map(str::to_owned);
    }

    None
}

/// Replace connection IDs in semantic model JSON structures.
/// Looks for `connectionId`, `connection_id`, or connection string patterns.
fn replace_connection_id_in_json(value: &mut serde_json::Value, new_connection_id: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            let mut modified = false;

            // Look for connection ID fields
            for key in &["connectionId", "connection_id", "pbiModelDatabaseName"] {
                if let Some(v) = map.get_mut(*key) {
                    if v.is_string() {
                        let old = v.as_str().unwrap_or_default();
                        // Only replace if it looks like a GUID
                        if old.len() == 36 && old.contains('-') {
                            *v = serde_json::Value::String(new_connection_id.to_owned());
                            modified = true;
                        }
                    }
                }
            }

            // Also handle connectionString containing semanticmodelid=<UUID>
            if let Some(v) = map.get_mut("connectionString") {
                if let Some(cs) = v.as_str() {
                    if cs.contains("semanticmodelid=") {
                        let re =
                            Regex::new(r"semanticmodelid=([0-9a-fA-F-]{36})").expect("valid regex");
                        let new_cs =
                            re.replace(cs, format!("semanticmodelid={new_connection_id}").as_str());
                        if new_cs != cs {
                            *v = serde_json::Value::String(new_cs.into_owned());
                            modified = true;
                        }
                    }
                }
            }

            // Recurse
            for v in map.values_mut() {
                if replace_connection_id_in_json(v, new_connection_id) {
                    modified = true;
                }
            }
            modified
        }
        serde_json::Value::Array(arr) => {
            let mut modified = false;
            for v in arr {
                if replace_connection_id_in_json(v, new_connection_id) {
                    modified = true;
                }
            }
            modified
        }
        _ => false,
    }
}

/// Get an environment value from a string `HashMap` (for `connection_id` maps).
fn get_env_value_str<'a>(map: &'a HashMap<String, String>, env: &str) -> Option<&'a str> {
    for (key, value) in map {
        if key.eq_ignore_ascii_case(env) {
            return Some(value.as_str());
        }
    }
    for (key, value) in map {
        if key.eq_ignore_ascii_case("_ALL_") {
            return Some(value.as_str());
        }
    }
    None
}

/// Get an environment value from a JSON value `HashMap`.
fn get_env_value_json<'a>(
    map: &'a HashMap<String, serde_json::Value>,
    env: &str,
) -> Option<&'a serde_json::Value> {
    for (key, value) in map {
        if key.eq_ignore_ascii_case(env) {
            return Some(value);
        }
    }
    for (key, value) in map {
        if key.eq_ignore_ascii_case("_ALL_") {
            return Some(value);
        }
    }
    None
}

/// Get spark pool config for a specific environment.
fn get_spark_pool_config<'a>(
    map: &'a HashMap<String, SparkPoolConfig>,
    env: &str,
) -> Option<&'a SparkPoolConfig> {
    for (key, value) in map {
        if key.eq_ignore_ascii_case(env) {
            return Some(value);
        }
    }
    for (key, value) in map {
        if key.eq_ignore_ascii_case("_ALL_") {
            return Some(value);
        }
    }
    None
}

/// Replace the content matched by capture group 1 in a regex match.
///
/// fabric-cicd semantics: the full match is preserved, but the content of
/// group 1 is replaced with the new value.
fn replace_capture_group(re: &Regex, text: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;

    for caps in re.captures_iter(text) {
        let full_match = caps.get(0).unwrap();

        // Append text before this match
        result.push_str(&text[last_end..full_match.start()]);

        if let Some(group1) = caps.get(1) {
            // Replace group 1 within the full match
            let before_group = &text[full_match.start()..group1.start()];
            let after_group = &text[group1.end()..full_match.end()];
            result.push_str(before_group);
            result.push_str(replacement);
            result.push_str(after_group);
        } else {
            // No group 1 — keep original match
            result.push_str(full_match.as_str());
        }

        last_end = full_match.end();
    }

    // Append remaining text
    result.push_str(&text[last_end..]);
    result
}

/// Compute content hash (duplicated from platform.rs to avoid circular dependency
/// in the substitution flow — we need to recompute after modification).
fn compute_content_hash(parts: &[DefinitionPart]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    let mut hasher = Sha256::new();
    let mut sorted: Vec<(&str, &str)> = parts
        .iter()
        .map(|p| (p.path.as_str(), p.payload.as_str()))
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

/// Internal: compiled rule for efficient application.
struct CompiledRule<'a> {
    pattern: RulePattern,
    replacement: String,
    rule: &'a FindReplaceRule,
}

/// Internal: a pattern that has been validated/compiled.
enum RulePattern {
    Literal(String),
    Regex(Regex),
}

#[cfg(test)]
mod tests {
    use super::super::platform::SourceItem;
    use super::*;

    #[test]
    fn test_parse_parameters_basic() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("params.json");
        std::fs::write(
            &path,
            r#"{
                "find_replace": [
                    {
                        "find_value": "dev-server.database.windows.net",
                        "replace_value": {
                            "prod": "prod-server.database.windows.net"
                        }
                    }
                ]
            }"#,
        )
        .unwrap();

        let params = parse_parameters(&path).unwrap();
        assert_eq!(params.find_replace.len(), 1);
        assert_eq!(
            params.find_replace[0].find_value,
            "dev-server.database.windows.net"
        );
        assert!(!params.find_replace[0].is_regex);
    }

    #[test]
    fn test_parse_parameters_regex_rule() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("params.json");
        std::fs::write(
            &path,
            r#"{
                "find_replace": [
                    {
                        "find_value": "\"lakehouseId\":\\s*\"([0-9a-f-]{36})\"",
                        "replace_value": { "_ALL_": "new-lakehouse-id" },
                        "is_regex": true
                    }
                ]
            }"#,
        )
        .unwrap();

        let params = parse_parameters(&path).unwrap();
        assert_eq!(params.find_replace.len(), 1);
        assert!(params.find_replace[0].is_regex);
    }

    #[test]
    fn test_parse_parameters_invalid_regex() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("params.json");
        std::fs::write(
            &path,
            r#"{
                "find_replace": [
                    {
                        "find_value": "[invalid",
                        "replace_value": { "prod": "x" },
                        "is_regex": true
                    }
                ]
            }"#,
        )
        .unwrap();

        let result = parse_parameters(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid regex"));
    }

    #[test]
    fn test_resolve_value_workspace_id() {
        let ctx = SubstitutionContext {
            workspace_id: "ws-123",
            workspace_name: Some("MyWorkspace"),
            deployed_items: &HashMap::new(),
        };
        assert_eq!(resolve_value("$workspace.id", &ctx).unwrap(), "ws-123");
        assert_eq!(
            resolve_value("$workspace.name", &ctx).unwrap(),
            "MyWorkspace"
        );
    }

    #[test]
    fn test_resolve_value_items_ref() {
        let mut items = HashMap::new();
        items.insert(
            ("Lakehouse".to_owned(), "SalesLH".to_owned()),
            "guid-123".to_owned(),
        );

        let ctx = SubstitutionContext {
            workspace_id: "ws-1",
            workspace_name: None,
            deployed_items: &items,
        };

        assert_eq!(
            resolve_value("$items.Lakehouse.SalesLH.id", &ctx).unwrap(),
            "guid-123"
        );
    }

    #[test]
    fn test_resolve_value_env_var() {
        // Use a standard env var that's always present on all platforms
        let ctx = SubstitutionContext {
            workspace_id: "ws-1",
            workspace_name: None,
            deployed_items: &HashMap::new(),
        };

        // PATH exists on all CI/dev systems
        let result = resolve_value("$ENV:PATH", &ctx);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());

        // Non-existent env var should error
        let result = resolve_value("$ENV:FABIO_NONEXISTENT_VAR_12345", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not set"));
    }

    #[test]
    fn test_resolve_value_literal() {
        let ctx = SubstitutionContext {
            workspace_id: "ws-1",
            workspace_name: None,
            deployed_items: &HashMap::new(),
        };
        assert_eq!(resolve_value("plain-string", &ctx).unwrap(), "plain-string");
    }

    #[test]
    fn test_get_env_value_exact_match() {
        let rule = FindReplaceRule {
            find_value: "x".to_owned(),
            replace_value: HashMap::from([
                ("dev".to_owned(), "dev-val".to_owned()),
                ("prod".to_owned(), "prod-val".to_owned()),
            ]),
            is_regex: false,
            item_type: None,
            item_name: None,
            file_path: None,
        };

        assert_eq!(get_env_value(&rule, "prod"), Some("prod-val"));
        assert_eq!(get_env_value(&rule, "staging"), None);
    }

    #[test]
    fn test_get_env_value_all_wildcard() {
        let rule = FindReplaceRule {
            find_value: "x".to_owned(),
            replace_value: HashMap::from([("_ALL_".to_owned(), "universal".to_owned())]),
            is_regex: false,
            item_type: None,
            item_name: None,
            file_path: None,
        };

        assert_eq!(get_env_value(&rule, "anything"), Some("universal"));
    }

    #[test]
    fn test_replace_capture_group() {
        let re = Regex::new(r#""lakehouseId":\s*"([^"]+)""#).unwrap();
        let text = r#"{"lakehouseId": "old-guid-123"}"#;
        let result = replace_capture_group(&re, text, "new-guid-456");
        assert_eq!(result, r#"{"lakehouseId": "new-guid-456"}"#);
    }

    #[test]
    fn test_replace_capture_group_multiple_matches() {
        let re = Regex::new(r#"id="([^"]+)""#).unwrap();
        let text = r#"id="aaa" and id="bbb""#;
        let result = replace_capture_group(&re, text, "xxx");
        assert_eq!(result, r#"id="xxx" and id="xxx""#);
    }

    #[test]
    fn test_literal_substitution() {
        let params = Parameters {
            find_replace: vec![FindReplaceRule {
                find_value: "old-server.database.windows.net".to_owned(),
                replace_value: HashMap::from([(
                    "prod".to_owned(),
                    "prod-server.database.windows.net".to_owned(),
                )]),
                is_regex: false,
                item_type: None,
                item_name: None,
                file_path: None,
            }],
            key_value_replace: Vec::new(),
            spark_pool: Vec::new(),
            semantic_model_binding: None,
        };

        let content = r#"{"server": "old-server.database.windows.net"}"#;
        let encoded = BASE64.encode(content.as_bytes());

        let mut source = SourceWorkspace {
            items: vec![SourceItem {
                metadata: super::super::platform::PlatformMetadata {
                    item_type: "DataPipeline".to_owned(),
                    display_name: "ETL".to_owned(),
                    logical_id: None,
                    description: None,
                    definition_format: None,
                },
                parts: vec![DefinitionPart {
                    path: "pipeline-content.json".to_owned(),
                    payload: encoded,
                    payload_type: "InlineBase64".to_owned(),
                }],
                content_hash: "sha256:old".to_owned(),
                source_path: std::path::PathBuf::from("/tmp"),
            }],
            logical_id_index: HashMap::new(),
            type_name_index: HashMap::new(),
        };

        let ctx = SubstitutionContext {
            workspace_id: "ws-1",
            workspace_name: None,
            deployed_items: &HashMap::new(),
        };

        let warnings = apply_parameters(&mut source, &params, "prod", &ctx).unwrap();
        assert!(warnings.is_empty());

        // Verify the payload was substituted
        let new_payload = &source.items[0].parts[0].payload;
        let decoded = BASE64.decode(new_payload).unwrap();
        let text = String::from_utf8(decoded).unwrap();
        assert!(text.contains("prod-server.database.windows.net"));
        assert!(!text.contains("old-server.database.windows.net"));

        // Hash should have changed
        assert_ne!(source.items[0].content_hash, "sha256:old");
    }

    #[test]
    fn test_substitution_scoped_by_item_type() {
        let params = Parameters {
            find_replace: vec![FindReplaceRule {
                find_value: "REPLACE_ME".to_owned(),
                replace_value: HashMap::from([("prod".to_owned(), "REPLACED".to_owned())]),
                is_regex: false,
                item_type: Some(StringOrVec::Single("Notebook".to_owned())),
                item_name: None,
                file_path: None,
            }],
            key_value_replace: Vec::new(),
            spark_pool: Vec::new(),
            semantic_model_binding: None,
        };

        let content = "REPLACE_ME";
        let encoded = BASE64.encode(content.as_bytes());

        let mut source = SourceWorkspace {
            items: vec![
                SourceItem {
                    metadata: super::super::platform::PlatformMetadata {
                        item_type: "Notebook".to_owned(),
                        display_name: "NB1".to_owned(),
                        logical_id: None,
                        description: None,
                        definition_format: None,
                    },
                    parts: vec![DefinitionPart {
                        path: "notebook-content.py".to_owned(),
                        payload: encoded.clone(),
                        payload_type: "InlineBase64".to_owned(),
                    }],
                    content_hash: "sha256:a".to_owned(),
                    source_path: std::path::PathBuf::from("/tmp"),
                },
                SourceItem {
                    metadata: super::super::platform::PlatformMetadata {
                        item_type: "DataPipeline".to_owned(),
                        display_name: "PL1".to_owned(),
                        logical_id: None,
                        description: None,
                        definition_format: None,
                    },
                    parts: vec![DefinitionPart {
                        path: "pipeline-content.json".to_owned(),
                        payload: encoded,
                        payload_type: "InlineBase64".to_owned(),
                    }],
                    content_hash: "sha256:b".to_owned(),
                    source_path: std::path::PathBuf::from("/tmp"),
                },
            ],
            logical_id_index: HashMap::new(),
            type_name_index: HashMap::new(),
        };

        let ctx = SubstitutionContext {
            workspace_id: "ws-1",
            workspace_name: None,
            deployed_items: &HashMap::new(),
        };

        apply_parameters(&mut source, &params, "prod", &ctx).unwrap();

        // Notebook should be substituted
        let nb_payload = BASE64.decode(&source.items[0].parts[0].payload).unwrap();
        assert_eq!(String::from_utf8(nb_payload).unwrap(), "REPLACED");

        // Pipeline should NOT be substituted (wrong type)
        let pl_payload = BASE64.decode(&source.items[1].parts[0].payload).unwrap();
        assert_eq!(String::from_utf8(pl_payload).unwrap(), "REPLACE_ME");
    }
}
