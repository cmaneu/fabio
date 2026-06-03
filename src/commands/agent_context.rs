use anyhow::Result;
use serde::Serialize;

use crate::cli::Cli;
use crate::output;

/// Schema version for the agent-context output. Bump on breaking changes.
const SCHEMA_VERSION: &str = "1";

#[derive(Serialize)]
struct AgentContext {
    schema_version: &'static str,
    name: &'static str,
    version: String,
    description: &'static str,
    global_flags: Vec<Flag>,
    environment_variables: Vec<EnvVar>,
    commands: serde_json::Value,
    error_codes: Vec<ErrorCodeInfo>,
    job_types: serde_json::Value,
    definition_paths: serde_json::Value,
    portal_only_operations: Vec<PortalOnlyOp>,
}

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

pub fn execute(cli: &Cli) -> Result<()> {
    let context = AgentContext {
        schema_version: SCHEMA_VERSION,
        name: "fabio",
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Agent-first CLI for managing Microsoft Fabric artifacts and data",
        global_flags: global_flags(),
        environment_variables: environment_variables(),
        commands: commands_schema(),
        error_codes: error_codes(),
        job_types: job_types(),
        definition_paths: definition_paths(),
        portal_only_operations: portal_only_operations(),
    };

    let value = serde_json::to_value(&context)?;
    output::render_object(cli, &value, "name");
    Ok(())
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
            description: "Dot-notation field projection (e.g., 'id' or 'data.name')",
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
    serde_json::json!({
        "Notebook": ["RunNotebook"],
        "DataPipeline": ["Pipeline"],
        "SparkJobDefinition": ["sparkjob"],
        "Lakehouse": ["tableMaintenance", "refreshMaterializedLakeViews"],
        "SemanticModel": ["refresh"],
        "GraphModel": ["RefreshGraph"],
        "Eventstream": ["RunEventstream"],
        "MirroredDatabase": ["startMirroring", "stopMirroring"],
        "SQLDatabase": ["startMirroring", "stopMirroring"]
    })
}

fn definition_paths() -> serde_json::Value {
    serde_json::json!({
        "Reflex": "ReflexEntities.json",
        "CopyJob": "CopyJobV1.json",
        "Dataflow": "dataflow.json",
        "KQLQueryset": "RealTimeQueryset.json",
        "KQLDashboard": "RealTimeDashboard.json",
        "Map": "map.json",
        "Ontology": "definition.json + EntityTypes/{ID}/definition.json + DataBindings/{UUID}.json",
        "Notebook": "notebook-content.py (format: ipynb)",
        "DataAgent": "Files/Config/data_agent.json + Files/Config/draft/stage_config.json + Files/Config/draft/{type}-{Name}/datasource.json",
        "Eventstream": "eventstream.json + eventstreamProperties.json",
        "GraphQLApi": "graphql-definition.json",
        "Report": "definition.pbir + report.json (PBIR-Legacy) or definition/ folder (PBIR)",
        "SemanticModel": "definition.pbism + model.tmdl files or model.bim",
        "SparkJobDefinition": "SparkJobDefinitionV1.json",
        "Environment": "environment.metadata.json",
        "MirroredDatabase": "mirroring.json"
    })
}

fn portal_only_operations() -> Vec<PortalOnlyOp> {
    vec![
        PortalOnlyOp {
            operation: "publish",
            item_type: "DataAgent",
            reason: "Publishing activates the chat endpoint. No REST API endpoint exists. Use the portal Publish button.",
        },
        PortalOnlyOp {
            operation: "initialize",
            item_type: "GraphModel",
            reason: "First-time graph loading provisions internal VersionConfig. REST API refresh fails until the graph is opened in the portal.",
        },
        PortalOnlyOp {
            operation: "configure-kql-source",
            item_type: "Reflex",
            reason: "KQL source via REST API always fails with 'importArtifactRequest field is required'. Configure through portal, then manage definitions programmatically.",
        },
        PortalOnlyOp {
            operation: "configure-credentials",
            item_type: "SemanticModel (DirectQuery)",
            reason: "DirectQuery OAuth2 credentials require interactive portal binding via 'Manage connections and gateways'. Direct Lake avoids this issue.",
        },
    ]
}

#[allow(clippy::too_many_lines, clippy::large_stack_frames)]
fn commands_schema() -> serde_json::Value {
    serde_json::json!({
        "auth": {
            "description": "Manage authentication",
            "subcommands": {
                "login": {
                    "description": "Log in to Microsoft Fabric",
                    "mutates": false,
                    "flags": {
                        "--device-code": {"type": "bool", "description": "Use device code flow instead of browser"},
                        "--tenant": {"type": "string", "description": "Azure AD tenant ID"}
                    }
                },
                "logout": {
                    "description": "Log out and clear cached credentials",
                    "mutates": true
                },
                "status": {
                    "description": "Show current authentication status",
                    "mutates": false
                }
            }
        },
        "workspace": {
            "description": "Manage workspaces (46 subcommands)",
            "subcommands": {
                "list": {"description": "List all workspaces", "mutates": false, "flags": {"--roles": {"type": "string", "description": "Filter by role: Admin,Member,Contributor,Viewer"}, "--capacity": {"type": "string", "description": "Filter by capacity ID (client-side)"}}},
                "show": {"description": "Show workspace details", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a workspace", "mutates": true, "flags": {"--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update workspace name/description", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a workspace", "mutates": true, "destructive": true, "flags": {"--id": {"type": "string", "required": true}}},
                "assign-capacity": {"description": "Assign workspace to a capacity", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--capacity": {"type": "string", "required": true}}},
                "unassign-capacity": {"description": "Unassign workspace from capacity", "mutates": true, "flags": {"--id": {"type": "string", "required": true}}},
                "provision-identity": {"description": "Provision workspace identity (service principal)", "mutates": true, "async": true, "flags": {"--id": {"type": "string", "required": true}}},
                "deprovision-identity": {"description": "Deprovision workspace identity", "mutates": true, "flags": {"--id": {"type": "string", "required": true}}},
                "list-role-assignments": {"description": "List workspace role assignments", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "add-role-assignment": {"description": "Add a role assignment", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--principal-id": {"type": "string", "required": true}, "--principal-type": {"type": "enum", "values": ["User", "Group", "ServicePrincipal", "ServicePrincipalProfile"]}, "--role": {"type": "enum", "values": ["Admin", "Member", "Contributor", "Viewer"]}}},
                "update-role-assignment": {"description": "Update a role assignment", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--principal-id": {"type": "string", "required": true}, "--principal-type": {"type": "string"}, "--role": {"type": "string", "required": true}}},
                "delete-role-assignment": {"description": "Delete a role assignment", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--principal-id": {"type": "string", "required": true}, "--principal-type": {"type": "string"}}},
                "get-settings": {"description": "Get workspace settings and properties", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "update-settings": {"description": "Update workspace settings via PATCH", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "get-network-policy": {"description": "Get workspace network communication policy", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "set-network-policy": {"description": "Set workspace network communication policy", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "get-firewall-rules": {"description": "Get IP firewall rules", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "set-firewall-rules": {"description": "Set IP firewall rules (replaces all, max 256)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "get-git-outbound": {"description": "Get git outbound policy", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "set-git-outbound": {"description": "Set git outbound policy (requires OAP)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "get-inbound-azure-resources": {"description": "Get inbound Azure resource rules", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "set-inbound-azure-resources": {"description": "Set inbound Azure resource rules", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "get-outbound-cloud-connections": {"description": "Get outbound cloud connection rules", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "set-outbound-cloud-connections": {"description": "Set outbound cloud connection rules", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "get-outbound-gateways": {"description": "Get outbound gateway rules", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "set-outbound-gateways": {"description": "Set outbound gateway rules", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "get-onelake-settings": {"description": "Get OneLake settings (tier, diagnostics, immutability)", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "modify-default-tier": {"description": "Modify OneLake default tier (Hot/Cool/Cold)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--tier": {"type": "enum", "values": ["Hot", "Cool", "Cold"]}}},
                "modify-diagnostics": {"description": "Modify OneLake diagnostics settings", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "modify-immutability-policy": {"description": "Modify OneLake immutability policy", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "export-lifecycle-policy": {"description": "Export OneLake lifecycle policy", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "import-lifecycle-policy": {"description": "Import OneLake lifecycle policy", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "reset-shortcut-cache": {"description": "Reset OneLake shortcut cache", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}}},
                "assign-to-domain": {"description": "Assign workspace to a domain", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--domain-id": {"type": "string", "required": true}}},
                "unassign-from-domain": {"description": "Unassign workspace from domain", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}}},
                "get-storage-format": {"description": "Get default dataset storage format", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "set-storage-format": {"description": "Set default dataset storage format (Small/Large)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--format": {"type": "enum", "values": ["Small", "Large"]}}},
                "apply-tags": {"description": "Apply tags to workspace", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--tag-ids": {"type": "string", "required": true}}},
                "unapply-tags": {"description": "Remove tags from workspace", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--tag-ids": {"type": "string", "required": true}}},
                "list-folders": {"description": "List folders in workspace", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "create-folder": {"description": "Create a folder", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--parent-folder-id": {"type": "string"}}},
                "update-folder": {"description": "Update folder name/description", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--folder-id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete-folder": {"description": "Delete a folder", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--folder-id": {"type": "string", "required": true}}},
                "move-folder": {"description": "Move a folder", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--folder-id": {"type": "string", "required": true}, "--target-folder-id": {"type": "string"}}},
                "list-items": {"description": "List all items in workspace", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--type": {"type": "string"}}},
                "url": {"description": "Get Fabric portal URL for a workspace", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}}
            }
        },
        "item": {
            "description": "Manage items (18 subcommands: CRUD + copy/move + definitions + exists/url/inspect + bulk-create/bulk-delete + move-to-folder + external-data-share)",
            "subcommands": {
                "list": {
                    "description": "List items in a workspace",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--type": {"type": "string", "description": "Filter by item type (e.g., Notebook, Lakehouse, Warehouse)"},
                        "--folder": {"type": "string", "description": "Filter by folder ID (server-side rootFolderId)"},
                        "--recursive": {"type": "bool", "description": "Include items in subfolders"},
                        "--include": {"type": "string", "description": "Additional metadata to include in response"}
                    }
                },
                "show": {
                    "description": "Show details of an item",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"}
                    }
                },
                "get-definition": {
                    "description": "Get the definition (source code/content) of an item",
                    "mutates": false,
                    "async": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"},
                        "--format": {"type": "string", "description": "Definition format (optional, item-type dependent)"}
                    }
                },
                "list-connections": {
                    "description": "List connections used by an item",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"}
                    }
                },
                "create": {
                    "description": "Create a new item",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--name": {"type": "string", "required": true, "description": "Item display name"},
                        "--type": {"type": "string", "required": true, "description": "Item type (e.g., Lakehouse, Warehouse)"},
                        "--description": {"type": "string", "description": "Optional description"}
                    }
                },
                "update": {
                    "description": "Update item properties (name and/or description)",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"},
                        "--name": {"type": "string", "description": "New display name"},
                        "--description": {"type": "string", "description": "New description"}
                    }
                },
                "update-definition": {
                    "description": "Update (override) item definition from file or inline JSON",
                    "mutates": true,
                    "async": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"},
                        "--file": {"type": "string", "description": "Path to definition file (base64-encoded as single part)"},
                        "--definition": {"type": "string", "description": "Inline JSON definition payload with parts array"},
                        "--update-metadata": {"type": "bool", "description": "Also update item metadata from .platform file"}
                    }
                },
                "delete": {
                    "description": "Delete an item",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"},
                        "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}
                    }
                },
                "move-to-folder": {
                    "description": "Move an item to a folder within the same workspace (or to root)",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"},
                        "--folder-id": {"type": "string", "description": "Target folder ID (omit to move to workspace root)"}
                    }
                },
                "bulk-create": {
                    "description": "Create multiple items in parallel from JSON spec",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--content": {"type": "string", "required": true, "description": "JSON array of items to create"}
                    }
                },
                "bulk-delete": {
                    "description": "Delete multiple items in parallel",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--ids": {"type": "string", "required": true, "description": "Comma-separated item IDs"},
                        "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}
                    }
                },
                "create-external-data-share": {
                    "description": "Create an external data share for an item",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"},
                        "--paths": {"type": "string", "required": true, "description": "Comma-separated paths to share"},
                        "--recipient-tenant-id": {"type": "string", "required": true, "description": "Recipient tenant ID"},
                        "--recipient-type": {"type": "string", "description": "Recipient type (User or ServicePrincipal)"},
                        "--recipient-id": {"type": "string", "description": "Recipient principal ID (required if --recipient-type is set)"}
                    }
                },
                "copy": {
                    "description": "Copy an item to another workspace",
                    "mutates": true,
                    "flags": {
                        "--source-workspace": {"type": "string", "required": true, "description": "Source workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID to copy"},
                        "--dest-workspace": {"type": "string", "required": true, "description": "Destination workspace ID"},
                        "--name": {"type": "string", "description": "New name for the copy"}
                    }
                },
                "move": {
                    "description": "Move an item to another workspace (copy + delete source)",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--source-workspace": {"type": "string", "required": true, "description": "Source workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID to move"},
                        "--dest-workspace": {"type": "string", "required": true, "description": "Destination workspace ID"},
                        "--name": {"type": "string", "description": "New name"}
                    }
                },
                "exists": {
                    "description": "Check if an item exists (returns {exists: true/false}, never errors on 404)",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"}
                    }
                },
                "url": {
                    "description": "Get Fabric portal URL for an item",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"},
                        "--type": {"type": "string", "description": "Item type for accurate portal path segment"}
                    }
                },
                "inspect": {
                    "description": "Aggregated view: metadata + definition (best-effort) + connections (best-effort)",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Item ID"}
                    }
                }
            }
        },
        "lakehouse": {
            "description": "Manage lakehouses (tables, files, shortcuts, maintenance)",
            "subcommands": {
                "list": {"description": "List lakehouses in a workspace", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show lakehouse details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a lakehouse", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}, "--enable-schemas": {"type": "bool", "description": "Enable multi-schema lakehouse"}}},
                "update": {"description": "Update lakehouse properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a lakehouse", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "list-tables": {
                    "description": "List tables in a lakehouse",
                    "aliases": ["tables"],
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Lakehouse ID"}
                    }
                },
                "list-files": {
                    "description": "List files in a lakehouse",
                    "aliases": ["files"],
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Lakehouse ID"},
                        "--path": {"type": "string", "description": "Directory path to list"}
                    }
                },
                "upload": {
                    "description": "Upload a file to a lakehouse",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Lakehouse ID"},
                        "--source-path": {"type": "string", "required": true, "description": "Local source path"},
                        "--dest-path": {"type": "string", "required": true, "description": "Remote destination path"}
                    }
                },
                "download": {
                    "description": "Download a file from a lakehouse",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Lakehouse ID"},
                        "--source-path": {"type": "string", "required": true, "description": "Remote source path"},
                        "--dest-path": {"type": "string", "required": true, "description": "Local destination path"}
                    }
                },
                "load-table": {
                    "description": "Load a file into a Delta table",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Lakehouse ID"},
                        "--source-path": {"type": "string", "required": true, "description": "Relative path to source file"},
                        "--table": {"type": "string", "required": true, "description": "Table name"},
                        "--mode": {"type": "enum", "values": ["Overwrite", "Append"], "default": "Overwrite"},
                        "--format": {"type": "enum", "values": ["Csv", "Parquet", "Json"], "default": "Csv"},
                        "--schema": {"type": "string", "description": "Schema name for multi-schema lakehouses (uses beta API)"}
                    }
                },
                "copy-file": {
                    "description": "Copy a file between lakehouses (server-side)",
                    "mutates": true,
                    "flags": {
                        "--source-workspace": {"type": "string", "required": true},
                        "--source-id": {"type": "string", "required": true},
                        "--source-path": {"type": "string", "required": true},
                        "--dest-workspace": {"type": "string", "required": true},
                        "--dest-id": {"type": "string", "required": true},
                        "--dest-path": {"type": "string", "required": true}
                    }
                },
                "delete-file": {
                    "description": "Delete a file from a lakehouse",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--path": {"type": "string", "required": true, "description": "File path to delete"}
                    }
                },
                "move-file": {
                    "description": "Move a file between lakehouses (copy + delete source)",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--source-workspace": {"type": "string", "required": true},
                        "--source-id": {"type": "string", "required": true},
                        "--source-path": {"type": "string", "required": true},
                        "--dest-workspace": {"type": "string", "required": true},
                        "--dest-id": {"type": "string", "required": true},
                        "--dest-path": {"type": "string", "required": true}
                    }
                },
                "delete-table": {
                    "description": "Delete a table from a lakehouse",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--table": {"type": "string", "required": true, "description": "Table name (supports glob patterns)"}
                    }
                },
                "copy-table": {
                    "description": "Copy a table between lakehouses",
                    "mutates": true,
                    "flags": {
                        "--source-workspace": {"type": "string", "required": true},
                        "--source-id": {"type": "string", "required": true},
                        "--source-table": {"type": "string", "required": true, "description": "Source table name (supports glob)"},
                        "--dest-workspace": {"type": "string", "required": true},
                        "--dest-id": {"type": "string", "required": true},
                        "--dest-table": {"type": "string", "description": "Destination table name"}
                    }
                },
                "move-table": {
                    "description": "Move a table between lakehouses (copy + delete source)",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--source-workspace": {"type": "string", "required": true},
                        "--source-id": {"type": "string", "required": true},
                        "--source-table": {"type": "string", "required": true},
                        "--dest-workspace": {"type": "string", "required": true},
                        "--dest-id": {"type": "string", "required": true},
                        "--dest-table": {"type": "string", "description": "Destination table name"}
                    }
                },
                "create-shortcut": {
                    "description": "Create a shortcut",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--name": {"type": "string", "required": true, "description": "Shortcut name"},
                        "--path": {"type": "string", "required": true, "description": "Shortcut path (Tables or Files)"},
                        "--target-type": {"type": "enum", "values": ["OneLake", "AdlsGen2", "S3"], "required": true},
                        "--target": {"type": "string", "required": true, "description": "Target body as JSON"}
                    }
                },
                "get-shortcut": {
                    "description": "Get shortcut details",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--name": {"type": "string", "required": true},
                        "--path": {"type": "string", "required": true}
                    }
                },
                "delete-shortcut": {
                    "description": "Delete a shortcut",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--name": {"type": "string", "required": true},
                        "--path": {"type": "string", "required": true}
                    }
                },
                "bulk-create-shortcuts": {
                    "description": "Bulk-create multiple shortcuts (LRO)",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--file": {"type": "string", "description": "Path to JSON file with shortcut requests"},
                        "--content": {"type": "string", "description": "Inline JSON with shortcut requests"},
                        "--conflict-policy": {"type": "string", "description": "Abort|GenerateUniqueName|CreateOrOverwrite|OverwriteOnly"}
                    },
                    "example": "fabio lakehouse bulk-create-shortcuts --workspace <WS> --id <ID> --file shortcuts.json"
                },
                "optimize-table": {
                    "description": "Optimize a Delta table (V-Order compaction + optional Z-Order clustering)",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Lakehouse ID"},
                        "--table": {"type": "string", "required": true, "description": "Table name"},
                        "--schema": {"type": "string", "description": "Schema name (multi-schema lakehouses)"},
                        "--vorder": {"type": "bool", "default": "true", "description": "Enable V-Order optimization"},
                        "--zorder": {"type": "string", "description": "Columns for Z-Order clustering (comma-separated)"}
                    },
                    "example": "fabio lakehouse optimize-table --workspace <WS> --id <ID> --table sales --zorder region,date"
                },
                "vacuum-table": {
                    "description": "Vacuum a Delta table (remove old files beyond retention period)",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Lakehouse ID"},
                        "--table": {"type": "string", "required": true, "description": "Table name"},
                        "--schema": {"type": "string", "description": "Schema name (multi-schema lakehouses)"},
                        "--retain-hours": {"type": "integer", "default": "168", "description": "Retention period in hours (default: 7 days)"}
                    },
                    "example": "fabio lakehouse vacuum-table --workspace <WS> --id <ID> --table logs --retain-hours 48"
                },
                "table-schema": {
                    "description": "Show Delta table schema (reads from OneLake _delta_log without Spark/SQL)",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--id": {"type": "string", "required": true, "description": "Lakehouse ID"},
                        "--table": {"type": "string", "required": true, "description": "Table name"}
                    },
                    "example": "fabio lakehouse table-schema --workspace <WS> --id <ID> --table customers"
                },
                "sync": {
                    "description": "Sync files between lakehouses (copies new/modified, optionally deletes orphans)",
                    "mutates": true,
                    "flags": {
                        "--source-workspace": {"type": "string", "required": true},
                        "--source-id": {"type": "string", "required": true},
                        "--dest-workspace": {"type": "string", "required": true},
                        "--dest-id": {"type": "string", "required": true},
                        "--path": {"type": "string", "description": "Subdirectory to sync (default: all Files)"},
                        "--delete": {"type": "bool", "description": "Delete files at destination not present in source"},
                        "--checksum": {"type": "bool", "description": "Use Content-MD5 comparison instead of ETag"}
                    }
                }
            }
        },
        "notebook": {
            "description": "Manage notebooks",
            "subcommands": {
                "list": {"description": "List notebooks in a workspace", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show notebook details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {
                    "description": "Create a new notebook",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--name": {"type": "string", "required": true, "description": "Notebook display name"},
                        "--content": {"type": "string", "description": "Notebook content (Python/PySpark code)"}
                    }
                },
                "get-definition": {
                    "description": "Get the definition (source code) of a notebook",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--strip-output": {"type": "bool", "description": "Clear outputs/execution_count from ipynb cells"}
                    }
                },
                "run": {
                    "description": "Run a notebook",
                    "mutates": true,
                    "async": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--parameters": {"type": "string", "description": "JSON array of {name, value, type} objects"},
                        "--compute-type": {"type": "string", "description": "Compute type (e.g. Spark, DataFactory)"},
                        "--execution-data": {"type": "string", "description": "Full executionData JSON (advanced)"},
                        "--wait": {"type": "bool", "description": "Wait for completion (polls until finished)"},
                        "--timeout": {"type": "integer", "default": "600", "description": "Maximum wait in seconds"}
                    }
                },
                "status": {
                    "description": "Check the status of a notebook run",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--job-id": {"type": "string", "required": true}
                    }
                },
                "stop": {
                    "description": "Stop a running notebook",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--job-id": {"type": "string", "required": true}
                    }
                },
                "delete": {
                    "description": "Delete a notebook",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true}
                    }
                }
            }
        },
        "warehouse": {
            "description": "Manage warehouses and run SQL queries",
            "subcommands": {
                "list": {"description": "List warehouses", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show warehouse details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a warehouse", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update warehouse properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a warehouse", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "query": {
                    "description": "Execute a SQL query against a warehouse",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--sql": {"type": "string", "description": "SQL query (prefix @ to read from file, omit for stdin)"}
                    }
                },
                "connection-string": {
                    "description": "Get TDS connection string for a warehouse",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--guest-tenant-id": {"type": "string", "description": "Guest tenant AAD object ID (for cross-tenant access)"},
                        "--private-link-type": {"type": "string", "description": "Private link type (Dfs or Blob)"}
                    }
                }
            }
        },
        "data-agent": {
            "description": "Manage data agents (create, query, and interact with AI agents)",
            "subcommands": {
                "list": {
                    "description": "List data agents in a workspace",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true}
                    }
                },
                "show": {
                    "description": "Show details of a data agent",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true}
                    }
                },
                "create": {
                    "description": "Create a new data agent",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--name": {"type": "string", "required": true},
                        "--description": {"type": "string", "description": "Max 256 characters"}
                    }
                },
                "update": {
                    "description": "Update a data agent",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--name": {"type": "string"},
                        "--description": {"type": "string"}
                    }
                },
                "delete": {
                    "description": "Delete a data agent",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true}
                    }
                },
                "query": {
                    "description": "Query (chat with) a published data agent using natural language",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--prompt": {"type": "string", "description": "Natural language question (omit for stdin)"}
                    }
                },
                "get-definition": {"description": "Get data agent definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update data agent definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "agent-context": {
            "description": "Machine-readable CLI schema for agent introspection",
            "mutates": false
        },
        "environment": {
            "description": "Manage environments (Spark compute, libraries, publish)",
            "subcommands": {
                "list": {"description": "List environments in a workspace", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details of an environment", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a new environment", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update environment properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete an environment", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "publish": {"description": "Publish staged changes", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "cancel-publish": {"description": "Cancel a pending publish", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-spark-settings": {"description": "Get published Spark settings", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-staging-spark-settings": {"description": "Get staging Spark settings", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "upload-staging-library": {"description": "Upload a library file to the staging area", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string", "required": true, "description": "Path to library file (.whl, .jar, .tar.gz)"}, "--library-name": {"type": "string", "description": "Override library name (defaults to filename)"}}}
            }
        },
        "data-pipeline": {
            "description": "Manage data pipelines (orchestration, scheduling)",
            "subcommands": {
                "list": {"description": "List data pipelines in a workspace", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details of a data pipeline", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a new data pipeline", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update data pipeline properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a data pipeline", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "run": {"description": "Run a data pipeline", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "eventhouse": {
            "description": "Manage eventhouses (real-time analytics)",
            "subcommands": {
                "list": {"description": "List eventhouses in a workspace", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details of an eventhouse", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a new eventhouse", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update eventhouse properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete an eventhouse", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}}
            }
        },
        "connection": {
            "description": "Manage connections (cloud, on-premises, virtual network)",
            "subcommands": {
                "list": {"description": "List all connections", "mutates": false},
                "show": {"description": "Show details of a connection", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a new connection", "mutates": true, "flags": {"--name": {"type": "string", "required": true}, "--connectivity-type": {"type": "enum", "values": ["ShareableCloud", "OnPremises", "VirtualNetworkGateway", "PersonalCloud"]}, "--connection-type": {"type": "string", "required": true}, "--parameters": {"type": "string", "required": true, "description": "JSON object"}, "--credential-type": {"type": "enum", "values": ["Basic", "OAuth2", "Key", "Anonymous", "ServicePrincipal", "SharedAccessSignature"]}}},
                "update": {"description": "Update a connection", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--privacy-level": {"type": "enum", "values": ["None", "Public", "Organizational", "Private"]}, "--credential-type": {"type": "string"}, "--credentials": {"type": "string", "description": "JSON"}}},
                "delete": {"description": "Delete a connection", "mutates": true, "destructive": true, "flags": {"--id": {"type": "string", "required": true}}},
                "list-supported-types": {"description": "List supported connection types catalog", "mutates": false}
            }
        },
        "profile": {
            "description": "Manage saved configuration profiles",
            "subcommands": {
                "save": {
                    "description": "Save a named profile with default settings",
                    "mutates": true,
                    "flags": {
                        "--name": {"type": "string", "required": true, "description": "Profile name"},
                        "--workspace": {"type": "string", "description": "Default workspace ID"},
                        "--capacity": {"type": "string", "description": "Default capacity ID"},
                        "--default-output": {"type": "string", "description": "Default output format"}
                    }
                },
                "use": {
                    "description": "Set the active profile",
                    "mutates": true,
                    "flags": {
                        "--name": {"type": "string", "required": true}
                    }
                },
                "list": {
                    "description": "List all saved profiles",
                    "mutates": false
                },
                "show": {
                    "description": "Show details of a profile",
                    "mutates": false,
                    "flags": {
                        "--name": {"type": "string", "required": true}
                    }
                },
                "delete": {
                    "description": "Delete a profile",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--name": {"type": "string", "required": true}
                    }
                }
            }
        },
        "jobs": {
            "description": "Inspect and manage async job history",
            "subcommands": {
                "list": {
                    "description": "List recent jobs from local ledger",
                    "mutates": false,
                    "flags": {
                        "--status": {"type": "string", "description": "Filter by status (running, completed, failed)"}
                    },
                    "notes": "Uses global --limit (default 20 if unset)"
                },
                "get": {
                    "description": "Get details of a specific job",
                    "mutates": false,
                    "flags": {
                        "--id": {"type": "string", "required": true}
                    }
                },
                "prune": {
                    "description": "Remove completed/failed jobs from the ledger",
                    "mutates": true,
                    "flags": {
                        "--include-running": {"type": "bool", "description": "Remove all jobs including currently running ones"}
                    }
                }
            }
        },
        "job-scheduler": {
            "description": "Manage item job scheduling (run on demand, cancel, CRUD schedules)",
            "subcommands": {
                "list-instances": {"description": "List job instances for an item", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-instance": {"description": "Get details of a job instance", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--job-instance-id": {"type": "string", "required": true}}},
                "run-on-demand": {"description": "Run an on-demand job", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--job-type": {"type": "string", "default": "DefaultJob"}, "--execution-data": {"type": "string", "description": "JSON"}, "--wait": {"type": "bool", "description": "Poll until job completes"}, "--timeout": {"type": "u64", "default": "600", "description": "Max seconds to wait"}, "--cancel-on-timeout": {"type": "bool", "description": "Cancel job if timeout expires"}}},
                "cancel-instance": {"description": "Cancel a running job instance", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--job-instance-id": {"type": "string", "required": true}}},
                "list-schedules": {"description": "List schedules for an item job type", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--job-type": {"type": "string", "default": "DefaultJob"}}},
                "get-schedule": {"description": "Get schedule details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--job-type": {"type": "string"}, "--schedule-id": {"type": "string", "required": true}}},
                "create-schedule": {"description": "Create a schedule", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--job-type": {"type": "string"}, "--enabled": {"type": "bool"}, "--config": {"type": "string", "required": true, "description": "JSON schedule config"}}},
                "update-schedule": {"description": "Update a schedule", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--job-type": {"type": "string"}, "--schedule-id": {"type": "string", "required": true}, "--enabled": {"type": "bool"}, "--config": {"type": "string"}}},
                "delete-schedule": {"description": "Delete a schedule", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--job-type": {"type": "string"}, "--schedule-id": {"type": "string", "required": true}}}
            }
        },
        "deployment-pipeline": {
            "description": "Manage deployment pipelines (CI/CD stages, deploy items between stages)",
            "subcommands": {
                "list": {"description": "List deployment pipelines", "mutates": false},
                "show": {"description": "Show deployment pipeline details", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a deployment pipeline", "mutates": true, "flags": {"--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update a deployment pipeline", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a deployment pipeline", "mutates": true, "destructive": true, "flags": {"--id": {"type": "string", "required": true}}},
                "list-stages": {"description": "List stages in a pipeline", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "list-stage-items": {"description": "List items in a stage", "mutates": false, "flags": {"--id": {"type": "string", "required": true}, "--stage-id": {"type": "string", "required": true}}},
                "assign-workspace": {"description": "Assign workspace to a stage", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--stage-id": {"type": "string", "required": true}, "--workspace": {"type": "string", "required": true}}},
                "unassign-workspace": {"description": "Unassign workspace from a stage", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--stage-id": {"type": "string", "required": true}}},
                "deploy": {"description": "Deploy items between stages", "mutates": true, "async": true, "flags": {"--id": {"type": "string", "required": true}, "--source-stage-id": {"type": "string", "required": true}, "--target-stage-id": {"type": "string"}, "--items": {"type": "string", "description": "JSON array"}, "--note": {"type": "string"}}}
            }
        },
        "domain": {
            "description": "Manage domains (organize workspaces into business domains)",
            "subcommands": {
                "list": {"description": "List domains in the tenant", "mutates": false},
                "show": {"description": "Show domain details", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a domain", "mutates": true, "flags": {"--name": {"type": "string", "required": true}, "--description": {"type": "string"}, "--parent-domain-id": {"type": "string"}}},
                "update": {"description": "Update a domain", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a domain", "mutates": true, "destructive": true, "flags": {"--id": {"type": "string", "required": true}}},
                "list-workspaces": {"description": "List workspaces in a domain", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "assign-workspaces": {"description": "Assign workspaces to a domain", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--workspaces": {"type": "string", "required": true, "description": "Comma-separated workspace IDs"}}},
                "unassign-workspaces": {"description": "Unassign workspaces from a domain", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--workspaces": {"type": "string", "required": true}}},
                "assign-by-capacity": {"description": "Bulk-assign workspaces by capacity", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--capacities": {"type": "string", "required": true}}},
                "assign-by-principal": {"description": "Bulk-assign workspaces by principal", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--principals": {"type": "string", "required": true}, "--principal-type": {"type": "enum", "values": ["User", "Group", "ServicePrincipal"]}}}
            }
        },
        "spark": {
            "description": "Manage Spark compute (workspace settings, custom pools)",
            "subcommands": {
                "get-settings": {"description": "Get workspace Spark settings", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "update-settings": {"description": "Update workspace Spark settings", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--settings": {"type": "string", "required": true, "description": "JSON"}}},
                "list-pools": {"description": "List custom Spark pools", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "get-pool": {"description": "Get custom pool details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--pool-id": {"type": "string", "required": true}}},
                "create-pool": {"description": "Create a custom Spark pool", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--node-family": {"type": "string"}, "--node-size": {"type": "enum", "values": ["Small", "Medium", "Large", "XLarge", "XXLarge"]}}},
                "update-pool": {"description": "Update a custom pool", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--pool-id": {"type": "string", "required": true}, "--config": {"type": "string", "required": true, "description": "JSON"}}},
                "delete-pool": {"description": "Delete a custom pool", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--pool-id": {"type": "string", "required": true}}}
            }
        },
        "kql-database": {
            "description": "Manage KQL databases within eventhouses",
            "subcommands": {
                "list": {"description": "List KQL databases", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show KQL database details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a KQL database", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--eventhouse-id": {"type": "string", "required": true}, "--database-type": {"type": "enum", "values": ["ReadWrite", "ReadOnlyFollowing"]}}},
                "update": {"description": "Update KQL database properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a KQL database", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get KQL database definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update KQL database definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "query": {"description": "Execute a KQL query against a database", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--kql": {"type": "string", "required": true, "description": "KQL query text"}, "--query-uri": {"type": "string", "description": "Override Kusto query URI"}}}
            }
        },
        "mirrored-database": {
            "description": "Manage mirrored databases (real-time replication)",
            "subcommands": {
                "list": {"description": "List mirrored databases", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show mirrored database details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a mirrored database", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update mirrored database", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a mirrored database", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "start": {"description": "Start mirroring", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "stop": {"description": "Stop mirroring", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "status": {"description": "Get mirroring status", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "table-status": {"description": "Get tables mirroring status", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "capacity": {
            "description": "List and inspect Fabric capacities",
            "subcommands": {
                "list": {"description": "List available capacities", "mutates": false},
                "show": {"description": "Show capacity details", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}}
            }
        },
        "onelake-security": {
            "description": "Manage OneLake data access roles (row/column-level security)",
            "subcommands": {
                "list": {"description": "List data access roles for an item", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "show": {"description": "Show a data access role", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--role-name": {"type": "string", "required": true}}},
                "upsert": {"description": "Create or replace all data access roles", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--roles": {"type": "string", "required": true, "description": "JSON array or @file"}}},
                "delete": {"description": "Delete a data access role", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--role-name": {"type": "string", "required": true}}}
            }
        },
        "managed-private-endpoint": {
            "description": "Manage workspace managed private endpoints",
            "subcommands": {
                "list": {"description": "List managed private endpoints", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show endpoint details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a managed private endpoint", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--target-resource-id": {"type": "string", "required": true}, "--target-subresource-type": {"type": "string", "required": true}}},
                "delete": {"description": "Delete a managed private endpoint", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "eventstream": {
            "description": "Manage eventstreams (real-time data ingestion and routing)",
            "subcommands": {
                "list": {"description": "List eventstreams", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show eventstream details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create an eventstream", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update eventstream properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete an eventstream", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get eventstream definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update eventstream definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "get-topology": {"description": "Get eventstream topology (sources, streams, destinations)", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "pause": {"description": "Pause the eventstream", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "resume": {"description": "Resume the eventstream", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-source": {"description": "Get source details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--source-id": {"type": "string", "required": true}}},
                "pause-source": {"description": "Pause a source", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--source-id": {"type": "string", "required": true}}},
                "resume-source": {"description": "Resume a source", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--source-id": {"type": "string", "required": true}}},
                "get-source-connection": {"description": "Get source connection info (Event Hub endpoint)", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--source-id": {"type": "string", "required": true}}},
                "get-destination": {"description": "Get destination details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--destination-id": {"type": "string", "required": true}}},
                "pause-destination": {"description": "Pause a destination", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--destination-id": {"type": "string", "required": true}}},
                "resume-destination": {"description": "Resume a destination", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--destination-id": {"type": "string", "required": true}}},
                "get-destination-connection": {"description": "Get destination connection info", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--destination-id": {"type": "string", "required": true}}},
                "add-source": {"description": "Add a source to the eventstream", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--source-type": {"type": "string", "required": true}, "--properties": {"type": "string"}}},
                "add-destination": {"description": "Add a destination to the eventstream", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--destination-type": {"type": "string", "required": true}, "--input-node": {"type": "string", "required": true}, "--properties": {"type": "string"}}}
            }
        },
        "kql-queryset": {
            "description": "Manage KQL querysets (saved KQL queries)",
            "subcommands": {
                "list": {"description": "List KQL querysets", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show KQL queryset details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a KQL queryset", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update KQL queryset properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a KQL queryset", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get KQL queryset definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update KQL queryset definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "run": {"description": "Run a saved KQL query tab against its configured data source", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--tab": {"type": "string", "description": "Tab name or index (default: first tab)"}}}
            }
        },
        "spark-job-definition": {
            "description": "Manage Spark job definitions (batch Spark jobs)",
            "subcommands": {
                "list": {"description": "List Spark job definitions", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show Spark job definition details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a Spark job definition", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update Spark job definition properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a Spark job definition", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get Spark job definition content", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update Spark job definition content", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "run": {"description": "Run a Spark job definition", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "report": {
            "description": "Manage reports (Power BI)",
            "subcommands": {
                "list": {"description": "List reports", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show report details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a report from definition file", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}, "--file": {"type": "string", "required": true}}},
                "update": {"description": "Update report properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a report", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get report definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update report definition from file", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string", "required": true}}}
            }
        },
        "semantic-model": {
            "description": "Manage semantic models (Power BI datasets)",
            "subcommands": {
                "list": {"description": "List semantic models", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show semantic model details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a semantic model from definition file (model.bim)", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}, "--file": {"type": "string", "required": true}}},
                "update": {"description": "Update semantic model properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a semantic model", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get semantic model definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update semantic model definition from file", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string", "required": true}}},
                "query": {"description": "Execute DAX query against a semantic model", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--dax": {"type": "string"}, "--file": {"type": "string"}}},
                "refresh": {"description": "Trigger a refresh on a semantic model", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--type": {"type": "string"}}},
                "bind-connection": {"description": "Bind a semantic model to a connection (lakehouse SQL endpoint ID)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--connection-id": {"type": "string", "required": true}}},
                "unbind-connection": {"description": "Unbind a semantic model from its current connection", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "takeover": {"description": "Take ownership of a definition-managed semantic model (makes it editable in portal)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "list-parameters": {"description": "List parameters of a semantic model", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-parameters": {"description": "Update parameter values", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "list-datasources": {"description": "List data sources bound to a semantic model", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-datasources": {"description": "Update data source connection details", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "list-users": {"description": "List users with access to a semantic model", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "add-user": {"description": "Grant a user access to a semantic model", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--principal": {"type": "string", "required": true}, "--principal-type": {"type": "string", "required": true}, "--access-right": {"type": "string", "required": true}}},
                "delete-user": {"description": "Revoke a user's access to a semantic model", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--user": {"type": "string", "required": true}}},
                "refresh-status": {"description": "Show recent refresh history", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--top": {"type": "integer"}}},
                "list-upstream": {"description": "List upstream datasets this model depends on", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "clone": {"description": "Clone a semantic model to the same or different workspace", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--target-workspace": {"type": "string"}}},
                "export-pbix": {"description": "Export a semantic model as a .pbix file", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string", "required": true}}},
                "import-pbix": {"description": "Import a .pbix file as a new semantic model", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--file": {"type": "string", "required": true}, "--name-conflict": {"type": "string"}}}
            }
        },
        "copy-job": {
            "description": "Manage copy jobs (data movement)",
            "subcommands": {
                "list": {"description": "List copy jobs", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show copy job details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a copy job", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update copy job properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a copy job", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get copy job definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update copy job definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "dataflow": {
            "description": "Manage dataflows (Power BI data transformation)",
            "subcommands": {
                "list": {"description": "List dataflows", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show dataflow details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a dataflow", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update dataflow properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a dataflow", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get dataflow definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update dataflow definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "discover-parameters": {"description": "Discover parameters of a dataflow", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "run": {"description": "Run a dataflow job (execute or apply-changes)", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--job-type": {"type": "string", "description": "execute (default) or apply-changes"}, "--execute-option": {"type": "string", "description": "NoRefreshDuringSave or AutomaticRefresh"}, "--parameters": {"type": "string", "description": "JSON object of parameters"}, "--wait": {"type": "bool"}, "--timeout": {"type": "integer", "description": "Max wait seconds (default 600)"}, "--cancel-on-timeout": {"type": "bool"}}},
                "execute-query": {"description": "Execute a query (returns Apache Arrow IPC binary)", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--query-name": {"type": "string", "required": true}, "--mashup": {"type": "string", "description": "Custom M expression override"}, "--file": {"type": "string", "description": "Output file path for Arrow IPC bytes"}}}
            }
        },
        "kql-dashboard": {
            "description": "Manage KQL dashboards (real-time dashboards)",
            "subcommands": {
                "list": {"description": "List KQL dashboards", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show KQL dashboard details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a KQL dashboard", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update KQL dashboard properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a KQL dashboard", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get KQL dashboard definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update KQL dashboard definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "reflex": {
            "description": "Manage Reflex items (Data Activator triggers and alerts)",
            "subcommands": {
                "list": {"description": "List reflexes", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show reflex details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a reflex", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update reflex properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a reflex", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool", "description": "Permanently delete (skip recycle bin)"}}},
                "get-definition": {"description": "Get reflex definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update reflex definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "graphql-api": {
            "description": "Manage GraphQL APIs",
            "subcommands": {
                "list": {"description": "List GraphQL APIs", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show GraphQL API details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a GraphQL API", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update GraphQL API properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a GraphQL API", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get GraphQL API definition (schema.graphql)", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update GraphQL API definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "ml-model": {
            "description": "Manage ML models (data science)",
            "subcommands": {
                "list": {"description": "List ML models", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show ML model details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create an ML model", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update ML model properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete an ML model", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "ml-experiment": {
            "description": "Manage ML experiments (data science)",
            "subcommands": {
                "list": {"description": "List ML experiments", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show ML experiment details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create an ML experiment", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update ML experiment properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete an ML experiment", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "admin": {
            "description": "Tenant administration (49 subcommands, requires Fabric admin role)",
            "subcommands": {
                "list-tenant-settings": {"description": "List all tenant settings", "mutates": false},
                "update-tenant-setting": {"description": "Update a tenant setting", "mutates": true, "flags": {"--setting-name": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "list-capacities-tenant-overrides": {"description": "List tenant setting overrides across all capacities", "mutates": false, "flags": {"--setting-name": {"type": "string", "required": true}}},
                "list-capacity-tenant-overrides": {"description": "List overrides for a specific capacity", "mutates": false, "flags": {"--capacity-id": {"type": "string", "required": true}}},
                "update-capacity-tenant-override": {"description": "Update a capacity tenant setting override", "mutates": true, "flags": {"--capacity-id": {"type": "string", "required": true}, "--setting-name": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "delete-capacity-tenant-override": {"description": "Delete a capacity override", "mutates": true, "flags": {"--capacity-id": {"type": "string", "required": true}, "--setting-name": {"type": "string", "required": true}}},
                "list-domains-tenant-overrides": {"description": "List domain-level overrides", "mutates": false, "flags": {"--setting-name": {"type": "string", "required": true}}},
                "list-workspaces-tenant-overrides": {"description": "List workspace-level overrides", "mutates": false, "flags": {"--setting-name": {"type": "string", "required": true}}},
                "list-tags": {"description": "List tenant tags", "mutates": false},
                "create-tags": {"description": "Bulk-create tags", "mutates": true, "flags": {"--content": {"type": "string", "required": true}}},
                "update-tag": {"description": "Update a tag", "mutates": true, "flags": {"--tag-id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "delete-tag": {"description": "Delete a tag", "mutates": true, "destructive": true, "flags": {"--tag-id": {"type": "string", "required": true}}},
                "list-workloads": {"description": "List workloads", "mutates": false},
                "list-workload-assignments": {"description": "List workload assignments", "mutates": false},
                "create-workload-assignment": {"description": "Create a workload assignment", "mutates": true, "flags": {"--content": {"type": "string", "required": true}}},
                "delete-workload-assignment": {"description": "Delete a workload assignment", "mutates": true, "flags": {"--assignment-id": {"type": "string", "required": true}}},
                "list-workspaces": {"description": "List workspaces (admin view)", "mutates": false},
                "show-workspace": {"description": "Show workspace (admin view)", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "list-workspace-users": {"description": "List workspace users", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "list-git-connections": {"description": "List git connections across tenant", "mutates": false},
                "grant-admin-access": {"description": "Grant temporary admin access", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}}},
                "remove-admin-access": {"description": "Remove temporary admin access", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}}},
                "restore-workspace": {"description": "Restore a deleted workspace", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--capacity-id": {"type": "string", "required": true}}},
                "list-network-policies": {"description": "List network policies", "mutates": false},
                "list-items": {"description": "List items (admin view)", "mutates": false, "flags": {"--workspace": {"type": "string"}, "--type": {"type": "string"}}},
                "show-item": {"description": "Show item (admin view)", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "list-item-users": {"description": "List item users", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "bulk-set-labels": {"description": "Bulk set sensitivity labels", "mutates": true, "flags": {"--content": {"type": "string", "required": true}}},
                "bulk-remove-labels": {"description": "Bulk remove sensitivity labels", "mutates": true, "flags": {"--content": {"type": "string", "required": true}}},
                "list-external-data-shares": {"description": "List external data shares", "mutates": false},
                "revoke-external-data-share": {"description": "Revoke external data share", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--item-id": {"type": "string", "required": true}, "--share-id": {"type": "string", "required": true}}},
                "remove-all-sharing-links": {"description": "Remove all sharing links (LRO)", "mutates": true, "async": true, "flags": {"--content": {"type": "string", "required": true}}},
                "bulk-remove-sharing-links": {"description": "Bulk remove sharing links (LRO)", "mutates": true, "async": true, "flags": {"--content": {"type": "string", "required": true}}},
                "list-domains": {"description": "List domains", "mutates": false},
                "create-domain": {"description": "Create domain", "mutates": true, "flags": {"--name": {"type": "string", "required": true}, "--description": {"type": "string"}, "--parent-domain-id": {"type": "string"}}},
                "show-domain": {"description": "Show domain details", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "update-domain": {"description": "Update domain", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete-domain": {"description": "Delete domain", "mutates": true, "destructive": true, "flags": {"--id": {"type": "string", "required": true}}},
                "list-domain-workspaces": {"description": "List workspaces in domain", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "assign-domain-workspaces": {"description": "Assign workspaces to domain", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--workspaces": {"type": "string", "required": true}}},
                "unassign-domain-workspaces": {"description": "Unassign workspaces from domain", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--workspaces": {"type": "string", "required": true}}},
                "unassign-all-domain-workspaces": {"description": "Unassign all workspaces from domain", "mutates": true, "flags": {"--id": {"type": "string", "required": true}}},
                "list-domain-role-assignments": {"description": "List domain role assignments", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "bulk-assign-domain-roles": {"description": "Bulk assign domain roles", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "bulk-unassign-domain-roles": {"description": "Bulk unassign domain roles", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "sync-domain-roles-to-subdomains": {"description": "Sync domain roles to subdomains", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "assign-domain-workspaces-by-capacities": {"description": "Assign workspaces by capacity", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "assign-domain-workspaces-by-principals": {"description": "Assign workspaces by principal", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "list-user-access": {"description": "List user access across tenant", "mutates": false, "flags": {"--user-id": {"type": "string", "required": true}}}
            }
        },
        "catalog": {
            "description": "Tenant-level search across workspaces",
            "subcommands": {
                "search": {"description": "Search catalog items", "mutates": false, "flags": {"--content": {"type": "string", "description": "Full JSON search body (overrides convenience flags)"}, "--type": {"type": "string", "description": "Filter by item type(s), comma-separated (e.g., Notebook,Lakehouse)"}, "--exclude-type": {"type": "string", "description": "Exclude item type(s), comma-separated"}, "--top": {"type": "integer", "description": "Maximum results to return"}, "--search": {"type": "string", "description": "Search string (-s/--search)"}}}
            }
        },
        "sql-database": {
            "description": "Manage SQL databases (Fabric-native SQL Server)",
            "subcommands": {
                "list": {"description": "List SQL databases", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show SQL database details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a SQL database", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update SQL database properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a SQL database", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--hard-delete": {"type": "bool"}}},
                "query": {"description": "Execute SQL query", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--sql": {"type": "string"}}},
                "connection-string": {"description": "Get TDS connection string", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "import": {"description": "Import CSV/JSON into a table (creates table with type inference)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string", "required": true}, "--table": {"type": "string", "required": true}, "--drop-if-exists": {"type": "bool"}, "--batch-size": {"type": "integer", "default": "100"}}},
                "get-definition": {"description": "Get SQL database definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update SQL database definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "list-deleted": {"description": "List restorable deleted databases", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "start-mirroring": {"description": "Start mirroring", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "stop-mirroring": {"description": "Stop mirroring", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "revalidate-cmk": {"description": "Revalidate customer-managed key", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-audit-settings": {"description": "Get audit settings", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-audit-settings": {"description": "Update audit settings", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}}
            }
        },
        "sql-endpoint": {
            "description": "Manage SQL endpoints (read-only companion to lakehouses)",
            "subcommands": {
                "list": {"description": "List SQL endpoints", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show SQL endpoint details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "connection-string": {"description": "Get TDS connection string", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "refresh-metadata": {"description": "Refresh table sync metadata", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-audit-settings": {"description": "Get audit settings", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-audit-settings": {"description": "Update audit settings", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "set-audit-actions": {"description": "Set audit action groups", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--actions": {"type": "string", "required": true}}}
            }
        },
        "ontology": {
            "description": "Manage ontologies (entity types, relationships, data bindings)",
            "subcommands": {
                "list": {"description": "List ontologies", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show ontology details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create an ontology", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update ontology properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete an ontology", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get ontology definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--decode": {"type": "bool"}, "--dir": {"type": "string"}}},
                "update-definition": {"description": "Update ontology definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--dir": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "git": {
            "description": "Manage git integration (connect, commit, pull, status)",
            "subcommands": {
                "status": {"description": "Get workspace git status", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "commit": {"description": "Commit workspace changes to git", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--message": {"type": "string", "required": true}, "--items": {"type": "string"}}},
                "pull": {"description": "Pull remote changes into workspace", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}}},
                "connect": {"description": "Connect workspace to git repo", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--provider": {"type": "enum", "values": ["azureDevOps", "github"]}, "--org": {"type": "string", "required": true}, "--project": {"type": "string"}, "--repo": {"type": "string", "required": true}, "--branch": {"type": "string", "required": true}, "--directory": {"type": "string"}}},
                "disconnect": {"description": "Disconnect workspace from git", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}}},
                "init": {"description": "Initialize git connection (sync workspace/remote)", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--strategy": {"type": "enum", "values": ["prefer-workspace", "prefer-remote"]}}},
                "checkout": {"description": "Switch to a different branch", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--branch": {"type": "string", "required": true}}},
                "connection": {"description": "Show git connection details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "credentials": {"description": "Show or update git credentials", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show-tracked": {"description": "Show git-tracked items", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}}
            }
        },
        "deploy": {
            "description": "CI/CD deployment engine (stateless, content-hash convergence)",
            "subcommands": {
                "plan": {"description": "Generate deployment changeset (dry-run by default)", "mutates": false, "flags": {"--source": {"type": "string", "required": true, "description": "Source directory"}, "--workspace": {"type": "string", "required": true, "description": "Target workspace ID or name"}, "--item-types": {"type": "string"}, "--delete-orphans": {"type": "bool"}, "--force-all": {"type": "bool"}, "--out": {"type": "string"}, "--parameters": {"type": "string"}, "--env": {"type": "string"}}},
                "apply": {"description": "Execute deployment (create/update/rename/delete items)", "mutates": true, "flags": {"--source": {"type": "string"}, "--workspace": {"type": "string"}, "--plan": {"type": "string"}, "--item-types": {"type": "string"}, "--delete-orphans": {"type": "bool"}, "--fail-fast": {"type": "bool"}, "--force": {"type": "bool"}, "--force-all": {"type": "bool"}, "--concurrency": {"type": "integer", "default": "8"}, "--parameters": {"type": "string"}, "--env": {"type": "string"}, "--no-post-hooks": {"type": "bool"}}},
                "export": {"description": "Export workspace items to directory", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--dir": {"type": "string", "required": true}, "--item-types": {"type": "string"}, "--overwrite": {"type": "bool"}}},
                "init-params": {"description": "Scaffold parameters.json from GUIDs in source", "mutates": false, "flags": {"--source": {"type": "string", "required": true}, "--compare": {"type": "string"}, "--source-env": {"type": "string"}, "--compare-env": {"type": "string"}, "--out": {"type": "string"}}}
            }
        },
        "gateway": {
            "description": "Manage VNet gateways and role assignments",
            "subcommands": {
                "list": {"description": "List gateways", "mutates": false},
                "show": {"description": "Show gateway details", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a VNet gateway", "mutates": true, "flags": {"--name": {"type": "string", "required": true}, "--capacity-id": {"type": "string", "required": true}, "--subscription-id": {"type": "string", "required": true}, "--resource-group": {"type": "string", "required": true}, "--vnet-name": {"type": "string", "required": true}, "--subnet-name": {"type": "string", "required": true}}},
                "update": {"description": "Update gateway settings", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--inactivity-minutes": {"type": "integer"}, "--member-count": {"type": "integer"}}},
                "delete": {"description": "Delete a gateway", "mutates": true, "destructive": true, "flags": {"--id": {"type": "string", "required": true}}},
                "list-members": {"description": "List gateway members", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "update-member": {"description": "Update a gateway member", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--member-id": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "delete-member": {"description": "Delete a gateway member", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--member-id": {"type": "string", "required": true}}},
                "list-role-assignments": {"description": "List role assignments", "mutates": false, "flags": {"--id": {"type": "string", "required": true}}},
                "add-role-assignment": {"description": "Add role assignment", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--principal-id": {"type": "string", "required": true}, "--principal-type": {"type": "string", "required": true}, "--role": {"type": "enum", "values": ["Admin", "ConnectionCreator", "ConnectionCreatorWithResharing"]}}},
                "show-role-assignment": {"description": "Show role assignment", "mutates": false, "flags": {"--id": {"type": "string", "required": true}, "--assignment-id": {"type": "string", "required": true}}},
                "update-role-assignment": {"description": "Update role assignment", "mutates": true, "flags": {"--id": {"type": "string", "required": true}, "--assignment-id": {"type": "string", "required": true}, "--role": {"type": "string", "required": true}}},
                "delete-role-assignment": {"description": "Delete role assignment", "mutates": true, "destructive": true, "flags": {"--id": {"type": "string", "required": true}, "--assignment-id": {"type": "string", "required": true}}}
            }
        },
        "apache-airflow-job": {
            "description": "Manage Apache Airflow jobs (environment, files, compute)",
            "subcommands": {
                "list": {"description": "List Airflow jobs", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show Airflow job details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create an Airflow job", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update Airflow job properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete an Airflow job", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get Airflow job definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update Airflow job definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "start-environment": {"description": "Start Airflow environment", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "stop-environment": {"description": "Stop Airflow environment", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-environment": {"description": "Get environment status", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "list-files": {"description": "List DAG files", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-file": {"description": "Download a file", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--path": {"type": "string", "required": true}}},
                "upload-file": {"description": "Upload a file", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--path": {"type": "string", "required": true}, "--file": {"type": "string", "required": true}}},
                "delete-file": {"description": "Delete a file", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--path": {"type": "string", "required": true}}},
                "get-compute": {"description": "Get compute pool details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-workspace-settings": {"description": "Get workspace Airflow settings", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "deploy-requirements": {"description": "Deploy Python requirements", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string", "required": true}}}
            }
        },
        "anomaly-detector": {
            "description": "Manage anomaly detectors",
            "subcommands": {
                "list": {"description": "List anomaly detectors", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show anomaly detector details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create an anomaly detector", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update anomaly detector properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete an anomaly detector", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition (Configurations.json)", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "cosmos-db-database": {
            "description": "Manage Cosmos DB databases (mirrored)",
            "subcommands": {
                "list": {"description": "List Cosmos DB databases", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a Cosmos DB database", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "snowflake-database": {
            "description": "Manage Snowflake databases (mirrored, requires connection)",
            "subcommands": {
                "list": {"description": "List Snowflake databases", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create (requires connection payload)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "digital-twin-builder": {
            "description": "Manage Digital Twin Builders (links to lakehouse)",
            "subcommands": {
                "list": {"description": "List DTBs", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a DTB", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "digital-twin-builder-flow": {
            "description": "Manage Digital Twin Builder Flows (requires parent DTB)",
            "subcommands": {
                "list": {"description": "List flows", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a flow (requires --dtb-id)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--dtb-id": {"type": "string", "required": true}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "event-schema-set": {
            "description": "Manage event schema sets",
            "subcommands": {
                "list": {"description": "List event schema sets", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition (EventSchemaSetDefinition.json)", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "operations-agent": {
            "description": "Manage operations agents (goals, instructions, data sources, actions)",
            "subcommands": {
                "list": {"description": "List operations agents", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition (Configurations.json)", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "mounted-data-factory": {
            "description": "Manage mounted data factories (link to Azure Data Factory)",
            "subcommands": {
                "list": {"description": "List mounted data factories", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create (requires ADF ARM resource ID in definition)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--content": {"type": "string", "required": true}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "user-data-function": {
            "description": "Manage user data functions (Python runtime)",
            "subcommands": {
                "list": {"description": "List user data functions", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "variable-library": {
            "description": "Manage variable libraries (variables.json + settings.json)",
            "subcommands": {
                "list": {"description": "List variable libraries", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "map": {
            "description": "Manage maps (geospatial visualization with Azure Maps)",
            "subcommands": {
                "list": {"description": "List maps", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition (map.json)", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "graph-query-set": {
            "description": "Manage graph query sets (read-only definition export)",
            "subcommands": {
                "list": {"description": "List graph query sets", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition (exportedDefinition.json, read-only)", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "graph-model": {
            "description": "Manage graph models (ontology-linked, requires portal init for refresh)",
            "subcommands": {
                "list": {"description": "List graph models", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details (includes queryReadiness)", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create (optionally with --ontology)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}, "--ontology": {"type": "string", "description": "Ontology ID to link"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "refresh-graph": {"description": "Trigger graph refresh (requires portal initialization)", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "execute-query": {"description": "Execute a KQL query against the graph", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--query": {"type": "string", "required": true}}},
                "get-queryable-graph-type": {"description": "Get queryable graph type info", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "mirrored-catalog": {
            "description": "Manage mirrored catalogs (requires tenant feature flag)",
            "subcommands": {
                "list": {"description": "List mirrored catalogs", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create (requires tenant feature flag)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "refresh-metadata": {"description": "Refresh catalog metadata", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "mirroring-status": {"description": "Get mirroring status", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "tables-mirroring-status": {"description": "Get tables mirroring status", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "mirrored-databricks-catalog": {
            "description": "Manage mirrored Databricks catalogs",
            "subcommands": {
                "list": {"description": "List mirrored Databricks catalogs", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}},
                "discover-catalogs": {"description": "Discover available catalogs from Databricks", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "refresh-metadata": {"description": "Refresh metadata", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "mirroring-status": {"description": "Get mirroring status", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "warehouse-snapshot": {
            "description": "Manage warehouse snapshots",
            "subcommands": {
                "list": {"description": "List snapshots", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show snapshot details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create snapshot (requires --warehouse-id)", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--warehouse-id": {"type": "string", "required": true}}},
                "update": {"description": "Update snapshot properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete snapshot", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "mirrored-warehouse": {
            "description": "Manage mirrored warehouses (requires tenant feature flag)",
            "subcommands": {
                "list": {"description": "List mirrored warehouses", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}}
            }
        },
        "paginated-report": {
            "description": "Manage paginated reports (read-only creation via portal/SSRS)",
            "subcommands": {
                "list": {"description": "List paginated reports", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "update": {"description": "Update properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}}
            }
        },
        "dashboard": {
            "description": "Manage dashboards (read-only, created via portal)",
            "subcommands": {
                "list": {"description": "List dashboards", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}}
            }
        },
        "datamart": {
            "description": "Manage datamarts (read-only, created via portal)",
            "subcommands": {
                "list": {"description": "List datamarts", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}}
            }
        },
        "feedback": {
            "description": "Two-way CLI friction feedback channel",
            "subcommands": {
                "send": {"description": "Send feedback about CLI friction or feature requests", "mutates": true, "flags": {"--message": {"type": "string", "required": true}, "--category": {"type": "string"}}},
                "list": {"description": "List previously sent feedback", "mutates": false}
            }
        },
        "operation": {
            "description": "Inspect long-running operation state and results",
            "subcommands": {
                "get-state": {"description": "Get LRO operation state", "mutates": false, "flags": {"--operation-id": {"type": "string", "required": true}}},
                "get-result": {"description": "Get LRO operation result", "mutates": false, "flags": {"--operation-id": {"type": "string", "required": true}}}
            }
        },
        "rest": {
            "description": "Raw REST API passthrough (supports Fabric and Power BI APIs)",
            "subcommands": {
                "call": {
                    "description": "Execute a raw REST API call",
                    "mutates": true,
                    "flags": {
                        "--method": {"type": "enum", "values": ["get", "post", "put", "patch", "delete"], "required": true, "description": "HTTP method"},
                        "--path": {"type": "string", "required": true, "description": "API path (appended to base URL)"},
                        "--body": {"type": "string", "description": "Request body (inline JSON, @file, or @- for stdin)"},
                        "--query-params": {"type": "string", "description": "URL query parameters as key=value pairs"},
                        "--api": {"type": "enum", "values": ["fabric", "powerbi"], "default": "fabric", "description": "Target API endpoint"},
                        "--poll": {"type": "bool", "description": "Poll LRO until completion (Fabric API only)"}
                    },
                    "examples": [
                        "fabio rest call --method get --path /workspaces",
                        "fabio rest call --method get --path /groups/{ws}/datasets --api powerbi",
                        "fabio rest call --method post --path /groups/{ws}/datasets/{id}/refreshes --api powerbi --body '{}'",
                        "fabio rest call --method post --path /workspaces/{ws}/items/{id}/getDefinition --poll"
                    ]
                }
            }
        },
        "rti": {
            "description": "Real-Time Intelligence operations",
            "subcommands": {
                "nl-to-kql": {
                    "description": "Translate natural language to KQL query",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--item-id": {"type": "string", "required": true, "description": "KQL Database or Eventhouse ID for billing"},
                        "--cluster-url": {"type": "string", "required": true, "description": "Kusto query URI"},
                        "--database-name": {"type": "string", "required": true, "description": "Target database name"},
                        "--question": {"type": "string", "required": true, "description": "Natural language question"},
                        "--user-shots": {"type": "string", "description": "JSON array of {naturalLanguage, kqlQuery} examples"},
                        "--chat-messages": {"type": "string", "description": "JSON array of {role, content} for multi-turn context"}
                    }
                }
            }
        }
    })
}
