use std::io;

use serde_json::Value;

use super::accounts_shared::{AccountTableMode, render_accounts_summary, render_accounts_table};

pub fn render_accounts(data: &Value) -> io::Result<String> {
    let summary = data
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| io::Error::other("accounts output requires summary"))?;
    let rows = data
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("accounts output requires rows"))?;

    let mut lines = vec![
        "Ledger account summary:".to_string(),
        String::new(),
        "Summary:".to_string(),
    ];
    lines.extend(render_accounts_summary(summary, 2));

    if rows.is_empty() {
        lines.push(String::new());
        lines.push("No accounts found yet.".to_string());
        lines.push(String::new());
        lines.push("Import a statement first:".to_string());
        lines.push("  1. driggsby import create --help".to_string());
        lines.push("  2. driggsby import create --dry-run <path>".to_string());
        lines.push("  3. driggsby import create <path>".to_string());
        return Ok(lines.join("\n"));
    }

    lines.push(String::new());
    lines.push("Accounts:".to_string());
    lines.extend(render_accounts_table(rows, AccountTableMode::WithDateRange));

    Ok(lines.join("\n"))
}
