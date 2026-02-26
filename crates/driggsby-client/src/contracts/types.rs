use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ViewColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub column_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaSummaryData {
    pub db_path: String,
    pub readonly_uri: String,
    pub public_views: Vec<PublicView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaViewData {
    pub view_name: String,
    pub columns: Vec<ViewColumn>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataRange {
    pub earliest: Option<String>,
    pub latest: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicView {
    pub name: String,
    pub columns: Vec<ViewColumn>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryContext {
    pub readonly_uri: String,
    pub db_path: String,
    pub schema_version: String,
    pub data_range: DataRange,
    pub public_views: Vec<PublicView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportData {
    pub dry_run: bool,
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_inventory: Option<ImportKeyInventory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sign_profiles: Option<Vec<ImportSignProfile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift_warnings: Option<Vec<ImportDriftWarning>>,
    pub query_context: QueryContext,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportNextStep {
    pub label: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportAction {
    pub label: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportCreateSummary {
    pub rows_read: i64,
    pub rows_valid: i64,
    pub rows_invalid: i64,
    pub inserted: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportSummary {
    pub rows_read: i64,
    pub rows_valid: i64,
    pub rows_invalid: i64,
    pub inserted: i64,
    pub deduped: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportDuplicateSummary {
    pub total: i64,
    pub batch: i64,
    pub existing_ledger: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportDuplicateRow {
    pub source_row_index: i64,
    pub dedupe_reason: String,
    pub statement_id: String,
    pub account_key: String,
    pub posted_at: String,
    pub amount: f64,
    pub currency: String,
    pub description: String,
    pub external_id: Option<String>,
    pub matched_batch_row_index: Option<i64>,
    pub matched_txn_id: Option<String>,
    pub matched_import_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportDuplicatesPreview {
    pub returned: i64,
    pub truncated: bool,
    pub rows: Vec<ImportDuplicateRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportIssue {
    pub row: i64,
    pub field: String,
    pub code: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportDuplicatesData {
    pub import_id: String,
    pub total: i64,
    pub rows: Vec<ImportDuplicateRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportValueCount {
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportPropertyInventory {
    pub property: String,
    pub existing_values: Vec<String>,
    pub value_counts: Vec<ImportValueCount>,
    pub unique_count: i64,
    pub null_count: i64,
    pub total_rows: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportKeyInventory {
    pub account_key: ImportPropertyInventory,
    pub currency: ImportPropertyInventory,
    pub merchant: ImportPropertyInventory,
    pub category: ImportPropertyInventory,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportSignProfile {
    pub account_key: String,
    pub negative_count: i64,
    pub positive_count: i64,
    pub negative_ratio: f64,
    pub positive_ratio: f64,
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportDriftWarning {
    pub code: String,
    pub severity: String,
    pub property: String,
    pub incoming_value: String,
    pub message: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportKeysUniqData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    pub inventories: Vec<ImportPropertyInventory>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportListItem {
    pub import_id: String,
    pub status: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub committed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverted_at: Option<String>,
    pub rows_read: i64,
    pub rows_valid: i64,
    pub rows_invalid: i64,
    pub inserted: i64,
    pub deduped: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportListData {
    pub rows: Vec<ImportListItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportUndoSummary {
    pub rows_reverted: i64,
    pub rows_promoted: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportUndoData {
    pub import_id: String,
    pub message: String,
    pub summary: ImportUndoSummary,
    pub intelligence_refreshed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntelligenceRow {
    pub id: String,
    pub posted_at: String,
    pub merchant: String,
    pub amount: f64,
    pub currency: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataRangeHint {
    pub earliest: Option<String>,
    pub latest: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntelligenceData {
    pub from: Option<String>,
    pub to: Option<String>,
    pub rows: Vec<IntelligenceRow>,
    pub data_range_hint: DataRangeHint,
}

#[derive(Debug, Clone, Serialize)]
pub struct DemoData {
    pub topic: String,
    pub url: String,
    pub fallback_steps: Vec<String>,
    pub source: String,
}
