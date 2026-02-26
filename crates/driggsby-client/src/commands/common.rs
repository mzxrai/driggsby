use crate::contracts::types::{DataRange, DataRangeHint, PublicView, ViewColumn};

const REQUIRED_IMPORT_FIELDS: [(&str, &str); 5] = [
    ("account_key", "string"),
    ("posted_at", "date"),
    ("amount", "number"),
    ("currency", "string"),
    ("description", "string"),
];

const OPTIONAL_IMPORT_FIELDS: [(&str, &str); 4] = [
    ("statement_id", "string"),
    ("external_id", "string"),
    ("merchant", "string|null"),
    ("category", "string|null"),
];

pub(crate) fn required_import_field_names() -> Vec<&'static str> {
    REQUIRED_IMPORT_FIELDS
        .iter()
        .map(|(name, _)| *name)
        .collect()
}

pub(crate) fn optional_import_field_names() -> Vec<&'static str> {
    OPTIONAL_IMPORT_FIELDS
        .iter()
        .map(|(name, _)| *name)
        .collect()
}

pub fn public_view_contracts() -> Vec<PublicView> {
    vec![
        PublicView {
            name: "v1_transactions".to_string(),
            columns: vec![
                view_column("txn_id", "text"),
                view_column("import_id", "text"),
                view_column("statement_id", "text|null"),
                view_column("account_key", "text"),
                view_column("posted_at", "date"),
                view_column("amount", "real"),
                view_column("currency", "text"),
                view_column("description", "text"),
                view_column("external_id", "text|null"),
                view_column("merchant", "text|null"),
                view_column("category", "text|null"),
            ],
        },
        PublicView {
            name: "v1_accounts".to_string(),
            columns: vec![
                view_column("account_key", "text"),
                view_column("currency", "text"),
                view_column("first_posted_at", "date"),
                view_column("last_posted_at", "date"),
                view_column("txn_count", "integer"),
            ],
        },
        PublicView {
            name: "v1_imports".to_string(),
            columns: vec![
                view_column("import_id", "text"),
                view_column("status", "text"),
                view_column("created_at", "text"),
                view_column("committed_at", "text|null"),
                view_column("reverted_at", "text|null"),
                view_column("rows_read", "integer"),
                view_column("rows_valid", "integer"),
                view_column("rows_invalid", "integer"),
                view_column("inserted", "integer"),
                view_column("deduped", "integer"),
                view_column("source_kind", "text|null"),
                view_column("source_ref", "text|null"),
            ],
        },
        PublicView {
            name: "v1_recurring".to_string(),
            columns: vec![
                view_column("merchant", "text|null"),
                view_column("typical_amount", "real"),
                view_column("cadence", "text"),
            ],
        },
        PublicView {
            name: "v1_anomalies".to_string(),
            columns: vec![
                view_column("posted_at", "date"),
                view_column("amount", "real"),
                view_column("reason", "text"),
            ],
        },
    ]
}

pub fn data_range_hint(data_range: &DataRange) -> DataRangeHint {
    DataRangeHint {
        earliest: data_range.earliest.clone(),
        latest: data_range.latest.clone(),
    }
}

fn view_column(name: &str, column_type: &str) -> ViewColumn {
    ViewColumn {
        name: name.to_string(),
        column_type: column_type.to_string(),
        nullable: column_type.ends_with("|null"),
    }
}
