use std::cmp;
use std::io;

use serde_json::Value;

use super::format;

pub fn render_schema_summary(data: &Value) -> io::Result<String> {
    let db_path = get_string(data, "db_path").unwrap_or("unknown");
    let readonly_uri = get_string(data, "readonly_uri").unwrap_or("unknown");
    let public_views = data
        .get("public_views")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("schema summary requires public_views"))?;

    let mut lines = vec![
        "Your ledger database is stored locally and can be queried with sqlite3 or any SQL client."
            .to_string(),
        "These views and columns are the Driggsby semantic contract for agent and human queries."
            .to_string(),
        "SQLite physical metadata for views (for example `PRAGMA table_info(...)`) may differ."
            .to_string(),
        String::new(),
        "Summary:".to_string(),
    ];

    lines.extend(format::key_value_rows(
        &[
            ("Database path:", db_path.to_string()),
            ("Readonly URI:", readonly_uri.to_string()),
        ],
        2,
    ));

    lines.push(String::new());
    lines.push("Connect with sqlite3:".to_string());
    lines.push(format!("  sqlite3 \"{readonly_uri}\""));

    lines.push(String::new());
    lines.push("Example queries:".to_string());
    lines.push("  SELECT * FROM v1_transactions LIMIT 5;".to_string());
    lines.push("  SELECT * FROM v1_accounts LIMIT 5;".to_string());
    lines.push("  SELECT * FROM v1_imports ORDER BY created_at DESC LIMIT 5;".to_string());

    lines.push(String::new());
    lines.push("Public Views:".to_string());

    for view in public_views {
        let view_name = view
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let columns = view
            .get("columns")
            .and_then(Value::as_array)
            .ok_or_else(|| io::Error::other("schema view requires columns"))?;

        lines.push(String::new());
        lines.push(format!("View: {view_name}"));
        lines.extend(render_schema_columns(columns));
    }

    lines.push(String::new());
    lines.push("Inspect one view in detail:".to_string());
    lines.push("  driggsby schema view <name>".to_string());

    Ok(lines.join("\n"))
}

pub fn render_schema_view(data: &Value) -> io::Result<String> {
    let view_name = get_string(data, "view_name").unwrap_or("unknown");
    let columns = data
        .get("columns")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("schema view output requires columns"))?;

    let mut lines = vec![
        format!("View details for {view_name}."),
        "Columns shown here are the Driggsby semantic contract for this view.".to_string(),
        "SQLite physical metadata for views may differ (for example via PRAGMA).".to_string(),
        String::new(),
        "Columns:".to_string(),
    ];
    lines.extend(render_schema_columns(columns));

    Ok(lines.join("\n"))
}

fn render_schema_columns(columns: &[Value]) -> Vec<String> {
    let mut name_width = "column".len();
    let mut type_width = "type".len();

    for column in columns {
        let name = column
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let column_type = column
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        name_width = cmp::max(name_width, name.len());
        type_width = cmp::max(type_width, column_type.len());
    }

    columns
        .iter()
        .map(|column| {
            let name = column
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let column_type = column
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let nullable = if column
                .get("nullable")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                "nullable"
            } else {
                "not null"
            };

            format!("  {name:<name_width$}  {column_type:<type_width$}  {nullable}")
        })
        .collect()
}

fn get_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{render_schema_summary, render_schema_view};

    #[test]
    fn schema_summary_is_plaintext_and_lists_views() {
        let payload = json!({
            "db_path": "/tmp/ledger.db",
            "readonly_uri": "file:/tmp/ledger.db?mode=ro",
            "public_views": [
                {
                    "name": "v1_transactions",
                    "columns": [
                        { "name": "txn_id", "type": "text", "nullable": false }
                    ]
                }
            ]
        });

        let rendered = render_schema_summary(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.starts_with("Your ledger database is stored locally"));
            assert!(text.contains("Public Views:"));
            assert!(text.contains("semantic contract"));
            assert!(text.contains("View: v1_transactions"));
            assert!(text.contains("txn_id"));
        }
    }

    #[test]
    fn schema_view_is_plaintext() {
        let payload = json!({
            "view_name": "v1_transactions",
            "columns": [
                { "name": "txn_id", "type": "text", "nullable": false }
            ]
        });

        let rendered = render_schema_view(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.starts_with("View details for v1_transactions."));
            assert!(text.contains("Columns:"));
            assert!(text.contains("semantic contract"));
            assert!(text.contains("not null"));
        }
    }
}
