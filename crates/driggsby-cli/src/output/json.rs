use std::io;

use chrono::{Local, TimeZone, Utc};
use driggsby_client::{ClientError, SuccessEnvelope};
use serde::Serialize;
use serde_json::{Value, json};

use super::intelligence_text::{normalize_anomaly_rows, normalize_recurring_rows};

const JSON_VERSION: &str = "v1";

pub fn render_success_json(success: &SuccessEnvelope) -> io::Result<String> {
    let value = match success.command.as_str() {
        "account list" => render_accounts_json(&success.data),
        "import" => render_import_json(&success.data),
        "import list" => render_import_list_json(&success.data),
        "import duplicates" => render_import_duplicates_json(&success.data),
        "import keys uniq" => render_import_keys_uniq_json(&success.data),
        "import undo" => render_import_undo_json(&success.data),
        "db sql" => render_db_sql_json(&success.data),
        "anomalies" => render_anomalies_json(&success.data),
        "recurring" => render_recurring_json(&success.data),
        _ => {
            return Err(io::Error::other(format!(
                "JSON output is not supported for command `{}`",
                success.command
            )));
        }
    };

    serialize_json_pretty(&value)
}

fn render_accounts_json(data: &Value) -> Value {
    data.clone()
}

pub fn render_error_json(error: &ClientError) -> io::Result<String> {
    let mut error_payload = json!({
        "code": error.code,
        "message": error.message,
        "recovery_steps": error.recovery_steps,
    });
    if let Some(data) = &error.data
        && let Some(object) = error_payload.as_object_mut()
    {
        object.insert("data".to_string(), data.clone());
    }

    let payload = json!({
        "error": error_payload
    });
    serialize_json_pretty(&payload)
}

fn render_import_json(data: &Value) -> Value {
    render_edit_success_envelope(data)
}

fn render_import_list_json(data: &Value) -> Value {
    let mut rows = data
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    rows.sort_by(|left, right| {
        let left_created = parse_epoch_from_field(left, "created_at").unwrap_or(0);
        let right_created = parse_epoch_from_field(right, "created_at").unwrap_or(0);
        right_created
            .cmp(&left_created)
            .then_with(|| value_string(right, "import_id").cmp(&value_string(left, "import_id")))
    });

    for row in &mut rows {
        add_timestamp_bundle(row);
    }

    Value::Array(rows)
}

fn render_import_undo_json(data: &Value) -> Value {
    render_edit_success_envelope(data)
}

fn render_import_duplicates_json(data: &Value) -> Value {
    data.clone()
}

fn render_import_keys_uniq_json(data: &Value) -> Value {
    data.clone()
}

fn render_db_sql_json(data: &Value) -> Value {
    data.clone()
}

fn render_edit_success_envelope(data: &Value) -> Value {
    json!({
        "ok": true,
        "version": JSON_VERSION,
        "data": data.clone()
    })
}

fn render_anomalies_json(data: &Value) -> Value {
    let rows = data
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let normalized_rows = normalize_anomaly_rows(&rows);

    let range_hint = data.get("data_range_hint").cloned().unwrap_or(Value::Null);
    let data_covers = json!({
        "from": range_hint.get("earliest").cloned().unwrap_or(Value::Null),
        "to": range_hint.get("latest").cloned().unwrap_or(Value::Null),
    });

    json!({
        "from": data.get("from").cloned().unwrap_or(Value::Null),
        "to": data.get("to").cloned().unwrap_or(Value::Null),
        "data_covers": data_covers,
        "rows": normalized_rows,
    })
}

fn render_recurring_json(data: &Value) -> Value {
    let rows = data
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let normalized_rows = normalize_recurring_rows(&rows);

    json!({
        "from": data.get("from").cloned().unwrap_or(Value::Null),
        "to": data.get("to").cloned().unwrap_or(Value::Null),
        "rows": normalized_rows,
    })
}

fn parse_epoch(value: Option<&Value>) -> Option<i64> {
    let raw = value?;
    if let Some(epoch) = raw.as_i64() {
        return Some(epoch);
    }
    if let Some(text) = raw.as_str() {
        return text.parse::<i64>().ok();
    }
    None
}

fn parse_epoch_from_field(row: &Value, key: &str) -> Option<i64> {
    parse_epoch(row.get(key))
}

fn add_timestamp_bundle(row: &mut Value) {
    let Some(object) = row.as_object_mut() else {
        return;
    };

    let created = timestamp_value(object.get("created_at"));
    let committed = timestamp_value(object.get("committed_at"));
    let reverted = timestamp_value(object.get("reverted_at"));

    object.insert(
        "timestamps".to_string(),
        json!({
            "created": created,
            "committed": committed,
            "reverted": reverted,
        }),
    );
}

fn timestamp_value(raw: Option<&Value>) -> Value {
    let Some(epoch_s) = parse_epoch(raw) else {
        return Value::Null;
    };

    let Some(utc_dt) = Utc.timestamp_opt(epoch_s, 0).single() else {
        return Value::Null;
    };
    let Some(local_dt) = Local.timestamp_opt(epoch_s, 0).single() else {
        return Value::Null;
    };

    json!({
        "epoch_s": epoch_s,
        "utc": utc_dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "local": local_dt.format("%Y-%m-%d %H:%M:%S %:z").to_string(),
    })
}

fn value_string(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn serialize_json_pretty<T>(value: &T) -> io::Result<String>
where
    T: Serialize,
{
    serde_json::to_string_pretty(value).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use driggsby_client::SuccessEnvelope;
    use serde_json::json;

    use super::{render_error_json, render_success_json};

    fn success(command: &str, data: Value) -> SuccessEnvelope {
        SuccessEnvelope {
            ok: true,
            command: command.to_string(),
            version: "0.1.0".to_string(),
            data,
        }
    }

    #[test]
    fn import_list_json_returns_raw_array() {
        let payload = success(
            "import list",
            json!({
                "rows": [
                    {
                        "import_id": "imp_1",
                        "created_at": "1",
                        "committed_at": "2",
                        "reverted_at": null,
                        "status": "committed",
                        "accounts": [
                            {
                                "account_key": "acct_1",
                                "account_type": "checking",
                                "rows_read": 3,
                                "inserted": 2,
                                "deduped": 1
                            }
                        ]
                    }
                ]
            }),
        );

        let rendered = render_success_json(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            let parsed: Result<Value, _> = serde_json::from_str(&text);
            assert!(parsed.is_ok());
            if let Ok(value) = parsed {
                assert!(value.is_array());
                assert_eq!(value[0]["import_id"], Value::String("imp_1".to_string()));
                assert_eq!(
                    value[0]["accounts"][0]["account_key"],
                    Value::String("acct_1".to_string())
                );
                assert_eq!(value[0]["timestamps"]["created"]["epoch_s"], Value::from(1));
                assert!(value[0]["timestamps"]["created"]["utc"].is_string());
                assert!(value[0]["timestamps"]["created"]["local"].is_string());
                assert_eq!(
                    value[0]["timestamps"]["committed"]["epoch_s"],
                    Value::from(2)
                );
                assert!(value[0]["timestamps"]["reverted"].is_null());
            }
        }
    }

    #[test]
    fn accounts_json_returns_raw_data_shape() {
        let payload = success(
            "account list",
            json!({
                "summary": {
                    "account_count": 1,
                    "transaction_count": 3,
                    "earliest_posted_at": "2026-01-01",
                    "latest_posted_at": "2026-01-05",
                    "typed_account_count": 1,
                    "untyped_account_count": 0,
                    "net_amount": -12.25
                },
                "rows": [
                    {
                        "account_key": "acct_1",
                        "account_type": "checking",
                        "currency": "USD",
                        "txn_count": 3,
                        "first_posted_at": "2026-01-01",
                        "last_posted_at": "2026-01-05",
                        "net_amount": -12.25
                    }
                ]
            }),
        );

        let rendered = render_success_json(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            let parsed: Result<Value, _> = serde_json::from_str(&text);
            assert!(parsed.is_ok());
            if let Ok(value) = parsed {
                assert!(value["summary"].is_object());
                assert!(value["rows"].is_array());
                assert!(value.get("ok").is_none());
                assert!(value.get("version").is_none());
            }
        }
    }

    #[test]
    fn import_json_uses_edit_envelope_for_commit_and_dry_run() {
        let commit_payload = success(
            "import",
            json!({
                "dry_run": false,
                "import_id": "imp_1"
            }),
        );
        let commit_rendered = render_success_json(&commit_payload);
        assert!(commit_rendered.is_ok());
        if let Ok(text) = commit_rendered {
            let parsed: Result<Value, _> = serde_json::from_str(&text);
            assert!(parsed.is_ok());
            if let Ok(value) = parsed {
                assert_eq!(value["ok"], Value::Bool(true));
                assert_eq!(value["version"], Value::String("v1".to_string()));
                assert_eq!(
                    value["data"]["import_id"],
                    Value::String("imp_1".to_string())
                );
            }
        }

        let dry_run_payload = success(
            "import",
            json!({
                "dry_run": true,
                "summary": {
                    "rows_read": 1
                }
            }),
        );
        let dry_run_rendered = render_success_json(&dry_run_payload);
        assert!(dry_run_rendered.is_ok());
        if let Ok(text) = dry_run_rendered {
            let parsed: Result<Value, _> = serde_json::from_str(&text);
            assert!(parsed.is_ok());
            if let Ok(value) = parsed {
                assert_eq!(value["ok"], Value::Bool(true));
                assert_eq!(value["version"], Value::String("v1".to_string()));
                assert_eq!(value["data"]["dry_run"], Value::Bool(true));
                assert_eq!(value["data"]["summary"]["rows_read"], Value::from(1));
            }
        }
    }

    #[test]
    fn db_sql_json_returns_raw_query_payload_without_envelope() {
        let payload = success(
            "db sql",
            json!({
                "columns": [
                    {"name": "account_key", "type": "text", "nullable": false}
                ],
                "rows": [["acct_1"]],
                "row_count": 1,
                "truncated": false,
                "max_rows": 1000,
                "source": "inline"
            }),
        );

        let rendered = render_success_json(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            let parsed: Result<Value, _> = serde_json::from_str(&text);
            assert!(parsed.is_ok());
            if let Ok(value) = parsed {
                assert!(value["columns"].is_array());
                assert!(value["rows"].is_array());
                assert_eq!(value["row_count"], Value::from(1));
                assert_eq!(value["truncated"], Value::Bool(false));
                assert_eq!(value["source"], Value::String("inline".to_string()));
                assert!(value.get("ok").is_none());
                assert!(value.get("version").is_none());
            }
        }
    }

    #[test]
    fn import_list_json_orders_by_import_id_when_created_at_ties() {
        let payload = success(
            "import list",
            json!({
                "rows": [
                    {"import_id": "imp_a", "created_at": "10", "status": "committed"},
                    {"import_id": "imp_b", "created_at": "10", "status": "committed"}
                ]
            }),
        );

        let rendered = render_success_json(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            let parsed: Result<Value, _> = serde_json::from_str(&text);
            assert!(parsed.is_ok());
            if let Ok(value) = parsed {
                assert!(value.is_array());
                assert_eq!(value[0]["import_id"], Value::String("imp_b".to_string()));
                assert_eq!(value[1]["import_id"], Value::String("imp_a".to_string()));
            }
        }
    }

    #[test]
    fn runtime_error_json_uses_universal_shape() {
        let error =
            driggsby_client::ClientError::new("not_found", "missing", vec!["run list".to_string()])
                .with_data(json!({
                    "resource": "import",
                    "import_id": "imp_1"
                }));
        let rendered = render_error_json(&error);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            let parsed: Result<Value, _> = serde_json::from_str(&text);
            assert!(parsed.is_ok());
            if let Ok(value) = parsed {
                assert_eq!(
                    value["error"]["code"],
                    Value::String("not_found".to_string())
                );
                assert!(value.get("ok").is_none());
                assert!(value.get("data").is_none());
                assert_eq!(
                    value["error"]["data"]["resource"],
                    Value::String("import".to_string())
                );
                assert_eq!(
                    value["error"]["data"]["import_id"],
                    Value::String("imp_1".to_string())
                );
            }
        }
    }

    #[test]
    fn runtime_error_json_omits_error_data_when_absent() {
        let error =
            driggsby_client::ClientError::new("not_found", "missing", vec!["run list".to_string()]);
        let rendered = render_error_json(&error);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            let parsed: Result<Value, _> = serde_json::from_str(&text);
            assert!(parsed.is_ok());
            if let Ok(value) = parsed {
                assert_eq!(
                    value["error"]["code"],
                    Value::String("not_found".to_string())
                );
                assert!(value["error"].get("data").is_none());
                assert!(value.get("data").is_none());
            }
        }
    }

    #[test]
    fn import_duplicates_json_returns_raw_data_shape() {
        let payload = success(
            "import duplicates",
            json!({
                "import_id": "imp_1",
                "total": 0,
                "rows": []
            }),
        );

        let rendered = render_success_json(&payload);
        assert!(rendered.is_ok());
        if let Ok(text) = rendered {
            let parsed: Result<Value, _> = serde_json::from_str(&text);
            assert!(parsed.is_ok());
            if let Ok(value) = parsed {
                assert_eq!(value["import_id"], Value::String("imp_1".to_string()));
                assert!(value["rows"].is_array());
                assert!(value.get("ok").is_none());
                assert!(value.get("version").is_none());
                assert!(value.get("data").is_none());
            }
        }
    }

    use serde_json::Value;
}
