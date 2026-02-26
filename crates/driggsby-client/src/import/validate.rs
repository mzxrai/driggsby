use crate::contracts::types::{ImportIssue, ImportSummary};
use crate::import::CanonicalTransaction;
use crate::import::parse::ParsedRow;
use crate::{ClientError, ClientResult};

#[derive(Debug, Clone)]
pub(crate) struct ValidatedRows {
    pub(crate) rows: Vec<CanonicalTransaction>,
    pub(crate) summary: ImportSummary,
}

pub(crate) fn validate_rows(parsed_rows: Vec<ParsedRow>) -> ClientResult<ValidatedRows> {
    let total_rows = parsed_rows.len();
    let mut rows = Vec::new();
    let mut issues = Vec::new();

    for raw in parsed_rows {
        let mut row_issues = Vec::new();

        let statement_id = validate_required_string(
            raw.row,
            "statement_id",
            raw.statement_id,
            &mut row_issues,
            "statement_id must be present and non-empty.",
        );
        let account_key = validate_required_string(
            raw.row,
            "account_key",
            raw.account_key,
            &mut row_issues,
            "account_key must be present and non-empty.",
        );
        let posted_at = validate_posted_at(raw.row, raw.posted_at, &mut row_issues);
        let amount = validate_amount(raw.row, raw.amount, &mut row_issues);
        let currency = validate_currency(raw.row, raw.currency, &mut row_issues);
        let description = validate_required_string(
            raw.row,
            "description",
            raw.description,
            &mut row_issues,
            "description must be present and non-empty.",
        );
        let external_id = normalize_optional(raw.external_id);
        let merchant = normalize_optional(raw.merchant);
        let category = normalize_optional(raw.category);

        if row_issues.is_empty() {
            rows.push(CanonicalTransaction {
                statement_id: statement_id.unwrap_or_default(),
                account_key: account_key.unwrap_or_default(),
                posted_at: posted_at.unwrap_or_default(),
                amount: amount.unwrap_or_default(),
                currency: currency.unwrap_or_default(),
                description: description.unwrap_or_default(),
                external_id,
                merchant,
                category,
            });
        } else {
            issues.extend(row_issues);
        }
    }

    let summary = ImportSummary {
        rows_read: total_rows as i64,
        rows_valid: rows.len() as i64,
        rows_invalid: issues
            .iter()
            .map(|issue| issue.row)
            .collect::<std::collections::HashSet<i64>>()
            .len() as i64,
        inserted: 0,
        deduped: 0,
    };

    if !issues.is_empty() {
        return Err(ClientError::import_validation_failed(summary, issues));
    }

    Ok(ValidatedRows { rows, summary })
}

fn validate_required_string(
    row: i64,
    field: &str,
    value: Option<String>,
    issues: &mut Vec<ImportIssue>,
    description: &str,
) -> Option<String> {
    let normalized = normalize_optional(value);
    if normalized.is_none() {
        issues.push(ImportIssue {
            row,
            field: field.to_string(),
            code: "missing_required_field".to_string(),
            description: description.to_string(),
            expected: Some("non-empty string".to_string()),
            received: Some(String::new()),
        });
    }
    normalized
}

fn validate_posted_at(
    row: i64,
    value: Option<String>,
    issues: &mut Vec<ImportIssue>,
) -> Option<String> {
    let normalized = normalize_optional(value);
    let Some(candidate) = normalized else {
        issues.push(ImportIssue {
            row,
            field: "posted_at".to_string(),
            code: "missing_required_field".to_string(),
            description: "posted_at must be present and non-empty.".to_string(),
            expected: Some("YYYY-MM-DD".to_string()),
            received: Some(String::new()),
        });
        return None;
    };

    if !looks_like_iso_date(&candidate) {
        issues.push(ImportIssue {
            row,
            field: "posted_at".to_string(),
            code: "invalid_date".to_string(),
            description: format!("posted_at must be YYYY-MM-DD; got \"{candidate}\""),
            expected: Some("YYYY-MM-DD".to_string()),
            received: Some(candidate),
        });
        return None;
    }

    Some(candidate)
}

fn validate_amount(row: i64, value: Option<String>, issues: &mut Vec<ImportIssue>) -> Option<f64> {
    let normalized = normalize_optional(value);
    let Some(candidate) = normalized else {
        issues.push(ImportIssue {
            row,
            field: "amount".to_string(),
            code: "missing_required_field".to_string(),
            description: "amount must be present and non-empty.".to_string(),
            expected: Some("number (e.g. -42.15)".to_string()),
            received: Some(String::new()),
        });
        return None;
    };

    let parsed = candidate.parse::<f64>();
    if let Ok(amount) = parsed
        && amount.is_finite()
    {
        return Some(amount);
    }

    issues.push(ImportIssue {
        row,
        field: "amount".to_string(),
        code: "invalid_number".to_string(),
        description: format!("amount must be numeric; got \"{candidate}\""),
        expected: Some("number (e.g. -42.15)".to_string()),
        received: Some(candidate),
    });
    None
}

fn validate_currency(
    row: i64,
    value: Option<String>,
    issues: &mut Vec<ImportIssue>,
) -> Option<String> {
    let normalized = normalize_optional(value);
    let Some(candidate) = normalized else {
        issues.push(ImportIssue {
            row,
            field: "currency".to_string(),
            code: "missing_required_field".to_string(),
            description: "currency must be present and non-empty.".to_string(),
            expected: Some("non-empty string".to_string()),
            received: Some(String::new()),
        });
        return None;
    };
    Some(candidate.to_uppercase())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    let raw = value?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn looks_like_iso_date(value: &str) -> bool {
    if value.len() != 10 {
        return false;
    }

    let bytes = value.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }

    for index in [0usize, 1, 2, 3, 5, 6, 8, 9] {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
    }

    let month = value[5..7].parse::<u32>();
    let day = value[8..10].parse::<u32>();
    if let (Ok(m), Ok(d)) = (month, day) {
        return m > 0 && m <= 12 && d > 0 && d <= 31;
    }

    false
}
