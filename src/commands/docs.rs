use serde_json::{Value, json};

use crate::cli::Cli;
use crate::output;

// ─── Public entry points (called from context.rs) ────────────────────────────

pub fn list_topics_public(cli: &Cli) {
    list_topics(cli);
}

pub fn item_schema_public(cli: &Cli, item_type: &str) {
    item_schema(cli, item_type);
}

pub fn workflow_public(cli: &Cli, name: &str) {
    workflow(cli, name);
}

pub fn best_practices_public(cli: &Cli, topic: &str) {
    best_practices(cli, topic);
}

pub fn output_example_public(cli: &Cli, group: &str, command: &str) {
    output_example(cli, group, command);
}

// ─── List ────────────────────────────────────────────────────────────────────

fn list_topics(cli: &Cli) {
    let topics = json!({
        "item_schemas": ITEM_SCHEMAS.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "workflows": WORKFLOWS.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "output_examples": OUTPUT_EXAMPLES.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "best_practices": BEST_PRACTICES.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "usage": {
            "item_schema": "fabio docs item-schema <TYPE>",
            "workflow": "fabio docs workflow <NAME>",
            "output_example": "fabio docs output-example <GROUP> <COMMAND>",
            "best_practices": "fabio docs best-practices <TOPIC>"
        }
    });
    output::render_object(cli, &topics, "item_schemas");
}

// ─── Item Schema ─────────────────────────────────────────────────────────────

fn item_schema(cli: &Cli, item_type: &str) {
    let normalized = item_type.to_lowercase().replace(['-', '_'], "");
    if let Some(content) = find_entry(ITEM_SCHEMAS, &normalized) {
        let val: Value =
            serde_json::from_str(content).unwrap_or_else(|_| json!({"content": content}));
        output::render_object(cli, &val, "type");
    } else {
        let available: Vec<&str> = ITEM_SCHEMAS.iter().map(|(name, _)| *name).collect();
        let result = json!({
            "error": format!("No schema found for item type '{item_type}'"),
            "available_types": available,
            "hint": "Use 'fabio docs list' to see all available item types"
        });
        output::render_object(cli, &result, "error");
    }
}

// ─── Workflow ─────────────────────────────────────────────────────────────────

fn workflow(cli: &Cli, name: &str) {
    let normalized = name.to_lowercase().replace(['-', '_'], "");
    if let Some(content) = find_entry(WORKFLOWS, &normalized) {
        let val: Value =
            serde_json::from_str(content).unwrap_or_else(|_| json!({"content": content}));
        output::render_object(cli, &val, "name");
    } else {
        let available: Vec<&str> = WORKFLOWS.iter().map(|(name, _)| *name).collect();
        let result = json!({
            "error": format!("No workflow found for '{name}'"),
            "available_workflows": available,
            "hint": "Use 'fabio docs list' to see all available workflows"
        });
        output::render_object(cli, &result, "error");
    }
}

// ─── Output Example ──────────────────────────────────────────────────────────

fn output_example(cli: &Cli, group: &str, command: &str) {
    let key = format!("{group}/{command}");
    let normalized = key.to_lowercase().replace(['-', '_'], "");
    if let Some(content) = find_entry(OUTPUT_EXAMPLES, &normalized) {
        let val: Value =
            serde_json::from_str(content).unwrap_or_else(|_| json!({"content": content}));
        output::render_object(cli, &val, "command");
    } else {
        let available: Vec<&str> = OUTPUT_EXAMPLES.iter().map(|(name, _)| *name).collect();
        let result = json!({
            "error": format!("No output example found for '{group} {command}'"),
            "available_examples": available,
            "hint": "Use 'fabio docs list' to see all available examples"
        });
        output::render_object(cli, &result, "error");
    }
}

// ─── Best Practices ──────────────────────────────────────────────────────────

fn best_practices(cli: &Cli, topic: &str) {
    let normalized = topic.to_lowercase().replace(['-', '_'], "");
    if let Some(content) = find_entry(BEST_PRACTICES, &normalized) {
        let val: Value =
            serde_json::from_str(content).unwrap_or_else(|_| json!({"content": content}));
        output::render_object(cli, &val, "topic");
    } else {
        let available: Vec<&str> = BEST_PRACTICES.iter().map(|(name, _)| *name).collect();
        let result = json!({
            "error": format!("No best-practices topic found for '{topic}'"),
            "available_topics": available,
            "hint": "Use 'fabio docs list' to see all available topics"
        });
        output::render_object(cli, &result, "error");
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Find an entry in a static lookup table using normalized key matching.
fn find_entry<'a>(entries: &[(&str, &'a str)], normalized_key: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|(name, _)| name.to_lowercase().replace(['-', '_'], "") == *normalized_key)
        .map(|(_, content)| *content)
}

// ─── Embedded Content ────────────────────────────────────────────────────────

/// Item type to JSON schema/template showing the creation body structure.
const ITEM_SCHEMAS: &[(&str, &str)] = &[
    ("Notebook", include_str!("docs_data/schemas/notebook.json")),
    (
        "DataPipeline",
        include_str!("docs_data/schemas/data_pipeline.json"),
    ),
    (
        "SemanticModel",
        include_str!("docs_data/schemas/semantic_model.json"),
    ),
    (
        "Lakehouse",
        include_str!("docs_data/schemas/lakehouse.json"),
    ),
    (
        "KQLDatabase",
        include_str!("docs_data/schemas/kql_database.json"),
    ),
    (
        "Eventhouse",
        include_str!("docs_data/schemas/eventhouse.json"),
    ),
    (
        "Eventstream",
        include_str!("docs_data/schemas/eventstream.json"),
    ),
    (
        "Environment",
        include_str!("docs_data/schemas/environment.json"),
    ),
    (
        "Warehouse",
        include_str!("docs_data/schemas/warehouse.json"),
    ),
    ("Report", include_str!("docs_data/schemas/report.json")),
    (
        "DataAgent",
        include_str!("docs_data/schemas/data_agent.json"),
    ),
    (
        "SparkJobDefinition",
        include_str!("docs_data/schemas/spark_job_definition.json"),
    ),
    (
        "GraphQLApi",
        include_str!("docs_data/schemas/graphql_api.json"),
    ),
    ("CopyJob", include_str!("docs_data/schemas/copy_job.json")),
    ("Dataflow", include_str!("docs_data/schemas/dataflow.json")),
    (
        "MirroredDatabase",
        include_str!("docs_data/schemas/mirrored_database.json"),
    ),
    ("Reflex", include_str!("docs_data/schemas/reflex.json")),
    ("MLModel", include_str!("docs_data/schemas/ml_model.json")),
    (
        "MLExperiment",
        include_str!("docs_data/schemas/ml_experiment.json"),
    ),
    ("Ontology", include_str!("docs_data/schemas/ontology.json")),
    (
        "SQLDatabase",
        include_str!("docs_data/schemas/sql_database.json"),
    ),
    (
        "Connection",
        include_str!("docs_data/schemas/connection.json"),
    ),
];

/// Workflow name to JSON recipe with ordered steps.
const WORKFLOWS: &[(&str, &str)] = &[
    (
        "rti-pipeline",
        include_str!("docs_data/workflows/rti_pipeline.json"),
    ),
    (
        "direct-lake-report",
        include_str!("docs_data/workflows/direct_lake_report.json"),
    ),
    (
        "cicd-deploy",
        include_str!("docs_data/workflows/cicd_deploy.json"),
    ),
    (
        "lakehouse-etl",
        include_str!("docs_data/workflows/lakehouse_etl.json"),
    ),
    (
        "data-agent-setup",
        include_str!("docs_data/workflows/data_agent_setup.json"),
    ),
];

/// Group/command to JSON example output.
const OUTPUT_EXAMPLES: &[(&str, &str)] = &[
    (
        "lakehouse/list-tables",
        include_str!("docs_data/examples/lakehouse_list_tables.json"),
    ),
    (
        "lakehouse/iceberg-table",
        include_str!("docs_data/examples/lakehouse_iceberg_table.json"),
    ),
    (
        "lakehouse/iceberg-stats",
        include_str!("docs_data/examples/lakehouse_iceberg_stats.json"),
    ),
    (
        "workspace/list",
        include_str!("docs_data/examples/workspace_list.json"),
    ),
    (
        "item/list",
        include_str!("docs_data/examples/item_list.json"),
    ),
    (
        "deploy/plan",
        include_str!("docs_data/examples/deploy_plan.json"),
    ),
];

/// Best-practices topic to guidance document.
const BEST_PRACTICES: &[(&str, &str)] = &[
    (
        "throttling",
        include_str!("docs_data/best_practices/throttling.json"),
    ),
    ("lro", include_str!("docs_data/best_practices/lro.json")),
    (
        "pagination",
        include_str!("docs_data/best_practices/pagination.json"),
    ),
    (
        "admin-apis",
        include_str!("docs_data/best_practices/admin_apis.json"),
    ),
];
