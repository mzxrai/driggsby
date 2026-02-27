use std::collections::BTreeMap;

use crate::intelligence::normalize::counterparty_from_transaction;
use crate::intelligence::policy::{ANOMALIES_POLICY_V1, AnomaliesPolicy};
use crate::intelligence::types::NormalizedTransaction;

#[derive(Debug, Clone)]
pub struct AnomalyDetection {
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

#[derive(Debug, Clone)]
struct GroupedTransaction {
    merchant: String,
    quality_score: f64,
    rows: Vec<NormalizedTransaction>,
}

pub fn detect_anomalies(transactions: &[NormalizedTransaction]) -> Vec<AnomalyDetection> {
    detect_anomalies_with_policy(transactions, ANOMALIES_POLICY_V1)
}

fn detect_anomalies_with_policy(
    transactions: &[NormalizedTransaction],
    policy: AnomaliesPolicy,
) -> Vec<AnomalyDetection> {
    let mut groups: BTreeMap<String, GroupedTransaction> = BTreeMap::new();
    for transaction in transactions {
        let Some(counterparty) = counterparty_from_transaction(
            transaction.merchant.as_deref(),
            &transaction.description,
        ) else {
            continue;
        };

        let key = format!(
            "{}|{}|{}|{}",
            transaction.account_key,
            transaction.currency,
            transaction.amount_sign_key(),
            counterparty.key
        );

        let entry = groups.entry(key).or_insert_with(|| GroupedTransaction {
            merchant: counterparty.label.clone(),
            quality_score: counterparty.quality_score,
            rows: Vec::new(),
        });
        entry.rows.push(transaction.clone());
    }

    let mut anomalies = Vec::new();
    for group in groups.values_mut() {
        group.rows.sort_by(|left, right| {
            left.posted_at
                .cmp(&right.posted_at)
                .then_with(|| left.amount.total_cmp(&right.amount))
                .then_with(|| left.txn_id.cmp(&right.txn_id))
        });

        if group.rows.len() < policy.min_history_points {
            continue;
        }

        let abs_amounts = sorted_abs_amounts(&group.rows);
        let median_abs = median_f64(&abs_amounts).unwrap_or(0.0);
        if median_abs <= f64::EPSILON {
            continue;
        }

        let mad = median_absolute_deviation(&abs_amounts, median_abs);
        let tolerance = (policy.absolute_floor)
            .max(median_abs * policy.relative_floor)
            .max(mad * policy.mad_multiplier);

        for row in &group.rows {
            let absolute_amount = row.abs_amount();
            let delta = absolute_amount - median_abs;
            if delta <= tolerance {
                continue;
            }

            let spike_ratio = absolute_amount / median_abs;
            if spike_ratio < policy.min_spike_ratio {
                continue;
            }

            let delta_score = (delta / (tolerance * 2.5)).min(1.0);
            let ratio_score = ((spike_ratio - policy.min_spike_ratio) / 2.0).min(1.0);
            let score = round_to(
                (0.6 * delta_score) + (0.3 * ratio_score) + (0.1 * group.quality_score),
                4,
            );
            if score < policy.min_score {
                continue;
            }

            anomalies.push(AnomalyDetection {
                txn_id: row.txn_id.clone(),
                account_key: row.account_key.clone(),
                posted_at: row.posted_at.format("%Y-%m-%d").to_string(),
                merchant: group.merchant.clone(),
                amount: round_to(row.amount, 2),
                currency: row.currency.clone(),
                reason_code: "amount_spike".to_string(),
                reason: format!(
                    "Amount is unusually high for this merchant ({:.2}x typical).",
                    round_to(spike_ratio, 2)
                ),
                score,
                severity: severity_for_score(score).to_string(),
            });
        }
    }

    anomalies.sort_by(|left, right| {
        left.posted_at
            .cmp(&right.posted_at)
            .then_with(|| left.merchant.cmp(&right.merchant))
            .then_with(|| left.txn_id.cmp(&right.txn_id))
    });
    anomalies
}

fn sorted_abs_amounts(rows: &[NormalizedTransaction]) -> Vec<f64> {
    let mut values = rows
        .iter()
        .map(NormalizedTransaction::abs_amount)
        .collect::<Vec<f64>>();
    values.sort_by(|left, right| left.total_cmp(right));
    values
}

fn median_absolute_deviation(sorted_abs_amounts: &[f64], median_abs: f64) -> f64 {
    let mut deviations = sorted_abs_amounts
        .iter()
        .map(|value| (value - median_abs).abs())
        .collect::<Vec<f64>>();
    deviations.sort_by(|left, right| left.total_cmp(right));
    median_f64(&deviations).unwrap_or(0.0)
}

fn median_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        return Some((values[middle - 1] + values[middle]) / 2.0);
    }
    Some(values[middle])
}

fn severity_for_score(score: f64) -> &'static str {
    if score >= 0.92 {
        return "high";
    }
    if score >= 0.86 {
        return "medium";
    }
    "low"
}

fn round_to(value: f64, decimals: u32) -> f64 {
    let exponent = i32::try_from(decimals).unwrap_or(2);
    let factor = 10_f64.powi(exponent);
    (value * factor).round() / factor
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::intelligence::anomalies::detect_anomalies;
    use crate::intelligence::types::NormalizedTransaction;

    fn row(
        txn_id: &str,
        account_key: &str,
        posted_at: &str,
        amount: f64,
        merchant: &str,
    ) -> NormalizedTransaction {
        let posted_at_date = NaiveDate::parse_from_str(posted_at, "%Y-%m-%d")
            .ok()
            .or_else(|| NaiveDate::from_ymd_opt(2026, 1, 1))
            .unwrap_or(NaiveDate::MIN);
        NormalizedTransaction {
            txn_id: txn_id.to_string(),
            account_key: account_key.to_string(),
            posted_at: posted_at_date,
            amount,
            currency: "USD".to_string(),
            description: merchant.to_string(),
            merchant: Some(merchant.to_string()),
        }
    }

    #[test]
    fn detects_large_amount_spike_with_sufficient_history() {
        let rows = vec![
            row("txn_1", "acct_1", "2026-01-01", -20.0, "Fresh Mart"),
            row("txn_2", "acct_1", "2026-01-08", -21.0, "Fresh Mart"),
            row("txn_3", "acct_1", "2026-01-15", -19.5, "Fresh Mart"),
            row("txn_4", "acct_1", "2026-01-22", -20.5, "Fresh Mart"),
            row("txn_5", "acct_1", "2026-01-29", -21.5, "Fresh Mart"),
            row("txn_6", "acct_1", "2026-02-05", -240.0, "Fresh Mart"),
        ];

        let anomalies = detect_anomalies(&rows);
        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].txn_id, "txn_6".to_string());
        assert_eq!(anomalies[0].reason_code, "amount_spike".to_string());
    }

    #[test]
    fn does_not_flag_stable_amount_series() {
        let rows = vec![
            row("txn_1", "acct_1", "2026-01-01", -35.0, "Utilities"),
            row("txn_2", "acct_1", "2026-02-01", -36.0, "Utilities"),
            row("txn_3", "acct_1", "2026-03-01", -34.0, "Utilities"),
            row("txn_4", "acct_1", "2026-04-01", -35.5, "Utilities"),
            row("txn_5", "acct_1", "2026-05-01", -34.5, "Utilities"),
            row("txn_6", "acct_1", "2026-06-01", -36.0, "Utilities"),
        ];

        let anomalies = detect_anomalies(&rows);
        assert!(anomalies.is_empty());
    }
}
