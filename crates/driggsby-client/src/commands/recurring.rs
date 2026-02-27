use std::path::Path;

use rusqlite::params;

use crate::ClientResult;
use crate::commands::common::data_range_hint;
use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::{RecurringData, RecurringRow};
use crate::intelligence::date::{build_filter, format_iso_date};
use crate::intelligence::policy::RECURRING_POLICY_VERSION;
use crate::setup::{SetupContext, ensure_initialized, ensure_initialized_at};
use crate::state::{map_sqlite_error, open_connection};

#[derive(Debug, Default)]
pub struct RecurringRunOptions<'a> {
    pub from: Option<String>,
    pub to: Option<String>,
    pub home_override: Option<&'a Path>,
}

pub fn run(from: Option<&str>, to: Option<&str>) -> ClientResult<SuccessEnvelope> {
    run_with_options(RecurringRunOptions {
        from: from.map(std::string::ToString::to_string),
        to: to.map(std::string::ToString::to_string),
        home_override: None,
    })
}

#[doc(hidden)]
pub fn run_with_options(options: RecurringRunOptions<'_>) -> ClientResult<SuccessEnvelope> {
    let setup = load_setup(options.home_override)?;
    let filter = build_filter(options.from.as_deref(), options.to.as_deref(), "recurring")?;
    let db_path = std::path::PathBuf::from(&setup.db_path);
    let connection = open_connection(&db_path)?;
    let from_bound = filter.from.as_ref().map(format_iso_date);
    let to_bound = filter.to.as_ref().map(format_iso_date);
    let mut statement = connection
        .prepare(
            "SELECT
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
             FROM v1_recurring
             WHERE (?1 IS NULL OR last_seen_at >= ?1)
               AND (?2 IS NULL OR last_seen_at <= ?2)
             ORDER BY
                CASE WHEN next_expected_at IS NULL THEN 1 ELSE 0 END ASC,
                next_expected_at ASC,
                score DESC,
                merchant ASC,
                group_key ASC",
        )
        .map_err(|error| map_sqlite_error(&db_path, &error))?;

    let rows_iter = statement
        .query_map(params![from_bound, to_bound], |row| {
            let is_active_raw: i64 = row.get(10)?;
            Ok(RecurringRow {
                group_key: row.get(0)?,
                account_key: row.get(1)?,
                merchant: row.get(2)?,
                cadence: row.get(3)?,
                typical_amount: row.get(4)?,
                currency: row.get(5)?,
                last_seen_at: row.get(6)?,
                next_expected_at: row.get(7)?,
                occurrence_count: row.get(8)?,
                score: row.get(9)?,
                is_active: is_active_raw == 1,
            })
        })
        .map_err(|error| map_sqlite_error(&db_path, &error))?;

    let mut rows = Vec::new();
    for row in rows_iter {
        rows.push(row.map_err(|error| map_sqlite_error(&db_path, &error))?);
    }

    let data = RecurringData {
        policy_version: RECURRING_POLICY_VERSION.to_string(),
        from: filter.from.as_ref().map(format_iso_date),
        to: filter.to.as_ref().map(format_iso_date),
        rows,
        data_range_hint: data_range_hint(&setup.data_range),
    };

    success("recurring", data)
}

fn load_setup(home_override: Option<&Path>) -> ClientResult<SetupContext> {
    if let Some(home) = home_override {
        return ensure_initialized_at(home);
    }
    ensure_initialized()
}
