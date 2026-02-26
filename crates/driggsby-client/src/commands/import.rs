use std::path::Path;

use rusqlite::{OptionalExtension, params};

use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::{
    ImportData, ImportDuplicateRow, ImportDuplicatesData, ImportKeysUniqData, ImportListData,
    ImportListItem, ImportPropertyInventory, ImportUndoData, ImportUndoSummary, QueryContext,
};
use crate::import;
use crate::setup::{ensure_initialized, ensure_initialized_at};
use crate::state::{map_sqlite_error, open_connection};
use crate::{ClientError, ClientResult};

#[derive(Debug, Default)]
pub struct ImportRunOptions<'a> {
    pub path: Option<String>,
    pub dry_run: bool,
    pub home_override: Option<&'a Path>,
    pub stdin_override: Option<String>,
}

#[derive(Debug, Default)]
pub struct ImportListOptions<'a> {
    pub home_override: Option<&'a Path>,
}

#[derive(Debug, Default)]
pub struct ImportUndoOptions<'a> {
    pub home_override: Option<&'a Path>,
}

#[derive(Debug, Default)]
pub struct ImportDuplicatesOptions<'a> {
    pub home_override: Option<&'a Path>,
}

#[derive(Debug, Default)]
pub struct ImportKeysUniqOptions<'a> {
    pub property: Option<String>,
    pub home_override: Option<&'a Path>,
}

pub fn run(path: Option<String>, dry_run: bool) -> ClientResult<SuccessEnvelope> {
    run_with_options(ImportRunOptions {
        path,
        dry_run,
        home_override: None,
        stdin_override: None,
    })
}

#[doc(hidden)]
pub fn run_with_options(options: ImportRunOptions<'_>) -> ClientResult<SuccessEnvelope> {
    let setup = load_setup(options.home_override)?;
    let execution = import::execute(
        &setup,
        options.path.clone(),
        options.dry_run,
        options.stdin_override,
    )?;
    let context_setup = if options.dry_run {
        setup
    } else {
        load_setup(options.home_override)?
    };

    let query_context = QueryContext {
        readonly_uri: context_setup.readonly_uri,
        db_path: context_setup.db_path,
        schema_version: context_setup.schema_version,
        data_range: context_setup.data_range,
        public_views: context_setup.public_views,
    };

    let data = ImportData {
        dry_run: execution.dry_run,
        path: options.path,
        import_id: execution.import_id,
        message: execution.message,
        summary: execution.summary,
        duplicate_summary: execution.duplicate_summary,
        duplicates_preview: execution.duplicates_preview,
        next_step: execution.next_step,
        other_actions: execution.other_actions,
        issues: execution.issues,
        source_used: execution.source_used,
        source_ignored: execution.source_ignored,
        source_conflict: execution.source_conflict,
        warnings: execution.warnings,
        key_inventory: execution.key_inventory,
        sign_profiles: execution.sign_profiles,
        drift_warnings: execution.drift_warnings,
        query_context,
    };

    success("import", data)
}

pub fn list() -> ClientResult<SuccessEnvelope> {
    list_with_options(ImportListOptions {
        home_override: None,
    })
}

#[doc(hidden)]
pub fn list_with_options(options: ImportListOptions<'_>) -> ClientResult<SuccessEnvelope> {
    let setup = load_setup(options.home_override)?;
    let db_path = std::path::PathBuf::from(&setup.db_path);
    let connection = open_connection(&db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT
                import_id,
                status,
                created_at,
                committed_at,
                reverted_at,
                rows_read,
                rows_valid,
                rows_invalid,
                inserted,
                deduped,
                source_kind,
                source_ref
             FROM internal_import_runs
             ORDER BY CAST(created_at AS INTEGER) DESC, import_id DESC",
        )
        .map_err(|error| map_sqlite_error(&db_path, &error))?;
    let rows_iter = statement
        .query_map([], |row| {
            Ok(ImportListItem {
                import_id: row.get(0)?,
                status: row.get(1)?,
                created_at: row.get(2)?,
                committed_at: row.get(3)?,
                reverted_at: row.get(4)?,
                rows_read: row.get(5)?,
                rows_valid: row.get(6)?,
                rows_invalid: row.get(7)?,
                inserted: row.get(8)?,
                deduped: row.get(9)?,
                source_kind: row.get::<_, Option<String>>(10)?,
                source_ref: row.get::<_, Option<String>>(11)?,
            })
        })
        .map_err(|error| map_sqlite_error(&db_path, &error))?;

    let mut rows = Vec::new();
    for row in rows_iter {
        let item = row.map_err(|error| map_sqlite_error(&db_path, &error))?;
        rows.push(item);
    }

    success("import list", ImportListData { rows })
}

pub fn keys_uniq(property: Option<String>) -> ClientResult<SuccessEnvelope> {
    keys_uniq_with_options(ImportKeysUniqOptions {
        property,
        home_override: None,
    })
}

pub fn duplicates(import_id: &str) -> ClientResult<SuccessEnvelope> {
    duplicates_with_options(
        import_id,
        ImportDuplicatesOptions {
            home_override: None,
        },
    )
}

#[doc(hidden)]
pub fn duplicates_with_options(
    import_id: &str,
    options: ImportDuplicatesOptions<'_>,
) -> ClientResult<SuccessEnvelope> {
    let setup = load_setup(options.home_override)?;
    let db_path = std::path::PathBuf::from(&setup.db_path);
    let connection = open_connection(&db_path)?;

    let exists = connection
        .query_row(
            "SELECT 1 FROM internal_import_runs WHERE import_id = ?1 LIMIT 1",
            params![import_id],
            |_row| Ok(true),
        )
        .optional()
        .map_err(|error| map_sqlite_error(&db_path, &error))?
        .unwrap_or(false);
    if !exists {
        return Err(ClientError::import_duplicates_id_not_found(import_id));
    }

    let mut statement = connection
        .prepare(
            "SELECT
                c.source_row_index,
                c.dedupe_reason,
                c.statement_id,
                c.account_key,
                c.posted_at,
                c.amount,
                c.currency,
                c.description,
                c.external_id,
                c.matched_batch_row_index,
                COALESCE(promoted.txn_id, direct.txn_id, fallback.txn_id) AS matched_txn_id,
                COALESCE(promoted.import_id, direct.import_id, fallback.import_id) AS matched_import_id,
                c.matched_txn_id AS matched_txn_id_at_dedupe,
                c.matched_import_id AS matched_import_id_at_dedupe
             FROM internal_transaction_dedupe_candidates c
             LEFT JOIN internal_transactions promoted
               ON promoted.txn_id = c.promoted_txn_id
             LEFT JOIN internal_transactions direct
               ON direct.txn_id = c.matched_txn_id
             LEFT JOIN internal_transactions fallback
               ON fallback.txn_id = (
                    SELECT t.txn_id
                    FROM internal_transactions t
                    -- Keep this predicate aligned with import::dedupe::find_existing_match.
                    WHERE c.dedupe_reason = 'existing_ledger'
                      AND (
                        (c.external_id IS NOT NULL
                         AND t.account_key = c.account_key
                         AND t.external_id = c.external_id)
                        OR
                        (c.external_id IS NULL
                         AND t.account_key = c.account_key
                         AND t.posted_at = c.posted_at
                         AND t.amount = c.amount
                         AND t.currency = c.currency
                         AND t.description = c.description
                         AND t.statement_id != c.statement_id)
                      )
                    ORDER BY t.txn_id ASC
                    LIMIT 1
               )
             WHERE c.import_id = ?1
             ORDER BY c.source_row_index ASC, c.dedupe_reason ASC, c.candidate_id ASC",
        )
        .map_err(|error| map_sqlite_error(&db_path, &error))?;

    let rows_iter = statement
        .query_map(params![import_id], |row| {
            Ok(ImportDuplicateRow {
                source_row_index: row.get(0)?,
                dedupe_reason: row.get(1)?,
                statement_id: row.get(2)?,
                account_key: row.get(3)?,
                posted_at: row.get(4)?,
                amount: row.get(5)?,
                currency: row.get(6)?,
                description: row.get(7)?,
                external_id: row.get(8)?,
                matched_batch_row_index: row.get(9)?,
                matched_txn_id: row.get(10)?,
                matched_import_id: row.get(11)?,
                matched_txn_id_at_dedupe: row.get(12)?,
                matched_import_id_at_dedupe: row.get(13)?,
            })
        })
        .map_err(|error| map_sqlite_error(&db_path, &error))?;

    let mut rows = Vec::new();
    for row in rows_iter {
        let item = row.map_err(|error| map_sqlite_error(&db_path, &error))?;
        rows.push(item);
    }

    success(
        "import duplicates",
        ImportDuplicatesData {
            import_id: import_id.to_string(),
            total: rows.len() as i64,
            rows,
        },
    )
}

#[doc(hidden)]
pub fn keys_uniq_with_options(options: ImportKeysUniqOptions<'_>) -> ClientResult<SuccessEnvelope> {
    let property = parse_requested_property(options.property)?;
    let setup = load_setup(options.home_override)?;
    let db_path = std::path::PathBuf::from(&setup.db_path);
    let connection = open_connection(&db_path)?;

    let inventory = import::inventory::query_key_inventory(&connection, &db_path)?;
    let (requested_property, inventories) = if let Some(requested) = property {
        (
            Some(requested.as_str().to_string()),
            vec![select_property_inventory(&inventory, requested)],
        )
    } else {
        (None, import::inventory::inventory_to_vec(&inventory))
    };

    success(
        "import keys uniq",
        ImportKeysUniqData {
            property: requested_property,
            inventories,
        },
    )
}

pub fn undo(import_id: &str) -> ClientResult<SuccessEnvelope> {
    undo_with_options(
        import_id,
        ImportUndoOptions {
            home_override: None,
        },
    )
}

#[doc(hidden)]
pub fn undo_with_options(
    import_id: &str,
    options: ImportUndoOptions<'_>,
) -> ClientResult<SuccessEnvelope> {
    let setup = load_setup(options.home_override)?;
    let db_path = std::path::PathBuf::from(&setup.db_path);
    let mut connection = open_connection(&db_path)?;
    let result = import::undo::undo_import(&mut connection, &db_path, import_id)?;
    success(
        "import undo",
        ImportUndoData {
            import_id: result.import_id,
            message: "Import reverted successfully.".to_string(),
            summary: ImportUndoSummary {
                rows_reverted: result.rows_reverted,
                rows_promoted: result.rows_promoted,
            },
            intelligence_refreshed: false,
        },
    )
}

fn load_setup(home_override: Option<&Path>) -> ClientResult<crate::setup::SetupContext> {
    if let Some(path) = home_override {
        return ensure_initialized_at(path);
    }
    ensure_initialized()
}

fn parse_requested_property(
    value: Option<String>,
) -> ClientResult<Option<import::inventory::TrackedProperty>> {
    let Some(raw_property) = value else {
        return Ok(None);
    };

    let Some(property) = import::inventory::TrackedProperty::parse(&raw_property) else {
        return Err(ClientError::invalid_argument_with_recovery(
            &format!(
                "Invalid property `{raw_property}`. Supported values: account_key, currency, merchant, category."
            ),
            vec![
                "Use one of: account_key, currency, merchant, category.".to_string(),
                "Run `driggsby import keys uniq --help` for usage.".to_string(),
            ],
        ));
    };

    Ok(Some(property))
}

fn select_property_inventory(
    inventory: &crate::contracts::types::ImportKeyInventory,
    property: import::inventory::TrackedProperty,
) -> ImportPropertyInventory {
    match property {
        import::inventory::TrackedProperty::AccountKey => inventory.account_key.clone(),
        import::inventory::TrackedProperty::Currency => inventory.currency.clone(),
        import::inventory::TrackedProperty::Merchant => inventory.merchant.clone(),
        import::inventory::TrackedProperty::Category => inventory.category.clone(),
    }
}
