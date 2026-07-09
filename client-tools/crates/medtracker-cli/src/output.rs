use anyhow::Result;
use comfy_table::{Cell, Table, presets::UTF8_FULL};
use medtracker_api_client::ApiError;
use serde_json::Value;

use crate::args::OutputFormat;

pub fn print_value(value: &Value, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        OutputFormat::Table => print_table(value),
    }

    Ok(())
}

pub fn format_api_error(error: ApiError) -> anyhow::Error {
    match error {
        ApiError::Server {
            code,
            message,
            request_id,
            retry_after,
            ..
        } => {
            let mut text = format!("{code}: {message}");
            if let Some(request_id) = request_id {
                text.push_str(&format!(" request_id={request_id}"));
            }
            if let Some(retry_after) = retry_after {
                text.push_str(&format!(" retry_after={retry_after}"));
            }
            anyhow::anyhow!(text)
        }
        other => anyhow::anyhow!(other),
    }
}

fn print_table(value: &Value) {
    match value {
        Value::Array(items) => print_rows(items),
        Value::Object(object) => {
            if let Some(Value::Array(items)) = object.values().find(|item| item.is_array()) {
                print_rows(items);
            } else {
                print_key_values(value);
            }
        }
        _ => println!("{value}"),
    }
}

fn print_rows(items: &[Value]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["id", "name", "type", "summary"]);

    for item in items {
        table.add_row(vec![
            Cell::new(display_field(item, "id")),
            Cell::new(display_field(item, "name")),
            Cell::new(display_field(item, "type")),
            Cell::new(summary(item)),
        ]);
    }

    println!("{table}");
}

fn print_key_values(value: &Value) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["field", "value"]);

    if let Value::Object(object) = value {
        for (key, item) in object {
            table.add_row(vec![Cell::new(key), Cell::new(compact(item))]);
        }
    }

    println!("{table}");
}

fn display_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .or_else(|| value.get(key).map(compact))
        .unwrap_or_default()
}

fn compact(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn summary(value: &Value) -> String {
    match value {
        Value::Object(object) => object
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "id" | "name" | "type"))
            .take(3)
            .map(|(key, value)| format!("{key}={}", compact(value)))
            .collect::<Vec<_>>()
            .join(", "),
        _ => compact(value),
    }
}
