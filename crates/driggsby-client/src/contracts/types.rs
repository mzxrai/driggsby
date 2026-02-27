use serde::Serialize;
use serde_json::Value;

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
pub struct SqlColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub column_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SqlQueryData {
    pub columns: Vec<SqlColumn>,
    pub rows: Vec<Vec<Value>>,
    pub row_count: i64,
    pub truncated: bool,
    pub max_rows: i64,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger_accounts: Option<AccountsData>,
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
    pub statement_id: Option<String>,
    pub account_key: String,
    pub posted_at: String,
    pub amount: f64,
    pub currency: String,
    pub description: String,
    pub external_id: Option<String>,
    pub matched_batch_row_index: Option<i64>,
    pub matched_txn_id: Option<String>,
    pub matched_import_id: Option<String>,
    pub matched_txn_id_at_dedupe: Option<String>,
    pub matched_import_id_at_dedupe: Option<String>,
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
    pub account_type: ImportPropertyInventory,
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
    pub accounts: Vec<ImportListAccountStat>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportListData {
    pub rows: Vec<ImportListItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportListAccountStat {
    pub account_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_type: Option<String>,
    pub rows_read: i64,
    pub inserted: i64,
    pub deduped: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountsSummary {
    pub account_count: i64,
    pub transaction_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earliest_posted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_posted_at: Option<String>,
    pub typed_account_count: i64,
    pub untyped_account_count: i64,
    pub net_amount: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountRow {
    pub account_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_type: Option<String>,
    pub currency: String,
    pub txn_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_posted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_posted_at: Option<String>,
    pub net_amount: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountsData {
    pub summary: AccountsSummary,
    pub rows: Vec<AccountRow>,
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
pub struct AnomalyRow {
    pub txn_id: String,
    pub account_key: String,
    pub posted_at: String,
    pub merchant: String,
    pub amount: f64,
    pub currency: String,
    pub reason_code: String,
    pub reason: String,
    pub score: f64,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataRangeHint {
    pub earliest: Option<String>,
    pub latest: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnomaliesData {
    pub policy_version: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub rows: Vec<AnomalyRow>,
    pub data_range_hint: DataRangeHint,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecurringRow {
    pub group_key: String,
    pub account_key: String,
    pub merchant: String,
    pub cadence: String,
    pub typical_amount: f64,
    pub currency: String,
    pub last_seen_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_expected_at: Option<String>,
    pub occurrence_count: i64,
    pub score: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecurringData {
    pub policy_version: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub rows: Vec<RecurringRow>,
    pub data_range_hint: DataRangeHint,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntelligenceRefreshData {
    pub recurring_rows: i64,
    pub anomaly_rows: i64,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DemoData {
    pub topic: String,
    pub url: String,
    pub fallback_steps: Vec<String>,
    pub source: String,
}
