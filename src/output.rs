use comfy_table::{Cell, Table};
use serde::Serialize;
use serde_json::Value;

use crate::cli::{Cli, OutputFormat};
use crate::errors::FabioError;

/// JSON envelope for list responses.
#[derive(Serialize)]
pub struct ListEnvelope {
    pub data: Value,
    pub count: usize,
}

/// JSON envelope for single-object responses.
#[derive(Serialize)]
pub struct ObjectEnvelope {
    pub data: Value,
}

/// JSON envelope for errors.
#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

/// Render a list of items respecting --quiet and --query flags.
pub fn render_list(
    cli: &Cli,
    items: &[Value],
    columns: &[&str],
    headers: &[&str],
    plain_key: &str,
) {
    if cli.quiet {
        return;
    }

    let data = Value::Array(items.to_vec());
    let output_data = match cli.query {
        Some(ref q) => apply_query(&data, q),
        None => data,
    };

    #[allow(clippy::redundant_clone)]
    match cli.output {
        OutputFormat::Json => {
            let display_items = if let Value::Array(ref arr) = output_data {
                arr.clone()
            } else {
                vec![output_data.clone()]
            };
            let envelope = ListEnvelope {
                count: display_items.len(),
                data: Value::Array(display_items),
            };
            println!("{}", serde_json::to_string(&envelope).unwrap());
        }
        OutputFormat::Table => {
            if let Value::Array(ref arr) = output_data {
                render_table(arr, columns, headers);
            } else {
                render_table(items, columns, headers);
            }
        }
        OutputFormat::Plain => {
            let arr = if let Value::Array(ref a) = output_data {
                a.as_slice()
            } else {
                items
            };
            for item in arr {
                if let Some(val) = item.get(plain_key) {
                    println!("{}", format_value(val));
                } else {
                    // If query resolved to scalar values, print directly
                    println!("{}", format_value(item));
                }
            }
        }
    }
}

/// Render a single object respecting --quiet and --query flags.
pub fn render_object(cli: &Cli, obj: &Value, plain_key: &str) {
    if cli.quiet {
        return;
    }

    let output_data = cli
        .query
        .as_ref()
        .map_or_else(|| obj.clone(), |q| apply_query(obj, q));

    match cli.output {
        OutputFormat::Json => {
            let envelope = ObjectEnvelope { data: output_data };
            println!("{}", serde_json::to_string(&envelope).unwrap());
        }
        OutputFormat::Table => {
            // For single objects, render as key-value pairs
            if let Value::Object(map) = &output_data {
                let mut table = Table::new();
                table.set_header(vec!["KEY", "VALUE"]);
                for (key, val) in map {
                    table.add_row(vec![Cell::new(key), Cell::new(format_value(val))]);
                }
                println!("{table}");
            } else {
                // Scalar result from query
                println!("{}", format_value(&output_data));
            }
        }
        OutputFormat::Plain => {
            if let Some(val) = output_data.get(plain_key) {
                println!("{}", format_value(val));
            } else {
                // If output is a scalar or the key doesn't exist, print raw
                match &output_data {
                    Value::String(s) => println!("{s}"),
                    Value::Null => {}
                    other => println!("{}", serde_json::to_string_pretty(other).unwrap()),
                }
            }
        }
    }
}

/// Render an error to stderr as structured JSON.
pub fn render_error(err: &FabioError) {
    let envelope = ErrorEnvelope {
        error: ErrorBody {
            code: err.code.to_string(),
            message: err.message.clone(),
        },
    };
    eprintln!("{}", serde_json::to_string(&envelope).unwrap());
}

/// Render items as an ASCII table.
fn render_table(items: &[Value], columns: &[&str], headers: &[&str]) {
    let mut table = Table::new();
    table.set_header(headers.iter().map(|h| Cell::new(*h)).collect::<Vec<_>>());

    for item in items {
        let row: Vec<Cell> = columns
            .iter()
            .map(|col| {
                let val = item.get(*col).unwrap_or(&Value::Null);
                Cell::new(format_value(val))
            })
            .collect();
        table.add_row(row);
    }

    println!("{table}");
}

/// Format a JSON value for display.
fn format_value(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => serde_json::to_string(val).unwrap_or_default(),
    }
}

/// Apply a simple dot-notation query to extract a field.
pub fn apply_query(value: &Value, query: &str) -> Value {
    let parts: Vec<&str> = query.split('.').collect();
    let mut current = value;
    for part in parts {
        match current {
            Value::Object(map) => {
                current = map.get(part).unwrap_or(&Value::Null);
            }
            Value::Array(arr) => {
                let extracted: Vec<Value> = arr
                    .iter()
                    .filter_map(|item| item.get(part).cloned())
                    .collect();
                return Value::Array(extracted);
            }
            _ => return Value::Null,
        }
    }
    current.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_query_extracts_object_field() {
        let obj = serde_json::json!({"name": "test", "id": "123"});
        assert_eq!(apply_query(&obj, "name"), Value::String("test".to_string()));
    }

    #[test]
    fn apply_query_extracts_nested_field() {
        let obj = serde_json::json!({"a": {"b": {"c": 42}}});
        assert_eq!(apply_query(&obj, "a.b.c"), serde_json::json!(42));
    }

    #[test]
    fn apply_query_extracts_array_field() {
        let arr = serde_json::json!([
            {"name": "alpha", "id": "1"},
            {"name": "beta", "id": "2"},
        ]);
        let result = apply_query(&arr, "name");
        assert_eq!(result, serde_json::json!(["alpha", "beta"]));
    }

    #[test]
    fn apply_query_missing_field_returns_null() {
        let obj = serde_json::json!({"name": "test"});
        assert_eq!(apply_query(&obj, "missing"), Value::Null);
    }

    #[test]
    fn apply_query_on_null_returns_null() {
        assert_eq!(apply_query(&Value::Null, "anything"), Value::Null);
    }

    #[test]
    fn format_value_handles_types() {
        assert_eq!(format_value(&Value::String("hello".into())), "hello");
        assert_eq!(format_value(&serde_json::json!(42)), "42");
        assert_eq!(format_value(&serde_json::json!(true)), "true");
        assert_eq!(format_value(&Value::Null), "");
        assert_eq!(format_value(&serde_json::json!({"a": 1})), r#"{"a":1}"#);
    }
}
