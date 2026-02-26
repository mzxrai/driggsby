use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::ClientResult;
use crate::import::CanonicalTransaction;
use crate::state::map_sqlite_error;

#[derive(Debug, Clone)]
pub(crate) struct BatchRow {
    pub(crate) row: CanonicalTransaction,
    pub(crate) source_row_index: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum DedupeReason {
    Batch,
    ExistingLedger,
}

impl DedupeReason {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Batch => "batch",
            Self::ExistingLedger => "existing_ledger",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DuplicateRecord {
    pub(crate) row: CanonicalTransaction,
    pub(crate) source_row_index: i64,
    pub(crate) dedupe_reason: DedupeReason,
    pub(crate) matched_batch_row_index: Option<i64>,
    pub(crate) matched_txn_id: Option<String>,
    pub(crate) matched_import_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BatchDedupeResult {
    pub(crate) candidate_rows: Vec<BatchRow>,
    pub(crate) duplicate_rows: Vec<DuplicateRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct ExistingDedupeResult {
    pub(crate) insertable_rows: Vec<BatchRow>,
    pub(crate) duplicate_rows: Vec<DuplicateRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct LedgerMatch {
    pub(crate) txn_id: String,
    pub(crate) import_id: String,
}

pub(crate) fn dedupe_batch(rows: Vec<CanonicalTransaction>) -> BatchDedupeResult {
    let mut ext_seen: HashMap<String, i64> = HashMap::new();
    let mut fallback_seen: HashMap<String, Vec<(String, i64)>> = HashMap::new();
    let mut candidate_rows = Vec::new();
    let mut duplicate_rows = Vec::new();

    for (index, row) in rows.into_iter().enumerate() {
        let source_row_index = (index as i64) + 1;
        let key = dedupe_key(&row);

        if row.external_id.is_some() {
            if let Some(matched_batch_row_index) = ext_seen.get(&key) {
                duplicate_rows.push(DuplicateRecord {
                    row,
                    source_row_index,
                    dedupe_reason: DedupeReason::Batch,
                    matched_batch_row_index: Some(*matched_batch_row_index),
                    matched_txn_id: None,
                    matched_import_id: None,
                });
                continue;
            }

            ext_seen.insert(key, source_row_index);
            candidate_rows.push(BatchRow {
                row,
                source_row_index,
            });
            continue;
        }

        let seen_entries = fallback_seen.entry(key).or_default();
        let matched_batch_row_index = seen_entries
            .iter()
            .find(|(scope_id, _)| scope_id != &row.dedupe_scope_id)
            .map(|(_, matched_index)| *matched_index);

        if let Some(matched_index) = matched_batch_row_index {
            duplicate_rows.push(DuplicateRecord {
                row,
                source_row_index,
                dedupe_reason: DedupeReason::Batch,
                matched_batch_row_index: Some(matched_index),
                matched_txn_id: None,
                matched_import_id: None,
            });
            continue;
        }

        seen_entries.push((row.dedupe_scope_id.clone(), source_row_index));
        candidate_rows.push(BatchRow {
            row,
            source_row_index,
        });
    }

    BatchDedupeResult {
        candidate_rows,
        duplicate_rows,
    }
}

pub(crate) fn dedupe_against_existing(
    connection: &Connection,
    rows: &[BatchRow],
    db_path: &Path,
) -> ClientResult<ExistingDedupeResult> {
    let mut insertable_rows = Vec::new();
    let mut duplicate_rows = Vec::new();

    for row in rows {
        if let Some(existing) = find_existing_match(connection, &row.row, db_path)? {
            duplicate_rows.push(DuplicateRecord {
                row: row.row.clone(),
                source_row_index: row.source_row_index,
                dedupe_reason: DedupeReason::ExistingLedger,
                matched_batch_row_index: None,
                matched_txn_id: Some(existing.txn_id),
                matched_import_id: Some(existing.import_id),
            });
            continue;
        }
        insertable_rows.push(row.clone());
    }

    Ok(ExistingDedupeResult {
        insertable_rows,
        duplicate_rows,
    })
}

pub(crate) fn dedupe_key(row: &CanonicalTransaction) -> String {
    if let Some(external_id) = row.external_id.as_ref() {
        return format!("ext|{}|{}", row.account_key, external_id);
    }

    format!(
        "fallback|{}|{}|{}|{}|{}",
        row.account_key, row.posted_at, row.amount, row.currency, row.description
    )
}

pub(crate) fn find_existing_match(
    connection: &Connection,
    row: &CanonicalTransaction,
    db_path: &Path,
) -> ClientResult<Option<LedgerMatch>> {
    if let Some(external_id) = row.external_id.as_ref() {
        return connection
            .query_row(
                "SELECT txn_id, import_id
                 FROM internal_transactions
                 WHERE account_key = ?1
                   AND external_id = ?2
                 ORDER BY txn_id ASC
                 LIMIT 1",
                params![&row.account_key, external_id],
                |result| {
                    Ok(LedgerMatch {
                        txn_id: result.get(0)?,
                        import_id: result.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|error| map_sqlite_error(db_path, &error));
    }

    connection
        .query_row(
            "SELECT txn_id, import_id
             FROM internal_transactions
             WHERE account_key = ?1
               AND posted_at = ?2
               AND amount = ?3
               AND currency = ?4
               AND description = ?5
               AND dedupe_scope_id != ?6
             ORDER BY txn_id ASC
             LIMIT 1",
            params![
                &row.account_key,
                &row.posted_at,
                row.amount,
                &row.currency,
                &row.description,
                &row.dedupe_scope_id
            ],
            |result| {
                Ok(LedgerMatch {
                    txn_id: result.get(0)?,
                    import_id: result.get(1)?,
                })
            },
        )
        .optional()
        .map_err(|error| map_sqlite_error(db_path, &error))
}
