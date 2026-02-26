use serde_json::{Map, Value};

use super::format::{self, Align, Column};

pub(super) enum AccountTableMode {
    Compact,
    WithDateRange,
}

pub(super) fn render_accounts_summary(summary: &Map<String, Value>, indent: usize) -> Vec<String> {
    format::key_value_rows(
        &[
            (
                "Account count:",
                summary
                    .get("account_count")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .to_string(),
            ),
            (
                "Typed accounts:",
                summary
                    .get("typed_account_count")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .to_string(),
            ),
            (
                "Untyped accounts:",
                summary
                    .get("untyped_account_count")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .to_string(),
            ),
            (
                "Transaction count:",
                summary
                    .get("transaction_count")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .to_string(),
            ),
            (
                "Earliest posted_at:",
                summary
                    .get("earliest_posted_at")
                    .and_then(Value::as_str)
                    .unwrap_or("none")
                    .to_string(),
            ),
            (
                "Latest posted_at:",
                summary
                    .get("latest_posted_at")
                    .and_then(Value::as_str)
                    .unwrap_or("none")
                    .to_string(),
            ),
            (
                "Net amount:",
                format!(
                    "{:.2}",
                    summary
                        .get("net_amount")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0)
                ),
            ),
        ],
        indent,
    )
}

pub(super) fn render_accounts_table(rows: &[Value], mode: AccountTableMode) -> Vec<String> {
    match mode {
        AccountTableMode::Compact => {
            let columns = [
                Column {
                    name: "Account Key",
                    align: Align::Left,
                },
                Column {
                    name: "Type",
                    align: Align::Left,
                },
                Column {
                    name: "Currency",
                    align: Align::Left,
                },
                Column {
                    name: "Txn Count",
                    align: Align::Right,
                },
                Column {
                    name: "Net",
                    align: Align::Right,
                },
            ];
            let table_rows = rows
                .iter()
                .map(|row| {
                    vec![
                        row.get("account_key")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string(),
                        row.get("account_type")
                            .and_then(Value::as_str)
                            .unwrap_or("untyped")
                            .to_string(),
                        row.get("currency")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string(),
                        row.get("txn_count")
                            .and_then(Value::as_i64)
                            .unwrap_or(0)
                            .to_string(),
                        format!(
                            "{:.2}",
                            row.get("net_amount").and_then(Value::as_f64).unwrap_or(0.0)
                        ),
                    ]
                })
                .collect::<Vec<Vec<String>>>();
            format::render_table_or_blocks(
                &columns,
                &table_rows,
                format::terminal_width(),
                "Account",
            )
        }
        AccountTableMode::WithDateRange => {
            let columns = [
                Column {
                    name: "Account Key",
                    align: Align::Left,
                },
                Column {
                    name: "Type",
                    align: Align::Left,
                },
                Column {
                    name: "Currency",
                    align: Align::Left,
                },
                Column {
                    name: "Txn Count",
                    align: Align::Right,
                },
                Column {
                    name: "First",
                    align: Align::Left,
                },
                Column {
                    name: "Last",
                    align: Align::Left,
                },
                Column {
                    name: "Net",
                    align: Align::Right,
                },
            ];
            let table_rows = rows
                .iter()
                .map(|row| {
                    vec![
                        row.get("account_key")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string(),
                        row.get("account_type")
                            .and_then(Value::as_str)
                            .unwrap_or("untyped")
                            .to_string(),
                        row.get("currency")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string(),
                        row.get("txn_count")
                            .and_then(Value::as_i64)
                            .unwrap_or(0)
                            .to_string(),
                        row.get("first_posted_at")
                            .and_then(Value::as_str)
                            .unwrap_or("none")
                            .to_string(),
                        row.get("last_posted_at")
                            .and_then(Value::as_str)
                            .unwrap_or("none")
                            .to_string(),
                        format!(
                            "{:.2}",
                            row.get("net_amount").and_then(Value::as_f64).unwrap_or(0.0)
                        ),
                    ]
                })
                .collect::<Vec<Vec<String>>>();
            format::render_table_or_blocks(
                &columns,
                &table_rows,
                format::terminal_width(),
                "Account",
            )
        }
    }
}
