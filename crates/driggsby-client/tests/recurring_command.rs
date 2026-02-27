mod support;

use driggsby_client::commands::anomalies::{self, AnomaliesRunOptions};
use driggsby_client::commands::recurring::{self, RecurringRunOptions};
use driggsby_client::setup::ensure_initialized_at;
use driggsby_client::state::open_connection;
use rusqlite::params;
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
    assert!(row["cadence"].is_string());
    assert!(row["typical_amount"].is_f64());
    assert!(row["currency"].is_string());
    assert!(row["last_seen_at"].is_string());
    assert!(row["next_expected_at"].is_string() || row["next_expected_at"].is_null());
    assert!(row["occurrence_count"].is_i64());
    assert!(row["score"].is_f64());
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
    assert!(!scoped.is_empty());
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

#[test]
fn recurring_reads_materialized_rows_and_filters_on_last_seen_at() {
    let temp = temp_home_in_tmp("driggsby-recurring-sql-source");
    assert!(temp.is_ok());
    if let Ok((_dir, home)) = temp {
        let setup = ensure_initialized_at(&home);
        assert!(setup.is_ok());
        if let Ok(context) = setup {
            let db_path = std::path::PathBuf::from(context.db_path);
            let connection = open_connection(&db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                let deleted = conn.execute("DELETE FROM internal_recurring_materialized", []);
                assert!(deleted.is_ok());

                let inserted = conn.execute(
                    "INSERT INTO internal_recurring_materialized (
                        group_key,
                        account_key,
                        merchant,
                        cadence,
                        typical_amount,
                        currency,
                        last_seen_at,
                        next_expected_at,
                        occurrence_count,
                        score,
                        is_active
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        "acct_sql|USD|debit|sql recurring",
                        "acct_sql",
                        "SQL RECURRING",
                        "monthly",
                        -42.5_f64,
                        "USD",
                        "2026-01-15",
                        "2026-02-15",
                        4_i64,
                        0.93_f64,
                        1_i64
                    ],
                );
                assert!(inserted.is_ok());
            }
        }

        let full_payload = recurring_payload(&home, None, None);
        let full_rows = full_payload["data"]["rows"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(full_rows.len(), 1);
        assert_eq!(
            full_rows[0]["merchant"],
            Value::String("SQL RECURRING".to_string())
        );

        let filtered_payload = recurring_payload(&home, Some("2026-01-16"), None);
        let filtered_rows = filtered_payload["data"]["rows"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(filtered_rows.is_empty());
    }
}

#[test]
fn anomalies_reads_materialized_rows_and_filters_on_posted_at() {
    let temp = temp_home_in_tmp("driggsby-anomalies-sql-source");
    assert!(temp.is_ok());
    if let Ok((_dir, home)) = temp {
        let setup = ensure_initialized_at(&home);
        assert!(setup.is_ok());
        if let Ok(context) = setup {
            let db_path = std::path::PathBuf::from(context.db_path);
            let connection = open_connection(&db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                let deleted = conn.execute("DELETE FROM internal_anomalies_materialized", []);
                assert!(deleted.is_ok());

                let inserted = conn.execute(
                    "INSERT INTO internal_anomalies_materialized (
                        txn_id,
                        account_key,
                        posted_at,
                        merchant,
                        amount,
                        currency,
                        reason_code,
                        reason,
                        score,
                        severity
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        "txn_sql_1",
                        "acct_sql",
                        "2026-03-10",
                        "SQL MARKET",
                        -455.19_f64,
                        "USD",
                        "amount_spike",
                        "Amount is far above expected baseline for this merchant.",
                        0.97_f64,
                        "high"
                    ],
                );
                assert!(inserted.is_ok());
            }
        }

        let anomalies_result = anomalies::run_with_options(AnomaliesRunOptions {
            from: None,
            to: None,
            home_override: Some(&home),
        });
        assert!(anomalies_result.is_ok());
        if let Ok(success) = anomalies_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                let rows = value["data"]["rows"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0]["txn_id"], Value::String("txn_sql_1".to_string()));
                assert_eq!(
                    rows[0]["reason_code"],
                    Value::String("amount_spike".to_string())
                );
            }
        }

        let filtered_result = anomalies::run_with_options(AnomaliesRunOptions {
            from: Some("2026-03-11".to_string()),
            to: None,
            home_override: Some(&home),
        });
        assert!(filtered_result.is_ok());
        if let Ok(success) = filtered_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                let rows = value["data"]["rows"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                assert!(rows.is_empty());
            }
        }
    }
}

#[test]
fn recurring_command_rows_match_v1_recurring_projection_fields() {
    let rows = vec![
        transaction(
            "acct_projection",
            "2026-01-05",
            -14.99,
            "USD",
            "VIDEO STREAM",
            Some("Video Stream"),
        ),
        transaction(
            "acct_projection",
            "2026-02-05",
            -14.99,
            "USD",
            "VIDEO STREAM",
            Some("Video Stream"),
        ),
        transaction(
            "acct_projection",
            "2026-03-05",
            -14.99,
            "USD",
            "VIDEO STREAM",
            Some("Video Stream"),
        ),
        transaction(
            "acct_projection",
            "2026-04-05",
            -14.99,
            "USD",
            "VIDEO STREAM",
            Some("Video Stream"),
        ),
    ];

    let temp = temp_home_in_tmp("driggsby-recurring-parity");
    assert!(temp.is_ok());
    if let Ok((_dir, home)) = temp {
        import_rows(&home, &rows);
        let payload = recurring_payload(&home, None, None);
        let command_rows = payload["data"]["rows"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(command_rows.len(), 1);

        let setup = ensure_initialized_at(&home);
        assert!(setup.is_ok());
        if let Ok(context) = setup {
            let db_path = std::path::PathBuf::from(context.db_path);
            let connection = open_connection(&db_path);
            assert!(connection.is_ok());
            if let Ok(conn) = connection {
                let query = conn.prepare(
                    "SELECT
                        group_key,
                        account_key,
                        merchant,
                        cadence,
                        typical_amount,
                        currency,
                        last_seen_at,
                        next_expected_at,
                        occurrence_count,
                        score,
                        is_active
                     FROM v1_recurring
                     ORDER BY group_key ASC",
                );
                assert!(query.is_ok());
                if let Ok(mut statement) = query {
                    let rows_iter = statement.query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, f64>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, String>(6)?,
                            row.get::<_, Option<String>>(7)?,
                            row.get::<_, i64>(8)?,
                            row.get::<_, f64>(9)?,
                            row.get::<_, i64>(10)?,
                        ))
                    });
                    assert!(rows_iter.is_ok());
                    if let Ok(iter) = rows_iter {
                        let sql_rows = iter.filter_map(Result::ok).collect::<Vec<_>>();
                        assert_eq!(sql_rows.len(), 1);

                        let command = &command_rows[0];
                        let sql = &sql_rows[0];
                        assert_eq!(command["group_key"], Value::String(sql.0.clone()));
                        assert_eq!(command["account_key"], Value::String(sql.1.clone()));
                        assert_eq!(command["merchant"], Value::String(sql.2.clone()));
                        assert_eq!(command["cadence"], Value::String(sql.3.clone()));
                        assert_eq!(command["currency"], Value::String(sql.5.clone()));
                        assert_eq!(command["last_seen_at"], Value::String(sql.6.clone()));
                        assert_eq!(command["occurrence_count"], Value::from(sql.8));
                        assert_eq!(command["is_active"], Value::Bool(sql.10 == 1));
                    }
                }
            }
        }
    }
}
