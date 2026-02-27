use std::path::Path;

use crate::ClientResult;
use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::IntelligenceRefreshData;
use crate::import::persist::now_timestamp;
use crate::intelligence::refresh::refresh_all;
use crate::setup::{ensure_initialized, ensure_initialized_at};
use crate::state::open_connection;

#[derive(Debug, Default)]
pub struct IntelligenceRefreshOptions<'a> {
    pub home_override: Option<&'a Path>,
}

pub fn refresh() -> ClientResult<SuccessEnvelope> {
    refresh_with_options(IntelligenceRefreshOptions {
        home_override: None,
    })
}

#[doc(hidden)]
pub fn refresh_with_options(
    options: IntelligenceRefreshOptions<'_>,
) -> ClientResult<SuccessEnvelope> {
    let setup = if let Some(home) = options.home_override {
        ensure_initialized_at(home)?
    } else {
        ensure_initialized()?
    };
    let db_path = std::path::PathBuf::from(&setup.db_path);
    let mut connection = open_connection(&db_path)?;
    let summary = refresh_all(&mut connection, &db_path)?;
    let data = IntelligenceRefreshData {
        recurring_rows: summary.recurring_rows,
        anomaly_rows: summary.anomaly_rows,
        completed_at: now_timestamp(),
    };
    success("intelligence refresh", data)
}
