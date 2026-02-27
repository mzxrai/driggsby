use std::path::Path;

use rusqlite::params;

use crate::ClientResult;
use crate::commands::common::data_range_hint;
use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::{AnomaliesData, AnomalyRow};
use crate::intelligence::date::{build_filter, format_iso_date};
use crate::intelligence::policy::ANOMALIES_POLICY_VERSION;
use crate::setup::{SetupContext, ensure_initialized, ensure_initialized_at};
use crate::state::{map_sqlite_error, open_connection};

#[derive(Debug, Default)]
pub struct AnomaliesRunOptions<'a> {
    pub from: Option<String>,
    pub to: Option<String>,
    pub home_override: Option<&'a Path>,
}

pub fn run(from: Option<&str>, to: Option<&str>) -> ClientResult<SuccessEnvelope> {
    run_with_options(AnomaliesRunOptions {
        from: from.map(std::string::ToString::to_string),
        to: to.map(std::string::ToString::to_string),
        home_override: None,
    })
}

#[doc(hidden)]
pub fn run_with_options(options: AnomaliesRunOptions<'_>) -> ClientResult<SuccessEnvelope> {
    let setup = load_setup(options.home_override)?;
    let filter = build_filter(options.from.as_deref(), options.to.as_deref(), "anomalies")?;
    let db_path = std::path::PathBuf::from(&setup.db_path);
    let connection = open_connection(&db_path)?;
    let from_bound = filter.from.as_ref().map(format_iso_date);
    let to_bound = filter.to.as_ref().map(format_iso_date);
    let mut statement = connection
        .prepare(
            "SELECT
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
             FROM v1_anomalies
             WHERE (?1 IS NULL OR posted_at >= ?1)
               AND (?2 IS NULL OR posted_at <= ?2)
             ORDER BY posted_at ASC, merchant ASC, txn_id ASC",
        )
        .map_err(|error| map_sqlite_error(&db_path, &error))?;
    let rows_iter = statement
        .query_map(params![from_bound, to_bound], |row| {
            Ok(AnomalyRow {
                txn_id: row.get(0)?,
                account_key: row.get(1)?,
                posted_at: row.get(2)?,
                merchant: row.get(3)?,
                amount: row.get(4)?,
                currency: row.get(5)?,
                reason_code: row.get(6)?,
                reason: row.get(7)?,
                score: row.get(8)?,
                severity: row.get(9)?,
            })
        })
        .map_err(|error| map_sqlite_error(&db_path, &error))?;

    let mut rows = Vec::new();
    for row in rows_iter {
        rows.push(row.map_err(|error| map_sqlite_error(&db_path, &error))?);
    }

    let data = AnomaliesData {
        policy_version: ANOMALIES_POLICY_VERSION.to_string(),
        from: filter.from.as_ref().map(format_iso_date),
        to: filter.to.as_ref().map(format_iso_date),
        rows,
        data_range_hint: data_range_hint(&setup.data_range),
    };

    success("anomalies", data)
}

fn load_setup(home_override: Option<&Path>) -> ClientResult<SetupContext> {
    if let Some(home) = home_override {
        return ensure_initialized_at(home);
    }
    ensure_initialized()
}
