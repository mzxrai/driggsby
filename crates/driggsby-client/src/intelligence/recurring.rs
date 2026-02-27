use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;

use crate::intelligence::date::CadenceKind;
use crate::intelligence::normalize::{
    CounterpartyIdentity, CounterpartySource, counterparty_from_transaction,
};
use crate::intelligence::policy::{RECURRING_POLICY_V1, RecurringPolicy};
use crate::intelligence::types::NormalizedTransaction;

#[derive(Debug, Clone)]
pub struct RecurringDetection {
    pub group_key: String,
    pub account_key: String,
    pub counterparty: String,
    pub counterparty_source: CounterpartySource,
    pub cadence: CadenceKind,
    pub typical_amount: f64,
    pub currency: String,
    pub first_seen_at: NaiveDate,
    pub last_seen_at: NaiveDate,
    pub next_expected_at: Option<NaiveDate>,
    pub occurrence_count: i64,
    pub cadence_fit: f64,
    pub amount_fit: f64,
    pub score: f64,
    pub amount_min: f64,
    pub amount_max: f64,
    pub sample_description: String,
    pub quality_flags: Vec<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
struct RecurringGroup {
    group_key: String,
    account_key: String,
    currency: String,
    counterparty: CounterpartyIdentity,
    rows: Vec<NormalizedTransaction>,
}

#[derive(Debug, Clone)]
struct CandidateScore {
    cadence: CadenceKind,
    cadence_fit: f64,
    median_interval_error: i64,
    occurrence_count: usize,
    amount_fit: f64,
    score: f64,
}

#[derive(Debug, Clone)]
struct AmountStats {
    fit: f64,
    typical_amount: f64,
    amount_min: f64,
    amount_max: f64,
}

pub fn detect_recurring(transactions: &[NormalizedTransaction]) -> Vec<RecurringDetection> {
    detect_recurring_with_policy(transactions, RECURRING_POLICY_V1)
}

fn detect_recurring_with_policy(
    transactions: &[NormalizedTransaction],
    policy: RecurringPolicy,
) -> Vec<RecurringDetection> {
    let mut groups: BTreeMap<String, RecurringGroup> = BTreeMap::new();
    for transaction in transactions {
        let Some(counterparty) = counterparty_from_transaction(
            transaction.merchant.as_deref(),
            &transaction.description,
        ) else {
            continue;
        };

        let group_key = format!(
            "{}|{}|{}|{}",
            transaction.account_key,
            transaction.currency,
            transaction.amount_sign_key(),
            counterparty.key
        );

        let entry = groups
            .entry(group_key.clone())
            .or_insert_with(|| RecurringGroup {
                group_key: group_key.clone(),
                account_key: transaction.account_key.clone(),
                currency: transaction.currency.clone(),
                counterparty: counterparty.clone(),
                rows: Vec::new(),
            });
        entry.rows.push(transaction.clone());
    }

    let global_latest = transactions.iter().map(|row| row.posted_at).max();
    let mut detections: Vec<RecurringDetection> = Vec::new();

    for group in groups.values_mut() {
        group.rows.sort_by(|left, right| {
            left.posted_at
                .cmp(&right.posted_at)
                .then_with(|| left.amount.total_cmp(&right.amount))
                .then_with(|| left.description.cmp(&right.description))
        });

        if group.rows.is_empty() {
            continue;
        }
        if group.counterparty.source == CounterpartySource::Description
            && !group.counterparty.fallback_eligible
        {
            continue;
        }

        let amount_stats = compute_amount_stats(&group.rows, policy);
        let mut candidates: Vec<CandidateScore> = Vec::new();
        for cadence in [
            CadenceKind::Weekly,
            CadenceKind::Biweekly,
            CadenceKind::Monthly,
        ] {
            let Some(candidate) = score_candidate(cadence, group, &amount_stats, policy) else {
                continue;
            };
            candidates.push(candidate);
        }

        let Some(best) = select_best_candidate(&candidates) else {
            continue;
        };

        let first_seen = group.rows[0].posted_at;
        let last_seen = group.rows[group.rows.len() - 1].posted_at;
        let next_expected = Some(best.cadence.advance(last_seen));
        let sample_description = group.rows[0].description.clone();

        let mut quality_flags = group.counterparty.quality_flags.clone();
        if best.cadence_fit < 1.0 {
            quality_flags.push("cadence_variance".to_string());
        }
        if best.amount_fit < 1.0 {
            quality_flags.push("amount_variance".to_string());
        }
        let mut unique_flags = BTreeSet::new();
        for flag in quality_flags {
            unique_flags.insert(flag);
        }

        let is_active = if let Some(latest) = global_latest {
            let age_days = (latest - last_seen).num_days();
            age_days <= policy.cadence_active_window_days(best.cadence) * 2
        } else {
            true
        };

        detections.push(RecurringDetection {
            group_key: group.group_key.clone(),
            account_key: group.account_key.clone(),
            counterparty: group.counterparty.label.clone(),
            counterparty_source: group.counterparty.source,
            cadence: best.cadence,
            typical_amount: round_to(amount_stats.typical_amount, 2),
            currency: group.currency.clone(),
            first_seen_at: first_seen,
            last_seen_at: last_seen,
            next_expected_at: next_expected,
            occurrence_count: i64::try_from(best.occurrence_count).unwrap_or(0),
            cadence_fit: round_to(best.cadence_fit, 4),
            amount_fit: round_to(best.amount_fit, 4),
            score: round_to(best.score, 4),
            amount_min: round_to(amount_stats.amount_min, 2),
            amount_max: round_to(amount_stats.amount_max, 2),
            sample_description,
            quality_flags: unique_flags.into_iter().collect(),
            is_active,
        });
    }

    detections.sort_by(compare_detections);
    detections
}

fn score_candidate(
    cadence: CadenceKind,
    group: &RecurringGroup,
    amount_stats: &AmountStats,
    policy: RecurringPolicy,
) -> Option<CandidateScore> {
    if group.rows.len() < policy.cadence_min_occurrences(cadence) {
        return None;
    }

    let (cadence_fit, median_interval_error) = cadence_fit(group, cadence, policy);
    let score = policy.score(
        cadence_fit,
        amount_stats.fit,
        group.counterparty.quality_score,
    );
    if !policy.passes_hard_gates(cadence_fit, amount_stats.fit, score) {
        return None;
    }

    Some(CandidateScore {
        cadence,
        cadence_fit,
        median_interval_error,
        occurrence_count: group.rows.len(),
        amount_fit: amount_stats.fit,
        score,
    })
}

fn select_best_candidate(candidates: &[CandidateScore]) -> Option<CandidateScore> {
    let mut sorted = candidates.to_vec();
    sorted.sort_by(compare_candidate_scores);
    sorted.into_iter().next()
}

fn compare_candidate_scores(left: &CandidateScore, right: &CandidateScore) -> Ordering {
    right
        .cadence_fit
        .total_cmp(&left.cadence_fit)
        .then_with(|| left.median_interval_error.cmp(&right.median_interval_error))
        .then_with(|| right.occurrence_count.cmp(&left.occurrence_count))
        .then_with(|| {
            RECURRING_POLICY_V1
                .cadence_priority(right.cadence)
                .cmp(&RECURRING_POLICY_V1.cadence_priority(left.cadence))
        })
}

fn compare_detections(left: &RecurringDetection, right: &RecurringDetection) -> Ordering {
    compare_optional_dates(left.next_expected_at, right.next_expected_at)
        .then_with(|| right.score.total_cmp(&left.score))
        .then_with(|| left.counterparty.cmp(&right.counterparty))
        .then_with(|| left.group_key.cmp(&right.group_key))
}

fn compare_optional_dates(left: Option<NaiveDate>, right: Option<NaiveDate>) -> Ordering {
    match (left, right) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn cadence_fit(
    group: &RecurringGroup,
    cadence: CadenceKind,
    policy: RecurringPolicy,
) -> (f64, i64) {
    if group.rows.len() < 2 {
        return (0.0, i64::MAX);
    }

    let mut matches = 0usize;
    let mut errors: Vec<i64> = Vec::new();
    for index in 1..group.rows.len() {
        let previous = group.rows[index - 1].posted_at;
        let current = group.rows[index].posted_at;
        let error = cadence_interval_error(previous, current, cadence);
        if error <= policy.cadence_tolerance_days(cadence) {
            matches += 1;
        }
        errors.push(error);
    }

    let total_intervals = group.rows.len() - 1;
    let fit = (matches as f64) / (total_intervals as f64);
    let median_error = median_i64(&errors).unwrap_or(i64::MAX);
    (fit, median_error)
}

fn cadence_interval_error(previous: NaiveDate, current: NaiveDate, cadence: CadenceKind) -> i64 {
    match cadence {
        CadenceKind::Monthly => {
            let expected = cadence.advance(previous);
            (current - expected).num_days().abs()
        }
        CadenceKind::Weekly | CadenceKind::Biweekly => {
            let actual = (current - previous).num_days().abs();
            (actual - cadence.expected_interval_days()).abs()
        }
    }
}

fn compute_amount_stats(rows: &[NormalizedTransaction], policy: RecurringPolicy) -> AmountStats {
    let mut absolute_amounts: Vec<f64> =
        rows.iter().map(NormalizedTransaction::abs_amount).collect();
    absolute_amounts.sort_by(|left, right| left.total_cmp(right));
    let median_abs = median_f64(&absolute_amounts).unwrap_or(0.0);
    let tolerance = policy.amount_tolerance(median_abs);

    let in_tolerance = rows
        .iter()
        .filter(|row| (row.abs_amount() - median_abs).abs() <= tolerance)
        .count();

    let mut signed_amounts: Vec<f64> = rows.iter().map(|row| row.amount).collect();
    signed_amounts.sort_by(|left, right| left.total_cmp(right));
    let typical_amount = median_f64(&signed_amounts).unwrap_or(0.0);
    let amount_min = signed_amounts.first().copied().unwrap_or(0.0);
    let amount_max = signed_amounts.last().copied().unwrap_or(0.0);

    AmountStats {
        fit: (in_tolerance as f64) / (rows.len() as f64),
        typical_amount,
        amount_min,
        amount_max,
    }
}

fn median_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        return Some((values[mid - 1] + values[mid]) / 2.0);
    }
    Some(values[mid])
}

fn median_i64(values: &[i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        return Some((sorted[mid - 1] + sorted[mid]) / 2);
    }
    Some(sorted[mid])
}

fn round_to(value: f64, decimals: u32) -> f64 {
    let exponent = i32::try_from(decimals).unwrap_or(2);
    let factor = 10_f64.powi(exponent);
    (value * factor).round() / factor
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use chrono::NaiveDate;

    use crate::intelligence::date::CadenceKind;
    use crate::intelligence::types::NormalizedTransaction;

    use super::{compare_candidate_scores, detect_recurring};

    fn row(
        account_key: &str,
        date: &str,
        amount: f64,
        currency: &str,
        description: &str,
        merchant: Option<&str>,
    ) -> NormalizedTransaction {
        let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d");
        assert!(parsed.is_ok());
        NormalizedTransaction {
            account_key: account_key.to_string(),
            posted_at: parsed.unwrap_or(NaiveDate::MIN),
            amount,
            currency: currency.to_string(),
            description: description.to_string(),
            merchant: merchant.map(std::string::ToString::to_string),
        }
    }

    #[test]
    fn grouping_key_is_sign_and_currency_sensitive() {
        let input = vec![
            row(
                "acct",
                "2026-01-01",
                -10.0,
                "USD",
                "MONTHLY PLAN",
                Some("Plan Co"),
            ),
            row(
                "acct",
                "2026-02-01",
                -10.0,
                "USD",
                "MONTHLY PLAN",
                Some("Plan Co"),
            ),
            row(
                "acct",
                "2026-03-01",
                -10.0,
                "USD",
                "MONTHLY PLAN",
                Some("Plan Co"),
            ),
            row(
                "acct",
                "2026-01-02",
                10.0,
                "USD",
                "MONTHLY PLAN",
                Some("Plan Co"),
            ),
            row(
                "acct",
                "2026-02-02",
                10.0,
                "USD",
                "MONTHLY PLAN",
                Some("Plan Co"),
            ),
            row(
                "acct",
                "2026-03-02",
                10.0,
                "USD",
                "MONTHLY PLAN",
                Some("Plan Co"),
            ),
        ];
        let recurring = detect_recurring(&input);
        assert!(recurring.len() >= 2);
    }

    #[test]
    fn weekly_detection_requires_min_occurrences() {
        let input = vec![
            row(
                "acct",
                "2026-01-01",
                -5.0,
                "USD",
                "WEEKLY PLAN",
                Some("Weekly Co"),
            ),
            row(
                "acct",
                "2026-01-08",
                -5.0,
                "USD",
                "WEEKLY PLAN",
                Some("Weekly Co"),
            ),
            row(
                "acct",
                "2026-01-15",
                -5.0,
                "USD",
                "WEEKLY PLAN",
                Some("Weekly Co"),
            ),
            row(
                "acct",
                "2026-01-22",
                -5.0,
                "USD",
                "WEEKLY PLAN",
                Some("Weekly Co"),
            ),
        ];
        let recurring = detect_recurring(&input);
        assert_eq!(recurring[0].cadence.as_str(), "weekly");
    }

    #[test]
    fn volatile_amounts_are_filtered_out() {
        let input = vec![
            row(
                "acct",
                "2026-01-01",
                -5.0,
                "USD",
                "UTILITY",
                Some("Grid Co"),
            ),
            row(
                "acct",
                "2026-02-01",
                -100.0,
                "USD",
                "UTILITY",
                Some("Grid Co"),
            ),
            row(
                "acct",
                "2026-03-01",
                -10.0,
                "USD",
                "UTILITY",
                Some("Grid Co"),
            ),
        ];
        let recurring = detect_recurring(&input);
        assert!(recurring.is_empty());
    }

    #[test]
    fn cadence_tie_break_prefers_higher_priority() {
        let left = super::CandidateScore {
            cadence: CadenceKind::Weekly,
            cadence_fit: 0.9,
            median_interval_error: 0,
            occurrence_count: 4,
            amount_fit: 1.0,
            score: 0.89,
        };
        let right = super::CandidateScore {
            cadence: CadenceKind::Monthly,
            cadence_fit: 0.9,
            median_interval_error: 0,
            occurrence_count: 4,
            amount_fit: 1.0,
            score: 0.89,
        };
        assert_eq!(compare_candidate_scores(&left, &right), Ordering::Greater);
    }
}
