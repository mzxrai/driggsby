use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, TransactionBehavior, params};
use ulid::Ulid;

use crate::ClientResult;
use crate::import::CanonicalTransaction;
use crate::import::dedupe::{BatchRow, DuplicateRecord, dedupe_key};
use crate::intelligence::refresh::refresh_all_in_transaction;
use crate::state::map_sqlite_error;

#[derive(Debug, Clone)]
pub(crate) struct PersistResult {
    pub(crate) import_id: String,
    pub(crate) inserted: i64,
    pub(crate) duplicate_rows: Vec<DuplicateRecord>,
}

pub(crate) struct PersistInput<'a> {
    pub(crate) import_id: &'a str,
    pub(crate) candidate_rows: &'a [BatchRow],
    pub(crate) duplicate_rows: &'a [DuplicateRecord],
    pub(crate) rows_read: i64,
    pub(crate) rows_valid: i64,
    pub(crate) rows_invalid: i64,
    pub(crate) source_kind: &'a str,
    pub(crate) source_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Default)]
struct AccountImportStatCounter {
    rows_read: i64,
    inserted: i64,
    deduped: i64,
}

pub(crate) fn persist_import(
    connection: &mut Connection,
    db_path: &Path,
    input: PersistInput<'_>,
) -> ClientResult<PersistResult> {
    let timestamp = now_timestamp();

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut inserted = 0_i64;
    let mut account_stats: HashMap<String, AccountImportStatCounter> = HashMap::new();
    for batch_row in input.candidate_rows {
        insert_canonical_row(&transaction, db_path, input.import_id, &batch_row.row)?;
        upsert_account_metadata(
            &transaction,
            db_path,
            &batch_row.row.account_key,
            batch_row.row.account_type.as_deref(),
            &timestamp,
        )?;
        let stat = account_stats
            .entry(batch_row.row.account_key.clone())
            .or_default();
        stat.rows_read += 1;
        stat.inserted += 1;
        inserted += 1;
    }

    for duplicate_row in input.duplicate_rows {
        upsert_account_metadata(
            &transaction,
            db_path,
            &duplicate_row.row.account_key,
            duplicate_row.row.account_type.as_deref(),
            &timestamp,
        )?;
        let stat = account_stats
            .entry(duplicate_row.row.account_key.clone())
            .or_default();
        stat.rows_read += 1;
        stat.deduped += 1;
        insert_dedupe_candidate(
            &transaction,
            db_path,
            input.import_id,
            duplicate_row,
            &timestamp,
        )?;
    }

    let deduped_total = input.duplicate_rows.len() as i64;

    transaction
        .execute(
            "INSERT INTO internal_import_runs (
                import_id,
                status,
                created_at,
                committed_at,
                rows_read,
                rows_valid,
                rows_invalid,
                inserted,
                deduped,
                source_kind,
                source_ref
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                input.import_id,
                "committed",
                &timestamp,
                &timestamp,
                input.rows_read,
                input.rows_valid,
                input.rows_invalid,
                inserted,
                deduped_total,
                input.source_kind,
                input.source_ref
            ],
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    insert_import_account_stats(&transaction, db_path, input.import_id, &account_stats)?;
    refresh_all_in_transaction(&transaction, db_path)?;

    transaction
        .commit()
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    Ok(PersistResult {
        import_id: input.import_id.to_string(),
        inserted,
        duplicate_rows: input.duplicate_rows.to_vec(),
    })
}

fn insert_canonical_row(
    transaction: &rusqlite::Transaction<'_>,
    db_path: &Path,
    import_id: &str,
    row: &CanonicalTransaction,
) -> ClientResult<()> {
    let txn_id = format!("txn_{}", Ulid::new());
    transaction
        .execute(
            "INSERT INTO internal_transactions (
                txn_id,
                import_id,
                statement_id,
                dedupe_scope_id,
                account_key,
                posted_at,
                amount,
                currency,
                description,
                external_id,
                merchant,
                category
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                &txn_id,
                import_id,
                &row.statement_id,
                &row.dedupe_scope_id,
                &row.account_key,
                &row.posted_at,
                row.amount,
                &row.currency,
                &row.description,
                &row.external_id,
                &row.merchant,
                &row.category,
            ],
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    Ok(())
}

fn insert_dedupe_candidate(
    transaction: &rusqlite::Transaction<'_>,
    db_path: &Path,
    import_id: &str,
    duplicate_row: &DuplicateRecord,
    timestamp: &str,
) -> ClientResult<()> {
    let candidate_id = format!("cand_{}", Ulid::new());
    let key = dedupe_key(&duplicate_row.row);
    transaction
        .execute(
            "INSERT INTO internal_transaction_dedupe_candidates (
                candidate_id,
                import_id,
                dedupe_key,
                statement_id,
                dedupe_scope_id,
                account_key,
                posted_at,
                amount,
                currency,
                description,
                external_id,
                merchant,
                category,
                source_row_index,
                dedupe_reason,
                matched_txn_id,
                matched_import_id,
                matched_batch_row_index,
                created_at,
                promoted_txn_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, NULL)",
            params![
                candidate_id,
                import_id,
                key,
                &duplicate_row.row.statement_id,
                &duplicate_row.row.dedupe_scope_id,
                &duplicate_row.row.account_key,
                &duplicate_row.row.posted_at,
                duplicate_row.row.amount,
                &duplicate_row.row.currency,
                &duplicate_row.row.description,
                &duplicate_row.row.external_id,
                &duplicate_row.row.merchant,
                &duplicate_row.row.category,
                duplicate_row.source_row_index,
                duplicate_row.dedupe_reason.as_str(),
                &duplicate_row.matched_txn_id,
                &duplicate_row.matched_import_id,
                duplicate_row.matched_batch_row_index,
                timestamp
            ],
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    Ok(())
}

fn upsert_account_metadata(
    transaction: &rusqlite::Transaction<'_>,
    db_path: &Path,
    account_key: &str,
    account_type: Option<&str>,
    timestamp: &str,
) -> ClientResult<()> {
    transaction
        .execute(
            "INSERT INTO internal_accounts (account_key, account_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(account_key) DO UPDATE SET
               account_type = CASE
                   WHEN excluded.account_type IS NULL OR TRIM(excluded.account_type) = ''
                       THEN internal_accounts.account_type
                   WHEN internal_accounts.account_type IS NULL OR TRIM(internal_accounts.account_type) = ''
                       THEN excluded.account_type
                   ELSE internal_accounts.account_type
               END,
               updated_at = excluded.updated_at",
            params![account_key, account_type, timestamp],
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    Ok(())
}

fn insert_import_account_stats(
    transaction: &rusqlite::Transaction<'_>,
    db_path: &Path,
    import_id: &str,
    account_stats: &HashMap<String, AccountImportStatCounter>,
) -> ClientResult<()> {
    for (account_key, stat) in account_stats {
        transaction
            .execute(
                "INSERT INTO internal_import_account_stats (
                    import_id,
                    account_key,
                    rows_read,
                    inserted,
                    deduped
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(import_id, account_key) DO UPDATE SET
                    rows_read = excluded.rows_read,
                    inserted = excluded.inserted,
                    deduped = excluded.deduped",
                params![
                    import_id,
                    account_key,
                    stat.rows_read,
                    stat.inserted,
                    stat.deduped
                ],
            )
            .map_err(|error| map_sqlite_error(db_path, &error))?;
    }
    Ok(())
}

pub(crate) fn now_timestamp() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH);
    match now {
        Ok(duration) => format!("{}", duration.as_secs()),
        Err(_) => "0".to_string(),
    }
}
