use std::path::Path;

use rusqlite::{Connection, Transaction, TransactionBehavior, params};

use crate::ClientResult;
use crate::intelligence::anomalies::detect_anomalies;
use crate::intelligence::query::load_transactions_from_connection;
use crate::intelligence::recurring::detect_recurring;
use crate::intelligence::types::IntelligenceFilter;
use crate::state::map_sqlite_error;

#[derive(Debug, Clone, Copy)]
pub struct RefreshSummary {
    pub recurring_rows: i64,
    pub anomaly_rows: i64,
}

pub fn refresh_all(connection: &mut Connection, db_path: &Path) -> ClientResult<RefreshSummary> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    let summary = refresh_all_internal(&transaction, db_path)?;
    transaction
        .commit()
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    Ok(summary)
}

pub fn refresh_all_in_transaction(
    transaction: &Transaction<'_>,
    db_path: &Path,
) -> ClientResult<RefreshSummary> {
    refresh_all_internal(transaction, db_path)
}

fn refresh_all_internal(connection: &Connection, db_path: &Path) -> ClientResult<RefreshSummary> {
    let filter = IntelligenceFilter {
        from: None,
        to: None,
    };
    let transactions = load_transactions_from_connection(connection, db_path, &filter)?;
    let recurring_rows = detect_recurring(&transactions);
    let anomaly_rows = detect_anomalies(&transactions);

    connection
        .execute("DELETE FROM internal_recurring_materialized", [])
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    connection
        .execute("DELETE FROM internal_anomalies_materialized", [])
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut recurring_inserted = 0_i64;
    if !recurring_rows.is_empty() {
        let mut statement = connection
            .prepare(
                "INSERT INTO internal_recurring_materialized (
                    group_key,
                    account_key,
                    merchant,
                    cadence,
                    typical_amount,
                    currency,
                    last_seen_at,
                    next_expected_at,
                    occurrence_count,
                    score,
                    is_active
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )
            .map_err(|error| map_sqlite_error(db_path, &error))?;

        for row in &recurring_rows {
            let next_expected_at = row
                .next_expected_at
                .map(|value| value.format("%Y-%m-%d").to_string());
            statement
                .execute(params![
                    &row.group_key,
                    &row.account_key,
                    &row.counterparty,
                    row.cadence.as_str(),
                    row.typical_amount,
                    &row.currency,
                    row.last_seen_at.format("%Y-%m-%d").to_string(),
                    next_expected_at,
                    row.occurrence_count,
                    row.score,
                    if row.is_active { 1_i64 } else { 0_i64 }
                ])
                .map_err(|error| map_sqlite_error(db_path, &error))?;
            recurring_inserted += 1;
        }
    }

    let mut anomalies_inserted = 0_i64;
    if !anomaly_rows.is_empty() {
        let mut statement = connection
            .prepare(
                "INSERT INTO internal_anomalies_materialized (
                    txn_id,
                    account_key,
                    posted_at,
                    merchant,
                    amount,
                    currency,
                    reason_code,
                    reason,
                    score,
                    severity
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )
            .map_err(|error| map_sqlite_error(db_path, &error))?;

        for row in &anomaly_rows {
            statement
                .execute(params![
                    &row.txn_id,
                    &row.account_key,
                    &row.posted_at,
                    &row.merchant,
                    row.amount,
                    &row.currency,
                    &row.reason_code,
                    &row.reason,
                    row.score,
                    &row.severity,
                ])
                .map_err(|error| map_sqlite_error(db_path, &error))?;
            anomalies_inserted += 1;
        }
    }

    Ok(RefreshSummary {
        recurring_rows: recurring_inserted,
        anomaly_rows: anomalies_inserted,
    })
}
