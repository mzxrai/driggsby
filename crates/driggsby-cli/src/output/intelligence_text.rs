use std::io;

use serde_json::{Map, Value};

use super::format::{self, Align, Column};

pub fn render_anomalies(data: &Value) -> io::Result<String> {
    let rows = data
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("anomalies output requires rows"))?;

    let normalized = normalize_anomaly_rows(rows);
    if normalized.is_empty() {
        return Ok([
            "No anomalies found.",
            "",
            "Your database has no imported transactions yet. Once you import",
            "transaction data, Driggsby will automatically detect spending anomalies.",
        ]
        .join("\n"));
    }

    let from = data.get("from").and_then(Value::as_str);
    let to = data.get("to").and_then(Value::as_str);

    let mut lines = vec![
        anomalies_heading(normalized.len(), from, to),
        String::new(),
        "Findings:".to_string(),
    ];

    let columns = [
        Column {
            name: "Date",
            align: Align::Left,
        },
        Column {
            name: "Merchant",
            align: Align::Left,
        },
        Column {
            name: "Amount",
            align: Align::Right,
        },
        Column {
            name: "Reason",
            align: Align::Left,
        },
    ];

    let table_rows = normalized
        .iter()
        .map(|row| {
            vec![
                row.get("posted_at")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                row.get("merchant")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                format_amount(row),
                row.get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            ]
        })
        .collect::<Vec<Vec<String>>>();

    lines.extend(format::render_table_or_blocks(
        &columns,
        &table_rows,
        format::terminal_width(),
        "Finding",
    ));

    if let Some(range_hint) = data.get("data_range_hint") {
        let earliest = range_hint.get("earliest").and_then(Value::as_str);
        let latest = range_hint.get("latest").and_then(Value::as_str);
        if earliest.is_some() || latest.is_some() {
            lines.push(String::new());
            lines.push("Summary:".to_string());
            lines.push(format!(
                "  Data covers:  {} to {}",
                earliest.unwrap_or("unknown"),
                latest.unwrap_or("unknown")
            ));
        }
    }

    Ok(lines.join("\n"))
}

pub fn render_recurring(data: &Value) -> io::Result<String> {
    let rows = data
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("recurring output requires rows"))?;

    let normalized = normalize_recurring_rows(rows);
    if normalized.is_empty() {
        return Ok([
            "No recurring patterns found.",
            "",
            "Your database has no imported transactions yet. Once you import",
            "transaction data, Driggsby will automatically detect recurring patterns.",
        ]
        .join("\n"));
    }

    let from = data.get("from").and_then(Value::as_str);
    let to = data.get("to").and_then(Value::as_str);

    let mut lines = vec![
        recurring_heading(normalized.len(), from, to),
        String::new(),
        "Patterns:".to_string(),
    ];

    let columns = [
        Column {
            name: "Merchant",
            align: Align::Left,
        },
        Column {
            name: "Cadence",
            align: Align::Left,
        },
        Column {
            name: "Typical Amount",
            align: Align::Right,
        },
        Column {
            name: "Last Seen",
            align: Align::Left,
        },
        Column {
            name: "Next Expected",
            align: Align::Left,
        },
    ];

    let table_rows = normalized
        .iter()
        .map(|row| {
            vec![
                row.get("merchant")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                row.get("cadence")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                format_amount_like(row, "typical_amount"),
                row.get("last_seen_at")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                row.get("next_expected_at")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            ]
        })
        .collect::<Vec<Vec<String>>>();

    lines.extend(format::render_table_or_blocks(
        &columns,
        &table_rows,
        format::terminal_width(),
        "Pattern",
    ));

    Ok(lines.join("\n"))
}

pub fn normalize_anomaly_rows(rows: &[Value]) -> Vec<Value> {
    let mut normalized = rows
        .iter()
        .map(|row| {
            let mut object = Map::new();
            object.insert(
                "txn_id".to_string(),
                Value::String(
                    row.get("txn_id")
                        .or_else(|| row.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ),
            );
            object.insert(
                "posted_at".to_string(),
                Value::String(
                    row.get("posted_at")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ),
            );
            object.insert(
                "merchant".to_string(),
                Value::String(
                    row.get("merchant")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ),
            );
            object.insert(
                "amount".to_string(),
                row.get("amount").cloned().unwrap_or(Value::from(0.0)),
            );
            object.insert(
                "currency".to_string(),
                Value::String(
                    row.get("currency")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ),
            );
            object.insert(
                "reason".to_string(),
                Value::String(
                    row.get("reason")
                        .or_else(|| row.get("note"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ),
            );
            Value::Object(object)
        })
        .collect::<Vec<Value>>();

    normalized.sort_by(|left, right| {
        value_str(left, "posted_at")
            .cmp(value_str(right, "posted_at"))
            .then_with(|| value_str(left, "merchant").cmp(value_str(right, "merchant")))
            .then_with(|| value_str(left, "txn_id").cmp(value_str(right, "txn_id")))
    });

    normalized
}

pub fn normalize_recurring_rows(rows: &[Value]) -> Vec<Value> {
    let mut normalized = rows
        .iter()
        .map(|row| {
            let mut object = Map::new();
            object.insert(
                "merchant".to_string(),
                Value::String(
                    row.get("merchant")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ),
            );
            object.insert(
                "cadence".to_string(),
                Value::String(
                    row.get("cadence")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                ),
            );
            object.insert(
                "typical_amount".to_string(),
                row.get("typical_amount")
                    .or_else(|| row.get("amount"))
                    .cloned()
                    .unwrap_or(Value::from(0.0)),
            );
            object.insert(
                "currency".to_string(),
                Value::String(
                    row.get("currency")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ),
            );
            object.insert(
                "last_seen_at".to_string(),
                Value::String(
                    row.get("last_seen_at")
                        .or_else(|| row.get("posted_at"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ),
            );
            object.insert(
                "next_expected_at".to_string(),
                row.get("next_expected_at")
                    .and_then(Value::as_str)
                    .map(|value| Value::String(value.to_string()))
                    .unwrap_or(Value::Null),
            );
            Value::Object(object)
        })
        .collect::<Vec<Value>>();

    normalized.sort_by(|left, right| {
        value_str(left, "next_expected_at")
            .cmp(value_str(right, "next_expected_at"))
            .then_with(|| value_str(left, "merchant").cmp(value_str(right, "merchant")))
    });

    normalized
}

fn format_amount(row: &Value) -> String {
    let amount = row.get("amount").and_then(Value::as_f64).unwrap_or(0.0);
    let currency = row.get("currency").and_then(Value::as_str).unwrap_or("USD");
    format!("{amount:.2} {currency}")
}

fn format_amount_like(row: &Value, key: &str) -> String {
    let amount = row.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    let currency = row.get("currency").and_then(Value::as_str).unwrap_or("USD");
    format!("{amount:.2} {currency}")
}

fn anomalies_heading(count: usize, from: Option<&str>, to: Option<&str>) -> String {
    match (from, to) {
        (Some(start), Some(end)) => format!("{count} anomalies detected from {start} to {end}."),
        (Some(start), None) => format!("{count} anomalies detected from {start} onward."),
        (None, Some(end)) => format!("{count} anomalies detected up to {end}."),
        (None, None) => format!("{count} anomalies detected."),
    }
}

fn recurring_heading(count: usize, from: Option<&str>, to: Option<&str>) -> String {
    match (from, to) {
        (Some(start), Some(end)) => {
            format!("{count} recurring patterns detected from {start} to {end}.")
        }
        (Some(start), None) => format!("{count} recurring patterns detected from {start} onward."),
        (None, Some(end)) => format!("{count} recurring patterns detected up to {end}."),
        (None, None) => format!("{count} recurring patterns detected."),
    }
}

fn value_str<'a>(value: &'a Value, key: &str) -> &'a str {
    match value.get(key) {
        Some(Value::String(inner)) => inner.as_str(),
        Some(Value::Null) | None => "",
        Some(other) => other.as_str().unwrap_or(""),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{
        normalize_anomaly_rows, normalize_recurring_rows, render_anomalies, render_recurring,
    };

    #[test]
    fn anomalies_sort_by_posted_at_then_merchant_then_txn_id() {
        let rows = vec![
            json!({ "id": "b", "posted_at": "2026-01-01", "merchant": "Z" }),
            json!({ "id": "a", "posted_at": "2026-01-01", "merchant": "A" }),
            json!({ "id": "c", "posted_at": "2026-01-02", "merchant": "A" }),
        ];

        let normalized = normalize_anomaly_rows(&rows);
        assert_eq!(normalized[0]["txn_id"], Value::String("a".to_string()));
        assert_eq!(normalized[1]["txn_id"], Value::String("b".to_string()));
        assert_eq!(normalized[2]["txn_id"], Value::String("c".to_string()));
    }

    #[test]
    fn recurring_sort_by_next_expected_at_then_merchant() {
        let rows = vec![
            json!({ "merchant": "Z", "next_expected_at": "2026-02-02" }),
            json!({ "merchant": "A", "next_expected_at": "2026-02-01" }),
            json!({ "merchant": "B", "next_expected_at": "2026-02-01" }),
        ];

        let normalized = normalize_recurring_rows(&rows);
        assert_eq!(normalized[0]["merchant"], Value::String("A".to_string()));
        assert_eq!(normalized[1]["merchant"], Value::String("B".to_string()));
        assert_eq!(normalized[2]["merchant"], Value::String("Z".to_string()));
    }

    #[test]
    fn empty_intelligence_outputs_use_plaintext_no_data_messages() {
        let anomalies_payload = json!({ "rows": [] });
        let recurring_payload = json!({ "rows": [] });

        let anomalies = render_anomalies(&anomalies_payload);
        assert!(anomalies.is_ok());
        if let Ok(text) = anomalies {
            assert!(text.starts_with("No anomalies found."));
        }

        let recurring = render_recurring(&recurring_payload);
        assert!(recurring.is_ok());
        if let Ok(text) = recurring {
            assert!(text.starts_with("No recurring patterns found."));
        }
    }
}
