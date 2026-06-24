//! Agent introspection, offline docs, and workspace graph extraction.

mod agent;
mod best_practices;
mod examples;
mod schemas;
pub mod tenant;
mod workflows;

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

// ── CLI definition ──────────────────────────────────────────────────────────

/// Output format for context graph.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum ContextFormat {
    /// Native format: nodes/edges/workspaces/summary arrays (for fabio `--merge`, `JMESPath`, agents)
    #[default]
    Graph,
    /// JSON-LD instance data: actual items as RDF resources (for triple stores, SPARQL endpoints)
    Jsonld,
    /// OWL schema as JSON-LD: type definitions importable by `fabio ontology import --file`
    Owl,
    /// OWL schema as RDF/XML: type definitions importable by `fabio ontology import --file` and Ontology Playground
    Rdf,
    /// Full RDF/XML: schema + instances in one file (Ontology Playground + triple stores + Fabric import)
    Full,
}

#[derive(Debug, Subcommand)]
pub enum ContextCommand {
    /// Machine-readable CLI schema for agent introspection (flags, types, mutability, examples)
    #[command(display_order = 0)]
    Agent,

    /// Show the definition schema/template for a Fabric item type
    #[command(display_order = 1)]
    Schema {
        /// Item type (e.g. `Notebook`, `DataPipeline`, `SemanticModel`)
        #[arg(name = "TYPE")]
        item_type: String,
    },

    /// Show a multi-step workflow recipe
    #[command(display_order = 2)]
    Workflow {
        /// Workflow name (use `fabio context list` to see available workflows)
        #[arg(name = "NAME")]
        name: String,
    },

    /// Show best-practices guidance for a topic
    #[command(display_order = 3)]
    BestPractices {
        /// Topic name (`throttling`, `lro`, `pagination`, `admin-apis`)
        #[arg(name = "TOPIC")]
        topic: String,
    },

    /// Show example output for a command (response shape + `JMESPath` tips)
    #[command(display_order = 4)]
    Examples {
        /// Command group (e.g. `lakehouse`, `warehouse`, `deploy`)
        #[arg(name = "GROUP")]
        group: String,

        /// Subcommand (e.g. `query`, `list-tables`, `plan`). Omit to list all examples for the group.
        #[arg(name = "COMMAND")]
        command: Option<String>,
    },

    /// List all available documentation topics (schemas, workflows, examples, best-practices)
    #[command(display_order = 5)]
    List,

    /// Scan your Fabric tenant — build a relationship graph from workspace(s)
    #[command(display_order = 10)]
    Tenant {
        /// Workspace ID(s) or name(s) to scan (repeatable)
        #[arg(short, long, env = "FABIO_WORKSPACE", num_args = 1..)]
        workspace: Vec<String>,

        /// Fetch item definitions to discover embedded references (slower)
        #[arg(long)]
        deep: bool,

        /// Also fetch item connections
        #[arg(long)]
        include_connections: bool,

        /// Filter to specific item types (comma-separated, case-insensitive)
        #[arg(long)]
        item_types: Option<String>,

        /// Skip type-specific detail fetching (fast inventory-only mode)
        #[arg(long)]
        no_properties: bool,

        /// Output format:
        ///   graph (default) — native arrays for fabio merge/query;
        ///   jsonld — instance data as RDF for triple stores;
        ///   owl — OWL schema as JSON-LD for `fabio ontology import`;
        ///   rdf — OWL schema as RDF/XML for `fabio ontology import` and Ontology Playground
        #[arg(long, value_enum, default_value = "graph")]
        format: ContextFormat,

        /// Merge results into an existing graph file (incremental extraction)
        #[arg(long)]
        merge: Option<PathBuf>,

        /// Write output to a file instead of stdout
        #[arg(long)]
        output_file: Option<PathBuf>,

        /// Max concurrency for API calls (default: auto-scaled to CPU count)
        #[arg(long)]
        concurrency: Option<usize>,
    },
}

// ── Dispatch ────────────────────────────────────────────────────────────────

pub async fn execute(cli: &Cli, client: &FabricClient, command: &ContextCommand) -> Result<()> {
    match command {
        ContextCommand::Agent => {
            agent::execute(cli);
            Ok(())
        }
        ContextCommand::Schema { item_type } => {
            schemas::execute(cli, item_type);
            Ok(())
        }
        ContextCommand::Workflow { name } => {
            workflows::execute(cli, name);
            Ok(())
        }
        ContextCommand::BestPractices { topic } => {
            best_practices::execute(cli, topic);
            Ok(())
        }
        ContextCommand::Examples { group, command } => {
            examples::execute(cli, group, command.as_deref());
            Ok(())
        }
        ContextCommand::List => {
            list_topics(cli);
            Ok(())
        }
        ContextCommand::Tenant {
            workspace,
            deep,
            include_connections,
            item_types,
            no_properties,
            format,
            merge,
            output_file,
            concurrency,
        } => {
            let params = tenant::ExtractParams {
                workspaces: workspace,
                deep: *deep,
                include_connections: *include_connections,
                item_types_filter: item_types.as_deref(),
                no_properties: *no_properties,
                format: *format,
                merge: merge.as_deref(),
                output_file: output_file.as_deref(),
                concurrency: concurrency.unwrap_or_else(crate::parallel::default_concurrency),
            };
            tenant::execute(cli, client, &params).await
        }
    }
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

fn list_topics(cli: &Cli) {
    let topics = json!({
        "item_schemas": schemas::list_names(),
        "workflows": workflows::list_names(),
        "output_examples": examples::list_names(),
        "best_practices": best_practices::list_names(),
        "usage": {
            "schema": "fabio context schema <TYPE>",
            "workflow": "fabio context workflow <NAME>",
            "examples": "fabio context examples <GROUP> <COMMAND>",
            "best_practices": "fabio context best-practices <TOPIC>"
        }
    });
    output::render_object(cli, &topics, "item_schemas");
}

/// Find an entry in a static lookup table using normalized key matching.
fn find_entry<'a>(entries: &[(&str, &'a str)], normalized_key: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|(name, _)| name.to_lowercase().replace(['-', '_'], "") == *normalized_key)
        .map(|(_, content)| *content)
}
