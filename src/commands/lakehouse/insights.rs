use anyhow::Result;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::commands::tds_utils::{
    capture_query_plan, execute_and_render_sql, parse_connection_string, resolve_sql_input,
};
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

/// Resolve lakehouse SQL endpoint connection: returns (server, database).
async fn resolve_lakehouse_sql(
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<(String, String)> {
    let data = client
        .get(&format!("/workspaces/{workspace}/lakehouses/{id}"))
        .await
        .map_err(|e| enrich_forbidden(e, "lakehouse", "Viewer"))?;

    let connection_string = data
        .get("properties")
        .and_then(|p| p.get("sqlEndpointProperties"))
        .and_then(|s| s.get("connectionString"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::NotFound,
                "Lakehouse SQL endpoint not available.",
                "Wait for provisioning to complete, then retry.",
            )
        })?;

    let display_name = data
        .get("displayName")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let (server, parsed_db) = parse_connection_string(connection_string);
    let database = if display_name.is_empty() {
        parsed_db
    } else {
        display_name.to_string()
    };

    Ok((server, database))
}

/// Helper: resolve lakehouse SQL connection and execute a TDS query.
async fn execute_lakehouse_query(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    sql_text: &str,
) -> Result<()> {
    let (server, database) = resolve_lakehouse_sql(client, workspace, id).await?;
    execute_and_render_sql(cli, client, &server, &database, sql_text).await
}

// ─── Plan ────────────────────────────────────────────────────────────────────

pub(super) async fn plan(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    sql: Option<&str>,
) -> Result<()> {
    let sql_text = resolve_sql_input(sql)?;
    let (server, database) = resolve_lakehouse_sql(client, workspace, id).await?;

    let plans = capture_query_plan(client, &server, &database, &sql_text).await?;

    let plan_objects: Vec<Value> = plans
        .iter()
        .enumerate()
        .map(|(i, xml)| {
            serde_json::json!({
                "statementIndex": i,
                "planXml": xml
            })
        })
        .collect();
    let obj = serde_json::json!({
        "statementCount": plans.len(),
        "plans": plan_objects
    });
    output::render_object(cli, &obj, "statementCount");

    Ok(())
}

// ─── Query Insights ──────────────────────────────────────────────────────────

pub(super) async fn queries_running(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let sql = "SELECT r.session_id, r.status, \
               r.command, r.start_time, r.total_elapsed_time \
               FROM sys.dm_exec_requests r \
               WHERE r.status != 'background' \
               ORDER BY r.total_elapsed_time DESC";
    execute_lakehouse_query(cli, client, workspace, id, sql).await
}

pub(super) async fn queries_frequent(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    top: u32,
) -> Result<()> {
    let sql = format!(
        "SELECT TOP ({top}) \
         last_run_command, number_of_runs, \
         avg_total_elapsed_time_ms, \
         min_run_total_elapsed_time_ms, \
         max_run_total_elapsed_time_ms, \
         number_of_successful_runs, \
         query_hash \
         FROM queryinsights.frequently_run_queries \
         ORDER BY number_of_runs DESC"
    );
    execute_lakehouse_query(cli, client, workspace, id, &sql).await
}

pub(super) async fn queries_long_running(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    top: u32,
) -> Result<()> {
    let sql = format!(
        "SELECT TOP ({top}) \
         last_run_command, number_of_runs, \
         median_total_elapsed_time_ms, \
         last_run_total_elapsed_time_ms, \
         last_run_start_time, \
         query_hash \
         FROM queryinsights.long_running_queries \
         ORDER BY median_total_elapsed_time_ms DESC"
    );
    execute_lakehouse_query(cli, client, workspace, id, &sql).await
}

pub(super) async fn queries_history(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    top: u32,
) -> Result<()> {
    let sql = format!(
        "SELECT TOP ({top}) \
         command, status, \
         total_elapsed_time_ms, \
         login_name, \
         start_time, end_time, \
         row_count, \
         query_hash \
         FROM queryinsights.exec_requests_history \
         ORDER BY start_time DESC"
    );
    execute_lakehouse_query(cli, client, workspace, id, &sql).await
}
