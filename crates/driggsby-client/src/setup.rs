use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::commands::common::public_view_contracts;
use crate::contracts::types::{DataRange, PublicView};
use crate::migrations::{
    REQUIRED_INDEX_NAMES, REQUIRED_META_KEYS, REQUIRED_VIEW_NAMES, run_pending,
    safe_repair_statement,
};
use crate::state::{
    ensure_ledger_directory, ledger_db_path, map_sqlite_error, open_connection, resolve_ledger_home,
};
use crate::{ClientError, ClientResult};

const INTERNAL_META_COLUMNS: [&str; 2] = ["key", "value"];
const INTERNAL_IMPORT_RUNS_COLUMNS: [&str; 12] = [
    "import_id",
    "status",
    "created_at",
    "committed_at",
    "reverted_at",
    "rows_read",
    "rows_valid",
    "rows_invalid",
    "inserted",
    "deduped",
    "source_kind",
    "source_ref",
];
const INTERNAL_TRANSACTIONS_COLUMNS: [&str; 12] = [
    "txn_id",
    "import_id",
    "statement_id",
    "dedupe_scope_id",
    "account_key",
    "posted_at",
    "amount",
    "currency",
    "description",
    "external_id",
    "merchant",
    "category",
];
const INTERNAL_ACCOUNTS_COLUMNS: [&str; 4] =
    ["account_key", "account_type", "created_at", "updated_at"];
const INTERNAL_IMPORT_ACCOUNT_STATS_COLUMNS: [&str; 5] = [
    "import_id",
    "account_key",
    "rows_read",
    "inserted",
    "deduped",
];
const INTERNAL_TRANSACTION_DEDUPE_CANDIDATES_COLUMNS: [&str; 20] = [
    "candidate_id",
    "import_id",
    "dedupe_key",
    "statement_id",
    "dedupe_scope_id",
    "account_key",
    "posted_at",
    "amount",
    "currency",
    "description",
    "external_id",
    "merchant",
    "category",
    "source_row_index",
    "dedupe_reason",
    "matched_txn_id",
    "matched_import_id",
    "matched_batch_row_index",
    "created_at",
    "promoted_txn_id",
];
const INTERNAL_RECURRING_COLUMNS: [&str; 3] = ["merchant", "typical_amount", "cadence"];
const INTERNAL_ANOMALIES_COLUMNS: [&str; 3] = ["posted_at", "amount", "reason"];
const EXPECTED_USER_VERSION: i64 = 5;

const REQUIRED_CORE_TABLES: [(&str, &[&str]); 8] = [
    ("internal_meta", &INTERNAL_META_COLUMNS),
    ("internal_import_runs", &INTERNAL_IMPORT_RUNS_COLUMNS),
    ("internal_transactions", &INTERNAL_TRANSACTIONS_COLUMNS),
    ("internal_accounts", &INTERNAL_ACCOUNTS_COLUMNS),
    (
        "internal_import_account_stats",
        &INTERNAL_IMPORT_ACCOUNT_STATS_COLUMNS,
    ),
    (
        "internal_transaction_dedupe_candidates",
        &INTERNAL_TRANSACTION_DEDUPE_CANDIDATES_COLUMNS,
    ),
    (
        "internal_recurring_materialized",
        &INTERNAL_RECURRING_COLUMNS,
    ),
    (
        "internal_anomalies_materialized",
        &INTERNAL_ANOMALIES_COLUMNS,
    ),
];

#[derive(Debug, Clone)]
pub struct SetupContext {
    pub db_path: String,
    pub readonly_uri: String,
    pub schema_version: String,
    pub public_views: Vec<PublicView>,
    pub data_range: DataRange,
}

pub fn ensure_initialized() -> ClientResult<SetupContext> {
    ensure_initialized_with_home_override(None)
}

pub fn ensure_initialized_at(home_override: &Path) -> ClientResult<SetupContext> {
    ensure_initialized_with_home_override(Some(home_override))
}

fn ensure_initialized_with_home_override(
    home_override: Option<&Path>,
) -> ClientResult<SetupContext> {
    let ledger_home = resolve_ledger_home(home_override)?;
    ensure_ledger_directory(&ledger_home)?;

    let db_path = ledger_db_path(&ledger_home);
    let mut connection = open_connection(&db_path)?;

    run_pending(&mut connection).map_err(|error| map_migration_error(&db_path, &error))?;

    verify_core_tables(&connection, &db_path)?;
    repair_safe_objects(&connection, &db_path)?;
    verify_post_repair_objects(&connection, &db_path)?;

    let schema_version = read_schema_version(&connection, &db_path)?;
    let data_range = read_data_range(&connection, &db_path)?;

    let db_path_string = db_path.display().to_string();
    Ok(SetupContext {
        readonly_uri: format!("file:{db_path_string}?mode=ro"),
        db_path: db_path_string,
        schema_version,
        public_views: public_view_contracts(),
        data_range,
    })
}

fn map_migration_error(db_path: &Path, error: &rusqlite_migration::Error) -> ClientError {
    match error {
        rusqlite_migration::Error::RusqliteError { query: _, err } => {
            let mapped = map_sqlite_error(db_path, err);
            if mapped.code == "ledger_locked"
                || mapped.code == "ledger_corrupt"
                || mapped.code == "ledger_init_permission_denied"
            {
                mapped
            } else {
                ClientError::migration_failed(db_path, &error.to_string())
            }
        }
        _ => ClientError::migration_failed(db_path, &error.to_string()),
    }
}

fn verify_core_tables(connection: &Connection, db_path: &Path) -> ClientResult<()> {
    for (table_name, required_columns) in REQUIRED_CORE_TABLES {
        if !sqlite_object_exists(connection, "table", table_name, db_path)? {
            return Err(ClientError::ledger_corrupt(db_path));
        }

        let columns = table_columns(connection, table_name, db_path)?;
        for required_column in required_columns {
            if !columns.iter().any(|column| column == required_column) {
                return Err(ClientError::ledger_corrupt(db_path));
            }
        }
    }

    Ok(())
}

fn repair_safe_objects(connection: &Connection, db_path: &Path) -> ClientResult<()> {
    // Meta repair is intentionally insert-only: missing required keys are restored,
    // while unexpected value drift is treated as risky and rejected in verification.
    for (meta_key, default_value) in REQUIRED_META_KEYS {
        connection
            .execute(
                "INSERT OR IGNORE INTO internal_meta (key, value) VALUES (?1, ?2)",
                params![meta_key, default_value],
            )
            .map_err(|error| map_sqlite_error(db_path, &error))?;
    }

    for view_name in REQUIRED_VIEW_NAMES {
        if !sqlite_object_exists(connection, "view", view_name, db_path)? {
            let sql = safe_repair_statement(view_name).ok_or_else(|| {
                ClientError::ledger_init_failed(db_path, "Missing canonical SQL for view repair.")
            })?;
            connection
                .execute_batch(&sql)
                .map_err(|error| map_sqlite_error(db_path, &error))?;
        }
    }

    for index_name in REQUIRED_INDEX_NAMES {
        if !sqlite_object_exists(connection, "index", index_name, db_path)? {
            let sql = safe_repair_statement(index_name).ok_or_else(|| {
                ClientError::ledger_init_failed(db_path, "Missing canonical SQL for index repair.")
            })?;
            connection
                .execute_batch(&sql)
                .map_err(|error| map_sqlite_error(db_path, &error))?;
        }
    }

    Ok(())
}

fn verify_post_repair_objects(connection: &Connection, db_path: &Path) -> ClientResult<()> {
    let user_version = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    if user_version != EXPECTED_USER_VERSION {
        return Err(ClientError::ledger_corrupt(db_path));
    }

    for (meta_key, expected_value) in REQUIRED_META_KEYS {
        let value = connection
            .query_row(
                "SELECT value FROM internal_meta WHERE key = ?1 LIMIT 1",
                [meta_key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| map_sqlite_error(db_path, &error))?;

        if value.is_none() {
            return Err(ClientError::ledger_corrupt(db_path));
        }

        if let Some(actual) = value
            && actual != expected_value
        {
            return Err(ClientError::ledger_corrupt(db_path));
        }
    }

    for view_name in REQUIRED_VIEW_NAMES {
        if !sqlite_object_exists(connection, "view", view_name, db_path)? {
            return Err(ClientError::ledger_corrupt(db_path));
        }
    }
    verify_canonical_view_sql(connection, db_path)?;

    for index_name in REQUIRED_INDEX_NAMES {
        if !sqlite_object_exists(connection, "index", index_name, db_path)? {
            return Err(ClientError::ledger_corrupt(db_path));
        }
    }

    Ok(())
}

fn verify_canonical_view_sql(connection: &Connection, db_path: &Path) -> ClientResult<()> {
    for view_name in REQUIRED_VIEW_NAMES {
        let actual_sql = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'view' AND name = ?1 LIMIT 1",
                [view_name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| map_sqlite_error(db_path, &error))?;

        let Some(actual_view_sql) = actual_sql else {
            return Err(ClientError::ledger_corrupt(db_path));
        };

        let expected_block = safe_repair_statement(view_name).ok_or_else(|| {
            ClientError::ledger_init_failed(db_path, "Missing canonical SQL for view verification.")
        })?;
        let expected_create = extract_create_view_sql(&expected_block).ok_or_else(|| {
            ClientError::ledger_init_failed(
                db_path,
                "Missing canonical CREATE VIEW SQL for verification.",
            )
        })?;

        if normalize_sql(&actual_view_sql) != normalize_sql(expected_create) {
            return Err(ClientError::ledger_corrupt(db_path));
        }
    }

    Ok(())
}

fn extract_create_view_sql(statement_block: &str) -> Option<&str> {
    statement_block
        .split(';')
        .map(str::trim)
        .find(|statement| statement.to_ascii_lowercase().starts_with("create view "))
}

fn normalize_sql(sql: &str) -> String {
    sql.chars()
        .filter(|value| !value.is_whitespace() && *value != ';')
        .flat_map(char::to_lowercase)
        .collect()
}

fn sqlite_object_exists(
    connection: &Connection,
    object_type: &str,
    object_name: &str,
    db_path: &Path,
) -> ClientResult<bool> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = ?1 AND name = ?2 LIMIT 1",
            params![object_type, object_name],
            |_row| Ok(true),
        )
        .optional()
        .map_err(|error| map_sqlite_error(db_path, &error))?
        .unwrap_or(false);

    Ok(exists)
}

fn table_columns(
    connection: &Connection,
    table_name: &str,
    db_path: &Path,
) -> ClientResult<Vec<String>> {
    if !is_required_core_table(table_name) {
        return Err(ClientError::ledger_init_failed(
            db_path,
            "Refused PRAGMA table inspection for non-core table.",
        ));
    }

    // SAFETY: `table_name` is restricted to the compile-time allowlist from
    // REQUIRED_CORE_TABLES above and never originates from user input.
    let sql = format!("PRAGMA table_info({table_name})");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let column_iter = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut columns: Vec<String> = Vec::new();
    for row in column_iter {
        let column = row.map_err(|error| map_sqlite_error(db_path, &error))?;
        columns.push(column);
    }

    Ok(columns)
}

fn is_required_core_table(table_name: &str) -> bool {
    REQUIRED_CORE_TABLES
        .iter()
        .any(|(required_name, _)| required_name == &table_name)
}

fn read_schema_version(connection: &Connection, db_path: &Path) -> ClientResult<String> {
    let value = connection
        .query_row(
            "SELECT value FROM internal_meta WHERE key = 'schema_version' LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    Ok(value.unwrap_or_else(|| "v1".to_string()))
}

fn read_data_range(connection: &Connection, db_path: &Path) -> ClientResult<DataRange> {
    let mut statement = connection
        .prepare("SELECT MIN(posted_at), MAX(posted_at) FROM internal_transactions")
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let row = statement
        .query_row([], |result_row| {
            let earliest = result_row.get::<_, Option<String>>(0)?;
            let latest = result_row.get::<_, Option<String>>(1)?;
            Ok(DataRange { earliest, latest })
        })
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    Ok(row)
}
