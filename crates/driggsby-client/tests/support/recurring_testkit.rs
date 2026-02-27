use std::fs;
use std::path::{Path, PathBuf};

use driggsby_client::commands::import::{self, ImportRunOptions};
use driggsby_client::commands::recurring::{self, RecurringRunOptions};
use serde_json::{Value, json};
use tempfile::{Builder, TempDir};

pub fn temp_home_in_tmp(prefix: &str) -> std::io::Result<(TempDir, PathBuf)> {
    let dir = Builder::new().prefix(prefix).tempdir_in("/tmp")?;
    let home = dir.path().join("ledger-home");
    fs::create_dir_all(&home)?;
    Ok((dir, home))
}

pub fn import_rows(home: &Path, rows: &[Value]) {
    let temp_dir = Builder::new()
        .prefix("driggsby-recurring-fixture")
        .tempdir_in("/tmp");
    assert!(temp_dir.is_ok());
    if let Ok(dir) = temp_dir {
        let fixture = write_fixture_json(dir.path(), "rows.json", rows);
        assert!(fixture.is_ok());
        if let Ok(path) = fixture {
            let result = import::run_with_options(ImportRunOptions {
                path: Some(path.display().to_string()),
                dry_run: false,
                home_override: Some(home),
                stdin_override: None,
            });
            assert!(result.is_ok());
        }
    }
}

pub fn recurring_payload(home: &Path, from: Option<&str>, to: Option<&str>) -> Value {
    let result = recurring::run_with_options(RecurringRunOptions {
        from: from.map(std::string::ToString::to_string),
        to: to.map(std::string::ToString::to_string),
        home_override: Some(home),
    });
    assert!(result.is_ok());
    if let Ok(success) = result {
        let payload = serde_json::to_value(success);
        assert!(payload.is_ok());
        if let Ok(value) = payload {
            return value;
        }
    }
    Value::Null
}

pub fn recurring_rows(home: &Path, from: Option<&str>, to: Option<&str>) -> Vec<Value> {
    recurring_payload(home, from, to)["data"]["rows"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

pub fn transaction(
    account_key: &str,
    posted_at: &str,
    amount: f64,
    currency: &str,
    description: &str,
    merchant: Option<&str>,
) -> Value {
    json!({
        "statement_id": "statement_2026_01",
        "account_key": account_key,
        "posted_at": posted_at,
        "amount": amount,
        "currency": currency,
        "description": description,
        "merchant": merchant,
    })
}

pub fn recurring_group_exists(rows: &[Value], merchant: &str, cadence: &str) -> bool {
    rows.iter().any(|row| {
        row.get("merchant").and_then(Value::as_str) == Some(merchant)
            && row.get("cadence").and_then(Value::as_str) == Some(cadence)
    })
}

pub fn run_scenario(rows: &[Value], from: Option<&str>, to: Option<&str>) -> Vec<Value> {
    let temp = temp_home_in_tmp("driggsby-recurring-scenario");
    assert!(temp.is_ok());
    if let Ok((_dir, home)) = temp {
        import_rows(&home, rows);
        return recurring_rows(&home, from, to);
    }
    Vec::new()
}

fn write_fixture_json(base: &Path, name: &str, rows: &[Value]) -> std::io::Result<PathBuf> {
    let path = base.join(name);
    let body = serde_json::to_string_pretty(rows).map_err(std::io::Error::other)?;
    fs::write(&path, body)?;
    Ok(path)
}
