use std::path::Path;

use rusqlite::{Connection, params};

use crate::ClientResult;
use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::{AccountRow, AccountsData, AccountsSummary};
use crate::setup::{ensure_initialized, ensure_initialized_at};
use crate::state::{map_sqlite_error, open_connection};

pub fn run() -> ClientResult<SuccessEnvelope> {
    run_with_home_override(None)
}

#[doc(hidden)]
pub fn run_with_home_override(home_override: Option<&Path>) -> ClientResult<SuccessEnvelope> {
    let setup = if let Some(home) = home_override {
        ensure_initialized_at(home)?
    } else {
        ensure_initialized()?
    };
    let db_path = std::path::PathBuf::from(&setup.db_path);
    let connection = open_connection(&db_path)?;
    let data = query_accounts_data(&connection, &db_path)?;
    success("account list", data)
}

pub(crate) fn query_accounts_data(
    connection: &Connection,
    db_path: &Path,
) -> ClientResult<AccountsData> {
    let summary = connection
        .query_row(
            "SELECT
                COUNT(DISTINCT t.account_key) AS account_count,
                COUNT(*) AS transaction_count,
                MIN(t.posted_at) AS earliest_posted_at,
                MAX(t.posted_at) AS latest_posted_at,
                COUNT(DISTINCT CASE
                    WHEN a.account_type IS NOT NULL AND TRIM(a.account_type) <> '' THEN t.account_key
                    ELSE NULL
                END) AS typed_account_count,
                ROUND(COALESCE(SUM(t.amount), 0), 2) AS net_amount
             FROM internal_transactions t
             LEFT JOIN internal_accounts a ON a.account_key = t.account_key",
            [],
            |row| {
                let account_count = row.get::<_, i64>(0)?;
                let typed_account_count = row.get::<_, i64>(4)?;
                Ok(AccountsSummary {
                    account_count,
                    transaction_count: row.get(1)?,
                    earliest_posted_at: row.get(2)?,
                    latest_posted_at: row.get(3)?,
                    typed_account_count,
                    untyped_account_count: account_count.saturating_sub(typed_account_count),
                    net_amount: row.get(5)?,
                })
            },
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut statement = connection
        .prepare(
            "SELECT
                t.account_key,
                a.account_type,
                t.currency,
                COUNT(*) AS txn_count,
                MIN(t.posted_at) AS first_posted_at,
                MAX(t.posted_at) AS last_posted_at,
                ROUND(COALESCE(SUM(t.amount), 0), 2) AS net_amount
             FROM internal_transactions t
             LEFT JOIN internal_accounts a ON a.account_key = t.account_key
             GROUP BY t.account_key, a.account_type, t.currency
             ORDER BY t.account_key ASC, t.currency ASC",
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let rows_iter = statement
        .query_map(params![], |row| {
            Ok(AccountRow {
                account_key: row.get(0)?,
                account_type: row.get(1)?,
                currency: row.get(2)?,
                txn_count: row.get(3)?,
                first_posted_at: row.get(4)?,
                last_posted_at: row.get(5)?,
                net_amount: row.get(6)?,
            })
        })
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut rows = Vec::new();
    for row in rows_iter {
        rows.push(row.map_err(|error| map_sqlite_error(db_path, &error))?);
    }

    Ok(AccountsData { summary, rows })
}
