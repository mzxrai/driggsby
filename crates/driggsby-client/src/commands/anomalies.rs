use crate::ClientResult;
use crate::commands::common::data_range_hint;
use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::IntelligenceData;
use crate::setup::ensure_initialized;

pub fn run(from: Option<&str>, to: Option<&str>) -> ClientResult<SuccessEnvelope> {
    let setup = ensure_initialized()?;
    let data = IntelligenceData {
        from: from.map(str::to_string),
        to: to.map(str::to_string),
        rows: Vec::new(),
        data_range_hint: data_range_hint(&setup.data_range),
    };

    success("anomalies", data)
}
