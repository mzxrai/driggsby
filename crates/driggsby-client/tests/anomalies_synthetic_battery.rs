use std::fs;
use std::path::{Path, PathBuf};

use driggsby_client::commands::anomalies::{self, AnomaliesRunOptions};
use driggsby_client::commands::import::{self, ImportRunOptions};
use driggsby_client::setup::ensure_initialized_at;
use driggsby_client::state::{map_sqlite_error, open_connection};
use rusqlite::Connection;
use serde_json::{Value, json};
use tempfile::{Builder, TempDir};

fn temp_home_in_tmp(prefix: &str) -> std::io::Result<(TempDir, PathBuf)> {
    let dir = Builder::new().prefix(prefix).tempdir_in("/tmp")?;
    let home = dir.path().join("ledger-home");
    fs::create_dir_all(&home)?;
    Ok((dir, home))
}

fn write_fixture_json(base: &Path, name: &str, rows: &[Value]) -> std::io::Result<PathBuf> {
    let path = base.join(name);
    let body = serde_json::to_string_pretty(rows).map_err(std::io::Error::other)?;
    fs::write(&path, body)?;
    Ok(path)
}

fn transaction(
    statement_id: &str,
    account_key: &str,
    posted_at: &str,
    amount: f64,
    merchant: &str,
) -> Value {
    json!({
        "statement_id": statement_id,
        "account_key": account_key,
        "posted_at": posted_at,
        "amount": amount,
        "currency": "USD",
        "description": merchant,
        "merchant": merchant
    })
}

fn import_rows(home: &Path, rows: &[Value]) {
    let temp_fixture = Builder::new()
        .prefix("driggsby-anomaly-fixture")
        .tempdir_in("/tmp");
    assert!(temp_fixture.is_ok());
    if let Ok(dir) = temp_fixture {
        let fixture = write_fixture_json(dir.path(), "rows.json", rows);
        assert!(fixture.is_ok());
        if let Ok(path) = fixture {
            let imported = import::run_with_options(ImportRunOptions {
                path: Some(path.display().to_string()),
                dry_run: false,
                home_override: Some(home),
                stdin_override: None,
            });
            assert!(imported.is_ok());
        }
    }
}

fn anomalies_txn_ids(home: &Path) -> Vec<String> {
    let response = anomalies::run_with_options(AnomaliesRunOptions {
        from: None,
        to: None,
        home_override: Some(home),
    });
    assert!(response.is_ok());
    if let Ok(success) = response {
        let payload = serde_json::to_value(success);
        assert!(payload.is_ok());
        if let Ok(value) = payload {
            let mut ids = value["data"]["rows"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|row| {
                    row.get("txn_id")
                        .and_then(Value::as_str)
                        .map(std::string::ToString::to_string)
                })
                .collect::<Vec<String>>();
            ids.sort();
            return ids;
        }
    }
    Vec::new()
}

fn anomaly_sql_txn_ids(home: &Path) -> Vec<String> {
    let setup = ensure_initialized_at(home);
    assert!(setup.is_ok());
    if let Ok(context) = setup {
        let db_path = PathBuf::from(context.db_path);
        let connection = open_connection(&db_path);
        assert!(connection.is_ok());
        if let Ok(conn) = connection {
            return query_txn_ids_from_view(&conn, &db_path);
        }
    }
    Vec::new()
}

fn query_txn_ids_from_view(connection: &Connection, db_path: &Path) -> Vec<String> {
    let mut statement = connection
        .prepare("SELECT txn_id FROM v1_anomalies ORDER BY txn_id ASC")
        .map_err(|error| map_sqlite_error(db_path, &error));
    assert!(statement.is_ok());
    if let Ok(ref mut stmt) = statement {
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| map_sqlite_error(db_path, &error));
        assert!(rows.is_ok());
        if let Ok(iter) = rows {
            let mut out = Vec::new();
            for value in iter.flatten() {
                out.push(value);
            }
            return out;
        }
    }
    Vec::new()
}

#[test]
fn synthetic_battery_detects_spikes_avoids_false_positives_and_matches_sql() {
    let temp = temp_home_in_tmp("driggsby-anomalies-battery");
    assert!(temp.is_ok());
    if let Ok((_dir, home)) = temp {
        let mut rows = vec![
            // Stable monthly recurring spend should not be flagged.
            transaction("stmt_1", "acct_main", "2026-01-05", -45.0, "Gym Club"),
            transaction("stmt_1", "acct_main", "2026-02-05", -45.5, "Gym Club"),
            transaction("stmt_1", "acct_main", "2026-03-05", -44.8, "Gym Club"),
            transaction("stmt_1", "acct_main", "2026-04-05", -45.2, "Gym Club"),
            transaction("stmt_1", "acct_main", "2026-05-05", -45.1, "Gym Club"),
            transaction("stmt_1", "acct_main", "2026-06-05", -45.0, "Gym Club"),
            // Grocery history with one clear spike.
            transaction("stmt_1", "acct_main", "2026-01-03", -22.0, "Fresh Mart"),
            transaction("stmt_1", "acct_main", "2026-01-10", -21.5, "Fresh Mart"),
            transaction("stmt_1", "acct_main", "2026-01-17", -22.25, "Fresh Mart"),
            transaction("stmt_1", "acct_main", "2026-01-24", -22.1, "Fresh Mart"),
            transaction("stmt_1", "acct_main", "2026-01-31", -21.95, "Fresh Mart"),
            transaction("stmt_1", "acct_main", "2026-02-07", -318.4, "Fresh Mart"),
            // Separate account stable spend should remain clean.
            transaction("stmt_1", "acct_alt", "2026-01-08", -19.5, "Corner Market"),
            transaction("stmt_1", "acct_alt", "2026-01-15", -20.0, "Corner Market"),
            transaction("stmt_1", "acct_alt", "2026-01-22", -19.75, "Corner Market"),
            transaction("stmt_1", "acct_alt", "2026-01-29", -20.1, "Corner Market"),
            transaction("stmt_1", "acct_alt", "2026-02-05", -20.25, "Corner Market"),
            transaction("stmt_1", "acct_alt", "2026-02-12", -20.0, "Corner Market"),
        ];

        rows.sort_by(|left, right| {
            left["posted_at"]
                .as_str()
                .unwrap_or_default()
                .cmp(right["posted_at"].as_str().unwrap_or_default())
                .then_with(|| {
                    left["description"]
                        .as_str()
                        .unwrap_or_default()
                        .cmp(right["description"].as_str().unwrap_or_default())
                })
        });

        import_rows(&home, &rows);
        let command_ids = anomalies_txn_ids(&home);
        let sql_ids = anomaly_sql_txn_ids(&home);

        assert_eq!(command_ids, sql_ids);
        assert_eq!(command_ids.len(), 1);
        let only_id = command_ids.first().cloned().unwrap_or_default();
        assert!(!only_id.is_empty());
    }
}
