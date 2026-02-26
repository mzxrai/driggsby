use std::fs;
use std::path::{Path, PathBuf};

use driggsby_client::commands::import;
use driggsby_client::commands::import::{ImportKeysUniqOptions, ImportRunOptions};
use serde_json::{Value, json};
use tempfile::tempdir;

fn temp_home() -> std::io::Result<(tempfile::TempDir, PathBuf)> {
    let dir = tempdir()?;
    let home = dir.path().join("ledger-home");
    Ok((dir, home))
}

fn write_json(path: &Path, body: &Value) {
    let serialized = serde_json::to_string_pretty(body);
    assert!(serialized.is_ok());
    if let Ok(text) = serialized {
        let write = fs::write(path, text);
        assert!(write.is_ok());
    }
}

fn run_import(
    home: &Path,
    path: &Path,
    dry_run: bool,
) -> driggsby_client::ClientResult<driggsby_client::SuccessEnvelope> {
    import::run_with_options(ImportRunOptions {
        path: Some(path.display().to_string()),
        dry_run,
        home_override: Some(home),
        stdin_override: None,
    })
}

fn warning_codes(payload: &Value) -> Vec<String> {
    payload["data"]["drift_warnings"]
        .as_array()
        .map(|warnings| {
            warnings
                .iter()
                .filter_map(|warning| {
                    warning
                        .get("code")
                        .and_then(Value::as_str)
                        .map(|value| value.to_string())
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

#[test]
fn dry_run_includes_inventory_sign_profiles_and_drift_warnings() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let baseline_path = home.join("baseline.json");
        let mut baseline_rows = Vec::new();
        for index in 0..25 {
            let amount = if index == 0 { 10.0 } else { -10.0 };
            baseline_rows.push(json!({
                "statement_id": "chase_checking_1234_2026-01-31",
                "account_key": "chase_checking_1234",
                "posted_at": format!("2026-01-{:02}", (index % 28) + 1),
                "amount": amount,
                "currency": "USD",
                "description": format!("BASELINE-{index}"),
                "merchant": "Existing Merchant",
                "category": "Groceries"
            }));
        }
        write_json(&baseline_path, &Value::Array(baseline_rows));

        let baseline_result = run_import(&home, &baseline_path, false);
        assert!(baseline_result.is_ok());

        let dry_run_path = home.join("dry-run.json");
        let dry_run_rows = json!([
            {
                "statement_id": "chase_checking_1234_2026-03-31",
                "account_key": "chase_checking_1234",
                "posted_at": "2026-03-01",
                "amount": 120.00,
                "currency": "USD",
                "description": "PAYMENT-1",
                "merchant": "Existing Merchant",
                "category": "Groceries"
            },
            {
                "statement_id": "chase_checking_1234_2026-03-31",
                "account_key": "chase_checking_1234",
                "posted_at": "2026-03-02",
                "amount": 130.00,
                "currency": "USD",
                "description": "PAYMENT-2",
                "merchant": "New Merchant",
                "category": "Groceries"
            },
            {
                "statement_id": "chase_checking_1234_2026-03-31",
                "account_key": "chase_checking_1234",
                "posted_at": "2026-03-03",
                "amount": 140.00,
                "currency": "USD",
                "description": "PAYMENT-3",
                "merchant": "Existing Merchant",
                "category": "Travel"
            },
            {
                "statement_id": "chase_checking_1234_2026-03-31",
                "account_key": "chase_checking_1234",
                "posted_at": "2026-03-04",
                "amount": 150.00,
                "currency": "EUR",
                "description": "PAYMENT-4",
                "merchant": "Existing Merchant",
                "category": "Groceries"
            },
            {
                "statement_id": "chase_checking_1234_2026-03-31",
                "account_key": "chase_checking_1234",
                "posted_at": "2026-03-05",
                "amount": 160.00,
                "currency": "USD",
                "description": "PAYMENT-5",
                "merchant": "Existing Merchant",
                "category": "Groceries"
            },
            {
                "statement_id": "chase_checkng_1234_2026-01-31",
                "account_key": "chase_checkng_1234",
                "posted_at": "2026-03-06",
                "amount": -12.00,
                "currency": "USD",
                "description": "TYPO-ACCOUNT",
                "merchant": "Existing Merchant",
                "category": "Groceries"
            }
        ]);
        write_json(&dry_run_path, &dry_run_rows);

        let dry_run_result = run_import(&home, &dry_run_path, true);
        assert!(dry_run_result.is_ok());

        if let Ok(success) = dry_run_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["dry_run"], Value::Bool(true));
                assert!(value["data"]["key_inventory"].is_object());
                assert!(value["data"]["sign_profiles"].is_array());
                assert!(value["data"]["drift_warnings"].is_array());
                assert!(
                    value["data"]["key_inventory"]["account_key"]["existing_values"].is_array()
                );

                let codes = warning_codes(&value);
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
            }
        }
    }
}

#[test]
fn dry_run_deduped_duplicates_do_not_trigger_sign_profile_anomaly() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let baseline_path = home.join("baseline-sign.json");
        let mut baseline_rows = Vec::new();
        for index in 0..25 {
            let amount = if index == 0 { 20.0 } else { -10.0 };
            baseline_rows.push(json!({
                "statement_id": "acct_sign_1_2026-01-31",
                "account_key": "acct_sign_1",
                "posted_at": format!("2026-02-{:02}", (index % 28) + 1),
                "amount": amount,
                "currency": "USD",
                "description": format!("BASELINE-SIGN-{index}")
            }));
        }
        write_json(&baseline_path, &Value::Array(baseline_rows));

        let baseline_result = run_import(&home, &baseline_path, false);
        assert!(baseline_result.is_ok());

        let duplicate_path = home.join("duplicate-dry-run.json");
        write_json(
            &duplicate_path,
            &json!([
                {
                    "statement_id": "acct_sign_1_2026-03-31",
                    "account_key": "acct_sign_1",
                    "posted_at": "2026-04-01",
                    "amount": 40.0,
                    "currency": "USD",
                    "description": "DUPLICATE-POSITIVE"
                },
                {
                    "statement_id": "acct_sign_1_2026-04-30",
                    "account_key": "acct_sign_1",
                    "posted_at": "2026-04-01",
                    "amount": 40.0,
                    "currency": "USD",
                    "description": "DUPLICATE-POSITIVE"
                },
                {
                    "statement_id": "acct_sign_1_2026-04-30",
                    "account_key": "acct_sign_1",
                    "posted_at": "2026-04-01",
                    "amount": 40.0,
                    "currency": "USD",
                    "description": "DUPLICATE-POSITIVE"
                },
                {
                    "statement_id": "acct_sign_1_2026-04-30",
                    "account_key": "acct_sign_1",
                    "posted_at": "2026-04-01",
                    "amount": 40.0,
                    "currency": "USD",
                    "description": "DUPLICATE-POSITIVE"
                },
                {
                    "statement_id": "acct_sign_1_2026-04-30",
                    "account_key": "acct_sign_1",
                    "posted_at": "2026-04-01",
                    "amount": 40.0,
                    "currency": "USD",
                    "description": "DUPLICATE-POSITIVE"
                }
            ]),
        );

        let dry_run_result = run_import(&home, &duplicate_path, true);
        assert!(dry_run_result.is_ok());
        if let Ok(success) = dry_run_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(value["data"]["duplicate_summary"]["total"], Value::from(4));
                let codes = warning_codes(&value);
                assert!(
                    !codes
                        .iter()
                        .any(|code| code == "account_sign_profile_anomaly")
                );
            }
        }
    }
}

#[test]
fn keys_uniq_returns_sorted_values_for_property_and_all_properties() {
    let temp = temp_home();
    assert!(temp.is_ok());
    if let Ok((_temp, home)) = temp {
        let create_home = fs::create_dir_all(&home);
        assert!(create_home.is_ok());

        let source_path = home.join("keys-source.json");
        write_json(
            &source_path,
            &json!([
                {
                    "statement_id": "b_account_2026-01-31",
                    "account_key": "b_account",
                    "posted_at": "2026-04-01",
                    "amount": -5.0,
                    "currency": "USD",
                    "description": "row1",
                    "merchant": "Zeta",
                    "category": "Dining"
                },
                {
                    "statement_id": "a_account_2026-01-31",
                    "account_key": "a_account",
                    "posted_at": "2026-04-02",
                    "amount": -7.0,
                    "currency": "USD",
                    "description": "row2",
                    "merchant": "Alpha",
                    "category": "Groceries"
                },
                {
                    "statement_id": "a_account_2026-01-31",
                    "account_key": "a_account",
                    "posted_at": "2026-04-03",
                    "amount": -9.0,
                    "currency": "USD",
                    "description": "row3",
                    "merchant": "Alpha",
                    "category": "Groceries"
                }
            ]),
        );

        let import_result = run_import(&home, &source_path, false);
        assert!(import_result.is_ok());

        let property_result = import::keys_uniq_with_options(ImportKeysUniqOptions {
            property: Some("merchant".to_string()),
            home_override: Some(&home),
        });
        assert!(property_result.is_ok());
        if let Ok(success) = property_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(
                    value["command"],
                    Value::String("import keys uniq".to_string())
                );
                assert_eq!(
                    value["data"]["property"],
                    Value::String("merchant".to_string())
                );
                assert_eq!(
                    value["data"]["inventories"][0]["existing_values"],
                    json!(["Alpha", "Zeta"])
                );
                assert_eq!(
                    value["data"]["inventories"][0]["value_counts"],
                    json!([
                        {"value": "Alpha", "count": 2},
                        {"value": "Zeta", "count": 1}
                    ])
                );
            }
        }

        let all_result = import::keys_uniq_with_options(ImportKeysUniqOptions {
            property: None,
            home_override: Some(&home),
        });
        assert!(all_result.is_ok());
        if let Ok(success) = all_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert!(value["data"]["inventories"].is_array());
                assert_eq!(value["data"]["inventories"][0]["property"], "account_key");
                assert_eq!(value["data"]["inventories"][1]["property"], "account_type");
                assert_eq!(value["data"]["inventories"][2]["property"], "currency");
                assert_eq!(value["data"]["inventories"][3]["property"], "merchant");
                assert_eq!(value["data"]["inventories"][4]["property"], "category");
            }
        }

        let account_type_result = import::keys_uniq_with_options(ImportKeysUniqOptions {
            property: Some("account_type".to_string()),
            home_override: Some(&home),
        });
        assert!(account_type_result.is_ok());
        if let Ok(success) = account_type_result {
            let payload = serde_json::to_value(success);
            assert!(payload.is_ok());
            if let Ok(value) = payload {
                assert_eq!(
                    value["data"]["property"],
                    Value::String("account_type".to_string())
                );
                assert!(value["data"]["inventories"][0]["existing_values"].is_array());
            }
        }
    }
}
