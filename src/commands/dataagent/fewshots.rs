use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError};
use crate::output;

use super::{decode_part_payload, find_datasource_dir, get_definition_parts};

/// List few-shot examples for a specific data source.
pub(super) async fn list_fewshots(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
) -> Result<()> {
    let parts = get_definition_parts(client, workspace, id).await?;
    let fewshots = extract_fewshots_for_datasource(&parts, datasource)?;

    output::render_list_with_token(
        cli,
        &fewshots,
        &["id", "question", "query"],
        &["ID", "QUESTION", "QUERY"],
        "id",
        None,
    );
    Ok(())
}

/// Add a few-shot example to a data source.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) async fn add_fewshot(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
    question: &str,
    query_text: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent add-fewshot",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
            "question": question,
            "query": query_text,
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;

    // Find the fewshots part for this datasource and the datasource directory prefix
    let ds_dir = find_datasource_dir(&parts, datasource)?;
    let fewshots_path = format!("{ds_dir}/fewshots.json");

    // Find existing fewshots content or create empty
    let existing_payload = parts.iter().find_map(|part| {
        let path = part.get("path").and_then(Value::as_str)?;
        if path == fewshots_path {
            part.get("payload")
                .and_then(Value::as_str)
                .map(String::from)
        } else {
            None
        }
    });

    let mut fewshots_data = existing_payload.as_ref().map_or_else(
        || serde_json::json!({
            "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/fewShots/1.0.0/schema.json",
            "fewShots": []
        }),
        |payload| {
            decode_part_payload(payload)
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| serde_json::json!({"fewShots": []}))
        },
    );

    // Add the new fewshot (with duplicate detection)
    let fewshots_arr = fewshots_data
        .get_mut("fewShots")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "Invalid fewshots structure in definition",
            )
        })?;

    // Check for duplicates (case-insensitive)
    let question_lower = question.to_lowercase();
    let has_duplicate = fewshots_arr.iter().any(|f| {
        f.get("question")
            .or_else(|| f.get("Question"))
            .and_then(Value::as_str)
            .is_some_and(|q| q.to_lowercase() == question_lower)
    });

    let saved_question = if has_duplicate {
        // Find next available suffix
        let mut suffix = 1;
        loop {
            let candidate = format!("{question} [{suffix}]").to_lowercase();
            let exists = fewshots_arr.iter().any(|f| {
                f.get("question")
                    .or_else(|| f.get("Question"))
                    .and_then(Value::as_str)
                    .is_some_and(|q| q.to_lowercase() == candidate)
            });
            if !exists {
                break;
            }
            suffix += 1;
        }
        format!("{question} [{suffix}]")
    } else {
        question.to_string()
    };

    let new_id = uuid::Uuid::new_v4().to_string();
    fewshots_arr.push(serde_json::json!({
        "id": new_id,
        "question": saved_question,
        "query": query_text,
    }));

    // Rebuild definition parts
    let encoded = BASE64.encode(serde_json::to_string(&fewshots_data)?.as_bytes());

    let mut new_parts: Vec<Value> = parts
        .iter()
        .filter(|p| p.get("path").and_then(Value::as_str) != Some(&fewshots_path))
        .cloned()
        .collect();
    new_parts.push(serde_json::json!({
        "path": fewshots_path,
        "payload": encoded,
        "payloadType": "InlineBase64"
    }));

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "status": "fewshot_added",
        "id": new_id,
        "question": saved_question,
        "query": query_text,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Remove a few-shot example by ID.
pub(super) async fn remove_fewshot(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
    fewshot_id: &str,
) -> Result<()> {
    if output::dry_run_guard(
        cli,
        "data-agent remove-fewshot",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
            "fewshotId": fewshot_id,
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;
    let ds_dir = find_datasource_dir(&parts, datasource)?;
    let fewshots_path = format!("{ds_dir}/fewshots.json");

    let payload = parts
        .iter()
        .find_map(|part| {
            let path = part.get("path").and_then(Value::as_str)?;
            if path == fewshots_path {
                part.get("payload").and_then(Value::as_str).map(String::from)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            FabioError::with_hint(
                ErrorCode::NotFound,
                format!("No fewshots found for data source '{datasource}'"),
                "Add fewshots first: fabio data-agent add-fewshot -w <workspace> --id <id> --datasource <ds> --question '...' --answer '...'",
            )
        })?;

    let mut fewshots_data = decode_part_payload(&payload)
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| serde_json::json!({"fewShots": []}));

    let arr = fewshots_data
        .get_mut("fewShots")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| FabioError::new(ErrorCode::ApiError, "Invalid fewshots structure"))?;

    let removed: Vec<_> = arr
        .extract_if(.., |f| {
            f.get("id")
                .and_then(Value::as_str)
                .is_some_and(|fid| fid == fewshot_id)
        })
        .collect();

    if removed.is_empty() {
        return Err(FabioError::with_hint(
            ErrorCode::NotFound,
            format!("Few-shot '{fewshot_id}' not found"),
            "List fewshots: fabio data-agent list-fewshots -w <workspace> --id <id> --datasource <ds>",
        )
        .into());
    }

    let encoded = BASE64.encode(serde_json::to_string(&fewshots_data)?.as_bytes());

    let new_parts: Vec<Value> = parts
        .iter()
        .map(|p| {
            if p.get("path").and_then(Value::as_str) == Some(&fewshots_path) {
                serde_json::json!({
                    "path": fewshots_path,
                    "payload": encoded,
                    "payloadType": "InlineBase64"
                })
            } else {
                p.clone()
            }
        })
        .collect();

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "id": id,
        "status": "fewshot_removed",
        "fewshotId": fewshot_id,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

/// Bulk upload few-shot examples from a JSON file.
#[allow(clippy::too_many_lines)]
pub(super) async fn upload_fewshots(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    datasource: &str,
    file: &str,
) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{file}': {e}"))?;

    // Detect format by file extension
    let ext = std::path::Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let items: Vec<Value> = if ext == "csv" || ext == "tsv" {
        parse_fewshots_csv(&content, file)?
    } else {
        serde_json::from_str(&content).map_err(|e| {
            FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Invalid JSON in '{file}': {e}"),
                r#"Expected JSON format: [{"question":"...","query":"..."}] or use .csv file with question,query columns"#,
            )
        })?
    };

    if items.is_empty() {
        return Err(
            FabioError::invalid_input("File contains no few-shot examples (empty array)").into(),
        );
    }

    // Validate all entries have question + query
    for (i, item) in items.iter().enumerate() {
        if item.get("question").and_then(Value::as_str).is_none() {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Item {i} is missing 'question' field"),
                r#"Each item must have: {{"question":"...", "query":"..."}}"#,
            )
            .into());
        }
        if item.get("query").and_then(Value::as_str).is_none() {
            return Err(FabioError::with_hint(
                ErrorCode::InvalidInput,
                format!("Item {i} is missing 'query' field"),
                r#"Each item must have: {{"question":"...", "query":"..."}}"#,
            )
            .into());
        }
    }

    if output::dry_run_guard(
        cli,
        "data-agent upload-fewshots",
        &serde_json::json!({
            "workspace": workspace,
            "id": id,
            "datasource": datasource,
            "file": file,
            "count": items.len(),
        }),
    ) {
        return Ok(());
    }

    let parts = get_definition_parts(client, workspace, id).await?;
    let ds_dir = find_datasource_dir(&parts, datasource)?;
    let fewshots_path = format!("{ds_dir}/fewshots.json");

    // Load existing fewshots
    let existing_payload = parts.iter().find_map(|part| {
        let path = part.get("path").and_then(Value::as_str)?;
        if path == fewshots_path {
            part.get("payload")
                .and_then(Value::as_str)
                .map(String::from)
        } else {
            None
        }
    });

    let mut fewshots_data = existing_payload.as_ref().map_or_else(
        || {
            serde_json::json!({
                "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/dataAgent/definition/fewShots/1.0.0/schema.json",
                "fewShots": []
            })
        },
        |payload| {
            decode_part_payload(payload)
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| serde_json::json!({"fewShots": []}))
        },
    );

    let fewshots_arr = fewshots_data
        .get_mut("fewShots")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            FabioError::new(
                ErrorCode::ApiError,
                "Invalid fewshots structure in definition",
            )
        })?;

    // Build set of existing questions for duplicate detection
    let mut existing_questions: std::collections::HashSet<String> = fewshots_arr
        .iter()
        .filter_map(|f| {
            f.get("question")
                .or_else(|| f.get("Question"))
                .and_then(Value::as_str)
                .map(str::to_lowercase)
        })
        .collect();

    let mut added = 0;
    let mut renamed = 0;

    for item in &items {
        let question = item
            .get("question")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let query_text = item
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or_default();

        let mut saved_question = question.to_string();
        if existing_questions.contains(&saved_question.to_lowercase()) {
            let mut suffix = 1;
            loop {
                let candidate = format!("{question} [{suffix}]").to_lowercase();
                if !existing_questions.contains(&candidate) {
                    break;
                }
                suffix += 1;
            }
            saved_question = format!("{question} [{suffix}]");
            renamed += 1;
        }

        existing_questions.insert(saved_question.to_lowercase());
        fewshots_arr.push(serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "question": saved_question,
            "query": query_text,
        }));
        added += 1;
    }

    let total = fewshots_arr.len();

    // Update definition (fewshots_arr borrow ends here)
    let encoded = BASE64.encode(serde_json::to_string(&fewshots_data)?.as_bytes());

    let new_parts: Vec<Value> = parts
        .iter()
        .filter(|p| p.get("path").and_then(Value::as_str) != Some(&fewshots_path))
        .cloned()
        .chain(std::iter::once(serde_json::json!({
            "path": fewshots_path,
            "payload": encoded,
            "payloadType": "InlineBase64"
        })))
        .collect();

    let update_body = serde_json::json!({ "definition": { "parts": new_parts } });
    client
        .post(
            &format!("/workspaces/{workspace}/dataAgents/{id}/updateDefinition"),
            &update_body,
            true,
        )
        .await?;

    let result = serde_json::json!({
        "status": "fewshots_uploaded",
        "added": added,
        "renamed": renamed,
        "total": total,
    });
    output::render_object(cli, &result, "status");
    Ok(())
}

// ─── Private Helpers ─────────────────────────────────────────────────────────

/// Extract few-shot examples for a specific data source.
fn extract_fewshots_for_datasource(parts: &[Value], datasource: &str) -> Result<Vec<Value>> {
    let ds_dir = find_datasource_dir(parts, datasource)?;
    let fewshots_path = format!("{ds_dir}/fewshots.json");

    for part in parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if path == fewshots_path {
            let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");
            if let Some(decoded) = decode_part_payload(payload)
                && let Ok(parsed) = serde_json::from_str::<Value>(&decoded)
            {
                return Ok(parsed
                    .get("fewShots")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default());
            }
        }
    }
    Ok(Vec::new())
}

/// Parse few-shot examples from a CSV/TSV file.
///
/// Expects columns named `question` and `query` (case-insensitive headers).
/// TSV is auto-detected from `.tsv` extension.
fn parse_fewshots_csv(content: &str, file: &str) -> Result<Vec<Value>> {
    let ext = std::path::Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let delimiter = if ext == "tsv" { b'\t' } else { b',' };

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    // Find column indices for "question" and "query" (case-insensitive)
    let headers = reader.headers().map_err(|e| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("Failed to parse CSV headers in '{file}': {e}"),
            "CSV must have a header row with 'question' and 'query' columns",
        )
    })?;

    let question_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("question"));
    let query_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("query") || h.eq_ignore_ascii_case("answer"));

    let question_idx = question_idx.ok_or_else(|| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("CSV file '{file}' is missing a 'question' column header"),
            format!(
                "Found columns: {}. Expected: question,query",
                headers.iter().collect::<Vec<_>>().join(", ")
            ),
        )
    })?;
    let query_idx = query_idx.ok_or_else(|| {
        FabioError::with_hint(
            ErrorCode::InvalidInput,
            format!("CSV file '{file}' is missing a 'query' (or 'answer') column header"),
            format!(
                "Found columns: {}. Expected: question,query",
                headers.iter().collect::<Vec<_>>().join(", ")
            ),
        )
    })?;

    let mut items = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record.map_err(|e| {
            FabioError::new(
                ErrorCode::InvalidInput,
                format!("Failed to parse CSV row {i} in '{file}': {e}"),
            )
        })?;

        let question = record.get(question_idx).unwrap_or("").trim();
        let query = record.get(query_idx).unwrap_or("").trim();

        if question.is_empty() || query.is_empty() {
            continue; // Skip empty rows
        }

        items.push(serde_json::json!({
            "question": question,
            "query": query,
        }));
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use serde_json::json;

    use super::*;

    #[test]
    fn extract_fewshots_for_datasource_found() {
        let ds_json =
            json!({"displayName": "TestLH", "type": "lakehouse_tables", "artifactId": "x"});
        let ds_payload = BASE64.encode(ds_json.to_string().as_bytes());
        let fs_json = json!({
            "fewShots": [
                {"id": "fs1", "question": "How many?", "query": "SELECT COUNT(*) FROM t"}
            ]
        });
        let fs_payload = BASE64.encode(fs_json.to_string().as_bytes());

        let parts = vec![
            json!({
                "path": "Files/Config/draft/lakehouse_tables-TestLH/datasource.json",
                "payload": ds_payload,
                "payloadType": "InlineBase64"
            }),
            json!({
                "path": "Files/Config/draft/lakehouse_tables-TestLH/fewshots.json",
                "payload": fs_payload,
                "payloadType": "InlineBase64"
            }),
        ];

        let fewshots = extract_fewshots_for_datasource(&parts, "TestLH").unwrap();
        assert_eq!(fewshots.len(), 1);
        assert_eq!(fewshots[0]["id"], "fs1");
        assert_eq!(fewshots[0]["question"], "How many?");
    }

    #[test]
    fn extract_fewshots_empty_when_no_file() {
        let ds_json =
            json!({"displayName": "TestLH", "type": "lakehouse_tables", "artifactId": "x"});
        let ds_payload = BASE64.encode(ds_json.to_string().as_bytes());
        let parts = vec![json!({
            "path": "Files/Config/draft/lakehouse_tables-TestLH/datasource.json",
            "payload": ds_payload,
            "payloadType": "InlineBase64"
        })];

        let fewshots = extract_fewshots_for_datasource(&parts, "TestLH").unwrap();
        assert!(fewshots.is_empty());
    }

    #[test]
    fn parse_csv_fewshots_basic() {
        let csv = "question,query\nHow many?,SELECT COUNT(*) FROM t\nMax price?,SELECT MAX(price) FROM p\n";
        let items = parse_fewshots_csv(csv, "test.csv").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["question"], "How many?");
        assert_eq!(items[0]["query"], "SELECT COUNT(*) FROM t");
        assert_eq!(items[1]["question"], "Max price?");
    }

    #[test]
    fn parse_csv_fewshots_case_insensitive_headers() {
        let csv = "Question,Query\nTest?,SELECT 1\n";
        let items = parse_fewshots_csv(csv, "test.csv").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["question"], "Test?");
    }

    #[test]
    fn parse_csv_fewshots_answer_column() {
        // 'answer' is an alias for 'query' column
        let csv = "question,answer\nHow many?,SELECT COUNT(*) FROM t\n";
        let items = parse_fewshots_csv(csv, "test.csv").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["query"], "SELECT COUNT(*) FROM t");
    }

    #[test]
    fn parse_csv_fewshots_tsv() {
        let tsv = "question\tquery\nHow many?\tSELECT COUNT(*) FROM t\n";
        let items = parse_fewshots_csv(tsv, "data.tsv").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["question"], "How many?");
    }

    #[test]
    fn parse_csv_fewshots_missing_question_column() {
        let csv = "prompt,query\nHow?,SELECT 1\n";
        let err = parse_fewshots_csv(csv, "bad.csv").unwrap_err();
        assert!(err.to_string().contains("question"));
    }

    #[test]
    fn parse_csv_fewshots_skips_empty_rows() {
        let csv =
            "question,query\nHow many?,SELECT COUNT(*) FROM t\n,\nMax?,SELECT MAX(x) FROM y\n";
        let items = parse_fewshots_csv(csv, "test.csv").unwrap();
        assert_eq!(items.len(), 2);
    }
}
