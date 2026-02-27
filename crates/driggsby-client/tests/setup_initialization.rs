use std::fs;
use std::path::Path;

use driggsby_client::setup::ensure_initialized_at;
use driggsby_client::state::map_io_error;
use rusqlite::Connection;
use tempfile::tempdir;

fn object_exists(connection: &Connection, object_type: &str, object_name: &str) -> bool {
    let query = "SELECT 1 FROM sqlite_master WHERE type = ?1 AND name = ?2";
    let statement = connection.prepare(query);
    if statement.is_err() {
        return false;
    }

    if let Ok(mut stmt) = statement {
        let mut rows = stmt.query([object_type, object_name]);
        if rows.is_err() {
            return false;
        }

        if let Ok(ref mut row_cursor) = rows {
            let next_row = row_cursor.next();
            if let Ok(row) = next_row {
                return row.is_some();
            }
        }
    }

    false
}

fn meta_value(connection: &Connection, key: &str) -> Option<String> {
    let query = "SELECT value FROM internal_meta WHERE key = ?1 LIMIT 1";
    let statement = connection.prepare(query).ok()?;
    let mut stmt = statement;
    let rows = stmt.query([key]).ok()?;
    let mut row_cursor = rows;
    let row = row_cursor.next().ok()??;
    row.get::<_, String>(0).ok()
}

fn user_version(connection: &Connection) -> Option<i64> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .ok()
}

fn object_has_column(connection: &Connection, table_or_view: &str, column_name: &str) -> bool {
    let sql = format!("PRAGMA table_info({table_or_view})");
    let statement = connection.prepare(&sql);
    if statement.is_err() {
        return false;
    }
    if let Ok(mut stmt) = statement {
        let rows = stmt.query_map([], |row| row.get::<_, String>(1));
        if rows.is_err() {
            return false;
        }
        if let Ok(iter) = rows {
            for maybe_name in iter {
                if let Ok(name) = maybe_name
                    && name == column_name
                {
                    return true;
                }
            }
        }
    }
    false
}

#[test]
fn setup_creates_ledger_db_at_home_override() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");

        let context = ensure_initialized_at(&home);
        assert!(context.is_ok());
        if let Ok(setup_context) = context {
            assert!(setup_context.db_path.ends_with("ledger.db"));
            assert!(setup_context.readonly_uri.contains("mode=ro"));
            assert!(home.join("ledger.db").exists());
        }
    }
}

#[test]
fn setup_is_idempotent_for_existing_ledger() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");

        let first = ensure_initialized_at(&home);
        assert!(first.is_ok());
        let second = ensure_initialized_at(&home);
        assert!(second.is_ok());

        if let (Ok(first_context), Ok(second_context)) = (first, second) {
            assert_eq!(first_context.db_path, second_context.db_path);
            assert_eq!(first_context.schema_version, second_context.schema_version);
        }
    }
}

#[test]
fn pending_migration_applies_exactly_once() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");

        let first = ensure_initialized_at(&home);
        assert!(first.is_ok());

        if let Ok(first_context) = first {
            let connection = Connection::open(&first_context.db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                let first_version = user_version(&conn);
                assert_eq!(first_version, Some(6));
            }
        }

        let second = ensure_initialized_at(&home);
        assert!(second.is_ok());
        if let Ok(second_context) = second {
            let connection = Connection::open(&second_context.db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                let second_version = user_version(&conn);
                assert_eq!(second_version, Some(6));
            }
        }
    }
}

#[test]
fn setup_creates_accounts_metadata_objects_and_columns() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");

        let context = ensure_initialized_at(&home);
        assert!(context.is_ok());
        if let Ok(setup_context) = context {
            let connection = Connection::open(&setup_context.db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                assert!(object_exists(&conn, "table", "internal_accounts"));
                assert!(object_exists(
                    &conn,
                    "table",
                    "internal_import_account_stats"
                ));
                assert!(object_has_column(&conn, "internal_accounts", "account_key"));
                assert!(object_has_column(
                    &conn,
                    "internal_accounts",
                    "account_type"
                ));
                assert!(object_has_column(
                    &conn,
                    "internal_import_account_stats",
                    "rows_read"
                ));
                assert!(object_has_column(
                    &conn,
                    "internal_import_account_stats",
                    "inserted"
                ));
                assert!(object_has_column(
                    &conn,
                    "internal_import_account_stats",
                    "deduped"
                ));
                assert!(object_exists(
                    &conn,
                    "index",
                    "idx_internal_import_account_stats_import_id"
                ));
                assert!(object_exists(
                    &conn,
                    "index",
                    "idx_internal_import_account_stats_account_key"
                ));
                assert!(object_has_column(&conn, "v1_transactions", "account_type"));
                assert!(object_has_column(&conn, "v1_accounts", "account_type"));
            }
        }
    }
}

#[test]
fn setup_repairs_missing_safe_view() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");

        let context = ensure_initialized_at(&home);
        assert!(context.is_ok());
        if let Ok(setup_context) = context {
            let connection = Connection::open(&setup_context.db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                let drop_result = conn.execute_batch("DROP VIEW v1_accounts;");
                assert!(drop_result.is_ok());
            }

            let repaired = ensure_initialized_at(&home);
            assert!(repaired.is_ok());

            let verify_connection = Connection::open(&setup_context.db_path);
            assert!(verify_connection.is_ok());
            if let Ok(conn) = verify_connection {
                assert!(object_exists(&conn, "view", "v1_accounts"));
            }
        }
    }
}

#[test]
fn setup_fails_when_required_view_sql_is_tampered() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");

        let context = ensure_initialized_at(&home);
        assert!(context.is_ok());
        if let Ok(setup_context) = context {
            let connection = Connection::open(&setup_context.db_path);
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

            let failed = ensure_initialized_at(&home);
            assert!(failed.is_err());
            if let Err(error) = failed {
                assert_eq!(error.code, "ledger_corrupt");
            }
        }
    }
}

#[test]
fn setup_repairs_missing_safe_index() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");

        let context = ensure_initialized_at(&home);
        assert!(context.is_ok());
        if let Ok(setup_context) = context {
            let connection = Connection::open(&setup_context.db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                let drop_result =
                    conn.execute_batch("DROP INDEX idx_internal_import_runs_created_at_desc;");
                assert!(drop_result.is_ok());
            }

            let repaired = ensure_initialized_at(&home);
            assert!(repaired.is_ok());

            let verify_connection = Connection::open(&setup_context.db_path);
            assert!(verify_connection.is_ok());
            if let Ok(conn) = verify_connection {
                assert!(object_exists(
                    &conn,
                    "index",
                    "idx_internal_import_runs_created_at_desc"
                ));
            }
        }
    }
}

#[test]
fn setup_repairs_missing_safe_meta_key() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");

        let context = ensure_initialized_at(&home);
        assert!(context.is_ok());
        if let Ok(setup_context) = context {
            let connection = Connection::open(&setup_context.db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                let delete_result = conn.execute(
                    "DELETE FROM internal_meta WHERE key = ?1",
                    ["public_views_version"],
                );
                assert!(delete_result.is_ok());
            }

            let repaired = ensure_initialized_at(&home);
            assert!(repaired.is_ok());

            let verify_connection = Connection::open(&setup_context.db_path);
            assert!(verify_connection.is_ok());
            if let Ok(conn) = verify_connection {
                let value = meta_value(&conn, "public_views_version");
                assert_eq!(value, Some("v1".to_string()));
            }
        }
    }
}

#[test]
fn setup_fails_when_core_table_missing() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");

        let context = ensure_initialized_at(&home);
        assert!(context.is_ok());
        if let Ok(setup_context) = context {
            let connection = Connection::open(&setup_context.db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                let drop_result = conn.execute_batch("DROP TABLE internal_transactions;");
                assert!(drop_result.is_ok());
            }

            let failed = ensure_initialized_at(&home);
            assert!(failed.is_err());
            if let Err(error) = failed {
                assert_eq!(error.code, "ledger_corrupt");
            }
        }
    }
}

#[test]
fn setup_maps_locked_database_to_ledger_locked() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");

        let context = ensure_initialized_at(&home);
        assert!(context.is_ok());
        if let Ok(setup_context) = context {
            let connection = Connection::open(&setup_context.db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                let begin_lock = conn.execute_batch("BEGIN EXCLUSIVE;");
                assert!(begin_lock.is_ok());

                let locked_error = ensure_initialized_at(&home);
                assert!(locked_error.is_err());
                if let Err(error) = locked_error {
                    assert_eq!(error.code, "ledger_locked");
                }

                let rollback = conn.execute_batch("ROLLBACK;");
                assert!(rollback.is_ok());
            }
        }
    }
}

#[test]
fn setup_maps_corrupt_database_to_ledger_corrupt() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let db_path = home.join("ledger.db");
        let write_file = fs::write(&db_path, "not-a-sqlite-database");
        assert!(write_file.is_ok());

        let result = ensure_initialized_at(&home);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "ledger_corrupt");
        }
    }
}

#[test]
fn setup_maps_migration_failure_to_migration_failed() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let home = temp_dir.path().join("ledger-home");
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let db_path = home.join("ledger.db");
        let connection = Connection::open(&db_path);
        assert!(connection.is_ok());
        if let Ok(conn) = connection {
            let create_conflict = conn.execute_batch("CREATE TABLE v1_transactions(id TEXT);");
            assert!(create_conflict.is_ok());
        }

        let result = ensure_initialized_at(&home);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "migration_failed");
        }
    }
}

#[test]
fn io_permission_denied_maps_to_ledger_init_permission_denied() {
    let io_error = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    let mapped = map_io_error(Path::new("/tmp/ledger-home"), &io_error);
    assert_eq!(mapped.code, "ledger_init_permission_denied");
}

#[test]
fn setup_maps_unexpected_path_error_to_ledger_init_failed() {
    let temp = tempdir();
    assert!(temp.is_ok());
    if let Ok(temp_dir) = temp {
        let file_as_home = temp_dir.path().join("not-a-dir");
        let write_file = fs::write(&file_as_home, "content");
        assert!(write_file.is_ok());

        let result = ensure_initialized_at(&file_as_home);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "ledger_init_failed");
        }
    }
}
