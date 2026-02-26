use std::collections::HashMap;

use serde_json::Value;

use crate::commands::common::{optional_import_field_names, required_import_field_names};
use crate::import::invalid_input_error;
use crate::{ClientError, ClientResult};

#[derive(Debug, Clone)]
pub(crate) struct ParsedRow {
    pub(crate) row: i64,
    pub(crate) statement_id: Option<String>,
    pub(crate) account_key: Option<String>,
    pub(crate) posted_at: Option<String>,
    pub(crate) amount: Option<String>,
    pub(crate) currency: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) external_id: Option<String>,
    pub(crate) merchant: Option<String>,
    pub(crate) category: Option<String>,
}

pub(crate) fn parse_source(content: &str) -> ClientResult<Vec<ParsedRow>> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(invalid_input_error("Import source is empty."));
    }

    if looks_like_ndjson(trimmed) {
        return Err(ClientError::invalid_import_format(
            "NDJSON is not supported in this phase. Provide a JSON array or CSV.",
            "ndjson",
        ));
    }

    if trimmed.starts_with('[') {
        return parse_json_array(trimmed);
    }

    if looks_like_csv(trimmed) {
        return parse_csv(trimmed);
    }

    if serde_json::from_str::<Value>(trimmed).is_ok() {
        return Err(ClientError::invalid_import_format(
            "JSON input must be a top-level array of transaction objects.",
            "json_non_array",
        ));
    }

    Err(ClientError::invalid_import_format(
        "Unsupported import format. Provide a JSON array or CSV with headers.",
        "unknown",
    ))
}

fn parse_json_array(content: &str) -> ClientResult<Vec<ParsedRow>> {
    let parsed = serde_json::from_str::<Value>(content)
        .map_err(|_| invalid_input_error("Invalid JSON input. Provide a valid JSON array."))?;

    let Some(items) = parsed.as_array() else {
        return Err(invalid_input_error(
            "JSON input must be a top-level array of transaction objects.",
        ));
    };

    let mut rows = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(object) = item.as_object() else {
            return Err(invalid_input_error(
                "JSON array entries must all be objects with transaction fields.",
            ));
        };

        rows.push(ParsedRow {
            row: (index as i64) + 1,
            statement_id: read_optional_string(object.get("statement_id")),
            account_key: read_optional_string(object.get("account_key")),
            posted_at: read_optional_string(object.get("posted_at")),
            amount: read_optional_string(object.get("amount")),
            currency: read_optional_string(object.get("currency")),
            description: read_optional_string(object.get("description")),
            external_id: read_optional_string(object.get("external_id")),
            merchant: read_optional_string(object.get("merchant")),
            category: read_optional_string(object.get("category")),
        });
    }

    Ok(rows)
}

fn parse_csv(content: &str) -> ClientResult<Vec<ParsedRow>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());

    let headers = reader
        .headers()
        .map_err(|_| invalid_input_error("CSV header row is missing or unreadable."))?
        .iter()
        .map(|value| value.trim().to_string())
        .collect::<Vec<String>>();

    if !headers_are_valid(&headers) {
        return Err(ClientError::import_schema_mismatch(
            expected_headers(),
            headers,
        ));
    }

    let index_by_name = headers
        .iter()
        .enumerate()
        .map(|(index, name)| (name.to_string(), index))
        .collect::<HashMap<String, usize>>();

    let mut rows = Vec::new();
    for (row_index, result_row) in reader.records().enumerate() {
        let record =
            result_row.map_err(|_| invalid_input_error("CSV rows are malformed or not UTF-8."))?;

        rows.push(ParsedRow {
            row: (row_index as i64) + 1,
            statement_id: value_for(&record, &index_by_name, "statement_id"),
            account_key: value_for(&record, &index_by_name, "account_key"),
            posted_at: value_for(&record, &index_by_name, "posted_at"),
            amount: value_for(&record, &index_by_name, "amount"),
            currency: value_for(&record, &index_by_name, "currency"),
            description: value_for(&record, &index_by_name, "description"),
            external_id: value_for(&record, &index_by_name, "external_id"),
            merchant: value_for(&record, &index_by_name, "merchant"),
            category: value_for(&record, &index_by_name, "category"),
        });
    }

    Ok(rows)
}

fn value_for(
    record: &csv::StringRecord,
    index_by_name: &HashMap<String, usize>,
    field_name: &str,
) -> Option<String> {
    let index = index_by_name.get(field_name)?;
    let value = record.get(*index)?;
    Some(value.to_string())
}

fn read_optional_string(value: Option<&Value>) -> Option<String> {
    let current = value?;

    if current.is_null() {
        return None;
    }

    if let Some(string_value) = current.as_str() {
        return Some(string_value.to_string());
    }

    if let Some(number_value) = current.as_f64() {
        return Some(number_value.to_string());
    }

    Some(current.to_string())
}

fn looks_like_ndjson(content: &str) -> bool {
    let lines = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<&str>>();
    if lines.len() < 2 {
        return false;
    }

    lines.iter().all(|line| {
        let parsed = serde_json::from_str::<Value>(line.trim());
        if let Ok(value) = parsed {
            return value.is_object();
        }
        false
    })
}

fn looks_like_csv(content: &str) -> bool {
    let Some(first_line) = content.lines().find(|line| !line.trim().is_empty()) else {
        return false;
    };
    first_line.contains(',')
}

fn headers_are_valid(actual_headers: &[String]) -> bool {
    let required_fields = required_import_field_names();
    let optional_fields = optional_import_field_names();

    for required in &required_fields {
        if !actual_headers.iter().any(|value| value == required) {
            return false;
        }
    }

    for header in actual_headers {
        let allowed = required_fields
            .iter()
            .any(|value| value == &header.as_str())
            || optional_fields
                .iter()
                .any(|value| value == &header.as_str());
        if !allowed {
            return false;
        }
    }

    true
}

fn expected_headers() -> Vec<String> {
    let required_fields = required_import_field_names();
    let optional_fields = optional_import_field_names();

    let mut headers = required_fields
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<String>>();
    headers.extend(optional_fields.iter().map(|value| value.to_string()));
    headers
}
