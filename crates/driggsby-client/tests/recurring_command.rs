mod support;

use driggsby_client::commands::anomalies::{self, AnomaliesRunOptions};
use driggsby_client::commands::recurring::{self, RecurringRunOptions};
use serde_json::Value;
use support::recurring_testkit::{
    import_rows, recurring_group_exists, recurring_payload, run_scenario, temp_home_in_tmp,
    transaction,
};

#[test]
fn recurring_rejects_invalid_date_ranges_with_invalid_argument() {
    let temp = temp_home_in_tmp("driggsby-recurring-range");
    assert!(temp.is_ok());
    if let Ok((_dir, home)) = temp {
        let result = recurring::run_with_options(RecurringRunOptions {
            from: Some("2026-03-01".to_string()),
            to: Some("2026-02-01".to_string()),
            home_override: Some(&home),
        });
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("from"));
        }
    }
}

#[test]
fn anomalies_reject_invalid_calendar_dates_using_shared_validation() {
    let temp = temp_home_in_tmp("driggsby-anomalies-date");
    assert!(temp.is_ok());
    if let Ok((_dir, home)) = temp {
        let result = anomalies::run_with_options(AnomaliesRunOptions {
            from: Some("2026-02-31".to_string()),
            to: None,
            home_override: Some(&home),
        });
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code, "invalid_argument");
            assert!(error.message.contains("calendar"));
        }
    }
}

#[test]
fn recurring_detects_monthly_pattern_and_emits_evidence_fields() {
    let rows = vec![
        transaction(
            "acct_checking",
            "2026-01-05",
            -15.99,
            "USD",
            "NETFLIX SUBSCRIPTION",
            Some("Netflix"),
        ),
        transaction(
            "acct_checking",
            "2026-02-05",
            -15.99,
            "USD",
            "NETFLIX SUBSCRIPTION",
            Some("Netflix"),
        ),
        transaction(
            "acct_checking",
            "2026-03-05",
            -15.99,
            "USD",
            "NETFLIX SUBSCRIPTION",
            Some("Netflix"),
        ),
        transaction(
            "acct_checking",
            "2026-04-05",
            -15.99,
            "USD",
            "NETFLIX SUBSCRIPTION",
            Some("Netflix"),
        ),
    ];

    let patterns = run_scenario(&rows, None, None);
    assert!(!patterns.is_empty());

    let row = &patterns[0];
    assert!(row["group_key"].is_string());
    assert!(row["account_key"].is_string());
    assert!(row["merchant"].is_string());
    assert!(row["counterparty"].is_string());
    assert!(row["counterparty_source"].is_string());
    assert!(row["cadence"].is_string());
    assert!(row["typical_amount"].is_f64());
    assert!(row["currency"].is_string());
    assert!(row["first_seen_at"].is_string());
    assert!(row["last_seen_at"].is_string());
    assert!(row["next_expected_at"].is_string() || row["next_expected_at"].is_null());
    assert!(row["occurrence_count"].is_i64());
    assert!(row["cadence_fit"].is_f64());
    assert!(row["amount_fit"].is_f64());
    assert!(row["score"].is_f64());
    assert!(row["amount_min"].is_f64());
    assert!(row["amount_max"].is_f64());
    assert!(row["sample_description"].is_string());
    assert!(row["quality_flags"].is_array());
    assert!(row["is_active"].is_boolean());
}

#[test]
fn recurring_applies_from_to_filter_window() {
    let rows = vec![
        transaction(
            "acct_checking",
            "2025-12-10",
            -45.00,
            "USD",
            "GYM MEMBERSHIP",
            Some("Gym Club"),
        ),
        transaction(
            "acct_checking",
            "2026-01-10",
            -45.00,
            "USD",
            "GYM MEMBERSHIP",
            Some("Gym Club"),
        ),
        transaction(
            "acct_checking",
            "2026-02-10",
            -45.00,
            "USD",
            "GYM MEMBERSHIP",
            Some("Gym Club"),
        ),
        transaction(
            "acct_checking",
            "2026-03-10",
            -45.00,
            "USD",
            "GYM MEMBERSHIP",
            Some("Gym Club"),
        ),
    ];

    let full = run_scenario(&rows, None, None);
    assert!(recurring_group_exists(&full, "GYM CLUB", "monthly"));

    let scoped = run_scenario(&rows, Some("2026-02-01"), Some("2026-03-31"));
    assert!(scoped.is_empty());
}

#[test]
fn recurring_includes_policy_version() {
    let rows = vec![
        transaction(
            "acct_checking",
            "2026-01-05",
            -15.99,
            "USD",
            "NETFLIX SUBSCRIPTION",
            Some("Netflix"),
        ),
        transaction(
            "acct_checking",
            "2026-02-05",
            -15.99,
            "USD",
            "NETFLIX SUBSCRIPTION",
            Some("Netflix"),
        ),
        transaction(
            "acct_checking",
            "2026-03-05",
            -15.99,
            "USD",
            "NETFLIX SUBSCRIPTION",
            Some("Netflix"),
        ),
    ];
    let temp = temp_home_in_tmp("driggsby-recurring-policy");
    assert!(temp.is_ok());
    if let Ok((_dir, home)) = temp {
        import_rows(&home, &rows);
        let payload = recurring_payload(&home, None, None);
        assert_eq!(
            payload["data"]["policy_version"],
            Value::String("recurring/v1".to_string())
        );
    }
}

#[test]
fn recurring_rows_are_deterministically_sorted() {
    let rows = vec![
        transaction(
            "acct_1",
            "2026-01-01",
            -50.0,
            "USD",
            "POWER BILL",
            Some("Power Co"),
        ),
        transaction(
            "acct_1",
            "2026-02-01",
            -50.0,
            "USD",
            "POWER BILL",
            Some("Power Co"),
        ),
        transaction(
            "acct_1",
            "2026-03-01",
            -50.0,
            "USD",
            "POWER BILL",
            Some("Power Co"),
        ),
        transaction(
            "acct_2",
            "2026-01-07",
            -20.0,
            "USD",
            "MUSIC PLAN",
            Some("Music Box"),
        ),
        transaction(
            "acct_2",
            "2026-02-07",
            -20.0,
            "USD",
            "MUSIC PLAN",
            Some("Music Box"),
        ),
        transaction(
            "acct_2",
            "2026-03-07",
            -20.0,
            "USD",
            "MUSIC PLAN",
            Some("Music Box"),
        ),
    ];
    let patterns = run_scenario(&rows, None, None);
    assert!(patterns.len() >= 2);

    let first_next = patterns
        .first()
        .and_then(|row| row.get("next_expected_at"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let second_next = patterns
        .get(1)
        .and_then(|row| row.get("next_expected_at"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(first_next <= second_next);
}
