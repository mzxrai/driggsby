use std::path::Path;

use rusqlite::Connection;

use crate::ClientResult;
use crate::contracts::types::{ImportDriftWarning, ImportKeyInventory, ImportSignProfile};
use crate::import::dedupe::BatchRow;
use crate::import::drift_warnings::build_drift_warnings;
use crate::import::inventory::{incoming_unique_values, query_key_inventory};
use crate::import::sign_profiles::{
    existing_sign_count_map, incoming_sign_count_map, profiles_from_sign_counts,
};

#[derive(Debug, Clone)]
pub(crate) struct DryRunAnalysis {
    pub(crate) key_inventory: ImportKeyInventory,
    pub(crate) sign_profiles: Vec<ImportSignProfile>,
    pub(crate) drift_warnings: Vec<ImportDriftWarning>,
}

pub(crate) fn analyze_dry_run(
    connection: &Connection,
    db_path: &Path,
    rows: &[BatchRow],
) -> ClientResult<DryRunAnalysis> {
    let key_inventory = query_key_inventory(connection, db_path)?;
    let existing_sign_counts = existing_sign_count_map(connection, db_path)?;
    let sign_profiles = profiles_from_sign_counts(&existing_sign_counts);

    let incoming_values = incoming_unique_values(rows.iter().map(|row| &row.row));
    let incoming_sign_counts = incoming_sign_count_map(rows.iter().map(|row| &row.row));
    let drift_warnings = build_drift_warnings(
        &key_inventory,
        &incoming_values,
        &existing_sign_counts,
        &incoming_sign_counts,
    );

    Ok(DryRunAnalysis {
        key_inventory,
        sign_profiles,
        drift_warnings,
    })
}
