use crate::intelligence::date::CadenceKind;

/// Deterministic recurring-classification policy identifier.
///
/// This is emitted with recurring results so future threshold changes remain
/// auditable and easy to reason about in diffs and support/debug sessions.
pub const RECURRING_POLICY_VERSION: &str = "recurring/v1";

/// v1 recurring classifier policy.
///
/// Notes:
/// - Thresholds are intentionally conservative (precision-first).
/// - `min_score` is a frozen bootstrap threshold for v1 and should only change
///   with explicit backtest evidence in a future policy version.
#[derive(Debug, Clone, Copy)]
pub struct RecurringPolicy {
    pub cadence_weight: f64,
    pub amount_weight: f64,
    pub counterparty_weight: f64,
    pub min_cadence_fit: f64,
    pub min_amount_fit: f64,
    pub min_score: f64,
    pub amount_tolerance_ratio: f64,
    pub amount_tolerance_floor: f64,
}

impl RecurringPolicy {
    pub fn score(self, cadence_fit: f64, amount_fit: f64, counterparty_quality: f64) -> f64 {
        (self.cadence_weight * cadence_fit)
            + (self.amount_weight * amount_fit)
            + (self.counterparty_weight * counterparty_quality)
    }

    pub fn passes_hard_gates(self, cadence_fit: f64, amount_fit: f64, score: f64) -> bool {
        cadence_fit >= self.min_cadence_fit
            && amount_fit >= self.min_amount_fit
            && score >= self.min_score
    }

    pub fn amount_tolerance(self, median_abs_amount: f64) -> f64 {
        (median_abs_amount * self.amount_tolerance_ratio).max(self.amount_tolerance_floor)
    }

    pub fn cadence_min_occurrences(self, cadence: CadenceKind) -> usize {
        match cadence {
            CadenceKind::Monthly => 3,
            CadenceKind::Weekly | CadenceKind::Biweekly => 4,
        }
    }

    pub fn cadence_tolerance_days(self, cadence: CadenceKind) -> i64 {
        match cadence {
            CadenceKind::Weekly => 1,
            CadenceKind::Biweekly => 2,
            CadenceKind::Monthly => 3,
        }
    }

    pub fn cadence_priority(self, cadence: CadenceKind) -> i8 {
        match cadence {
            CadenceKind::Weekly => 1,
            CadenceKind::Biweekly => 2,
            CadenceKind::Monthly => 3,
        }
    }

    pub fn cadence_active_window_days(self, cadence: CadenceKind) -> i64 {
        match cadence {
            CadenceKind::Weekly => 14,
            CadenceKind::Biweekly => 28,
            CadenceKind::Monthly => 62,
        }
    }
}

pub const RECURRING_POLICY_V1: RecurringPolicy = RecurringPolicy {
    cadence_weight: 0.65,
    amount_weight: 0.25,
    counterparty_weight: 0.10,
    min_cadence_fit: 0.75,
    min_amount_fit: 0.75,
    min_score: 0.78,
    amount_tolerance_ratio: 0.15,
    amount_tolerance_floor: 1.00,
};

#[cfg(test)]
mod tests {
    use crate::intelligence::policy::RECURRING_POLICY_V1;

    #[test]
    fn policy_weights_sum_to_one() {
        let sum = RECURRING_POLICY_V1.cadence_weight
            + RECURRING_POLICY_V1.amount_weight
            + RECURRING_POLICY_V1.counterparty_weight;
        assert!((sum - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn hard_gate_thresholds_use_inclusive_comparisons() {
        let policy = RECURRING_POLICY_V1;
        let exact_score = policy.min_score;
        assert!(policy.passes_hard_gates(
            policy.min_cadence_fit,
            policy.min_amount_fit,
            exact_score
        ));
        assert!(!policy.passes_hard_gates(
            policy.min_cadence_fit - 0.0001,
            policy.min_amount_fit,
            exact_score
        ));
        assert!(!policy.passes_hard_gates(
            policy.min_cadence_fit,
            policy.min_amount_fit - 0.0001,
            exact_score
        ));
        assert!(!policy.passes_hard_gates(
            policy.min_cadence_fit,
            policy.min_amount_fit,
            policy.min_score - 0.0001
        ));
    }

    #[test]
    fn composite_score_below_threshold_fails_even_when_fit_gates_pass() {
        let policy = RECURRING_POLICY_V1;
        let cadence_fit = policy.min_cadence_fit;
        let amount_fit = policy.min_amount_fit;
        let counterparty_quality = 0.0;
        let score = policy.score(cadence_fit, amount_fit, counterparty_quality);
        assert!(score < policy.min_score);
        assert!(!policy.passes_hard_gates(cadence_fit, amount_fit, score));
    }
}
