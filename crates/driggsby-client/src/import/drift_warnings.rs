use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::contracts::types::{ImportDriftWarning, ImportKeyInventory};
use crate::import::inventory::IncomingUniqueValues;
use crate::import::sign_profiles::SignCounts;

const SEVERITY_HIGH: &str = "high";
const SEVERITY_MEDIUM: &str = "medium";
const SIGN_DRIFT_THRESHOLD: f64 = 0.40;
const MIN_EXISTING_SIGN_SAMPLE: i64 = 20;
const MIN_INCOMING_SIGN_SAMPLE: i64 = 5;

pub(crate) fn build_drift_warnings(
    key_inventory: &ImportKeyInventory,
    incoming_values: &IncomingUniqueValues,
    existing_sign_counts: &BTreeMap<String, SignCounts>,
    incoming_sign_counts: &BTreeMap<String, SignCounts>,
) -> Vec<ImportDriftWarning> {
    let mut warnings = Vec::new();

    if key_inventory.account_key.unique_count > 0 {
        warnings.extend(account_key_warnings(
            &key_inventory.account_key.existing_values,
            &incoming_values.account_key,
        ));
    }

    if key_inventory.currency.unique_count > 0 {
        warnings.extend(unseen_value_warnings(
            "currency",
            "currency_unseen",
            "Incoming currency was not found in existing ledger history.",
            SEVERITY_MEDIUM,
            &key_inventory.currency.existing_values,
            &incoming_values.currency,
        ));
    }

    if key_inventory.merchant.unique_count > 0 {
        warnings.extend(unseen_value_warnings(
            "merchant",
            "merchant_unseen",
            "Incoming merchant was not found in existing ledger history.",
            SEVERITY_MEDIUM,
            &key_inventory.merchant.existing_values,
            &incoming_values.merchant,
        ));
    }

    if key_inventory.category.unique_count > 0 {
        warnings.extend(unseen_value_warnings(
            "category",
            "category_unseen",
            "Incoming category was not found in existing ledger history.",
            SEVERITY_MEDIUM,
            &key_inventory.category.existing_values,
            &incoming_values.category,
        ));
    }

    warnings.extend(sign_profile_anomaly_warnings(
        existing_sign_counts,
        incoming_sign_counts,
    ));

    warnings.sort_by(compare_warnings);
    warnings
}

fn account_key_warnings(
    existing_values: &[String],
    incoming_values: &BTreeSet<String>,
) -> Vec<ImportDriftWarning> {
    let existing_set = existing_values
        .iter()
        .cloned()
        .collect::<BTreeSet<String>>();
    let mut warnings = Vec::new();

    for incoming_value in incoming_values {
        if existing_set.contains(incoming_value) {
            continue;
        }

        warnings.push(ImportDriftWarning {
            code: "account_key_unseen".to_string(),
            severity: SEVERITY_HIGH.to_string(),
            property: "account_key".to_string(),
            incoming_value: incoming_value.clone(),
            message: "Incoming account_key was not found in existing ledger history.".to_string(),
            suggestions: Vec::new(),
        });

        let suggestions = nearest_account_key_suggestions(incoming_value, existing_values);
        if !suggestions.is_empty() {
            warnings.push(ImportDriftWarning {
                code: "account_key_possible_typo".to_string(),
                severity: SEVERITY_HIGH.to_string(),
                property: "account_key".to_string(),
                incoming_value: incoming_value.clone(),
                message: format!(
                    "Incoming account_key `{incoming_value}` is close to an existing account key."
                ),
                suggestions,
            });
        }
    }

    warnings
}

fn unseen_value_warnings(
    property: &str,
    code: &str,
    message: &str,
    severity: &str,
    existing_values: &[String],
    incoming_values: &BTreeSet<String>,
) -> Vec<ImportDriftWarning> {
    let existing_set = existing_values
        .iter()
        .cloned()
        .collect::<BTreeSet<String>>();

    incoming_values
        .iter()
        .filter(|value| !existing_set.contains(*value))
        .map(|incoming_value| ImportDriftWarning {
            code: code.to_string(),
            severity: severity.to_string(),
            property: property.to_string(),
            incoming_value: incoming_value.clone(),
            message: message.to_string(),
            suggestions: Vec::new(),
        })
        .collect()
}

fn sign_profile_anomaly_warnings(
    existing_sign_counts: &BTreeMap<String, SignCounts>,
    incoming_sign_counts: &BTreeMap<String, SignCounts>,
) -> Vec<ImportDriftWarning> {
    let mut warnings = Vec::new();

    for (account_key, incoming_counts) in incoming_sign_counts {
        let Some(existing_counts) = existing_sign_counts.get(account_key) else {
            continue;
        };

        if existing_counts.total_count() < MIN_EXISTING_SIGN_SAMPLE
            || incoming_counts.total_count() < MIN_INCOMING_SIGN_SAMPLE
        {
            continue;
        }

        let historical_ratio = existing_counts.negative_ratio();
        let incoming_ratio = incoming_counts.negative_ratio();
        let diff = (historical_ratio - incoming_ratio).abs();

        if diff < SIGN_DRIFT_THRESHOLD {
            continue;
        }

        warnings.push(ImportDriftWarning {
            code: "account_sign_profile_anomaly".to_string(),
            severity: SEVERITY_HIGH.to_string(),
            property: "account_key".to_string(),
            incoming_value: account_key.clone(),
            message: format!(
                "Incoming amount sign profile for `{account_key}` differs from history by {:.2} percentage points (historical {:.2}, incoming {:.2}).",
                diff * 100.0,
                historical_ratio * 100.0,
                incoming_ratio * 100.0
            ),
            suggestions: Vec::new(),
        });
    }

    warnings
}

fn nearest_account_key_suggestions(
    incoming_value: &str,
    existing_values: &[String],
) -> Vec<String> {
    let mut ranked = existing_values
        .iter()
        .map(|candidate| {
            (
                levenshtein_distance(&incoming_value.to_lowercase(), &candidate.to_lowercase()),
                candidate.clone(),
            )
        })
        .filter(|(distance, _)| *distance <= 3)
        .collect::<Vec<(usize, String)>>();

    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    ranked.into_iter().take(3).map(|(_, value)| value).collect()
}

fn compare_warnings(left: &ImportDriftWarning, right: &ImportDriftWarning) -> Ordering {
    severity_rank(&left.severity)
        .cmp(&severity_rank(&right.severity))
        .then_with(|| left.property.cmp(&right.property))
        .then_with(|| left.incoming_value.cmp(&right.incoming_value))
        .then_with(|| left.code.cmp(&right.code))
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        SEVERITY_HIGH => 0,
        SEVERITY_MEDIUM => 1,
        _ => 2,
    }
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }

    if left.is_empty() {
        return right.chars().count();
    }

    if right.is_empty() {
        return left.chars().count();
    }

    let right_chars = right.chars().collect::<Vec<char>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<usize>>();

    for (left_index, left_char) in left.chars().enumerate() {
        let mut current = vec![left_index + 1];

        for (right_index, right_char) in right_chars.iter().enumerate() {
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            let substitution = previous[right_index] + usize::from(left_char != *right_char);
            current.push(insertion.min(deletion).min(substitution));
        }

        previous = current;
    }

    previous[right_chars.len()]
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::contracts::types::{ImportKeyInventory, ImportPropertyInventory};
    use crate::import::inventory::IncomingUniqueValues;
    use crate::import::sign_profiles::SignCounts;

    use super::build_drift_warnings;

    #[test]
    fn warning_engine_captures_unseen_values_typo_and_sign_drift() {
        let inventory = ImportKeyInventory {
            account_key: ImportPropertyInventory {
                property: "account_key".to_string(),
                existing_values: vec!["chase_checking_1234".to_string()],
                value_counts: Vec::new(),
                unique_count: 1,
                null_count: 0,
                total_rows: 25,
            },
            account_type: ImportPropertyInventory {
                property: "account_type".to_string(),
                existing_values: vec!["checking".to_string()],
                value_counts: Vec::new(),
                unique_count: 1,
                null_count: 0,
                total_rows: 25,
            },
            currency: ImportPropertyInventory {
                property: "currency".to_string(),
                existing_values: vec!["USD".to_string()],
                value_counts: Vec::new(),
                unique_count: 1,
                null_count: 0,
                total_rows: 25,
            },
            merchant: ImportPropertyInventory {
                property: "merchant".to_string(),
                existing_values: vec!["Existing Merchant".to_string()],
                value_counts: Vec::new(),
                unique_count: 1,
                null_count: 0,
                total_rows: 25,
            },
            category: ImportPropertyInventory {
                property: "category".to_string(),
                existing_values: vec!["Groceries".to_string()],
                value_counts: Vec::new(),
                unique_count: 1,
                null_count: 0,
                total_rows: 25,
            },
        };

        let incoming_values = IncomingUniqueValues {
            account_key: BTreeSet::from([
                "chase_checking_1234".to_string(),
                "chase_checkng_1234".to_string(),
            ]),
            account_type: BTreeSet::from(["checking".to_string()]),
            currency: BTreeSet::from(["EUR".to_string(), "USD".to_string()]),
            merchant: BTreeSet::from(["Existing Merchant".to_string(), "New Merchant".to_string()]),
            category: BTreeSet::from(["Groceries".to_string(), "Travel".to_string()]),
        };

        let existing_sign_counts = BTreeMap::from([(
            "chase_checking_1234".to_string(),
            SignCounts {
                negative_count: 24,
                positive_count: 1,
            },
        )]);
        let incoming_sign_counts = BTreeMap::from([(
            "chase_checking_1234".to_string(),
            SignCounts {
                negative_count: 0,
                positive_count: 5,
            },
        )]);

        let warnings = build_drift_warnings(
            &inventory,
            &incoming_values,
            &existing_sign_counts,
            &incoming_sign_counts,
        );

        let codes = warnings
            .iter()
            .map(|warning| warning.code.clone())
            .collect::<Vec<String>>();

        assert!(codes.iter().any(|code| code == "account_key_unseen"));
        assert!(codes.iter().any(|code| code == "account_key_possible_typo"));
        assert!(codes.iter().any(|code| code == "currency_unseen"));
        assert!(codes.iter().any(|code| code == "merchant_unseen"));
        assert!(codes.iter().any(|code| code == "category_unseen"));
        assert!(
            codes
                .iter()
                .any(|code| code == "account_sign_profile_anomaly")
        );

        let typo_warning = warnings
            .iter()
            .find(|warning| warning.code == "account_key_possible_typo");
        assert!(typo_warning.is_some());
        if let Some(warning) = typo_warning {
            assert_eq!(warning.suggestions, vec!["chase_checking_1234".to_string()]);
        }
    }

    #[test]
    fn warning_engine_respects_sign_threshold_and_sample_size_gates() {
        let inventory = ImportKeyInventory {
            account_key: ImportPropertyInventory {
                property: "account_key".to_string(),
                existing_values: vec!["acct_1".to_string()],
                value_counts: Vec::new(),
                unique_count: 1,
                null_count: 0,
                total_rows: 20,
            },
            account_type: ImportPropertyInventory {
                property: "account_type".to_string(),
                existing_values: vec!["checking".to_string()],
                value_counts: Vec::new(),
                unique_count: 1,
                null_count: 0,
                total_rows: 20,
            },
            currency: ImportPropertyInventory {
                property: "currency".to_string(),
                existing_values: vec!["USD".to_string()],
                value_counts: Vec::new(),
                unique_count: 1,
                null_count: 0,
                total_rows: 20,
            },
            merchant: ImportPropertyInventory {
                property: "merchant".to_string(),
                existing_values: vec!["Shop".to_string()],
                value_counts: Vec::new(),
                unique_count: 1,
                null_count: 0,
                total_rows: 20,
            },
            category: ImportPropertyInventory {
                property: "category".to_string(),
                existing_values: vec!["Groceries".to_string()],
                value_counts: Vec::new(),
                unique_count: 1,
                null_count: 0,
                total_rows: 20,
            },
        };
        let incoming_values = IncomingUniqueValues {
            account_key: BTreeSet::from(["acct_1".to_string()]),
            account_type: BTreeSet::from(["checking".to_string()]),
            currency: BTreeSet::from(["USD".to_string()]),
            merchant: BTreeSet::from(["Shop".to_string()]),
            category: BTreeSet::from(["Groceries".to_string()]),
        };

        let existing_sign_counts = BTreeMap::from([(
            "acct_1".to_string(),
            SignCounts {
                negative_count: 16,
                positive_count: 4,
            },
        )]);

        let below_threshold = BTreeMap::from([(
            "acct_1".to_string(),
            SignCounts {
                negative_count: 3,
                positive_count: 2,
            },
        )]);
        let warnings = build_drift_warnings(
            &inventory,
            &incoming_values,
            &existing_sign_counts,
            &below_threshold,
        );
        assert!(
            !warnings
                .iter()
                .any(|warning| warning.code == "account_sign_profile_anomaly")
        );

        let boundary_threshold = BTreeMap::from([(
            "acct_1".to_string(),
            SignCounts {
                negative_count: 2,
                positive_count: 3,
            },
        )]);
        let warnings = build_drift_warnings(
            &inventory,
            &incoming_values,
            &existing_sign_counts,
            &boundary_threshold,
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.code == "account_sign_profile_anomaly")
        );

        let incoming_too_small = BTreeMap::from([(
            "acct_1".to_string(),
            SignCounts {
                negative_count: 2,
                positive_count: 2,
            },
        )]);
        let warnings = build_drift_warnings(
            &inventory,
            &incoming_values,
            &existing_sign_counts,
            &incoming_too_small,
        );
        assert!(
            !warnings
                .iter()
                .any(|warning| warning.code == "account_sign_profile_anomaly")
        );
    }

    #[test]
    fn warning_engine_suppresses_unseen_warnings_without_baseline_history() {
        let empty_inventory = ImportKeyInventory {
            account_key: ImportPropertyInventory {
                property: "account_key".to_string(),
                existing_values: Vec::new(),
                value_counts: Vec::new(),
                unique_count: 0,
                null_count: 0,
                total_rows: 0,
            },
            account_type: ImportPropertyInventory {
                property: "account_type".to_string(),
                existing_values: Vec::new(),
                value_counts: Vec::new(),
                unique_count: 0,
                null_count: 0,
                total_rows: 0,
            },
            currency: ImportPropertyInventory {
                property: "currency".to_string(),
                existing_values: Vec::new(),
                value_counts: Vec::new(),
                unique_count: 0,
                null_count: 0,
                total_rows: 0,
            },
            merchant: ImportPropertyInventory {
                property: "merchant".to_string(),
                existing_values: Vec::new(),
                value_counts: Vec::new(),
                unique_count: 0,
                null_count: 0,
                total_rows: 0,
            },
            category: ImportPropertyInventory {
                property: "category".to_string(),
                existing_values: Vec::new(),
                value_counts: Vec::new(),
                unique_count: 0,
                null_count: 0,
                total_rows: 0,
            },
        };
        let incoming_values = IncomingUniqueValues {
            account_key: BTreeSet::from(["acct_new".to_string()]),
            account_type: BTreeSet::from(["checking".to_string()]),
            currency: BTreeSet::from(["USD".to_string()]),
            merchant: BTreeSet::from(["New Merchant".to_string()]),
            category: BTreeSet::from(["New Category".to_string()]),
        };

        let warnings = build_drift_warnings(
            &empty_inventory,
            &incoming_values,
            &BTreeMap::new(),
            &BTreeMap::new(),
        );

        assert!(warnings.is_empty());
    }
}
