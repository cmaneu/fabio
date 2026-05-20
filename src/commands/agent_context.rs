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
    commands: serde_json::Value,
    error_codes: Vec<ErrorCodeInfo>,
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
        commands: commands_schema(),
        error_codes: error_codes(),
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

#[allow(clippy::too_many_lines)]
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
            "description": "Manage workspaces",
            "subcommands": {
                "list": {
                    "description": "List all workspaces",
                    "mutates": false
                },
                "show": {
                    "description": "Show details of a workspace",
                    "mutates": false,
                    "flags": {
                        "--id": {"type": "string", "required": true, "description": "Workspace ID"}
                    }
                },
                "create": {
                    "description": "Create a new workspace",
                    "mutates": true,
                    "flags": {
                        "--name": {"type": "string", "required": true, "description": "Display name"},
                        "--description": {"type": "string", "description": "Optional description"}
                    }
                },
                "delete": {
                    "description": "Delete a workspace",
                    "mutates": true,
                    "destructive": true,
                    "flags": {
                        "--id": {"type": "string", "required": true, "description": "Workspace ID"}
                    }
                },
                "assign-capacity": {
                    "description": "Assign a workspace to a capacity",
                    "mutates": true,
                    "flags": {
                        "--id": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--capacity": {"type": "string", "required": true, "description": "Target capacity ID"}
                    }
                }
            }
        },
        "item": {
            "description": "Manage items (datasets, reports, notebooks, etc.)",
            "subcommands": {
                "list": {
                    "description": "List items in a workspace",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--type": {"type": "string", "description": "Filter by item type (e.g., Notebook, Lakehouse, Warehouse)"}
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
                        "--id": {"type": "string", "required": true, "description": "Item ID"}
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
                }
            }
        },
        "lakehouse": {
            "description": "Manage lakehouses (tables, files, shortcuts)",
            "subcommands": {
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
                        "--format": {"type": "enum", "values": ["Csv", "Parquet", "Json"], "default": "Csv"}
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
                }
            }
        },
        "notebook": {
            "description": "Manage notebooks",
            "subcommands": {
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
                        "--id": {"type": "string", "required": true}
                    }
                },
                "run": {
                    "description": "Run a notebook",
                    "mutates": true,
                    "async": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
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
                "list": {
                    "description": "List warehouses in a workspace",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true}
                    }
                },
                "show": {
                    "description": "Show details of a warehouse",
                    "mutates": false,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true}
                    }
                },
                "query": {
                    "description": "Execute a SQL query against a warehouse",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true},
                        "--id": {"type": "string", "required": true},
                        "--sql": {"type": "string", "description": "SQL query (prefix @ to read from file, omit for stdin)"}
                    }
                }
            }
        },
        "data-agent": {
            "description": "Manage data agents (create, query, and interact with AI agents)",
            "aliases": ["da"],
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
                }
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
                "delete": {"description": "Delete an environment", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "publish": {"description": "Publish staged changes", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "cancel-publish": {"description": "Cancel a pending publish", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-spark-settings": {"description": "Get published Spark settings", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-staging-spark-settings": {"description": "Get staging Spark settings", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
            }
        },
        "data-pipeline": {
            "description": "Manage data pipelines (orchestration, scheduling)",
            "subcommands": {
                "list": {"description": "List data pipelines in a workspace", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show details of a data pipeline", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a new data pipeline", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update data pipeline properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a data pipeline", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
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
                "delete": {"description": "Delete an eventhouse", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}}
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
                "run-on-demand": {"description": "Run an on-demand job", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--job-type": {"type": "string", "default": "DefaultJob"}, "--execution-data": {"type": "string", "description": "JSON"}}},
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
                "delete": {"description": "Delete a KQL database", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get KQL database definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update KQL database definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "mirrored-database": {
            "description": "Manage mirrored databases (real-time replication)",
            "subcommands": {
                "list": {"description": "List mirrored databases", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show mirrored database details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a mirrored database", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update mirrored database", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a mirrored database", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
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
            "description": "Manage eventstreams (real-time data ingestion)",
            "subcommands": {
                "list": {"description": "List eventstreams", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show eventstream details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create an eventstream", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update eventstream properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete an eventstream", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get eventstream definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update eventstream definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "kql-queryset": {
            "description": "Manage KQL querysets (saved KQL queries)",
            "subcommands": {
                "list": {"description": "List KQL querysets", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show KQL queryset details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a KQL queryset", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update KQL queryset properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a KQL queryset", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get KQL queryset definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update KQL queryset definition", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string"}, "--content": {"type": "string"}}}
            }
        },
        "spark-job-definition": {
            "description": "Manage Spark job definitions (batch Spark jobs)",
            "subcommands": {
                "list": {"description": "List Spark job definitions", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}}},
                "show": {"description": "Show Spark job definition details", "mutates": false, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "create": {"description": "Create a Spark job definition", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--name": {"type": "string", "required": true}, "--description": {"type": "string"}}},
                "update": {"description": "Update Spark job definition properties", "mutates": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--name": {"type": "string"}, "--description": {"type": "string"}}},
                "delete": {"description": "Delete a Spark job definition", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
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
                "delete": {"description": "Delete a report", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
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
                "delete": {"description": "Delete a semantic model", "mutates": true, "destructive": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "get-definition": {"description": "Get semantic model definition", "mutates": false, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}}},
                "update-definition": {"description": "Update semantic model definition from file", "mutates": true, "async": true, "flags": {"--workspace": {"type": "string", "required": true}, "--id": {"type": "string", "required": true}, "--file": {"type": "string", "required": true}}}
            }
        }
    })
}
