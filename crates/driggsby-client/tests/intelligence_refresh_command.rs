use std::fs;
use std::path::{Path, PathBuf};

use driggsby_client::commands::import::{self, ImportRunOptions};
use driggsby_client::commands::intelligence::{self, IntelligenceRefreshOptions};
use rusqlite::Connection;
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

fn query_count(db_path: &Path, sql: &str) -> i64 {
    let connection = Connection::open(db_path);
    assert!(connection.is_ok());
    if let Ok(conn) = connection {
        return conn
            .query_row(sql, [], |row| row.get::<_, i64>(0))
            .unwrap_or(0);
    }
    0
}

#[test]
fn intelligence_refresh_is_atomic_on_failure() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let source_path = home.join("refresh-atomic.json");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());
        write_file(
            &source_path,
            r#"[
  {"statement_id":"stmt_refresh_1","account_key":"acct_refresh","posted_at":"2026-01-03","amount":-22.00,"currency":"USD","description":"GROCERIES","merchant":"Fresh Mart"},
  {"statement_id":"stmt_refresh_1","account_key":"acct_refresh","posted_at":"2026-01-10","amount":-21.50,"currency":"USD","description":"GROCERIES","merchant":"Fresh Mart"},
  {"statement_id":"stmt_refresh_1","account_key":"acct_refresh","posted_at":"2026-01-17","amount":-22.25,"currency":"USD","description":"GROCERIES","merchant":"Fresh Mart"},
  {"statement_id":"stmt_refresh_1","account_key":"acct_refresh","posted_at":"2026-01-24","amount":-22.10,"currency":"USD","description":"GROCERIES","merchant":"Fresh Mart"},
  {"statement_id":"stmt_refresh_1","account_key":"acct_refresh","posted_at":"2026-01-31","amount":-21.95,"currency":"USD","description":"GROCERIES","merchant":"Fresh Mart"},
  {"statement_id":"stmt_refresh_1","account_key":"acct_refresh","posted_at":"2026-02-07","amount":-318.40,"currency":"USD","description":"GROCERIES","merchant":"Fresh Mart"},
  {"statement_id":"stmt_refresh_1","account_key":"acct_refresh","posted_at":"2026-01-05","amount":-15.99,"currency":"USD","description":"NETFLIX MONTHLY","merchant":"Netflix"},
  {"statement_id":"stmt_refresh_1","account_key":"acct_refresh","posted_at":"2026-02-05","amount":-15.99,"currency":"USD","description":"NETFLIX MONTHLY","merchant":"Netflix"},
  {"statement_id":"stmt_refresh_1","account_key":"acct_refresh","posted_at":"2026-03-05","amount":-15.99,"currency":"USD","description":"NETFLIX MONTHLY","merchant":"Netflix"}
]"#,
        );
        run_import(&home, &source_path);

        let db_path = home.join("ledger.db");
        let before_recurring = query_count(&db_path, "SELECT COUNT(*) FROM v1_recurring");
        let before_anomalies = query_count(&db_path, "SELECT COUNT(*) FROM v1_anomalies");
        assert!(before_recurring > 0);
        assert!(before_anomalies > 0);

        let connection = Connection::open(&db_path);
        assert!(connection.is_ok());
        if let Ok(conn) = connection {
            let trigger = conn.execute_batch(
                "CREATE TRIGGER fail_anomaly_insert
                 BEFORE INSERT ON internal_anomalies_materialized
                 BEGIN
                   SELECT RAISE(ABORT, 'forced_refresh_failure');
                 END;",
            );
            assert!(trigger.is_ok());
        }

        let refresh = intelligence::refresh_with_options(IntelligenceRefreshOptions {
            home_override: Some(&home),
        });
        assert!(refresh.is_err());
        if let Err(error) = refresh {
            assert_eq!(error.code, "ledger_init_failed");
            assert!(error.message.contains("forced_refresh_failure"));
        }

        let after_recurring = query_count(&db_path, "SELECT COUNT(*) FROM v1_recurring");
        let after_anomalies = query_count(&db_path, "SELECT COUNT(*) FROM v1_anomalies");
        assert_eq!(before_recurring, after_recurring);
        assert_eq!(before_anomalies, after_anomalies);
    }
}
