use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use ulid::Ulid;

use crate::import::CanonicalTransaction;
use crate::import::dedupe::{dedupe_key, find_existing_match};
use crate::import::persist::now_timestamp;
use crate::state::map_sqlite_error;
use crate::{ClientError, ClientResult};

#[derive(Debug, Clone)]
pub(crate) struct UndoResult {
    pub(crate) import_id: String,
    pub(crate) rows_reverted: i64,
    pub(crate) rows_promoted: i64,
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

    transaction
        .commit()
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    Ok(UndoResult {
        import_id: import_id.to_string(),
        rows_reverted,
        rows_promoted,
    })
}

fn touched_key_counts_for_import(
    transaction: &rusqlite::Transaction<'_>,
    db_path: &Path,
    import_id: &str,
) -> ClientResult<BTreeMap<String, i64>> {
    let mut statement = transaction
        .prepare(
            "SELECT statement_id, account_key, posted_at, amount, currency, description, external_id
             FROM internal_transactions
             WHERE import_id = ?1",
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let rows = statement
        .query_map(params![import_id], |row| {
            Ok(CanonicalTransaction {
                statement_id: row.get(0)?,
                account_key: row.get(1)?,
                posted_at: row.get(2)?,
                amount: row.get(3)?,
                currency: row.get(4)?,
                description: row.get(5)?,
                external_id: row.get(6)?,
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
                    statement_id: row.get(2)?,
                    account_key: row.get(3)?,
                    posted_at: row.get(4)?,
                    amount: row.get(5)?,
                    currency: row.get(6)?,
                    description: row.get(7)?,
                    external_id: row.get(8)?,
                    merchant: row.get(9)?,
                    category: row.get(10)?,
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
                account_key,
                posted_at,
                amount,
                currency,
                description,
                external_id,
                merchant,
                category
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &txn_id,
                &candidate.import_id,
                &candidate.row.statement_id,
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
