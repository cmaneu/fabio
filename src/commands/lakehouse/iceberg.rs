use anyhow::Result;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

use super::iceberg_warehouse;

// ─── Iceberg REST Catalog (OneLake Table API) ────────────────────────────────

pub(super) async fn iceberg_config(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let wh = iceberg_warehouse(workspace, id);
    let path = format!("iceberg/v1/config?warehouse={wh}");
    let result = client.get_onelake_table_api(&path).await?;
    output::render_object(cli, &result, "defaults");
    Ok(())
}

pub(super) async fn iceberg_namespaces(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
) -> Result<()> {
    let wh = iceberg_warehouse(workspace, id);
    let path = format!("iceberg/v1/{wh}/namespaces");
    let result = client.get_onelake_table_api(&path).await?;
    output::render_object(cli, &result, "namespaces");
    Ok(())
}

pub(super) async fn iceberg_namespace(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    namespace: &str,
) -> Result<()> {
    let wh = iceberg_warehouse(workspace, id);
    let encoded_ns = urlencoding::encode(namespace);
    let path = format!("iceberg/v1/{wh}/namespaces/{encoded_ns}");
    let result = client.get_onelake_table_api(&path).await?;
    output::render_object(cli, &result, "namespace");
    Ok(())
}

pub(super) async fn iceberg_tables(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    namespace: &str,
) -> Result<()> {
    let wh = iceberg_warehouse(workspace, id);
    let encoded_ns = urlencoding::encode(namespace);
    let path = format!("iceberg/v1/{wh}/namespaces/{encoded_ns}/tables");
    let result = client.get_onelake_table_api(&path).await?;
    output::render_object(cli, &result, "identifiers");
    Ok(())
}

pub(super) async fn iceberg_table(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    namespace: &str,
    table: &str,
) -> Result<()> {
    let wh = iceberg_warehouse(workspace, id);
    let encoded_ns = urlencoding::encode(namespace);
    let encoded_table = urlencoding::encode(table);
    let path = format!("iceberg/v1/{wh}/namespaces/{encoded_ns}/tables/{encoded_table}");
    let result = client.get_onelake_table_api(&path).await?;
    output::render_object(cli, &result, "metadata");
    Ok(())
}

pub(super) async fn iceberg_table_exists(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    namespace: &str,
    table: &str,
) -> Result<()> {
    let wh = iceberg_warehouse(workspace, id);
    let encoded_ns = urlencoding::encode(namespace);
    let encoded_table = urlencoding::encode(table);
    let path = format!("iceberg/v1/{wh}/namespaces/{encoded_ns}/tables/{encoded_table}");
    let exists = client.head_onelake_table_api(&path).await?;
    output::render_object(cli, &serde_json::json!({"exists": exists}), "exists");
    Ok(())
}

pub(super) async fn iceberg_namespace_exists(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    namespace: &str,
) -> Result<()> {
    let wh = iceberg_warehouse(workspace, id);
    let encoded_ns = urlencoding::encode(namespace);
    let path = format!("iceberg/v1/{wh}/namespaces/{encoded_ns}");
    let exists = client.head_onelake_table_api(&path).await?;
    output::render_object(cli, &serde_json::json!({"exists": exists}), "exists");
    Ok(())
}

pub(super) async fn iceberg_credentials(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    namespace: &str,
    table: &str,
) -> Result<()> {
    let wh = iceberg_warehouse(workspace, id);
    let encoded_ns = urlencoding::encode(namespace);
    let encoded_table = urlencoding::encode(table);
    let path =
        format!("iceberg/v1/{wh}/namespaces/{encoded_ns}/tables/{encoded_table}/credentials");
    let result = client.get_onelake_table_api(&path).await?;
    output::render_object(cli, &result, "config");
    Ok(())
}

pub(super) async fn iceberg_stats(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    namespace: &str,
    table: &str,
) -> Result<()> {
    let wh = iceberg_warehouse(workspace, id);
    let encoded_ns = urlencoding::encode(namespace);
    let encoded_table = urlencoding::encode(table);
    let path = format!("iceberg/v1/{wh}/namespaces/{encoded_ns}/tables/{encoded_table}");
    let result = client.get_onelake_table_api(&path).await?;

    // Extract stats from the metadata
    let metadata = result.get("metadata").unwrap_or(&result);
    let format_version = metadata
        .get("format-version")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let current_schema_id = metadata
        .get("current-schema-id")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    // Count columns in the current schema
    let columns = metadata
        .get("schemas")
        .and_then(|s| s.as_array())
        .and_then(|schemas| {
            schemas.iter().find(|s| {
                s.get("schema-id").and_then(Value::as_u64).unwrap_or(0) == current_schema_id
            })
        })
        .and_then(|schema| schema.get("fields"))
        .and_then(|f| f.as_array())
        .map_or(0, Vec::len);

    // Extract latest snapshot summary
    let latest_snapshot = metadata
        .get("snapshots")
        .and_then(|s| s.as_array())
        .and_then(|snaps| snaps.last());

    let summary = latest_snapshot.and_then(|s| s.get("summary"));

    let total_records = summary
        .and_then(|s| s.get("total-records"))
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let total_files = summary
        .and_then(|s| s.get("total-data-files"))
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let total_size = summary
        .and_then(|s| s.get("total-files-size"))
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    // Extract compression and source format from properties
    let properties = metadata.get("properties");
    let compression = properties
        .and_then(|p| p.get("write.parquet.compression-codec"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let source_format = properties
        .and_then(|p| p.get("XTABLE_METADATA"))
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .and_then(|v| v.get("sourceTableFormat").cloned())
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    let last_updated = latest_snapshot
        .and_then(|s| s.get("timestamp-ms"))
        .and_then(Value::as_u64);

    let mut stats = serde_json::json!({
        "table": table,
        "namespace": namespace,
        "format_version": format_version,
        "current_schema_id": current_schema_id,
        "columns": columns,
        "total_records": total_records,
        "total_data_files": total_files,
        "total_size_bytes": total_size,
        "compression": compression,
    });

    if !source_format.is_empty() {
        stats["source_format"] = Value::String(source_format);
    }
    if let Some(ts) = last_updated {
        stats["last_updated_ms"] = Value::Number(ts.into());
    }

    output::render_object(cli, &stats, "table");
    Ok(())
}

pub(super) async fn iceberg_snapshots(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    namespace: &str,
    table: &str,
) -> Result<()> {
    let wh = iceberg_warehouse(workspace, id);
    let encoded_ns = urlencoding::encode(namespace);
    let encoded_table = urlencoding::encode(table);
    let path = format!("iceberg/v1/{wh}/namespaces/{encoded_ns}/tables/{encoded_table}");
    let result = client.get_onelake_table_api(&path).await?;

    let metadata = result.get("metadata").unwrap_or(&result);
    let snapshots = metadata
        .get("snapshots")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();

    // Build a concise snapshot history
    let history: Vec<Value> = snapshots
        .iter()
        .map(|snap| {
            let summary = snap.get("summary");
            let mut entry = serde_json::json!({
                "snapshot_id": snap.get("snapshot-id"),
                "timestamp_ms": snap.get("timestamp-ms"),
            });
            if let Some(s) = summary {
                if let Some(op) = s.get("operation") {
                    entry["operation"] = op.clone();
                }
                if let Some(v) = s.get("added-records") {
                    entry["added_records"] = v.clone();
                }
                if let Some(v) = s.get("total-records") {
                    entry["total_records"] = v.clone();
                }
                if let Some(v) = s.get("added-data-files") {
                    entry["added_data_files"] = v.clone();
                }
                if let Some(v) = s.get("total-data-files") {
                    entry["total_data_files"] = v.clone();
                }
                if let Some(v) = s.get("total-files-size") {
                    entry["total_size_bytes"] = v.clone();
                }
            }
            entry
        })
        .collect();

    let output_val = serde_json::json!({"snapshots": history, "count": history.len()});
    output::render_object(cli, &output_val, "snapshots");
    Ok(())
}
