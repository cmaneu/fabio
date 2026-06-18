use serde::Serialize;

use crate::cli::Cli;
use crate::output;

/// Schema version for the agent-context output. Bump on breaking changes.
const SCHEMA_VERSION: &str = "2";

#[derive(Serialize)]
struct PortalOnlyOp {
    operation: &'static str,
    item_type: &'static str,
    reason: &'static str,
}

#[derive(Serialize)]
struct Flag {
    name: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    description: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<&'static str>,
}

#[derive(Serialize)]
struct EnvVar {
    name: &'static str,
    description: &'static str,
    default: &'static str,
}

#[derive(Serialize)]
struct ErrorCodeInfo {
    code: &'static str,
    description: &'static str,
    exit_code: u8,
}

pub(super) fn execute(cli: &Cli) {
    // Build the JSON object field-by-field to avoid deep serde recursion on the stack.
    // On Windows the default stack is ~1 MB; serde_json::to_value() on a deeply nested
    // 146 KB JSON tree overflows it. By constructing the envelope manually and inserting
    // the pre-parsed serde_json::Value blobs directly we keep stack depth bounded.
    let mut value = serde_json::Map::new();
    value.insert(
        "schema_version".to_owned(),
        serde_json::json!(SCHEMA_VERSION),
    );
    value.insert("name".to_owned(), serde_json::json!("fabio"));
    value.insert(
        "version".to_owned(),
        serde_json::json!(env!("CARGO_PKG_VERSION")),
    );
    value.insert(
        "description".to_owned(),
        serde_json::json!("Agent-native CLI for managing Microsoft Fabric artifacts and data"),
    );
    value.insert(
        "global_flags".to_owned(),
        serde_json::to_value(global_flags()).expect("serialize global_flags"),
    );
    value.insert(
        "environment_variables".to_owned(),
        serde_json::to_value(environment_variables()).expect("serialize env_vars"),
    );
    // Large pre-parsed blobs inserted directly — no recursive to_value traversal.
    value.insert("commands".to_owned(), commands_schema());
    value.insert(
        "error_codes".to_owned(),
        serde_json::to_value(error_codes()).expect("serialize error_codes"),
    );
    value.insert("job_types".to_owned(), job_types());
    value.insert("definition_paths".to_owned(), definition_paths());
    value.insert(
        "portal_only_operations".to_owned(),
        serde_json::to_value(portal_only_operations()).expect("serialize portal_ops"),
    );
    value.insert("workflows".to_owned(), workflows());
    value.insert("output_conventions".to_owned(), output_conventions());

    let obj = serde_json::Value::Object(value);
    output::render_object(cli, &obj, "name");
}

fn global_flags() -> Vec<Flag> {
    vec![
        Flag {
            name: "--output",
            kind: "enum",
            description: "Output format",
            default: Some("json"),
        },
        Flag {
            name: "--json",
            kind: "bool",
            description: "Shorthand for --output json",
            default: Some("false"),
        },
        Flag {
            name: "--query",
            kind: "string",
            description: "JMESPath query expression (e.g., 'id', '[*].name', '[?size>`10`].id'). See https://jmespath.org/",
            default: None,
        },
        Flag {
            name: "--quiet",
            kind: "bool",
            description: "Suppress all stdout output",
            default: Some("false"),
        },
        Flag {
            name: "--force",
            kind: "bool",
            description: "Skip confirmation prompts for destructive operations",
            default: Some("false"),
        },
        Flag {
            name: "--dry-run",
            kind: "bool",
            description: "Preview what would happen without making changes",
            default: Some("false"),
        },
        Flag {
            name: "--limit",
            kind: "integer",
            description: "Maximum number of items to return in list commands",
            default: None,
        },
        Flag {
            name: "--all",
            kind: "bool",
            description: "Fetch all pages (auto-paginate). Without this, only the first page is returned with a continuationToken for manual pagination.",
            default: Some("false"),
        },
        Flag {
            name: "--continuation-token",
            kind: "string",
            description: "Resume pagination from a specific continuation token (returned by a previous list call)",
            default: None,
        },
        Flag {
            name: "--profile",
            kind: "string",
            description: "Use a named profile for default settings",
            default: None,
        },
        Flag {
            name: "--lro-timeout",
            kind: "integer",
            description: "Maximum seconds to wait for long-running operations (default: 120)",
            default: Some("120"),
        },
        Flag {
            name: "--verbose",
            kind: "bool",
            description: "Enable HTTP/LRO/auth diagnostic tracing on stderr. For debugging only — do not use in normal operation. Suppressed by --quiet.",
            default: Some("false"),
        },
    ]
}

fn environment_variables() -> Vec<EnvVar> {
    vec![
        EnvVar {
            name: "FABIO_FABRIC_API_ENDPOINT",
            description: "Override the Fabric REST API base URL (for sovereign clouds or private link)",
            default: "https://api.fabric.microsoft.com/v1",
        },
        EnvVar {
            name: "FABIO_ONELAKE_DFS_ENDPOINT",
            description: "Override the OneLake DFS base URL",
            default: "https://onelake.dfs.fabric.microsoft.com",
        },
        EnvVar {
            name: "FABIO_ONELAKE_BLOB_ENDPOINT",
            description: "Override the OneLake Blob base URL",
            default: "https://onelake.blob.fabric.microsoft.com",
        },
        EnvVar {
            name: "FABIO_ARM_ENDPOINT",
            description: "Override the Azure Resource Manager base URL",
            default: "https://management.azure.com",
        },
        EnvVar {
            name: "FABIO_FABRIC_SCOPE",
            description: "Override the Fabric API token scope",
            default: "https://api.fabric.microsoft.com/.default",
        },
        EnvVar {
            name: "FABIO_STORAGE_SCOPE",
            description: "Override the Azure Storage token scope",
            default: "https://storage.azure.com/.default",
        },
        EnvVar {
            name: "FABIO_SQL_SCOPE",
            description: "Override the SQL/TDS token scope",
            default: "https://database.windows.net/.default",
        },
        EnvVar {
            name: "FABIO_ARM_SCOPE",
            description: "Override the Azure Resource Manager token scope",
            default: "https://management.azure.com/.default",
        },
        EnvVar {
            name: "FABIO_POWERBI_ENDPOINT",
            description: "Override the Power BI REST API base URL (used by --api powerbi)",
            default: "https://api.powerbi.com/v1.0/myorg",
        },
    ]
}

fn error_codes() -> Vec<ErrorCodeInfo> {
    vec![
        ErrorCodeInfo {
            code: "AUTH_REQUIRED",
            description: "No valid credentials found. Run 'fabio auth login'.",
            exit_code: 1,
        },
        ErrorCodeInfo {
            code: "FORBIDDEN",
            description: "Insufficient permissions. Check workspace role (Admin/Member/Contributor/Viewer) and API scopes.",
            exit_code: 1,
        },
        ErrorCodeInfo {
            code: "NOT_FOUND",
            description: "Requested resource does not exist.",
            exit_code: 1,
        },
        ErrorCodeInfo {
            code: "CONFLICT",
            description: "Resource already exists or state conflict.",
            exit_code: 1,
        },
        ErrorCodeInfo {
            code: "RATE_LIMITED",
            description: "Too many requests. Retry after backoff.",
            exit_code: 1,
        },
        ErrorCodeInfo {
            code: "CAPACITY_INACTIVE",
            description: "Fabric capacity is paused or inactive.",
            exit_code: 1,
        },
        ErrorCodeInfo {
            code: "INVALID_INPUT",
            description: "Invalid argument value or missing required field.",
            exit_code: 1,
        },
        ErrorCodeInfo {
            code: "API_ERROR",
            description: "Upstream Fabric API returned an error.",
            exit_code: 1,
        },
        ErrorCodeInfo {
            code: "TIMEOUT",
            description: "Operation exceeded maximum wait time.",
            exit_code: 1,
        },
        ErrorCodeInfo {
            code: "NETWORK_ERROR",
            description: "Network connectivity issue.",
            exit_code: 1,
        },
    ]
}

fn job_types() -> serde_json::Value {
    serde_json::from_str(include_str!("data/agent/job_types.json"))
        .expect("job_types.json must contain valid JSON")
}

fn definition_paths() -> serde_json::Value {
    serde_json::from_str(include_str!("data/agent/definition_paths.json"))
        .expect("definition_paths.json must contain valid JSON")
}

fn portal_only_operations() -> Vec<PortalOnlyOp> {
    vec![
        PortalOnlyOp {
            operation: "initialize",
            item_type: "GraphModel",
            reason: "New 4-part definition format (graphType/graphDefinition/dataSources/styling) is documented for CI/CD but data loading does not complete via REST API alone (Jun 2026). Refresh triggers without VersionConfig error but stays NotStarted. Portal initialization still needed for the graph to become queryable.",
        },
        PortalOnlyOp {
            operation: "configure-kql-source",
            item_type: "Reflex",
            reason: "kqlSource-v1 is officially documented (Mar 2026) but updateDefinition still returns 'Invalid definition' (previously 'importArtifactRequest field is required'). Configure KQL sources through the portal first.",
        },
        PortalOnlyOp {
            operation: "configure-credentials",
            item_type: "SemanticModel (DirectQuery)",
            reason: "OAuth2 credentials cannot be created via REST API (only 1-hour raw access tokens via Gateways API). Fabric Connections API excludes OAuth2Credentials from create/update schemas. Use ServicePrincipal, WorkspaceIdentity, or Direct Lake instead.",
        },
    ]
}

fn commands_schema() -> serde_json::Value {
    serde_json::from_str(include_str!("data/agent/commands.json"))
        .expect("commands.json must contain valid JSON")
}

fn workflows() -> serde_json::Value {
    serde_json::from_str(include_str!("data/agent/workflows.json"))
        .expect("workflows.json must contain valid JSON")
}

fn output_conventions() -> serde_json::Value {
    serde_json::from_str(include_str!("data/agent/output_conventions.json"))
        .expect("output_conventions.json must contain valid JSON")
}
