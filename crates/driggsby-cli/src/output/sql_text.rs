use std::io;

use serde_json::Value;

use super::format;

pub fn render_sql_result(data: &Value) -> io::Result<String> {
    let columns = data
        .get("columns")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("sql output requires columns"))?;
    let rows = data
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("sql output requires rows"))?;

    let row_count = data.get("row_count").and_then(Value::as_i64).unwrap_or(0);
    let truncated = data
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let max_rows = data.get("max_rows").and_then(Value::as_i64).unwrap_or(0);
    let source = data
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let source_ref = data
        .get("source_ref")
        .and_then(Value::as_str)
        .map(std::string::ToString::to_string);

    let mut lines = vec![
        "Query completed successfully.".to_string(),
        String::new(),
        "Summary:".to_string(),
    ];

    let mut source_label = source.to_string();
    if let Some(reference) = source_ref {
        source_label = format!("{source} ({reference})");
    }

    lines.extend(format::key_value_rows(
        &[
            ("Rows returned:", row_count.to_string()),
            (
                "Truncated:",
                if truncated { "yes" } else { "no" }.to_string(),
            ),
            ("Row limit:", max_rows.to_string()),
            ("Source:", source_label),
        ],
        2,
    ));

    lines.push(String::new());
    if rows.is_empty() {
        lines.push("No rows returned.".to_string());
        return Ok(lines.join("\n"));
    }

    lines.push("Results:".to_string());
    lines.extend(render_rows_as_blocks(columns, rows));

    if truncated {
        lines.push(String::new());
        lines.push(format!(
            "Result set was truncated at {max_rows} rows. Narrow your query to inspect more rows."
        ));
    }

    Ok(lines.join("\n"))
}

fn render_rows_as_blocks(columns: &[Value], rows: &[Value]) -> Vec<String> {
    let labels = columns
        .iter()
        .map(|column| {
            let name = column
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            format!("{name}:")
        })
        .collect::<Vec<String>>();
    let label_width = labels.iter().map(|label| label.len()).max().unwrap_or(0);

    let mut lines = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        lines.push(format!("  Row {}:", row_index + 1));

        let values = row.as_array();
        if values.is_none() {
            lines.push("    warning: invalid row payload (expected array).".to_string());
        }

        for (column_index, label) in labels.iter().enumerate() {
            let value = if let Some(items) = values {
                items
                    .get(column_index)
                    .map(render_scalar)
                    .unwrap_or_default()
            } else {
                "<invalid row shape>".to_string()
            };
            let mut value_lines = value.lines();

            if let Some(first_line) = value_lines.next() {
                lines.push(format!("    {label:<label_width$}  {first_line}"));
            } else {
                lines.push(format!("    {label:<label_width$}"));
            }

            for continuation in value_lines {
                lines.push(format!("    {:<label_width$}  {continuation}", ""));
            }
        }

        if row_index + 1 < rows.len() {
            lines.push(String::new());
        }
    }

    lines
}

fn render_scalar(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::render_sql_result;

    #[test]
    fn sql_text_renders_summary_and_row_blocks() {
        let payload = json!({
            "columns": [
                {"name": "account_key", "type": "text", "nullable": false},
                {"name": "txn_count", "type": "integer", "nullable": false}
            ],
            "rows": [["acct_1", 2]],
            "row_count": 1,
            "truncated": false,
            "max_rows": 1000,
            "source": "inline"
        });

        let rendered = render_sql_result(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.starts_with("Query completed successfully."));
            assert!(text.contains("Summary:"));
            assert!(text.contains("Rows returned:"));
            assert!(text.contains("Results:"));
            assert!(text.contains("acct_1"));
            assert!(text.contains("Row 1:"));
            assert!(text.contains("account_key:"));
        }
    }

    #[test]
    fn sql_text_handles_zero_rows() {
        let payload = json!({
            "columns": [{"name": "one", "type": "integer", "nullable": false}],
            "rows": [],
            "row_count": 0,
            "truncated": false,
            "max_rows": 1000,
            "source": "inline"
        });

        let rendered = render_sql_result(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("No rows returned."));
        }
    }

    #[test]
    fn sql_text_renders_warning_for_non_array_rows() {
        let payload = json!({
            "columns": [
                {"name": "account_key", "type": "text", "nullable": false},
                {"name": "txn_count", "type": "integer", "nullable": false}
            ],
            "rows": [{"account_key": "acct_1", "txn_count": 2}],
            "row_count": 1,
            "truncated": false,
            "max_rows": 1000,
            "source": "inline"
        });

        let rendered = render_sql_result(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("warning: invalid row payload (expected array)."));
            assert!(text.contains("account_key:  <invalid row shape>"));
            assert!(text.contains("txn_count:    <invalid row shape>"));
        }
    }

    #[test]
    fn sql_text_renders_multiline_values_with_continuation_indent() {
        let payload = json!({
            "columns": [{"name": "description", "type": "text", "nullable": false}],
            "rows": [["line one\nline two"]],
            "row_count": 1,
            "truncated": false,
            "max_rows": 1000,
            "source": "inline"
        });

        let rendered = render_sql_result(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("description:  line one"));
            assert!(text.contains("               line two"));
        }
    }
}
