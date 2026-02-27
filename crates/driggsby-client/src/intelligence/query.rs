use std::path::Path;

use rusqlite::params;

use crate::ClientResult;
use crate::intelligence::date::{format_iso_date, parse_transaction_date};
use crate::intelligence::types::{IntelligenceFilter, NormalizedTransaction};
use crate::state::{map_sqlite_error, open_connection};

pub fn load_transactions(
    db_path: &Path,
    filter: &IntelligenceFilter,
) -> ClientResult<Vec<NormalizedTransaction>> {
    let connection = open_connection(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT
                account_key,
                posted_at,
                amount,
                currency,
                description,
                merchant
             FROM internal_transactions
             WHERE amount <> 0
               AND (?1 IS NULL OR posted_at >= ?1)
               AND (?2 IS NULL OR posted_at <= ?2)
             ORDER BY account_key ASC, currency ASC, posted_at ASC, txn_id ASC",
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let from_bound = filter.from.as_ref().map(format_iso_date);
    let to_bound = filter.to.as_ref().map(format_iso_date);

    let rows_iter = statement
        .query_map(params![from_bound, to_bound], |row| {
            let account_key: String = row.get(0)?;
            let posted_at: String = row.get(1)?;
            let amount: f64 = row.get(2)?;
            let currency: String = row.get(3)?;
            let description: String = row.get(4)?;
            let merchant: Option<String> = row.get(5)?;
            Ok((
                account_key,
                posted_at,
                amount,
                currency,
                description,
                merchant,
            ))
        })
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut rows: Vec<NormalizedTransaction> = Vec::new();
    for row in rows_iter {
        let (account_key, posted_at, amount, currency, description, merchant) =
            row.map_err(|error| map_sqlite_error(db_path, &error))?;
        if amount == 0.0 {
            continue;
        }
        let Some(parsed_date) = parse_transaction_date(&posted_at) else {
            continue;
        };

        rows.push(NormalizedTransaction {
            account_key,
            posted_at: parsed_date,
            amount,
            currency: currency.trim().to_ascii_uppercase(),
            description: description.trim().to_string(),
            merchant: merchant
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        });
    }

    Ok(rows)
}
