use anyhow::Result;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

use super::execute_insights_query;

/// List all statistics objects, optionally filtered by table.
pub(super) async fn list(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    table: Option<&str>,
) -> Result<()> {
    let base_query = "SELECT s.name AS statistic_name, \
         SCHEMA_NAME(t.schema_id) AS schema_name, \
         t.name AS table_name, \
         c.name AS column_name, \
         s.auto_created, s.user_created \
         FROM sys.stats s \
         JOIN sys.stats_columns sc ON s.object_id = sc.object_id AND s.stats_id = sc.stats_id \
         JOIN sys.columns c ON sc.object_id = c.object_id AND sc.column_id = c.column_id \
         JOIN sys.tables t ON s.object_id = t.object_id";

    let sql = table.map_or_else(
        || format!("{base_query} ORDER BY t.name, s.name"),
        |tbl| {
            let (schema, table_name) = tbl.split_once('.').unwrap_or(("dbo", tbl));
            format!(
                "{base_query} \
                 WHERE SCHEMA_NAME(t.schema_id) = '{schema}' AND t.name = '{table_name}' \
                 ORDER BY s.name"
            )
        },
    );
    execute_insights_query(cli, client, workspace, id, &sql).await
}

/// Show statistic details using `DBCC SHOW_STATISTICS`.
pub(super) async fn show(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: &str,
) -> Result<()> {
    // DBCC SHOW_STATISTICS requires (table_name, statistics_name).
    // We look up the table from sys.stats to find the owning table.
    let sql = format!(
        "DECLARE @tbl NVARCHAR(500); \
         SELECT @tbl = QUOTENAME(SCHEMA_NAME(t.schema_id)) + '.' + QUOTENAME(t.name) \
         FROM sys.stats s JOIN sys.tables t ON s.object_id = t.object_id \
         WHERE s.name = N'{name}'; \
         IF @tbl IS NULL RAISERROR('Statistic not found: {name}', 16, 1); \
         DBCC SHOW_STATISTICS (@tbl, N'{name}')"
    );
    execute_insights_query(cli, client, workspace, id, &sql).await
}

/// Create a user-defined statistic on a column.
pub(super) async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    table: &str,
    column: &str,
    name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "warehouse statistics-create",
        &serde_json::json!({ "table": table, "column": column, "name": name }),
    ) {
        return Ok(());
    }

    let sql = format!("CREATE STATISTICS [{name}] ON {table} ([{column}])");
    execute_insights_query(cli, client, workspace, id, &sql).await?;

    let obj = serde_json::json!({
        "name": name,
        "table": table,
        "column": column,
        "status": "created"
    });
    output::render_object(cli, &obj, "status");
    Ok(())
}

/// Update (refresh) an existing statistic.
pub(super) async fn update(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "warehouse statistics-update",
        &serde_json::json!({ "name": name }),
    ) {
        return Ok(());
    }

    let sql = format!("UPDATE STATISTICS [{name}]");
    execute_insights_query(cli, client, workspace, id, &sql).await?;

    let obj = serde_json::json!({ "name": name, "status": "updated" });
    output::render_object(cli, &obj, "status");
    Ok(())
}

/// Delete a user-defined statistic.
pub(super) async fn delete(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "warehouse statistics-delete",
        &serde_json::json!({ "name": name }),
    ) {
        return Ok(());
    }

    let sql = format!("DROP STATISTICS [{name}]");
    execute_insights_query(cli, client, workspace, id, &sql).await?;

    let obj = serde_json::json!({ "name": name, "status": "deleted" });
    output::render_object(cli, &obj, "status");
    Ok(())
}
