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
                "create": {
                    "description": "Create a new item",
                    "mutates": true,
                    "flags": {
                        "--workspace": {"type": "string", "required": true, "description": "Workspace ID"},
                        "--name": {"type": "string", "required": true, "description": "Item display name"},
                        "--type": {"type": "string", "required": true, "description": "Item type (e.g., Lakehouse, Warehouse)"}
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
        }
    })
}
