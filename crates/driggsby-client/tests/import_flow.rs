use std::fs;
use std::path::{Path, PathBuf};

use driggsby_client::commands::import;
use driggsby_client::commands::import::{
    ImportDuplicatesOptions, ImportListOptions, ImportRunOptions, ImportUndoOptions,
};
use driggsby_client::contracts::envelope::failure_from_error;
use rusqlite::Connection;
use serde_json::Value;
use tempfile::tempdir;

fn write_file(path: &Path, body: &str) {
    let result = fs::write(path, body);
    assert!(result.is_ok());
}

fn temp_home() -> std::io::Result<(tempfile::TempDir, PathBuf)> {
    let dir = tempdir()?;
    let home = dir.path().join("ledger-home");
    Ok((dir, home))
}

fn run_import(
    home: &Path,
    path: Option<&Path>,
    dry_run: bool,
    stdin_override: Option<&str>,
) -> driggsby_client::ClientResult<driggsby_client::SuccessEnvelope> {
    run_import_with_raw_path(
        home,
        path.map(|value| value.display().to_string()),
        dry_run,
        stdin_override,
    )
}

fn run_import_with_raw_path(
    home: &Path,
    path: Option<String>,
    dry_run: bool,
    stdin_override: Option<&str>,
) -> driggsby_client::ClientResult<driggsby_client::SuccessEnvelope> {
    import::run_with_options(ImportRunOptions {
        path,
        dry_run,
        home_override: Some(home),
        stdin_override: stdin_override.map(std::string::ToString::to_string),
    })
}

fn run_import_list(home: &Path) -> driggsby_client::ClientResult<driggsby_client::SuccessEnvelope> {
    import::list_with_options(ImportListOptions {
        home_override: Some(home),
    })
}

fn run_import_undo(
    home: &Path,
    import_id: &str,
) -> driggsby_client::ClientResult<driggsby_client::SuccessEnvelope> {
    import::undo_with_options(
        import_id,
        ImportUndoOptions {
            home_override: Some(home),
        },
    )
}

fn run_import_duplicates(
    home: &Path,
    import_id: &str,
) -> driggsby_client::ClientResult<driggsby_client::SuccessEnvelope> {
    import::duplicates_with_options(
        import_id,
        ImportDuplicatesOptions {
            home_override: Some(home),
        },
    )
}

fn query_count(db_path: &Path, sql: &str) -> i64 {
    let connection = Connection::open(db_path);
    assert!(connection.is_ok());
    if let Ok(conn) = connection {
        let value = conn.query_row(sql, [], |row| row.get::<_, i64>(0));
        assert!(value.is_ok());
        if let Ok(count) = value {
            return count;
        }
    }
    0
}

fn query_optional_string(db_path: &Path, sql: &str) -> Option<String> {
    let connection = Connection::open(db_path).ok()?;
    connection
        .query_row(sql, [], |row| row.get::<_, String>(0))
        .ok()
}

fn execute_sql(db_path: &Path, sql: &str) -> bool {
    let connection = Connection::open(db_path);
    assert!(connection.is_ok());
    if let Ok(conn) = connection {
        return conn.execute_batch(sql).is_ok();
    }
    false
}

fn extract_import_id(payload: &Value) -> Option<String> {
    payload
        .get("data")
        .and_then(|data| data.get("import_id"))
        .and_then(Value::as_str)
        .map(std::string::ToString::to_string)
}

fn assert_import_summary(payload: &Value, inserted: i64, deduped_total: i64) {
    assert!(payload["data"]["summary"]["rows_read"].is_i64());
    assert!(payload["data"]["summary"]["rows_valid"].is_i64());
    assert!(payload["data"]["summary"]["rows_invalid"].is_i64());
    assert_eq!(
        payload["data"]["summary"]["inserted"],
        Value::from(inserted)
    );
    assert_eq!(
        payload["data"]["duplicate_summary"]["total"],
        Value::from(deduped_total)
    );
}

fn action_commands(payload: &Value) -> Vec<String> {
    payload["data"]["other_actions"]
        .as_array()
        .map(|actions| {
            actions
                .iter()
                .filter_map(|action| action.get("command").and_then(Value::as_str))
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

#[test]
fn file_only_json_import_success_writes_rows() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("transactions.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        write_file(
            &source_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-01","amount":-42.15,"currency":"USD","description":"WHOLE FOODS","external_id":"txn_100"},
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-02","amount":-17.89,"currency":"USD","description":"SHELL OIL"}
]"#,
        );

        let result = run_import(&home, Some(&source_path), false, None);
        assert!(result.is_ok());
        if let Ok(success) = result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["ok"], Value::Bool(true));
                assert_eq!(value["command"], Value::String("import".to_string()));
                assert!(value["data"]["import_id"].is_string());
                assert!(value["data"].get("undo_id").is_none());
                assert_eq!(
                    value["data"]["next_step"]["command"],
                    Value::String("driggsby schema".to_string())
                );
                let commands = action_commands(&value);
                assert_eq!(
                    commands,
                    vec![
                        "driggsby import list".to_string(),
                        format!(
                            "driggsby import undo {}",
                            value["data"]["import_id"].as_str().unwrap_or_default()
                        ),
                    ]
                );
                assert_import_summary(&value, 2, 0);
                assert!(value["data"]["query_context"].is_object());
            }
        }

        let db_path = home.join("ledger.db");
        let txn_count = query_count(&db_path, "SELECT COUNT(*) FROM internal_transactions");
        let import_count = query_count(&db_path, "SELECT COUNT(*) FROM internal_import_runs");
        assert_eq!(txn_count, 2);
        assert_eq!(import_count, 1);
    }
}

#[test]
fn dry_run_does_not_write_import_or_transactions() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("transactions.csv");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        write_file(
            &source_path,
            "statement_id,account_key,posted_at,amount,currency,description\nchase_checking_1234_2026-01-31,chase_checking_1234,2026-01-03,-9.99,USD,COFFEE\n",
        );

        let result = run_import(&home, Some(&source_path), true, None);
        assert!(result.is_ok());
        if let Ok(success) = result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["ok"], Value::Bool(true));
                assert_eq!(value["data"]["dry_run"], Value::Bool(true));
                assert!(value["data"]["import_id"].is_null());
                assert_eq!(
                    value["data"]["next_step"]["command"],
                    Value::String("driggsby import create <path>".to_string())
                );
                let commands = action_commands(&value);
                assert!(commands.is_empty());
                assert_import_summary(&value, 0, 0);
            }
        }

        let db_path = home.join("ledger.db");
        let txn_count = query_count(&db_path, "SELECT COUNT(*) FROM internal_transactions");
        let import_count = query_count(&db_path, "SELECT COUNT(*) FROM internal_import_runs");
        assert_eq!(txn_count, 0);
        assert_eq!(import_count, 0);
    }
}

#[test]
fn dry_run_with_stdin_uses_stdin_commit_next_step() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let stdin_body = r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-03","amount":-9.99,"currency":"USD","description":"COFFEE"}
]"#;

        let result = run_import(&home, None, true, Some(stdin_body));
        assert!(result.is_ok());
        if let Ok(success) = result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(
                    value["data"]["source_used"],
                    Value::String("stdin".to_string())
                );
                assert_eq!(
                    value["data"]["next_step"]["command"],
                    Value::String("driggsby import create".to_string())
                );
                assert!(action_commands(&value).is_empty());
            }
        }
    }
}

#[test]
fn file_and_stdin_prefers_file_and_emits_warning_metadata() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("transactions.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        write_file(
            &source_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-04","amount":-25.00,"currency":"USD","description":"TARGET"}
]"#,
        );

        let stdin = r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-05","amount":-33.00,"currency":"USD","description":"IGNORED"}
]"#;

        let result = run_import(&home, Some(&source_path), true, Some(stdin));
        assert!(result.is_ok());
        if let Ok(success) = result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(
                    value["data"]["source_used"],
                    Value::String("file".to_string())
                );
                assert_eq!(
                    value["data"]["source_ignored"],
                    Value::String("stdin".to_string())
                );
                assert_eq!(value["data"]["source_conflict"], Value::Bool(true));
                assert_eq!(
                    value["data"]["warnings"][0]["code"],
                    Value::String("stdin_ignored_file_provided".to_string())
                );
                assert_eq!(value["data"]["summary"]["rows_read"], Value::from(1));
            }
        }
    }
}

#[test]
fn stdin_dash_alias_uses_stdin_source() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let stdin_body = r#"[
  {"statement_id":"chase_checking_1234_2026-09-30","account_key":"chase_checking_1234","posted_at":"2026-09-01","amount":-4.50,"currency":"USD","description":"DASH-STDIN"}
]"#;

        let result = run_import_with_raw_path(&home, Some("-".to_string()), true, Some(stdin_body));
        assert!(result.is_ok());
        if let Ok(success) = result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(
                    value["data"]["source_used"],
                    Value::String("stdin".to_string())
                );
                assert!(value["data"]["source_ignored"].is_null());
                assert_eq!(value["data"]["source_conflict"], Value::Bool(false));
                assert_eq!(value["data"]["summary"]["rows_read"], Value::from(1));
            }
        }
    }
}

#[test]
fn stdin_dash_alias_without_stdin_content_returns_guidance_error() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let result = run_import_with_raw_path(&home, Some("-".to_string()), true, Some(" \n "));
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("stdin"));
            assert!(error.message.contains("-"));
        }
    }
}

#[test]
fn ndjson_source_is_rejected() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("transactions.ndjson");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            "{\"account_key\":\"a\",\"posted_at\":\"2026-01-01\",\"amount\":1,\"currency\":\"USD\",\"description\":\"x\"}\n{\"account_key\":\"a\",\"posted_at\":\"2026-01-02\",\"amount\":2,\"currency\":\"USD\",\"description\":\"y\"}\n",
        );

        let result = run_import(&home, Some(&source_path), true, None);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("NDJSON"));
            let envelope = failure_from_error(&error);
            let as_json = serde_json::to_value(envelope);
            assert!(as_json.is_ok());
            if let Ok(value) = as_json {
                assert!(value.get("data").is_none());
                assert_eq!(
                    value["error"]["data"]["help_command"],
                    Value::String("driggsby import create --help".to_string())
                );
                assert_eq!(
                    value["error"]["data"]["help_section_title"],
                    Value::String("Import Troubleshooting".to_string())
                );
            }
        }
    }
}

#[test]
fn csv_header_mismatch_returns_import_schema_mismatch_with_data() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("transactions.csv");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            "account,posted_date,amount_usd,description\nx,2026-01-01,-1.00,Test\n",
        );

        let result = run_import(&home, Some(&source_path), true, None);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "import_schema_mismatch");
            let envelope = failure_from_error(&error);
            let as_json = serde_json::to_value(envelope);
            assert!(as_json.is_ok());
            if let Ok(value) = as_json {
                assert!(value.get("data").is_none());
                assert!(value["error"]["data"]["expected_headers"].is_array());
                assert!(value["error"]["data"]["actual_headers"].is_array());
                assert!(
                    value["error"]["data"]["expected_headers"]
                        .as_array()
                        .map(|headers| headers
                            .iter()
                            .any(|header| { header == &Value::String("statement_id".to_string()) }))
                        .unwrap_or(false)
                );
                assert_eq!(
                    value["error"]["data"]["help_command"],
                    Value::String("driggsby import create --help".to_string())
                );
                assert_eq!(
                    value["error"]["data"]["help_section_title"],
                    Value::String("Import Troubleshooting".to_string())
                );
            }
        }
    }
}

#[test]
fn row_validation_failures_return_deterministic_issues() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("invalid.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"01/12/26","amount":"forty two","currency":"USD","description":"Bad Row"},
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-06","amount":-9.10,"currency":"USD","description":"Valid Row"}
]"#,
        );

        let result = run_import(&home, Some(&source_path), false, None);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "import_validation_failed");
            let envelope = failure_from_error(&error);
            let as_json = serde_json::to_value(envelope);
            assert!(as_json.is_ok());
            if let Ok(value) = as_json {
                assert!(value.get("data").is_none());
                assert_eq!(
                    value["error"]["data"]["summary"]["rows_read"],
                    Value::from(2)
                );
                assert_eq!(
                    value["error"]["data"]["summary"]["rows_invalid"],
                    Value::from(1)
                );
                assert!(value["error"]["data"]["issues"].is_array());
                assert!(value["error"]["data"]["issues"][0]["row"].is_i64());
                assert!(value["error"]["data"]["issues"][0]["field"].is_string());
                assert!(value["error"]["data"]["issues"][0]["code"].is_string());
                assert_eq!(
                    value["error"]["data"]["help_command"],
                    Value::String("driggsby import create --help".to_string())
                );
                assert_eq!(
                    value["error"]["data"]["help_section_title"],
                    Value::String("Import Troubleshooting".to_string())
                );
            }
        }

        let db_path = home.join("ledger.db");
        let txn_count = query_count(&db_path, "SELECT COUNT(*) FROM internal_transactions");
        let import_count = query_count(&db_path, "SELECT COUNT(*) FROM internal_import_runs");
        assert_eq!(txn_count, 0);
        assert_eq!(import_count, 0);
    }
}

#[test]
fn dedupe_counts_batch_and_existing_rows() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let first_path = home.join("first.json");
        let second_path = home.join("second.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        write_file(
            &first_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-01","amount":-10.00,"currency":"USD","description":"A","external_id":"ext_1"},
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-02","amount":-20.00,"currency":"USD","description":"B"}
]"#,
        );
        let first = run_import(&home, Some(&first_path), false, None);
        assert!(first.is_ok());

        write_file(
            &second_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-01","amount":-10.00,"currency":"USD","description":"A","external_id":"ext_1"},
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-03","amount":-30.00,"currency":"USD","description":"C"},
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-03","amount":-30.00,"currency":"USD","description":"C"}
]"#,
        );
        let second = run_import(&home, Some(&second_path), false, None);
        assert!(second.is_ok());
        if let Ok(success) = second {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["summary"]["rows_read"], Value::from(3));
                assert_eq!(value["data"]["summary"]["inserted"], Value::from(2));
                assert_eq!(value["data"]["duplicate_summary"]["total"], Value::from(1));
                let import_id = value["data"]["import_id"].as_str().unwrap_or_default();
                let commands = action_commands(&value);
                assert_eq!(
                    commands,
                    vec![
                        "driggsby import list".to_string(),
                        format!("driggsby import duplicates {import_id}"),
                        format!("driggsby import undo {import_id}"),
                    ]
                );
            }
        }

        let db_path = home.join("ledger.db");
        let txn_count = query_count(&db_path, "SELECT COUNT(*) FROM internal_transactions");
        let import_count = query_count(&db_path, "SELECT COUNT(*) FROM internal_import_runs");
        assert_eq!(txn_count, 4);
        assert_eq!(import_count, 2);
    }
}

#[test]
fn json_import_missing_statement_id_is_null_and_dedupes_across_imports() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("missing-statement-id.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"account_key":"chase_checking_1234","posted_at":"2026-05-01","amount":-10.00,"currency":"USD","description":"MISSING-STATEMENT"}
]"#,
        );

        let first_result = run_import(&home, Some(&source_path), false, None);
        assert!(first_result.is_ok());

        if let Ok(success) = first_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["summary"]["inserted"], Value::from(1));
                assert_eq!(value["data"]["duplicate_summary"]["total"], Value::from(0));
            }
        }

        let db_path = home.join("ledger.db");
        let stored_statement_id = query_optional_string(
            &db_path,
            "SELECT statement_id FROM internal_transactions WHERE description = 'MISSING-STATEMENT' LIMIT 1",
        );
        assert!(stored_statement_id.is_none());

        let second_result = run_import(&home, Some(&source_path), false, None);
        assert!(second_result.is_ok());
        if let Ok(success) = second_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["summary"]["inserted"], Value::from(0));
                assert_eq!(value["data"]["duplicate_summary"]["total"], Value::from(1));
                assert_eq!(
                    value["data"]["duplicate_summary"]["existing_ledger"],
                    Value::from(1)
                );
            }
        }

        assert_eq!(
            query_count(
                &db_path,
                "SELECT COUNT(*) FROM internal_transactions WHERE description = 'MISSING-STATEMENT'"
            ),
            1
        );
    }
}

#[test]
fn csv_import_missing_statement_id_is_null_and_dedupes_across_imports() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("missing-statement-id.csv");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            "account_key,posted_at,amount,currency,description\nchase_checking_1234,2026-05-01,-10.00,USD,MISSING-STATEMENT-CSV\n",
        );

        let first_result = run_import(&home, Some(&source_path), false, None);
        assert!(first_result.is_ok());

        if let Ok(success) = first_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["summary"]["inserted"], Value::from(1));
                assert_eq!(value["data"]["duplicate_summary"]["total"], Value::from(0));
            }
        }

        let db_path = home.join("ledger.db");
        let stored_statement_id = query_optional_string(
            &db_path,
            "SELECT statement_id FROM internal_transactions WHERE description = 'MISSING-STATEMENT-CSV' LIMIT 1",
        );
        assert!(stored_statement_id.is_none());

        let second_result = run_import(&home, Some(&source_path), false, None);
        assert!(second_result.is_ok());
        if let Ok(success) = second_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["summary"]["inserted"], Value::from(0));
                assert_eq!(value["data"]["duplicate_summary"]["total"], Value::from(1));
                assert_eq!(
                    value["data"]["duplicate_summary"]["existing_ledger"],
                    Value::from(1)
                );
            }
        }

        assert_eq!(
            query_count(
                &db_path,
                "SELECT COUNT(*) FROM internal_transactions WHERE description = 'MISSING-STATEMENT-CSV'"
            ),
            1
        );
    }
}

#[test]
fn dry_run_missing_statement_id_returns_null_statement_in_duplicate_preview() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("missing-statement-id-dry-run.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"account_key":"chase_checking_1234","posted_at":"2026-05-01","amount":-10.00,"currency":"USD","description":"MISSING-STATEMENT-DRYRUN"}
]"#,
        );

        let first_commit = run_import(&home, Some(&source_path), false, None);
        assert!(first_commit.is_ok());

        let dry_run_one = run_import(&home, Some(&source_path), true, None);
        let dry_run_two = run_import(&home, Some(&source_path), true, None);
        assert!(dry_run_one.is_ok());
        assert!(dry_run_two.is_ok());

        let mut statement_one = None;
        let mut statement_two = None;

        if let Ok(success) = dry_run_one {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                statement_one = value["data"]["duplicates_preview"]["rows"]
                    .as_array()
                    .and_then(|rows| rows.first())
                    .and_then(|row| row.get("statement_id"))
                    .and_then(Value::as_str)
                    .map(std::string::ToString::to_string);
            }
        }

        if let Ok(success) = dry_run_two {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                statement_two = value["data"]["duplicates_preview"]["rows"]
                    .as_array()
                    .and_then(|rows| rows.first())
                    .and_then(|row| row.get("statement_id"))
                    .and_then(Value::as_str)
                    .map(std::string::ToString::to_string);
            }
        }

        assert_eq!(statement_one, None);
        assert_eq!(statement_two, None);
    }
}

#[test]
fn explicit_statement_id_with_internal_prefix_is_preserved() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("explicit-prefix-statement-id.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"statement_id":"gen_pending_import_manualscope","account_key":"chase_checking_1234","posted_at":"2026-05-01","amount":-10.00,"currency":"USD","description":"EXPLICIT-PREFIX"}
]"#,
        );

        let result = run_import(&home, Some(&source_path), false, None);
        assert!(result.is_ok());

        let db_path = home.join("ledger.db");
        assert_eq!(
            query_optional_string(
                &db_path,
                "SELECT statement_id FROM internal_transactions WHERE description = 'EXPLICIT-PREFIX' LIMIT 1"
            ),
            Some("gen_pending_import_manualscope".to_string())
        );
    }
}

#[test]
fn amount_with_more_than_two_decimals_fails_validation() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("invalid-amount-scale.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-10-31","account_key":"chase_checking_1234","posted_at":"2026-10-01","amount":-12.345,"currency":"USD","description":"TOO-MANY-DECIMALS"}
]"#,
        );

        let result = run_import(&home, Some(&source_path), true, None);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "import_validation_failed");
            let envelope = failure_from_error(&error);
            let as_json = serde_json::to_value(envelope);
            assert!(as_json.is_ok());
            if let Ok(value) = as_json {
                assert_eq!(
                    value["error"]["data"]["issues"][0]["field"],
                    Value::String("amount".to_string())
                );
                assert_eq!(
                    value["error"]["data"]["issues"][0]["code"],
                    Value::String("invalid_amount_scale".to_string())
                );
                assert!(
                    value["error"]["data"]["issues"][0]["description"]
                        .as_str()
                        .unwrap_or_default()
                        .contains("2 decimal")
                );
            }
        }
    }
}

#[test]
fn amount_with_scientific_notation_over_two_decimals_fails_validation() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("invalid-amount-scientific.csv");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            "statement_id,account_key,posted_at,amount,currency,description\nacct_scale_1_2026-10-31,acct_scale_1,2026-10-03,1e-3,USD,SCI-NOTATION\n",
        );

        let result = run_import(&home, Some(&source_path), true, None);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "import_validation_failed");
            let envelope = failure_from_error(&error);
            let as_json = serde_json::to_value(envelope);
            assert!(as_json.is_ok());
            if let Ok(value) = as_json {
                assert_eq!(
                    value["error"]["data"]["issues"][0]["code"],
                    Value::String("invalid_amount_scale".to_string())
                );
            }
        }
    }
}

#[test]
fn amount_with_leading_dot_over_two_decimals_fails_validation() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("invalid-amount-leading-dot.csv");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            "statement_id,account_key,posted_at,amount,currency,description\nacct_scale_1_2026-10-31,acct_scale_1,2026-10-04,.1234,USD,LEADING-DOT\n",
        );

        let result = run_import(&home, Some(&source_path), true, None);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "import_validation_failed");
            let envelope = failure_from_error(&error);
            let as_json = serde_json::to_value(envelope);
            assert!(as_json.is_ok());
            if let Ok(value) = as_json {
                assert_eq!(
                    value["error"]["data"]["issues"][0]["code"],
                    Value::String("invalid_amount_scale".to_string())
                );
            }
        }
    }
}

#[test]
fn two_decimal_and_integer_amounts_pass_validation() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("valid-amount-scale.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-10-31","account_key":"chase_checking_1234","posted_at":"2026-10-01","amount":-12.34,"currency":"USD","description":"TWO-DECIMAL"},
  {"statement_id":"chase_checking_1234_2026-10-31","account_key":"chase_checking_1234","posted_at":"2026-10-02","amount":15,"currency":"USD","description":"INTEGER"}
]"#,
        );

        let result = run_import(&home, Some(&source_path), false, None);
        assert!(result.is_ok());
        if let Ok(success) = result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["summary"]["rows_read"], Value::from(2));
                assert_eq!(value["data"]["summary"]["rows_invalid"], Value::from(0));
                assert_eq!(value["data"]["summary"]["inserted"], Value::from(2));
            }
        }
    }
}

#[test]
fn same_statement_repeated_fallback_rows_are_not_deduped() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("same-statement-duplicates.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-05-31","account_key":"chase_checking_1234","posted_at":"2026-05-10","amount":-12.34,"currency":"USD","description":"SAME-STATEMENT"},
  {"statement_id":"chase_checking_1234_2026-05-31","account_key":"chase_checking_1234","posted_at":"2026-05-10","amount":-12.34,"currency":"USD","description":"SAME-STATEMENT"}
]"#,
        );

        let result = run_import(&home, Some(&source_path), false, None);
        assert!(result.is_ok());
        if let Ok(success) = result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["summary"]["inserted"], Value::from(2));
                assert!(value["data"]["summary"]["deduped"].is_null());
                assert_eq!(value["data"]["duplicate_summary"]["total"], Value::from(0));
                let import_id = value["data"]["import_id"].as_str().unwrap_or_default();
                let commands = action_commands(&value);
                assert_eq!(
                    commands,
                    vec![
                        "driggsby import list".to_string(),
                        format!("driggsby import undo {import_id}"),
                    ]
                );
            }
        }
    }
}

#[test]
fn import_create_returns_duplicate_summary_and_preview_contract() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("import-preview.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-05-31","account_key":"chase_checking_1234","posted_at":"2026-05-01","amount":-1.00,"currency":"USD","description":"DUP"},
  {"statement_id":"chase_checking_1234_2026-06-30","account_key":"chase_checking_1234","posted_at":"2026-05-01","amount":-1.00,"currency":"USD","description":"DUP"}
]"#,
        );

        let result = run_import(&home, Some(&source_path), false, None);
        assert!(result.is_ok());
        if let Ok(success) = result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert!(value["data"]["duplicate_summary"]["total"].is_i64());
                assert!(value["data"]["duplicate_summary"]["batch"].is_i64());
                assert!(value["data"]["duplicate_summary"]["existing_ledger"].is_i64());
                assert!(value["data"]["duplicates_preview"]["returned"].is_i64());
                assert!(value["data"]["duplicates_preview"]["truncated"].is_boolean());
                assert!(value["data"]["duplicates_preview"]["rows"].is_array());
                assert!(value["data"]["summary"]["deduped"].is_null());
            }
        }
    }
}

#[test]
fn no_input_fails_with_invalid_argument() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let result = run_import(&home, None, true, None);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "invalid_argument");
        }
    }
}

#[test]
fn import_list_handles_null_source_ref_rows() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let stdin_body = r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-04-01","amount":-7.00,"currency":"USD","description":"STDIN-LIST"}
]"#;

        let imported = run_import(&home, None, false, Some(stdin_body));
        assert!(imported.is_ok());

        let listed = run_import_list(&home);
        assert!(listed.is_ok());
        if let Ok(success) = listed {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert!(value["data"]["rows"].is_array());
                if let Some(rows) = value["data"]["rows"].as_array() {
                    assert_eq!(rows.len(), 1);
                    assert_eq!(rows[0]["source_kind"], Value::String("stdin".to_string()));
                    assert!(rows[0]["source_ref"].is_null());
                }
            }
        }
    }
}

#[test]
fn list_history_returns_committed_and_reverted() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let first_path = home.join("first.json");
        let second_path = home.join("second.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        write_file(
            &first_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-02-01","amount":-10.00,"currency":"USD","description":"FIRST"}
]"#,
        );
        write_file(
            &second_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-02-02","amount":-20.00,"currency":"USD","description":"SECOND"}
]"#,
        );

        let first_import = run_import(&home, Some(&first_path), false, None);
        assert!(first_import.is_ok());
        let second_import = run_import(&home, Some(&second_path), false, None);
        assert!(second_import.is_ok());

        let mut first_import_id = None;
        let mut second_import_id = None;

        if let Ok(success) = first_import {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                first_import_id = extract_import_id(&value);
            }
        }
        if let Ok(success) = second_import {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                second_import_id = extract_import_id(&value);
            }
        }

        assert!(first_import_id.is_some());
        assert!(second_import_id.is_some());

        if let Some(first_id) = first_import_id.as_deref() {
            let undo_result = run_import_undo(&home, first_id);
            assert!(undo_result.is_ok());
        }

        let list_result = run_import_list(&home);
        assert!(list_result.is_ok());
        if let Ok(success) = list_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["command"], Value::String("import list".to_string()));
                assert!(value["data"]["rows"].is_array());
                if let Some(rows) = value["data"]["rows"].as_array() {
                    assert_eq!(rows.len(), 2);
                    for row in rows {
                        assert!(row["import_id"].is_string());
                        assert!(row["status"].is_string());
                        assert!(row["created_at"].is_string());
                        assert!(row["rows_read"].is_i64());
                        assert!(row["rows_valid"].is_i64());
                        assert!(row["rows_invalid"].is_i64());
                        assert!(row["inserted"].is_i64());
                        assert!(row["deduped"].is_i64());
                    }

                    let mut saw_reverted = false;
                    let mut saw_committed = false;

                    for row in rows {
                        let row_import_id = row["import_id"].as_str().unwrap_or_default();
                        let status = row["status"].as_str().unwrap_or_default();

                        if let Some(first_id) = &first_import_id
                            && row_import_id == first_id
                            && status == "reverted"
                        {
                            saw_reverted = true;
                        }

                        if let Some(second_id) = &second_import_id
                            && row_import_id == second_id
                            && status == "committed"
                        {
                            saw_committed = true;
                        }
                    }

                    assert!(saw_reverted);
                    assert!(saw_committed);
                }
            }
        }
    }
}

#[test]
fn undo_unknown_import_returns_error() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let result = run_import_undo(&home, "imp_missing");
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "import_id_not_found");
            assert!(!error.recovery_steps.is_empty());
        }
    }
}

#[test]
fn undo_already_reverted_returns_error() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("undo-twice.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-02-10","amount":-9.00,"currency":"USD","description":"UNDO-TWICE"}
]"#,
        );

        let run_result = run_import(&home, Some(&source_path), false, None);
        assert!(run_result.is_ok());
        let mut import_id = None;
        if let Ok(success) = run_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                import_id = extract_import_id(&value);
            }
        }

        assert!(import_id.is_some());
        if let Some(id) = import_id.as_deref() {
            let first_undo = run_import_undo(&home, id);
            assert!(first_undo.is_ok());

            let second_undo = run_import_undo(&home, id);
            assert!(second_undo.is_err());
            if let Err(error) = second_undo {
                assert_eq!(error.code, "import_already_reverted");
            }
        }

        let db_path = home.join("ledger.db");
        assert_eq!(
            query_count(
                &db_path,
                "SELECT COUNT(*) FROM internal_transactions WHERE description = 'UNDO-TWICE'"
            ),
            0
        );
    }
}

#[test]
fn undo_promotes_next_candidate() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let first_path = home.join("winner.json");
        let second_path = home.join("candidate.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        write_file(
            &first_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-02-20","amount":-31.00,"currency":"USD","description":"PROMOTE-ME"}
]"#,
        );
        write_file(
            &second_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-02-28","account_key":"chase_checking_1234","posted_at":"2026-02-20","amount":-31.00,"currency":"USD","description":"PROMOTE-ME"}
]"#,
        );

        let first_result = run_import(&home, Some(&first_path), false, None);
        assert!(first_result.is_ok());
        let second_result = run_import(&home, Some(&second_path), false, None);
        assert!(second_result.is_ok());

        let mut first_import_id = None;
        let mut second_import_id = None;
        if let Ok(success) = first_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                first_import_id = extract_import_id(&value);
            }
        }
        if let Ok(success) = second_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                second_import_id = extract_import_id(&value);
                assert_eq!(value["data"]["summary"]["inserted"], Value::from(0));
                assert_eq!(value["data"]["duplicate_summary"]["total"], Value::from(1));
            }
        }

        assert!(first_import_id.is_some());
        assert!(second_import_id.is_some());

        let db_path = home.join("ledger.db");
        assert_eq!(
            query_count(
                &db_path,
                "SELECT COUNT(*) FROM internal_transactions WHERE description = 'PROMOTE-ME'"
            ),
            1
        );

        if let Some(first_id) = first_import_id.as_deref() {
            let undo_result = run_import_undo(&home, first_id);
            assert!(undo_result.is_ok());
            if let Ok(success) = undo_result {
                let payload = serde_json::to_value(success);
                assert!(payload.is_ok());
                if let Ok(value) = payload {
                    assert_eq!(value["command"], Value::String("import undo".to_string()));
                    assert_eq!(value["data"]["summary"]["rows_reverted"], Value::from(1));
                    assert_eq!(value["data"]["summary"]["rows_promoted"], Value::from(1));
                }
            }
        }

        assert_eq!(
            query_count(
                &db_path,
                "SELECT COUNT(*) FROM internal_transactions WHERE description = 'PROMOTE-ME'"
            ),
            1
        );

        let surviving_import_id = query_optional_string(
            &db_path,
            "SELECT import_id FROM internal_transactions WHERE description = 'PROMOTE-ME' LIMIT 1",
        );
        assert_eq!(surviving_import_id, second_import_id);
        assert_eq!(
            query_count(
                &db_path,
                "SELECT COUNT(*) FROM internal_transaction_dedupe_candidates WHERE promoted_txn_id IS NOT NULL"
            ),
            1
        );
    }
}

#[test]
fn undo_atomicity_rolls_back_when_promotion_fails() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let first_path = home.join("winner-atomic.json");
        let second_path = home.join("candidate-atomic.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        write_file(
            &first_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-02-25","amount":-44.00,"currency":"USD","description":"ATOMIC-KEY"}
]"#,
        );
        write_file(
            &second_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-02-28","account_key":"chase_checking_1234","posted_at":"2026-02-25","amount":-44.00,"currency":"USD","description":"ATOMIC-KEY"}
]"#,
        );

        let first_result = run_import(&home, Some(&first_path), false, None);
        assert!(first_result.is_ok());
        let second_result = run_import(&home, Some(&second_path), false, None);
        assert!(second_result.is_ok());

        let mut first_import_id = None;
        if let Ok(success) = first_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                first_import_id = extract_import_id(&value);
            }
        }
        assert!(first_import_id.is_some());

        let db_path = home.join("ledger.db");
        assert!(execute_sql(
            &db_path,
            "CREATE TRIGGER fail_promotion_before_insert
             BEFORE INSERT ON internal_transactions
             BEGIN
               SELECT RAISE(ABORT, 'forced_undo_failure');
             END;"
        ));

        if let Some(first_id) = first_import_id.as_deref() {
            let undo_result = run_import_undo(&home, first_id);
            assert!(undo_result.is_err());
            if let Err(error) = undo_result {
                assert_eq!(error.code, "ledger_init_failed");
                assert!(error.message.contains("forced_undo_failure"));
            }
        }

        assert_eq!(
            query_count(
                &db_path,
                "SELECT COUNT(*) FROM internal_transactions WHERE description = 'ATOMIC-KEY'"
            ),
            1
        );
        assert_eq!(
            query_count(
                &db_path,
                "SELECT COUNT(*) FROM internal_import_runs WHERE status = 'committed'"
            ),
            2
        );
    }
}

#[test]
fn import_duplicates_not_found_returns_error() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let result = run_import_duplicates(&home, "imp_missing");
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "import_id_not_found");
        }
    }
}

#[test]
fn import_duplicates_empty_result_returns_deterministic_payload() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("duplicates-empty.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"statement_id":"chase_checking_1234_2026-07-31","account_key":"chase_checking_1234","posted_at":"2026-07-01","amount":-5.00,"currency":"USD","description":"NO-DUPE"}
]"#,
        );

        let imported = run_import(&home, Some(&source_path), false, None);
        assert!(imported.is_ok());
        let mut import_id = None;
        if let Ok(success) = imported {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                import_id = extract_import_id(&value);
            }
        }
        assert!(import_id.is_some());

        if let Some(id) = import_id.as_deref() {
            let duplicates = run_import_duplicates(&home, id);
            assert!(duplicates.is_ok());
            if let Ok(success) = duplicates {
                let payload = serde_json::to_value(success);
                assert!(payload.is_ok());
                if let Ok(value) = payload {
                    assert_eq!(
                        value["command"],
                        Value::String("import duplicates".to_string())
                    );
                    assert_eq!(value["data"]["import_id"], Value::String(id.to_string()));
                    assert_eq!(value["data"]["total"], Value::from(0));
                    assert_eq!(value["data"]["rows"], Value::Array(Vec::new()));
                }
            }
        }
    }
}

#[test]
fn duplicates_report_live_and_historical_match_pointers_after_promotion() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let winner_path = home.join("winner-live-pointer.json");
        let candidate_path = home.join("candidate-live-pointer.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        write_file(
            &winner_path,
            r#"[
  {"statement_id":"acct_live_1_2026-11-30","account_key":"acct_live_1","posted_at":"2026-11-10","amount":-31.00,"currency":"USD","description":"LIVE-POINTER"}
]"#,
        );
        write_file(
            &candidate_path,
            r#"[
  {"statement_id":"acct_live_1_2026-12-31","account_key":"acct_live_1","posted_at":"2026-11-10","amount":-31.00,"currency":"USD","description":"LIVE-POINTER"}
]"#,
        );

        let first_result = run_import(&home, Some(&winner_path), false, None);
        assert!(first_result.is_ok());
        let second_result = run_import(&home, Some(&candidate_path), false, None);
        assert!(second_result.is_ok());

        let mut first_import_id = None;
        let mut second_import_id = None;
        if let Ok(success) = first_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                first_import_id = extract_import_id(&value);
            }
        }
        if let Ok(success) = second_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                second_import_id = extract_import_id(&value);
            }
        }

        assert!(first_import_id.is_some());
        assert!(second_import_id.is_some());
        let db_path = home.join("ledger.db");

        let first_live_txn_id = query_optional_string(
            &db_path,
            "SELECT txn_id
             FROM internal_transactions
             WHERE description = 'LIVE-POINTER'
             ORDER BY txn_id ASC
             LIMIT 1",
        );
        assert!(first_live_txn_id.is_some());

        if let Some(second_id) = second_import_id.as_deref() {
            let before_duplicates = run_import_duplicates(&home, second_id);
            assert!(before_duplicates.is_ok());
            if let Ok(success) = before_duplicates {
                let payload = serde_json::to_value(success);
                assert!(payload.is_ok());
                if let Ok(value) = payload {
                    assert_eq!(value["data"]["total"], Value::from(1));
                    assert_eq!(
                        value["data"]["rows"][0]["matched_txn_id"],
                        Value::String(first_live_txn_id.clone().unwrap_or_default())
                    );
                    assert_eq!(
                        value["data"]["rows"][0]["matched_import_id_at_dedupe"],
                        Value::String(first_import_id.clone().unwrap_or_default())
                    );
                    assert_eq!(
                        value["data"]["rows"][0]["matched_txn_id_at_dedupe"],
                        Value::String(first_live_txn_id.clone().unwrap_or_default())
                    );
                }
            }
        }

        if let Some(first_id) = first_import_id.as_deref() {
            let undo_result = run_import_undo(&home, first_id);
            assert!(undo_result.is_ok());
        }

        let second_live_txn_id = query_optional_string(
            &db_path,
            &format!(
                "SELECT txn_id
                 FROM internal_transactions
                 WHERE import_id = '{}'
                 ORDER BY txn_id ASC
                 LIMIT 1",
                second_import_id.clone().unwrap_or_default()
            ),
        );
        assert!(second_live_txn_id.is_some());

        if let Some(second_id) = second_import_id.as_deref() {
            let after_duplicates = run_import_duplicates(&home, second_id);
            assert!(after_duplicates.is_ok());
            if let Ok(success) = after_duplicates {
                let payload = serde_json::to_value(success);
                assert!(payload.is_ok());
                if let Ok(value) = payload {
                    assert_eq!(
                        value["data"]["rows"][0]["matched_txn_id"],
                        Value::String(second_live_txn_id.clone().unwrap_or_default())
                    );
                    assert_eq!(
                        value["data"]["rows"][0]["matched_import_id"],
                        Value::String(second_id.to_string())
                    );
                    assert_eq!(
                        value["data"]["rows"][0]["matched_txn_id_at_dedupe"],
                        Value::String(first_live_txn_id.unwrap_or_default())
                    );
                    assert_eq!(
                        value["data"]["rows"][0]["matched_import_id_at_dedupe"],
                        Value::String(first_import_id.unwrap_or_default())
                    );
                }
            }
        }
    }
}
