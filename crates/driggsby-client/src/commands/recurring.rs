use std::path::Path;

use crate::ClientResult;
use crate::commands::common::data_range_hint;
use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::{RecurringData, RecurringRow};
use crate::intelligence::date::{build_filter, format_iso_date};
use crate::intelligence::policy::RECURRING_POLICY_VERSION;
use crate::intelligence::query::load_transactions;
use crate::intelligence::recurring::detect_recurring;
use crate::setup::{SetupContext, ensure_initialized, ensure_initialized_at};

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
    let transactions = load_transactions(&db_path, &filter)?;
    let recurring = detect_recurring(&transactions);

    let rows = recurring
        .iter()
        .map(|row| RecurringRow {
            group_key: row.group_key.clone(),
            account_key: row.account_key.clone(),
            merchant: row.counterparty.clone(),
            counterparty: row.counterparty.clone(),
            counterparty_source: row.counterparty_source.as_str().to_string(),
            cadence: row.cadence.as_str().to_string(),
            typical_amount: row.typical_amount,
            currency: row.currency.clone(),
            first_seen_at: format_iso_date(&row.first_seen_at),
            last_seen_at: format_iso_date(&row.last_seen_at),
            next_expected_at: row.next_expected_at.as_ref().map(format_iso_date),
            occurrence_count: row.occurrence_count,
            cadence_fit: row.cadence_fit,
            amount_fit: row.amount_fit,
            score: row.score,
            amount_min: row.amount_min,
            amount_max: row.amount_max,
            sample_description: row.sample_description.clone(),
            quality_flags: row.quality_flags.clone(),
            is_active: row.is_active,
        })
        .collect::<Vec<RecurringRow>>();

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
