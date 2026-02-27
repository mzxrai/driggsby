use std::fs;
use std::path::{Path, PathBuf};

use driggsby_client::commands::import;
use driggsby_client::commands::import::ImportRunOptions;
use driggsby_client::commands::sql;
use driggsby_client::commands::sql::SqlQueryOptions;
use rusqlite::Connection;
use serde_json::Value;
use tempfile::tempdir;

fn temp_home() -> std::io::Result<(tempfile::TempDir, PathBuf)> {
    let dir = tempdir()?;
    let home = dir.path().join("ledger-home");
    Ok((dir, home))
}

fn write_file(path: &Path, body: &str) {
    let result = fs::write(path, body);
    assert!(result.is_ok());
}

fn run_import(home: &Path, path: &Path) {
    let result = import::run_with_options(ImportRunOptions {
        path: Some(path.display().to_string()),
        dry_run: false,
        home_override: Some(home),
        stdin_override: None,
    });
    assert!(result.is_ok());
}

fn run_sql(
    home: &Path,
    query: Option<String>,
    file: Option<String>,
    stdin_override: Option<&str>,
) -> driggsby_client::ClientResult<driggsby_client::SuccessEnvelope> {
    sql::run_with_options(SqlQueryOptions {
        query,
        file,
        home_override: Some(home),
        stdin_override: stdin_override.map(std::string::ToString::to_string),
        max_rows: None,
    })
}

#[test]
fn sql_allows_select_from_public_views() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let source_path = home.join("sql-import.json");
        write_file(
            &source_path,
            r#"[
  {"statement_id":"acct_sql_1_2026-01-31","account_key":"acct_sql_1","posted_at":"2026-01-01","amount":-4.00,"currency":"USD","description":"COFFEE"},
  {"statement_id":"acct_sql_1_2026-01-31","account_key":"acct_sql_1","posted_at":"2026-01-02","amount":7.00,"currency":"USD","description":"REFUND"}
]"#,
        );
        run_import(&home, &source_path);

        let result = run_sql(
            &home,
            Some("SELECT account_key, txn_count FROM v1_accounts".to_string()),
            None,
            None,
        );
        assert!(result.is_ok());
        if let Ok(success) = result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["command"], Value::String("db sql".to_string()));
                assert_eq!(value["data"]["row_count"], Value::from(1));
                assert_eq!(value["data"]["truncated"], Value::Bool(false));
                assert_eq!(value["data"]["source"], Value::String("inline".to_string()));
                assert!(value["data"]["columns"].is_array());
                assert!(value["data"]["rows"].is_array());
            }
        }
    }
}

#[test]
fn sql_rejects_multi_statement_queries() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let result = run_sql(&home, Some("SELECT 1; SELECT 2;".to_string()), None, None);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("single statement"));
        }
    }
}

#[test]
fn sql_rejects_non_read_only_statements() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let result = run_sql(
            &home,
            Some("UPDATE internal_meta SET value = 'x' WHERE key = 'schema_version'".to_string()),
            None,
            None,
        );
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("read-only"));
        }
    }
}

#[test]
fn sql_rejects_internal_and_admin_surfaces() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let internal = run_sql(
            &home,
            Some("SELECT key FROM internal_meta".to_string()),
            None,
            None,
        );
        assert!(internal.is_err());
        if let Err(error) = internal {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("public v1_* views"));
        }

        let pragma = run_sql(&home, Some("PRAGMA user_version".to_string()), None, None);
        assert!(pragma.is_err());
        if let Err(error) = pragma {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("public v1_* views"));
        }

        let attach = run_sql(
            &home,
            Some("ATTACH DATABASE ':memory:' AS extra".to_string()),
            None,
            None,
        );
        assert!(attach.is_err());
        if let Err(error) = attach {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("read-only"));
        }

        let function = run_sql(
            &home,
            Some("SELECT sqlite_version()".to_string()),
            None,
            None,
        );
        assert!(function.is_err());
        if let Err(error) = function {
            assert_eq!(error.code, "invalid_argument");
            assert!(
                error.message.contains("public v1_* views")
                    || error.message.contains("SQL query could not be executed")
                    || error.message.contains("Malformed SQL query")
            );
        }
    }
}

#[test]
fn sql_user_execution_errors_use_invalid_argument_contract() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let load_extension = run_sql(
            &home,
            Some("SELECT load_extension('does_not_exist')".to_string()),
            None,
            None,
        );
        assert!(load_extension.is_err());
        if let Err(error) = load_extension {
            assert_eq!(error.code, "invalid_argument");
            assert!(
                error.message.contains("public v1_* views")
                    || error.message.contains("SQL query could not be executed")
                    || error.message.contains("Malformed SQL query")
            );
        }

        let placeholder = run_sql(&home, Some("SELECT ?1".to_string()), None, None);
        assert!(placeholder.is_err());
        if let Err(error) = placeholder {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("SQL query could not be executed"));
        }
    }
}

#[test]
fn sql_source_conflicts_and_missing_source_are_rejected() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let conflict = run_sql(
            &home,
            Some("SELECT 1".to_string()),
            Some("query.sql".to_string()),
            None,
        );
        assert!(conflict.is_err());
        if let Err(error) = conflict {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("exactly one SQL source"));
        }

        let missing = run_sql(&home, None, None, None);
        assert!(missing.is_err());
        if let Err(error) = missing {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("exactly one SQL source"));
        }
    }
}

#[test]
fn sql_file_and_stdin_sources_are_supported() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let sql_file = home.join("query.sql");
        write_file(&sql_file, "SELECT 1 AS one");

        let file_result = run_sql(&home, None, Some(sql_file.display().to_string()), None);
        assert!(file_result.is_ok());
        if let Ok(success) = file_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["source"], Value::String("file".to_string()));
            }
        }

        let stdin_result = run_sql(&home, None, Some("-".to_string()), Some("SELECT 1 AS one"));
        assert!(stdin_result.is_ok());
        if let Ok(success) = stdin_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["source"], Value::String("stdin".to_string()));
            }
        }
    }
}

#[test]
fn failed_write_attempt_does_not_modify_ledger_data() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let source_path = home.join("immutability-import.json");
        write_file(
            &source_path,
            r#"[
  {"statement_id":"acct_sql_immut_1_2026-01-31","account_key":"acct_sql_immut_1","posted_at":"2026-01-01","amount":-4.00,"currency":"USD","description":"COFFEE"}
]"#,
        );
        run_import(&home, &source_path);

        let write_attempt = run_sql(
            &home,
            Some("UPDATE internal_transactions SET amount = 999 WHERE account_key = 'acct_sql_immut_1'".to_string()),
            None,
            None,
        );
        assert!(write_attempt.is_err());

        let verify = run_sql(
            &home,
            Some(
                "SELECT amount FROM v1_transactions WHERE account_key = 'acct_sql_immut_1'"
                    .to_string(),
            ),
            None,
            None,
        );
        assert!(verify.is_ok());
        if let Ok(success) = verify {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["row_count"], Value::from(1));
                assert_eq!(value["data"]["rows"][0][0], Value::from(-4.0));
            }
        }
    }
}

#[test]
fn sql_column_metadata_is_derived_from_returned_values() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let null_alias = run_sql(
            &home,
            Some("SELECT NULL AS account_key".to_string()),
            None,
            None,
        );
        assert!(null_alias.is_ok());
        if let Ok(success) = null_alias {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(
                    value["data"]["columns"][0]["name"],
                    Value::String("account_key".to_string())
                );
                assert_eq!(
                    value["data"]["columns"][0]["type"],
                    Value::String("null".to_string())
                );
                assert_eq!(value["data"]["columns"][0]["nullable"], Value::Bool(true));
            }
        }

        let number_alias = run_sql(&home, Some("SELECT 1 AS posted_at".to_string()), None, None);
        assert!(number_alias.is_ok());
        if let Ok(success) = number_alias {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(
                    value["data"]["columns"][0]["type"],
                    Value::String("integer".to_string())
                );
                assert_eq!(value["data"]["columns"][0]["nullable"], Value::Bool(false));
            }
        }
    }
}

#[test]
fn sql_rejects_tampered_required_view_definitions() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let source_path = home.join("tamper-import.json");
        write_file(
            &source_path,
            r#"[
  {"statement_id":"acct_sql_tamper_1_2026-01-31","account_key":"acct_sql_tamper_1","posted_at":"2026-01-01","amount":-4.00,"currency":"USD","description":"COFFEE"}
]"#,
        );
        run_import(&home, &source_path);

        let db_path = home.join("ledger.db");
        let connection = Connection::open(&db_path);
        assert!(connection.is_ok());
        if let Ok(conn) = connection {
            let tamper_result = conn.execute_batch(
                "DROP VIEW v1_transactions;
                 CREATE VIEW v1_transactions AS
                 SELECT key AS txn_id, value AS description
                 FROM internal_meta;",
            );
            assert!(tamper_result.is_ok());
        }

        let result = run_sql(
            &home,
            Some("SELECT txn_id, description FROM v1_transactions".to_string()),
            None,
            None,
        );
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "ledger_corrupt");
        }
    }
}
