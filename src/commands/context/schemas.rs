//! Item definition schemas for AI agents.

use serde_json::{Value, json};

use crate::cli::Cli;
use crate::output;

use super::find_entry;

pub(super) fn execute(cli: &Cli, item_type: &str) {
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
            "hint": "Use 'fabio context list' to see all available item types"
        });
        output::render_object(cli, &result, "error");
    }
}

pub(super) fn list_names() -> Vec<&'static str> {
    ITEM_SCHEMAS.iter().map(|(name, _)| *name).collect()
}

const ITEM_SCHEMAS: &[(&str, &str)] = &[
    ("Notebook", include_str!("data/schemas/notebook.json")),
    (
        "DataPipeline",
        include_str!("data/schemas/data_pipeline.json"),
    ),
    (
        "SemanticModel",
        include_str!("data/schemas/semantic_model.json"),
    ),
    ("Lakehouse", include_str!("data/schemas/lakehouse.json")),
    (
        "KQLDatabase",
        include_str!("data/schemas/kql_database.json"),
    ),
    ("Eventhouse", include_str!("data/schemas/eventhouse.json")),
    ("Eventstream", include_str!("data/schemas/eventstream.json")),
    ("Environment", include_str!("data/schemas/environment.json")),
    ("Warehouse", include_str!("data/schemas/warehouse.json")),
    ("Report", include_str!("data/schemas/report.json")),
    ("DataAgent", include_str!("data/schemas/data_agent.json")),
    (
        "SparkJobDefinition",
        include_str!("data/schemas/spark_job_definition.json"),
    ),
    ("GraphQLApi", include_str!("data/schemas/graphql_api.json")),
    ("CopyJob", include_str!("data/schemas/copy_job.json")),
    ("Dataflow", include_str!("data/schemas/dataflow.json")),
    (
        "MirroredDatabase",
        include_str!("data/schemas/mirrored_database.json"),
    ),
    ("Reflex", include_str!("data/schemas/reflex.json")),
    ("MLModel", include_str!("data/schemas/ml_model.json")),
    (
        "MLExperiment",
        include_str!("data/schemas/ml_experiment.json"),
    ),
    ("Ontology", include_str!("data/schemas/ontology.json")),
    (
        "SQLDatabase",
        include_str!("data/schemas/sql_database.json"),
    ),
    ("Connection", include_str!("data/schemas/connection.json")),
    (
        "VariableLibrary",
        include_str!("data/schemas/variable_library.json"),
    ),
];
