use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use ulid::Ulid;

use crate::import::CanonicalTransaction;
use crate::import::dedupe::{dedupe_key, find_existing_match};
use crate::import::persist::now_timestamp;
use crate::intelligence::refresh::refresh_all_in_transaction;
use crate::state::map_sqlite_error;
use crate::{ClientError, ClientResult};

#[derive(Debug, Clone)]
pub(crate) struct UndoResult {
    pub(crate) import_id: String,
    pub(crate) rows_reverted: i64,
    pub(crate) rows_promoted: i64,
    pub(crate) intelligence_refreshed: bool,
}

#[derive(Debug, Clone)]
struct PromotionCandidate {
    candidate_id: String,
    import_id: String,
    row: CanonicalTransaction,
}

pub(crate) fn undo_import(
    connection: &mut Connection,
    db_path: &Path,
    import_id: &str,
) -> ClientResult<UndoResult> {
    let timestamp = now_timestamp();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let status = transaction
        .query_row(
            "SELECT status FROM internal_import_runs WHERE import_id = ?1 LIMIT 1",
            params![import_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let Some(current_status) = status else {
        return Err(ClientError::import_id_not_found(import_id));
    };
    if current_status == "reverted" {
        return Err(ClientError::import_already_reverted(import_id));
    }
    if current_status != "committed" {
        return Err(ClientError::ledger_corrupt(db_path));
    }

    let touched_account_keys = touched_account_keys_for_import(&transaction, db_path, import_id)?;
    let touched_key_counts = touched_key_counts_for_import(&transaction, db_path, import_id)?;
    let rows_reverted = transaction
        .execute(
            "DELETE FROM internal_transactions WHERE import_id = ?1",
            params![import_id],
        )
        .map_err(|error| map_sqlite_error(db_path, &error))? as i64;

    transaction
        .execute(
            "UPDATE internal_import_runs
             SET status = 'reverted', reverted_at = ?2
             WHERE import_id = ?1",
            params![import_id, &timestamp],
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    transaction
        .execute(
            "UPDATE internal_transaction_dedupe_candidates
             SET promoted_txn_id = COALESCE(promoted_txn_id, '__invalid__')
             WHERE import_id = ?1",
            params![import_id],
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut rows_promoted = 0_i64;
    for (key, target_promotions) in touched_key_counts {
        let mut promoted_for_key = 0_i64;
        let candidates = candidates_for_key(&transaction, db_path, &key)?;
        for candidate in candidates {
            if promoted_for_key >= target_promotions {
                break;
            }

            if find_existing_match(&transaction, &candidate.row, db_path)?.is_some() {
                // This candidate still conflicts with a committed canonical row.
                // Keep it pending so later undo calls can promote it when safe.
                continue;
            }

            promote_candidate(&transaction, db_path, &candidate)?;
            rows_promoted += 1;
            promoted_for_key += 1;
        }
    }

    reconcile_account_metadata_for_undo(&transaction, db_path, &touched_account_keys)?;
    refresh_all_in_transaction(&transaction, db_path)?;

    transaction
        .commit()
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    Ok(UndoResult {
        import_id: import_id.to_string(),
        rows_reverted,
        rows_promoted,
        intelligence_refreshed: true,
    })
}

fn touched_account_keys_for_import(
    transaction: &rusqlite::Transaction<'_>,
    db_path: &Path,
    import_id: &str,
) -> ClientResult<Vec<String>> {
    let mut statement = transaction
        .prepare(
            "SELECT DISTINCT account_key
             FROM internal_import_account_stats
             WHERE import_id = ?1
               AND account_key IS NOT NULL
               AND TRIM(account_key) <> ''
             ORDER BY account_key ASC",
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let rows = statement
        .query_map(params![import_id], |row| row.get::<_, String>(0))
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut account_keys = Vec::new();
    for row in rows {
        account_keys.push(row.map_err(|error| map_sqlite_error(db_path, &error))?);
    }

    Ok(account_keys)
}

fn reconcile_account_metadata_for_undo(
    transaction: &rusqlite::Transaction<'_>,
    db_path: &Path,
    account_keys: &[String],
) -> ClientResult<()> {
    for account_key in account_keys {
        let remaining_count = transaction
            .query_row(
                "SELECT COUNT(*) FROM internal_transactions WHERE account_key = ?1",
                params![account_key],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| map_sqlite_error(db_path, &error))?;

        if remaining_count == 0 {
            transaction
                .execute(
                    "DELETE FROM internal_accounts WHERE account_key = ?1",
                    params![account_key],
                )
                .map_err(|error| map_sqlite_error(db_path, &error))?;
        }
    }

    Ok(())
}

fn touched_key_counts_for_import(
    transaction: &rusqlite::Transaction<'_>,
    db_path: &Path,
    import_id: &str,
) -> ClientResult<BTreeMap<String, i64>> {
    let mut statement = transaction
        .prepare(
            "SELECT statement_id, dedupe_scope_id, account_key, posted_at, amount, currency, description, external_id
             FROM internal_transactions
             WHERE import_id = ?1",
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let rows = statement
        .query_map(params![import_id], |row| {
            Ok(CanonicalTransaction {
                statement_id: row.get::<_, Option<String>>(0)?,
                dedupe_scope_id: row.get(1)?,
                account_key: row.get(2)?,
                account_type: None,
                posted_at: row.get(3)?,
                amount: row.get(4)?,
                currency: row.get(5)?,
                description: row.get(6)?,
                external_id: row.get(7)?,
                merchant: None,
                category: None,
            })
        })
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut counts = BTreeMap::new();
    for row in rows {
        let canonical = row.map_err(|error| map_sqlite_error(db_path, &error))?;
        let key = dedupe_key(&canonical);
        let current = counts.entry(key).or_insert(0);
        *current += 1;
    }

    Ok(counts)
}

fn candidates_for_key(
    transaction: &rusqlite::Transaction<'_>,
    db_path: &Path,
    dedupe_key_value: &str,
) -> ClientResult<Vec<PromotionCandidate>> {
    let mut statement = transaction
        .prepare(
            "SELECT
                c.candidate_id,
                c.import_id,
                c.statement_id,
                c.dedupe_scope_id,
                c.account_key,
                c.posted_at,
                c.amount,
                c.currency,
                c.description,
                c.external_id,
                c.merchant,
                c.category
             FROM internal_transaction_dedupe_candidates c
             JOIN internal_import_runs i ON i.import_id = c.import_id
             WHERE c.dedupe_key = ?1
               AND c.promoted_txn_id IS NULL
               AND i.status = 'committed'
             ORDER BY CAST(i.created_at AS INTEGER) ASC,
                      c.source_row_index ASC,
                      c.dedupe_reason ASC,
                      c.candidate_id ASC",
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let rows = statement
        .query_map(params![dedupe_key_value], |row| {
            Ok(PromotionCandidate {
                candidate_id: row.get(0)?,
                import_id: row.get(1)?,
                row: CanonicalTransaction {
                    statement_id: row.get::<_, Option<String>>(2)?,
                    dedupe_scope_id: row.get(3)?,
                    account_key: row.get(4)?,
                    account_type: None,
                    posted_at: row.get(5)?,
                    amount: row.get(6)?,
                    currency: row.get(7)?,
                    description: row.get(8)?,
                    external_id: row.get(9)?,
                    merchant: row.get(10)?,
                    category: row.get(11)?,
                },
            })
        })
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut candidates = Vec::new();
    for row in rows {
        candidates.push(row.map_err(|error| map_sqlite_error(db_path, &error))?);
    }

    Ok(candidates)
}

fn promote_candidate(
    transaction: &rusqlite::Transaction<'_>,
    db_path: &Path,
    candidate: &PromotionCandidate,
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
                &candidate.import_id,
                &candidate.row.statement_id,
                &candidate.row.dedupe_scope_id,
                &candidate.row.account_key,
                &candidate.row.posted_at,
                candidate.row.amount,
                &candidate.row.currency,
                &candidate.row.description,
                &candidate.row.external_id,
                &candidate.row.merchant,
                &candidate.row.category
            ],
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    transaction
        .execute(
            "UPDATE internal_transaction_dedupe_candidates
             SET promoted_txn_id = ?2
             WHERE candidate_id = ?1",
            params![&candidate.candidate_id, &txn_id],
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    Ok(())
}
