use std::path::Path;

use crate::ClientResult;
use crate::commands::common::data_range_hint;
use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::IntelligenceData;
use crate::intelligence::date::{build_filter, format_iso_date};
use crate::setup::{SetupContext, ensure_initialized, ensure_initialized_at};

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

    let data = IntelligenceData {
        from: filter.from.as_ref().map(format_iso_date),
        to: filter.to.as_ref().map(format_iso_date),
        rows: Vec::new(),
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
