use std::cmp;
use std::io;

use chrono::{Local, TimeZone};
use serde_json::Value;

use super::accounts_shared::{AccountTableMode, render_accounts_summary, render_accounts_table};
use super::format::{self, Align, Column};

pub fn render_import_run(data: &Value) -> io::Result<String> {
    let dry_run = data
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let summary = data
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| io::Error::other("import output requires summary"))?;

    let mut lines = Vec::new();
    if dry_run {
        lines.push("Dry-run validation completed successfully.".to_string());
    } else {
        lines.push("Import completed successfully.".to_string());
    }

    lines.push(String::new());
    lines.push("Summary:".to_string());

    let mut entries = Vec::new();
    if !dry_run {
        let import_id = data
            .get("import_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        entries.push(("Import ID:", import_id.to_string()));
    }

    entries.push(("Rows read:", get_i64(summary, "rows_read").to_string()));
    entries.push(("Rows valid:", get_i64(summary, "rows_valid").to_string()));
    entries.push((
        "Rows invalid:",
        get_i64(summary, "rows_invalid").to_string(),
    ));
    entries.push(("Inserted:", get_i64(summary, "inserted").to_string()));

    lines.extend(format::key_value_rows(&entries, 2));
    let source_warnings = render_source_warnings(data);
    lines.push(String::new());
    if !source_warnings.is_empty() {
        lines.extend(source_warnings);
        lines.push(String::new());
    }
    lines.extend(render_duplicate_summary_and_preview(data));
    if !dry_run {
        let ledger_now = render_ledger_accounts_section(data);
        if !ledger_now.is_empty() {
            lines.push(String::new());
            lines.extend(ledger_now);
        }
    }

    if dry_run {
        lines.push(String::new());
        lines.push("Canonical existing values:".to_string());
        lines.extend(render_inventory_sections(data));

        lines.push(String::new());
        lines.push("Per-account sign profile:".to_string());
        lines.extend(render_sign_profile_section(data));

        lines.push(String::new());
        lines.push("Drift warnings:".to_string());
        lines.extend(render_drift_warnings_section(data));

        lines.push(String::new());
        lines.push("No rows were written because this was a dry run.".to_string());
    }

    lines.push(String::new());
    lines.extend(render_next_actions(data));

    Ok(lines.join("\n"))
}

pub fn render_import_list(data: &Value) -> io::Result<String> {
    let rows = data
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("import list output requires rows"))?;

    if rows.is_empty() {
        return Ok([
            "No imports found yet.",
            "",
            "Run your first import:",
            "  1. driggsby import create --help",
            "  2. driggsby import create --dry-run <path>",
            "  3. driggsby import create <path>",
        ]
        .join("\n"));
    }

    let mut ordered_rows = rows.to_vec();
    ordered_rows.sort_by(compare_import_rows);

    let count_label = if ordered_rows.len() == 1 {
        "1 import found.".to_string()
    } else {
        format!("{} imports found.", ordered_rows.len())
    };

    let columns = [
        Column {
            name: "Import ID",
            align: Align::Left,
        },
        Column {
            name: "Status",
            align: Align::Left,
        },
        Column {
            name: "Created (local)",
            align: Align::Left,
        },
        Column {
            name: "Rows Read",
            align: Align::Right,
        },
        Column {
            name: "Inserted",
            align: Align::Right,
        },
        Column {
            name: "Deduped",
            align: Align::Right,
        },
    ];

    let table_rows = ordered_rows
        .iter()
        .map(|row| {
            vec![
                row.get("import_id")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                row.get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                format_created_local(row),
                row.get("rows_read")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .to_string(),
                row.get("inserted")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .to_string(),
                row.get("deduped")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .to_string(),
            ]
        })
        .collect::<Vec<Vec<String>>>();

    let mut lines = vec![count_label, String::new(), "Imports:".to_string()];
    lines.extend(format::render_table_or_blocks(
        &columns,
        &table_rows,
        format::terminal_width(),
        "Import",
    ));

    let account_coverage = render_import_list_account_coverage(&ordered_rows);
    if !account_coverage.is_empty() {
        lines.push(String::new());
        lines.extend(account_coverage);
    }

    Ok(lines.join("\n"))
}

pub fn render_import_undo(data: &Value) -> io::Result<String> {
    let import_id = data
        .get("import_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let summary = data
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| io::Error::other("import undo output requires summary"))?;

    let mut lines = vec![
        "Import reverted successfully.".to_string(),
        String::new(),
        "Summary:".to_string(),
    ];

    lines.extend(format::key_value_rows(
        &[
            ("Import ID:", import_id.to_string()),
            (
                "Rows reverted:",
                get_i64(summary, "rows_reverted").to_string(),
            ),
            (
                "Rows promoted:",
                get_i64(summary, "rows_promoted").to_string(),
            ),
            (
                "Intelligence refreshed:",
                if data
                    .get("intelligence_refreshed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "yes".to_string()
                } else {
                    "no".to_string()
                },
            ),
        ],
        2,
    ));

    Ok(lines.join("\n"))
}

pub fn render_intelligence_refresh(data: &Value) -> io::Result<String> {
    let completed_at = data
        .get("completed_at")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let mut lines = vec![
        "Intelligence refresh completed.".to_string(),
        String::new(),
        "Summary:".to_string(),
    ];

    lines.extend(format::key_value_rows(
        &[
            (
                "Recurring rows:",
                data.get("recurring_rows")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .to_string(),
            ),
            (
                "Anomaly rows:",
                data.get("anomaly_rows")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .to_string(),
            ),
            ("Completed at:", completed_at.to_string()),
        ],
        2,
    ));

    Ok(lines.join("\n"))
}

pub fn render_import_duplicates(data: &Value) -> io::Result<String> {
    let import_id = data
        .get("import_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let total = data.get("total").and_then(Value::as_i64).unwrap_or(0);
    let rows = data
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("import duplicates output requires rows"))?;

    let mut lines = vec![
        format!("Duplicate rows for import {import_id}"),
        format!("Total duplicates: {total}"),
    ];

    if rows.is_empty() {
        lines.push(String::new());
        lines.push("No duplicate rows were recorded for this import.".to_string());
        return Ok(lines.join("\n"));
    }

    lines.push(String::new());
    for (index, row) in rows.iter().enumerate() {
        lines.extend(render_duplicate_row(row, index + 1));
        if index + 1 < rows.len() {
            lines.push(String::new());
        }
    }

    Ok(lines.join("\n"))
}

pub fn render_import_keys_uniq(data: &Value) -> io::Result<String> {
    let inventories = data
        .get("inventories")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("import keys uniq output requires inventories"))?;

    if let Some(property) = data.get("property").and_then(Value::as_str) {
        let inventory = inventories
            .iter()
            .find_map(|item| item.as_object())
            .ok_or_else(|| io::Error::other("import keys uniq output requires one inventory"))?;
        let mut lines = vec![
            format!("Canonical unique values for {property}."),
            String::new(),
            "Summary:".to_string(),
        ];
        lines.extend(format::key_value_rows(
            &[
                ("Property:", property.to_string()),
                (
                    "Unique count:",
                    get_i64(inventory, "unique_count").to_string(),
                ),
                ("Null count:", get_i64(inventory, "null_count").to_string()),
                ("Total rows:", get_i64(inventory, "total_rows").to_string()),
            ],
            2,
        ));
        lines.push(String::new());
        lines.extend(render_counted_values_list(inventory));
        return Ok(lines.join("\n"));
    }

    let mut lines = vec![
        "Canonical unique values from committed ledger transactions.".to_string(),
        String::new(),
    ];

    if inventories.is_empty() {
        lines.push("No committed transaction rows found yet.".to_string());
        return Ok(lines.join("\n"));
    }

    for (index, inventory) in inventories.iter().enumerate() {
        let Some(inventory_map) = inventory.as_object() else {
            continue;
        };
        let property = inventory_map
            .get("property")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        lines.push(format!("{property}:"));
        lines.extend(format::key_value_rows(
            &[
                (
                    "Unique count:",
                    get_i64(inventory_map, "unique_count").to_string(),
                ),
                (
                    "Null count:",
                    get_i64(inventory_map, "null_count").to_string(),
                ),
                (
                    "Total rows:",
                    get_i64(inventory_map, "total_rows").to_string(),
                ),
            ],
            2,
        ));
        lines.push(String::new());
        lines.extend(render_counted_values_list(inventory_map));

        if index + 1 < inventories.len() {
            lines.push(String::new());
        }
    }

    lines.push(String::new());
    lines.push("Need fewer rows? Run `driggsby import keys uniq <property>`.".to_string());

    Ok(lines.join("\n"))
}

fn render_duplicate_summary_and_preview(data: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    let duplicate_summary = data
        .get("duplicate_summary")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    lines.push("Duplicate Summary:".to_string());
    lines.extend(format::key_value_rows(
        &[
            ("Total:", get_i64(&duplicate_summary, "total").to_string()),
            ("Batch:", get_i64(&duplicate_summary, "batch").to_string()),
            (
                "Existing ledger:",
                get_i64(&duplicate_summary, "existing_ledger").to_string(),
            ),
        ],
        2,
    ));

    lines.push(String::new());

    let preview = data
        .get("duplicates_preview")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let returned = get_i64(&preview, "returned");
    let total = get_i64(&duplicate_summary, "total");
    let truncated = preview
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let status = if truncated {
        format!(
            "Duplicates Preview (showing first {returned} of {total} duplicate rows; truncated):"
        )
    } else {
        format!("Duplicates Preview (showing all {total} duplicate rows):")
    };
    lines.push(status);

    let rows = preview
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        lines.push("  None.".to_string());
    } else {
        for (index, row) in rows.iter().enumerate() {
            lines.extend(render_duplicate_row(row, index + 1));
            if index + 1 < rows.len() {
                lines.push(String::new());
            }
        }
    }

    lines
}

fn render_next_actions(data: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push("Next step:".to_string());
    if let Some(next_step) = data.get("next_step").and_then(Value::as_object) {
        let label = next_step
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("Run the next command");
        let command = next_step
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("missing_next_step_command");
        lines.push(format!("  {label}:"));
        lines.push(format!("  {command}"));
    } else {
        lines.push("  Missing `next_step` in import response.".to_string());
    }

    lines.push(String::new());
    lines.push("Other actions:".to_string());
    let Some(actions) = data.get("other_actions").and_then(Value::as_array) else {
        lines.push("  Missing `other_actions` in import response.".to_string());
        return lines;
    };

    if actions.is_empty() {
        lines.push("  None.".to_string());
        return lines;
    }

    for action in actions {
        let label = action
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("Action");
        let command = action
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("missing_other_action_command");
        let risk = action
            .get("risk")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if risk == "destructive" && !label.contains("(destructive)") {
            lines.push(format!("  - {label} (destructive): {command}"));
        } else {
            lines.push(format!("  - {label}: {command}"));
        }
    }

    lines
}

fn render_duplicate_row(row: &Value, ordinal: usize) -> Vec<String> {
    let source_row_index = row
        .get("source_row_index")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let reason_code = row
        .get("dedupe_reason")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let reason_label = match reason_code {
        "batch" => "Duplicate within this import",
        "existing_ledger" => "Already exists in ledger",
        _ => "Duplicate",
    };

    let statement_id = row
        .get("statement_id")
        .and_then(Value::as_str)
        .unwrap_or("(none)");
    let account_key = row
        .get("account_key")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let posted_at = row
        .get("posted_at")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let amount = row.get("amount").and_then(Value::as_f64).unwrap_or(0.0);
    let currency = row
        .get("currency")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let description = row
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let mut lines = vec![
        format!("{ordinal}) Row #{source_row_index} - {reason_label}"),
        format!("   Statement: {statement_id}"),
        format!(
            "   Transaction: {account_key} | {posted_at} | {amount:.2} {currency} | {description}"
        ),
    ];

    if reason_code == "batch" {
        let matched = row
            .get("matched_batch_row_index")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        lines.push(format!("   Match: Row #{matched} in this import"));
    } else if reason_code == "existing_ledger" {
        let matched_txn = row
            .get("matched_txn_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let matched_import = row
            .get("matched_import_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        lines.push(format!(
            "   Match: Ledger transaction {matched_txn} (from import {matched_import})"
        ));

        let historical_txn = row
            .get("matched_txn_id_at_dedupe")
            .and_then(Value::as_str)
            .unwrap_or("");
        let historical_import = row
            .get("matched_import_id_at_dedupe")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !historical_txn.is_empty()
            && !historical_import.is_empty()
            && (historical_txn != matched_txn || historical_import != matched_import)
        {
            lines.push(format!(
                "   Originally matched: {historical_txn} (from import {historical_import})"
            ));
        }
    }

    lines
}

fn render_source_warnings(data: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    let warnings = data
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let source_conflict = data
        .get("source_conflict")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if warnings.is_empty() && !source_conflict {
        return lines;
    }

    lines.push("Warnings:".to_string());

    if source_conflict {
        let source_used = data
            .get("source_used")
            .and_then(Value::as_str)
            .unwrap_or("file");
        let source_ignored = data
            .get("source_ignored")
            .and_then(Value::as_str)
            .unwrap_or("stdin");
        lines.push(format!(
            "  Both file and stdin were provided. Using {source_used}; {source_ignored} was ignored."
        ));
    }

    for warning in warnings {
        let code = warning.get("code").and_then(Value::as_str).unwrap_or("");
        if source_conflict && code == "stdin_ignored_file_provided" {
            continue;
        }
        if let Some(message) = warning.get("message").and_then(Value::as_str) {
            lines.push(format!("  {message}"));
        }
    }

    lines
}

fn get_i64(map: &serde_json::Map<String, Value>, key: &str) -> i64 {
    map.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn compare_import_rows(left: &Value, right: &Value) -> cmp::Ordering {
    let left_created_at = parse_created_at(left).unwrap_or(0);
    let right_created_at = parse_created_at(right).unwrap_or(0);

    right_created_at.cmp(&left_created_at).then_with(|| {
        right
            .get("import_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(left.get("import_id").and_then(Value::as_str).unwrap_or(""))
    })
}

fn parse_created_at(row: &Value) -> Option<i64> {
    if let Some(raw) = row.get("created_at") {
        if let Some(value) = raw.as_i64() {
            return Some(value);
        }
        if let Some(text) = raw.as_str() {
            return text.parse::<i64>().ok();
        }
    }
    None
}

fn format_created_local(row: &Value) -> String {
    let Some(created_at) = parse_created_at(row) else {
        return "unknown".to_string();
    };
    let Some(local_dt) = Local.timestamp_opt(created_at, 0).single() else {
        return "unknown".to_string();
    };
    local_dt.format("%Y-%m-%d %H:%M:%S %:z").to_string()
}

fn render_inventory_sections(data: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    let Some(inventory) = data.get("key_inventory").and_then(Value::as_object) else {
        lines.push("  No existing transaction history found.".to_string());
        return lines;
    };

    let properties = [
        "account_key",
        "account_type",
        "currency",
        "merchant",
        "category",
    ];
    for (index, property) in properties.iter().enumerate() {
        let Some(property_map) = inventory.get(*property).and_then(Value::as_object) else {
            continue;
        };

        lines.push(format!("  {property}:"));
        lines.extend(format::key_value_rows(
            &[
                (
                    "Unique count:",
                    get_i64(property_map, "unique_count").to_string(),
                ),
                (
                    "Null count:",
                    get_i64(property_map, "null_count").to_string(),
                ),
                (
                    "Total rows:",
                    get_i64(property_map, "total_rows").to_string(),
                ),
            ],
            4,
        ));
        lines.push("    Values:".to_string());
        lines.extend(render_values_list_with_indent(
            property_map.get("existing_values"),
            6,
        ));

        if index + 1 < properties.len() {
            lines.push(String::new());
        }
    }

    if lines.is_empty() {
        lines.push("  No existing transaction history found.".to_string());
    }

    lines
}

fn render_ledger_accounts_section(data: &Value) -> Vec<String> {
    let Some(ledger_accounts) = data.get("ledger_accounts").and_then(Value::as_object) else {
        return Vec::new();
    };
    let Some(summary) = ledger_accounts.get("summary").and_then(Value::as_object) else {
        return Vec::new();
    };
    let rows = ledger_accounts
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut lines = vec!["Your ledger now:".to_string(), "Summary:".to_string()];
    lines.extend(render_accounts_summary(summary, 2));

    if rows.is_empty() {
        return lines;
    }

    lines.push(String::new());
    lines.push("Accounts:".to_string());
    lines.extend(render_accounts_table(&rows, AccountTableMode::Compact));

    lines
}

fn render_import_list_account_coverage(rows: &[Value]) -> Vec<String> {
    let mut lines = vec!["Account coverage:".to_string()];
    let mut rendered_any = false;

    for row in rows {
        let import_id = row
            .get("import_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let accounts = row
            .get("accounts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if accounts.is_empty() {
            continue;
        }

        rendered_any = true;
        lines.push(format!("  {import_id}:"));
        for account in accounts {
            let account_key = account
                .get("account_key")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let account_type = account
                .get("account_type")
                .and_then(Value::as_str)
                .unwrap_or("untyped");
            let rows_read = account
                .get("rows_read")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let inserted = account.get("inserted").and_then(Value::as_i64).unwrap_or(0);
            let deduped = account.get("deduped").and_then(Value::as_i64).unwrap_or(0);
            lines.push(format!(
                "    {account_key} ({account_type}) rows_read={rows_read} inserted={inserted} deduped={deduped}"
            ));
        }
    }

    if !rendered_any {
        lines.push("  No account coverage recorded yet.".to_string());
    }

    lines
}

fn render_sign_profile_section(data: &Value) -> Vec<String> {
    let Some(profiles) = data.get("sign_profiles").and_then(Value::as_array) else {
        return vec!["  No sign profile history found.".to_string()];
    };

    if profiles.is_empty() {
        return vec!["  No sign profile history found.".to_string()];
    }

    let columns = [
        Column {
            name: "Account Key",
            align: Align::Left,
        },
        Column {
            name: "Negative",
            align: Align::Right,
        },
        Column {
            name: "Positive",
            align: Align::Right,
        },
        Column {
            name: "Neg %",
            align: Align::Right,
        },
        Column {
            name: "Pos %",
            align: Align::Right,
        },
    ];

    let rows = profiles
        .iter()
        .map(|profile| {
            let account_key = profile
                .get("account_key")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let negative = profile
                .get("negative_count")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .to_string();
            let positive = profile
                .get("positive_count")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .to_string();
            let negative_ratio = profile
                .get("negative_ratio")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let positive_ratio = profile
                .get("positive_ratio")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            vec![
                account_key,
                negative,
                positive,
                format!("{:.2}", negative_ratio * 100.0),
                format!("{:.2}", positive_ratio * 100.0),
            ]
        })
        .collect::<Vec<Vec<String>>>();

    format::render_table_or_blocks(&columns, &rows, format::terminal_width(), "Sign profile")
}

fn render_drift_warnings_section(data: &Value) -> Vec<String> {
    let Some(warnings) = data.get("drift_warnings").and_then(Value::as_array) else {
        return vec!["  None.".to_string()];
    };

    if warnings.is_empty() {
        return vec!["  None.".to_string()];
    }

    let mut lines = Vec::new();
    for warning in warnings {
        let severity = warning
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_uppercase();
        let property = warning
            .get("property")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let incoming_value = warning
            .get("incoming_value")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let message = warning
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("No warning message provided.");
        lines.push(format!("  [{severity}] {property} = {incoming_value}"));
        lines.push(format!("    {message}"));
        if let Some(suggestions) = warning.get("suggestions").and_then(Value::as_array)
            && !suggestions.is_empty()
        {
            let values = suggestions
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<&str>>()
                .join(", ");
            lines.push(format!("    Suggestions: {values}"));
        }
    }

    lines
}

fn render_counted_values_list(inventory_map: &serde_json::Map<String, Value>) -> Vec<String> {
    if let Some(value_counts) = inventory_map.get("value_counts").and_then(Value::as_array) {
        if value_counts.is_empty() {
            return vec!["  No values.".to_string()];
        }

        let rows = value_counts
            .iter()
            .map(|entry| {
                let value = entry
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let count = entry.get("count").and_then(Value::as_i64).unwrap_or(0);
                (value.to_string(), count.to_string())
            })
            .collect::<Vec<(String, String)>>();

        return render_value_count_table(&rows);
    }

    let fallback = render_values_list_with_indent(inventory_map.get("existing_values"), 2);
    if fallback.len() == 1 && fallback[0].trim() == "None" {
        return vec!["  No values.".to_string()];
    }
    fallback
}

fn render_value_count_table(rows: &[(String, String)]) -> Vec<String> {
    if rows.is_empty() {
        return vec!["  No values.".to_string()];
    }

    let count_width = rows
        .iter()
        .map(|(_, count)| count.len())
        .max()
        .unwrap_or(5)
        .max("Count".len());

    let terminal_width = format::terminal_width();
    let static_width = 2 + 2 + count_width;
    let mut value_width = terminal_width
        .saturating_sub(static_width)
        .max("Value".len())
        .max(8);

    let natural_value_width = rows
        .iter()
        .map(|(value, _)| value.len())
        .max()
        .unwrap_or(8)
        .max("Value".len());
    value_width = value_width.min(natural_value_width);

    let mut lines = Vec::new();
    lines.push(format!(
        "  {:<value_width$}  {:>count_width$}",
        "Value", "Count"
    ));
    for (value, count) in rows {
        lines.push(format!(
            "  {:<value_width$}  {:>count_width$}",
            truncate_ascii(value, value_width),
            count
        ));
    }
    lines
}

fn truncate_ascii(value: &str, width: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= width {
        return value.to_string();
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    let keep = width - 3;
    let prefix = value.chars().take(keep).collect::<String>();
    format!("{prefix}...")
}

fn render_values_list_with_indent(values: Option<&Value>, indent: usize) -> Vec<String> {
    let padding = " ".repeat(indent);
    let Some(existing_values) = values.and_then(Value::as_array) else {
        return vec![format!("{padding}None")];
    };

    if existing_values.is_empty() {
        return vec![format!("{padding}None")];
    }

    existing_values
        .iter()
        .filter_map(Value::as_str)
        .map(|value| format!("{padding}{value}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        render_import_keys_uniq, render_import_list, render_import_run, render_import_undo,
        render_intelligence_refresh,
    };

    #[test]
    fn import_run_renders_plaintext_summary() {
        let payload = json!({
            "dry_run": false,
            "import_id": "imp_1",
            "next_step": {
                "label": "Connect and query your data",
                "command": "driggsby db schema"
            },
            "other_actions": [
                {
                    "label": "View import list",
                    "command": "driggsby import list"
                },
                {
                    "label": "Undo this import (destructive)",
                    "command": "driggsby import undo imp_1",
                    "risk": "destructive"
                }
            ],
            "summary": {
                "rows_read": 10,
                "rows_valid": 10,
                "rows_invalid": 0,
                "inserted": 9,
                "deduped": 1
            }
        });

        let rendered = render_import_run(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.starts_with("Import completed successfully."));
            assert!(text.contains("Import ID:"));
            assert!(!text.contains("Undo ID:"));
            assert!(text.contains("Next step:"));
            assert!(text.contains("Other actions:"));
            assert!(text.contains("(destructive)"));
        }
    }

    #[test]
    fn committed_import_run_renders_ledger_accounts_section() {
        let payload = json!({
            "dry_run": false,
            "import_id": "imp_1",
            "next_step": {
                "label": "Connect and query your data",
                "command": "driggsby db schema"
            },
            "other_actions": [],
            "summary": {
                "rows_read": 2,
                "rows_valid": 2,
                "rows_invalid": 0,
                "inserted": 2
            },
            "duplicate_summary": {
                "total": 0,
                "batch": 0,
                "existing_ledger": 0
            },
            "duplicates_preview": {
                "returned": 0,
                "truncated": false,
                "rows": []
            },
            "ledger_accounts": {
                "summary": {
                    "account_count": 1,
                    "transaction_count": 2,
                    "earliest_posted_at": "2026-01-01",
                    "latest_posted_at": "2026-01-05",
                    "typed_account_count": 1,
                    "untyped_account_count": 0,
                    "net_amount": -20.0
                },
                "rows": [
                    {
                        "account_key": "acct_1",
                        "account_type": "checking",
                        "currency": "USD",
                        "txn_count": 2,
                        "first_posted_at": "2026-01-01",
                        "last_posted_at": "2026-01-05",
                        "net_amount": -20.0
                    }
                ]
            }
        });

        let rendered = render_import_run(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("Your ledger now:"));
            assert!(text.contains("Account count:"));
            assert!(text.contains("acct_1"));
            assert!(text.contains("checking"));
        }
    }

    #[test]
    fn dry_run_import_run_does_not_render_ledger_accounts_section() {
        let payload = json!({
            "dry_run": true,
            "summary": {
                "rows_read": 1,
                "rows_valid": 1,
                "rows_invalid": 0,
                "inserted": 0
            },
            "duplicate_summary": {
                "total": 0,
                "batch": 0,
                "existing_ledger": 0
            },
            "duplicates_preview": {
                "returned": 0,
                "truncated": false,
                "rows": []
            },
            "next_step": {
                "label": "Commit this import",
                "command": "driggsby import create <path>"
            },
            "other_actions": [],
            "ledger_accounts": {
                "summary": {"account_count": 99},
                "rows": []
            }
        });

        let rendered = render_import_run(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(!text.contains("Your ledger now:"));
        }
    }

    #[test]
    fn import_list_empty_guides_user() {
        let payload = json!({ "rows": [] });
        let rendered = render_import_list(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.starts_with("No imports found yet."));
            assert!(text.contains("driggsby import create --dry-run <path>"));
        }
    }

    #[test]
    fn import_undo_renders_summary() {
        let payload = json!({
            "import_id": "imp_1",
            "summary": {
                "rows_reverted": 4,
                "rows_promoted": 1
            }
        });

        let rendered = render_import_undo(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.starts_with("Import reverted successfully."));
            assert!(text.contains("Rows reverted:"));
            assert!(text.contains("Rows promoted:"));
        }
    }

    #[test]
    fn intelligence_refresh_renders_summary() {
        let payload = json!({
            "recurring_rows": 3,
            "anomaly_rows": 1,
            "completed_at": "2026-02-27T18:22:33Z"
        });

        let rendered = render_intelligence_refresh(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.starts_with("Intelligence refresh completed."));
            assert!(text.contains("Recurring rows:"));
            assert!(text.contains("Anomaly rows:"));
            assert!(text.contains("Completed at:"));
        }
    }

    #[test]
    fn dry_run_renders_inventory_sign_profile_and_drift_warning_sections() {
        let payload = json!({
            "dry_run": true,
            "summary": {
                "rows_read": 5,
                "rows_valid": 5,
                "rows_invalid": 0,
                "inserted": 0,
                "deduped": 0
            },
            "key_inventory": {
                "account_key": {
                    "property": "account_key",
                    "existing_values": ["chase_checking_1234"],
                    "unique_count": 1,
                    "null_count": 0,
                    "total_rows": 10
                }
            },
            "sign_profiles": [
                {
                    "account_key": "chase_checking_1234",
                    "negative_count": 24,
                    "positive_count": 1,
                    "negative_ratio": 0.96,
                    "positive_ratio": 0.04,
                    "total_count": 25
                }
            ],
            "drift_warnings": [
                {
                    "code": "account_key_unseen",
                    "severity": "high",
                    "property": "account_key",
                    "incoming_value": "chase_checkng_1234",
                    "message": "Incoming account_key was not found in existing ledger history.",
                    "suggestions": ["chase_checking_1234"]
                }
            ]
        });

        let rendered = render_import_run(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("Canonical existing values:"));
            assert!(text.contains("Per-account sign profile:"));
            assert!(text.contains("Drift warnings:"));
            assert!(text.contains("account_key"));
            assert!(text.contains("chase_checking_1234"));
            assert!(text.contains("HIGH"));
        }
    }

    #[test]
    fn import_keys_uniq_renders_value_counts_with_blank_line_before_table() {
        let payload = json!({
            "inventories": [
                {
                    "property": "account_key",
                    "unique_count": 2,
                    "null_count": 0,
                    "total_rows": 3,
                    "existing_values": ["a_account", "b_account"],
                    "value_counts": [
                        {"value": "a_account", "count": 2},
                        {"value": "b_account", "count": 1}
                    ]
                }
            ]
        });

        let rendered = render_import_keys_uniq(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("\n\n  Value"));
            assert!(text.contains("Value"));
            assert!(text.contains("Count"));
            assert!(text.contains("a_account"));
            assert!(text.contains("b_account"));
            assert!(text.contains("2"));
            assert!(text.contains("1"));
        }
    }

    #[test]
    fn import_keys_uniq_empty_values_state_is_explicit() {
        let payload = json!({
            "property": "merchant",
            "inventories": [
                {
                    "property": "merchant",
                    "unique_count": 0,
                    "null_count": 10,
                    "total_rows": 10,
                    "existing_values": [],
                    "value_counts": []
                }
            ]
        });

        let rendered = render_import_keys_uniq(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("\n\n  No values."));
        }
    }

    #[test]
    fn import_list_renders_created_local_column() {
        let payload = json!({
            "rows": [
                {
                    "import_id": "imp_1",
                    "status": "committed",
                    "created_at": "1735689600",
                    "rows_read": 1,
                    "inserted": 1,
                    "deduped": 0
                }
            ]
        });

        let rendered = render_import_list(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("Created (local)"));
        }
    }

    #[test]
    fn import_list_renders_account_coverage_section() {
        let payload = json!({
            "rows": [
                {
                    "import_id": "imp_1",
                    "status": "committed",
                    "created_at": "1735689600",
                    "rows_read": 3,
                    "inserted": 2,
                    "deduped": 1,
                    "accounts": [
                        {
                            "account_key": "acct_1",
                            "account_type": "savings",
                            "rows_read": 3,
                            "inserted": 2,
                            "deduped": 1
                        }
                    ]
                }
            ]
        });

        let rendered = render_import_list(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("Account coverage:"));
            assert!(text.contains("acct_1"));
            assert!(text.contains("savings"));
        }
    }

    #[test]
    fn import_list_orders_by_import_id_when_created_at_ties() {
        let payload = json!({
            "rows": [
                {
                    "import_id": "imp_a",
                    "status": "committed",
                    "created_at": "1735689600",
                    "rows_read": 1,
                    "inserted": 1,
                    "deduped": 0
                },
                {
                    "import_id": "imp_b",
                    "status": "committed",
                    "created_at": "1735689600",
                    "rows_read": 1,
                    "inserted": 1,
                    "deduped": 0
                }
            ]
        });

        let rendered = render_import_list(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            let first_index = text.find("imp_b");
            let second_index = text.find("imp_a");
            assert!(first_index.is_some());
            assert!(second_index.is_some());
            if let (Some(first), Some(second)) = (first_index, second_index) {
                assert!(first < second);
            }
        }
    }

    #[test]
    fn import_run_renders_duplicate_summary_and_preview_sections() {
        let payload = json!({
            "dry_run": false,
            "import_id": "imp_1",
            "next_step": {
                "label": "Connect and query your data",
                "command": "driggsby db schema"
            },
            "other_actions": [],
            "summary": {
                "rows_read": 2,
                "rows_valid": 2,
                "rows_invalid": 0,
                "inserted": 1
            },
            "duplicate_summary": {
                "total": 1,
                "batch": 1,
                "existing_ledger": 0
            },
            "duplicates_preview": {
                "returned": 1,
                "truncated": false,
                "rows": [
                    {
                        "source_row_index": 2,
                        "dedupe_reason": "batch",
                        "statement_id": "acct_1_2026-01-31",
                        "account_key": "acct_1",
                        "posted_at": "2026-01-10",
                        "amount": -5.0,
                        "currency": "USD",
                        "description": "COFFEE",
                        "external_id": null,
                        "matched_batch_row_index": 1,
                        "matched_txn_id": null,
                        "matched_import_id": null
                    }
                ]
            }
        });

        let rendered = render_import_run(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("Duplicate Summary:"));
            assert!(text.contains("Duplicates Preview (showing all 1 duplicate rows):"));
            assert!(text.contains("Duplicate within this import"));
            assert!(text.contains("Row #2"));
        }
    }

    #[test]
    fn dry_run_inventory_renders_account_type_property() {
        let payload = json!({
            "dry_run": true,
            "summary": {
                "rows_read": 1,
                "rows_valid": 1,
                "rows_invalid": 0,
                "inserted": 0
            },
            "duplicate_summary": {
                "total": 0,
                "batch": 0,
                "existing_ledger": 0
            },
            "duplicates_preview": {
                "returned": 0,
                "truncated": false,
                "rows": []
            },
            "key_inventory": {
                "account_key": {"unique_count": 1, "null_count": 0, "total_rows": 1, "existing_values": ["acct_1"]},
                "currency": {"unique_count": 1, "null_count": 0, "total_rows": 1, "existing_values": ["USD"]},
                "merchant": {"unique_count": 0, "null_count": 1, "total_rows": 1, "existing_values": []},
                "category": {"unique_count": 0, "null_count": 1, "total_rows": 1, "existing_values": []},
                "account_type": {"unique_count": 1, "null_count": 0, "total_rows": 1, "existing_values": ["checking"]}
            },
            "next_step": {"label": "Commit", "command": "driggsby import create <path>"},
            "other_actions": []
        });

        let rendered = render_import_run(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("account_type:"));
            assert!(text.contains("checking"));
        }
    }

    #[test]
    fn import_run_renders_truncated_duplicate_preview_label() {
        let payload = json!({
            "dry_run": false,
            "import_id": "imp_1",
            "next_step": {
                "label": "Connect and query your data",
                "command": "driggsby db schema"
            },
            "other_actions": [],
            "summary": {
                "rows_read": 60,
                "rows_valid": 60,
                "rows_invalid": 0,
                "inserted": 10
            },
            "duplicate_summary": {
                "total": 55,
                "batch": 55,
                "existing_ledger": 0
            },
            "duplicates_preview": {
                "returned": 50,
                "truncated": true,
                "rows": []
            }
        });

        let rendered = render_import_run(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains(
                "Duplicates Preview (showing first 50 of 55 duplicate rows; truncated):"
            ));
        }
    }

    #[test]
    fn import_run_renders_source_conflict_warnings() {
        let payload = json!({
            "dry_run": true,
            "summary": {
                "rows_read": 1,
                "rows_valid": 1,
                "rows_invalid": 0,
                "inserted": 0
            },
            "duplicate_summary": {
                "total": 0,
                "batch": 0,
                "existing_ledger": 0
            },
            "duplicates_preview": {
                "returned": 0,
                "truncated": false,
                "rows": []
            },
            "warnings": [
                {
                    "code": "stdin_ignored_file_provided",
                    "message": "Both stdin and file were provided; file input was used."
                }
            ],
            "source_conflict": true,
            "source_used": "file",
            "source_ignored": "stdin"
        });

        let rendered = render_import_run(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            assert!(text.contains("Warnings:"));
            assert!(text.contains("stdin"));
            assert!(text.contains("ignored"));
        }
    }
}
