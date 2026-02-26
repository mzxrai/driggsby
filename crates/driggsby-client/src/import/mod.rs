pub(crate) mod analysis;
pub(crate) mod dedupe;
pub(crate) mod drift_warnings;
pub(crate) mod input;
pub(crate) mod inventory;
pub(crate) mod parse;
pub(crate) mod persist;
pub(crate) mod sign_profiles;
pub(crate) mod undo;
pub(crate) mod validate;

use std::path::PathBuf;

use rusqlite::TransactionBehavior;

use crate::contracts::types::{
    ImportAction, ImportCreateSummary, ImportDriftWarning, ImportDuplicateRow,
    ImportDuplicateSummary, ImportDuplicatesPreview, ImportIssue, ImportKeyInventory,
    ImportNextStep, ImportSignProfile, ImportWarning,
};
use crate::setup::SetupContext;
use crate::state::open_connection;
use crate::{ClientError, ClientResult};

#[derive(Debug, Clone)]
pub(crate) struct CanonicalTransaction {
    pub statement_id: String,
    pub account_key: String,
    pub posted_at: String,
    pub amount: f64,
    pub currency: String,
    pub description: String,
    pub external_id: Option<String>,
    pub merchant: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportExecutionResult {
    pub dry_run: bool,
    pub import_id: Option<String>,
    pub message: String,
    pub summary: ImportCreateSummary,
    pub duplicate_summary: ImportDuplicateSummary,
    pub duplicates_preview: ImportDuplicatesPreview,
    pub next_step: ImportNextStep,
    pub other_actions: Vec<ImportAction>,
    pub issues: Vec<ImportIssue>,
    pub source_used: Option<String>,
    pub source_ignored: Option<String>,
    pub source_conflict: bool,
    pub warnings: Vec<ImportWarning>,
    pub key_inventory: Option<ImportKeyInventory>,
    pub sign_profiles: Option<Vec<ImportSignProfile>>,
    pub drift_warnings: Option<Vec<ImportDriftWarning>>,
}

pub(crate) fn execute(
    setup: &SetupContext,
    path: Option<String>,
    dry_run: bool,
    stdin_override: Option<String>,
) -> ClientResult<ImportExecutionResult> {
    let resolved_source = input::resolve_source(path, stdin_override)?;
    let parsed_rows = parse::parse_source(&resolved_source.content)?;
    let validated = validate::validate_rows(parsed_rows)?;
    let batch_deduped = dedupe::dedupe_batch(validated.rows);

    let db_path = PathBuf::from(&setup.db_path);
    let mut connection = open_connection(&db_path)?;

    if dry_run {
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| crate::state::map_sqlite_error(&db_path, &error))?;
        let ledger_deduped =
            dedupe::dedupe_against_existing(&transaction, &batch_deduped.candidate_rows, &db_path)?;
        let dry_run_analysis =
            analysis::analyze_dry_run(&transaction, &db_path, &ledger_deduped.insertable_rows)?;
        transaction
            .rollback()
            .map_err(|error| crate::state::map_sqlite_error(&db_path, &error))?;
        let duplicate_rows = merge_duplicate_rows(
            batch_deduped.duplicate_rows.clone(),
            ledger_deduped.duplicate_rows.clone(),
        );
        let summary = ImportCreateSummary {
            rows_read: validated.summary.rows_read,
            rows_valid: validated.summary.rows_valid,
            rows_invalid: validated.summary.rows_invalid,
            inserted: 0,
        };
        let duplicate_summary = build_duplicate_summary(
            batch_deduped.duplicate_rows.len() as i64,
            ledger_deduped.duplicate_rows.len() as i64,
        );
        let duplicates_preview = build_duplicates_preview(&duplicate_rows);
        let (next_step, other_actions) = build_next_actions(
            true,
            None,
            duplicate_summary.total,
            Some(resolved_source.source_kind.as_str()),
        );
        let message = if resolved_source.source_ignored.is_some() {
            "Validation passed. No rows were written. File input was used and stdin was ignored."
                .to_string()
        } else {
            "Validation passed. No rows were written.".to_string()
        };

        return Ok(ImportExecutionResult {
            dry_run: true,
            import_id: None,
            message,
            summary,
            duplicate_summary,
            duplicates_preview,
            next_step,
            other_actions,
            issues: Vec::new(),
            source_used: resolved_source.source_used,
            source_ignored: resolved_source.source_ignored,
            source_conflict: resolved_source.source_conflict,
            warnings: resolved_source.warnings,
            key_inventory: Some(dry_run_analysis.key_inventory),
            sign_profiles: Some(dry_run_analysis.sign_profiles),
            drift_warnings: Some(dry_run_analysis.drift_warnings),
        });
    }

    let existing_deduped =
        dedupe::dedupe_against_existing(&connection, &batch_deduped.candidate_rows, &db_path)?;
    let duplicate_rows = merge_duplicate_rows(
        batch_deduped.duplicate_rows.clone(),
        existing_deduped.duplicate_rows.clone(),
    );

    let persisted = persist::persist_import(
        &mut connection,
        &db_path,
        persist::PersistInput {
            candidate_rows: &existing_deduped.insertable_rows,
            rows_read: validated.summary.rows_read,
            rows_valid: validated.summary.rows_valid,
            rows_invalid: validated.summary.rows_invalid,
            duplicate_rows: &duplicate_rows,
            source_kind: resolved_source.source_kind.as_str(),
            source_ref: resolved_source.source_ref.as_deref(),
        },
    )?;
    let summary = ImportCreateSummary {
        rows_read: validated.summary.rows_read,
        rows_valid: validated.summary.rows_valid,
        rows_invalid: validated.summary.rows_invalid,
        inserted: persisted.inserted,
    };
    let duplicate_summary = build_duplicate_summary(
        batch_deduped.duplicate_rows.len() as i64,
        existing_deduped.duplicate_rows.len() as i64,
    );
    let duplicates_preview = build_duplicates_preview(&persisted.duplicate_rows);
    let (next_step, other_actions) = build_next_actions(
        false,
        Some(&persisted.import_id),
        duplicate_summary.total,
        Some(resolved_source.source_kind.as_str()),
    );

    let message = if resolved_source.source_ignored.is_some() {
        "Import completed successfully. File input was used and stdin was ignored.".to_string()
    } else {
        "Import completed successfully.".to_string()
    };

    Ok(ImportExecutionResult {
        dry_run: false,
        import_id: Some(persisted.import_id.clone()),
        message,
        summary,
        duplicate_summary,
        duplicates_preview,
        next_step,
        other_actions,
        issues: Vec::new(),
        source_used: resolved_source.source_used,
        source_ignored: resolved_source.source_ignored,
        source_conflict: resolved_source.source_conflict,
        warnings: resolved_source.warnings,
        key_inventory: None,
        sign_profiles: None,
        drift_warnings: None,
    })
}

fn merge_duplicate_rows(
    mut batch_rows: Vec<dedupe::DuplicateRecord>,
    mut existing_rows: Vec<dedupe::DuplicateRecord>,
) -> Vec<dedupe::DuplicateRecord> {
    let mut all_rows = Vec::new();
    all_rows.append(&mut batch_rows);
    all_rows.append(&mut existing_rows);
    all_rows.sort_by(|left, right| {
        left.source_row_index
            .cmp(&right.source_row_index)
            .then_with(|| {
                left.dedupe_reason
                    .as_str()
                    .cmp(right.dedupe_reason.as_str())
            })
    });
    all_rows
}

fn build_duplicate_summary(batch: i64, existing_ledger: i64) -> ImportDuplicateSummary {
    ImportDuplicateSummary {
        total: batch + existing_ledger,
        batch,
        existing_ledger,
    }
}

fn build_duplicates_preview(rows: &[dedupe::DuplicateRecord]) -> ImportDuplicatesPreview {
    let preview_rows = rows
        .iter()
        .take(50)
        .map(duplicate_record_to_contract)
        .collect::<Vec<ImportDuplicateRow>>();

    ImportDuplicatesPreview {
        returned: preview_rows.len() as i64,
        truncated: rows.len() > 50,
        rows: preview_rows,
    }
}

pub(crate) fn duplicate_record_to_contract(record: &dedupe::DuplicateRecord) -> ImportDuplicateRow {
    ImportDuplicateRow {
        source_row_index: record.source_row_index,
        dedupe_reason: record.dedupe_reason.as_str().to_string(),
        statement_id: record.row.statement_id.clone(),
        account_key: record.row.account_key.clone(),
        posted_at: record.row.posted_at.clone(),
        amount: record.row.amount,
        currency: record.row.currency.clone(),
        description: record.row.description.clone(),
        external_id: record.row.external_id.clone(),
        matched_batch_row_index: record.matched_batch_row_index,
        matched_txn_id: record.matched_txn_id.clone(),
        matched_import_id: record.matched_import_id.clone(),
    }
}

pub(crate) fn invalid_input_error(message: &str) -> ClientError {
    ClientError::invalid_argument_with_recovery(
        message,
        vec![
            "Provide JSON array or CSV input via path or stdin.".to_string(),
            "Run `driggsby import create --help` to confirm import field requirements.".to_string(),
        ],
    )
    .with_import_help()
}

fn build_next_actions(
    dry_run: bool,
    import_id: Option<&str>,
    duplicate_total: i64,
    source_kind: Option<&str>,
) -> (ImportNextStep, Vec<ImportAction>) {
    if dry_run {
        let dry_run_command = match source_kind {
            Some("stdin") => "driggsby import create",
            _ => "driggsby import create <path>",
        };
        return (
            ImportNextStep {
                label: "Commit this import".to_string(),
                command: dry_run_command.to_string(),
            },
            Vec::new(),
        );
    }

    let mut other_actions = vec![ImportAction {
        label: "View import list".to_string(),
        command: "driggsby import list".to_string(),
        risk: None,
    }];

    if duplicate_total > 0
        && let Some(id) = import_id
    {
        other_actions.push(ImportAction {
            label: "View duplicates".to_string(),
            command: format!("driggsby import duplicates {id}"),
            risk: None,
        });
    }

    if let Some(id) = import_id {
        other_actions.push(ImportAction {
            label: "Undo this import (destructive)".to_string(),
            command: format!("driggsby import undo {id}"),
            risk: Some("destructive".to_string()),
        });
    }

    (
        ImportNextStep {
            label: "Connect and query your data".to_string(),
            command: "driggsby schema".to_string(),
        },
        other_actions,
    )
}
