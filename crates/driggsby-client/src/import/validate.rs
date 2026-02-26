use std::collections::HashMap;

use crate::contracts::types::{ImportIssue, ImportSummary};
use crate::import::CanonicalTransaction;
use crate::import::parse::ParsedRow;
use crate::{ClientError, ClientResult};

#[derive(Debug, Clone)]
pub(crate) struct ValidatedRows {
    pub(crate) rows: Vec<CanonicalTransaction>,
    pub(crate) summary: ImportSummary,
    pub(crate) statement_id_rows: HashMap<(String, String), Vec<i64>>,
}

pub(crate) fn validate_rows(
    parsed_rows: Vec<ParsedRow>,
    statement_scope_id: &str,
) -> ClientResult<ValidatedRows> {
    let total_rows = parsed_rows.len();
    let mut rows = Vec::new();
    let mut issues = Vec::new();
    let mut statement_id_rows: HashMap<(String, String), Vec<i64>> = HashMap::new();

    for raw in parsed_rows {
        let mut row_issues = Vec::new();

        let account_key = validate_required_string(
            raw.row,
            "account_key",
            raw.account_key,
            &mut row_issues,
            "account_key must be present and non-empty.",
        );
        let statement_id = normalize_optional(raw.statement_id);
        let dedupe_scope_id = resolve_dedupe_scope_id(
            account_key.as_deref(),
            statement_id.as_deref(),
            statement_scope_id,
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
            if let (Some(account_key_value), Some(statement_id_value)) =
                (account_key.as_ref(), statement_id.as_ref())
            {
                statement_id_rows
                    .entry((account_key_value.clone(), statement_id_value.clone()))
                    .or_default()
                    .push(raw.row);
            }
            rows.push(CanonicalTransaction {
                statement_id,
                dedupe_scope_id: dedupe_scope_id.unwrap_or_default(),
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

    Ok(ValidatedRows {
        rows,
        summary,
        statement_id_rows,
    })
}

fn resolve_dedupe_scope_id(
    account_key: Option<&str>,
    statement_id: Option<&str>,
    statement_scope_id: &str,
) -> Option<String> {
    let account_key = account_key?;
    if let Some(statement_id) = statement_id {
        return Some(format!("stmt|{}|{}", account_key, statement_id));
    }
    Some(format!("gen|{}|{}", statement_scope_id, account_key))
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
    if let Ok(amount) = parsed {
        if !amount.is_finite() {
            issues.push(ImportIssue {
                row,
                field: "amount".to_string(),
                code: "invalid_number".to_string(),
                description: format!("amount must be numeric; got \"{candidate}\""),
                expected: Some("number (e.g. -42.15)".to_string()),
                received: Some(candidate),
            });
            return None;
        }

        if let Some(scale) = fractional_digits(&candidate)
            && scale > 2
        {
            issues.push(ImportIssue {
                row,
                field: "amount".to_string(),
                code: "invalid_amount_scale".to_string(),
                description: format!(
                    "amount must use at most 2 decimal places; got {scale} decimal places."
                ),
                expected: Some("number with <= 2 decimal places (e.g. -42.15)".to_string()),
                received: Some(candidate),
            });
            return None;
        }

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

fn fractional_digits(value: &str) -> Option<usize> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (mantissa_raw, exponent_raw) = match trimmed.find(['e', 'E']) {
        Some(index) => (&trimmed[..index], Some(&trimmed[index + 1..])),
        None => (trimmed, None),
    };
    let exponent = if let Some(raw) = exponent_raw {
        raw.parse::<i32>().ok()?
    } else {
        0
    };

    let mantissa = mantissa_raw
        .strip_prefix('+')
        .or_else(|| mantissa_raw.strip_prefix('-'))
        .unwrap_or(mantissa_raw);
    if mantissa.is_empty() {
        return None;
    }

    let mut parts = mantissa.split('.');
    let whole = parts.next()?;
    let fractional = parts.next();
    if parts.next().is_some() {
        return None;
    }

    let whole_is_digits_or_empty = whole.chars().all(|character| character.is_ascii_digit());
    if !whole_is_digits_or_empty {
        return None;
    }

    let base_scale = if let Some(fractional_digits) = fractional {
        if !fractional_digits
            .chars()
            .all(|character| character.is_ascii_digit())
        {
            return None;
        }
        if whole.is_empty() && fractional_digits.is_empty() {
            return None;
        }
        fractional_digits.len()
    } else {
        if whole.is_empty() {
            return None;
        }
        0
    };

    if exponent >= 0 {
        return Some(base_scale.saturating_sub(exponent as usize));
    }

    Some(base_scale.saturating_add(exponent.unsigned_abs() as usize))
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
