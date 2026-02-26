use std::io;

use driggsby_client::{ClientError, SuccessEnvelope};
use serde::Serialize;
use serde_json::{Value, json};

use super::intelligence_text::{normalize_anomaly_rows, normalize_recurring_rows};

const JSON_VERSION: &str = "v1";

pub fn render_success_json(success: &SuccessEnvelope) -> io::Result<String> {
    let value = match success.command.as_str() {
        "import" => render_import_json(&success.data),
        "import list" => render_import_list_json(&success.data),
        "import duplicates" => render_import_duplicates_json(&success.data),
        "import keys uniq" => render_import_keys_uniq_json(&success.data),
        "import undo" => render_import_undo_json(&success.data),
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

pub fn render_error_json(error: &ClientError) -> io::Result<String> {
    let payload = json!({
        "error": {
            "code": error.code,
            "message": error.message,
            "recovery_steps": error.recovery_steps,
        }
    });
    serialize_json_pretty(&payload)
}

fn render_import_json(data: &Value) -> Value {
    json!({
        "ok": true,
        "version": JSON_VERSION,
        "data": data.clone()
    })
}

fn render_import_list_json(data: &Value) -> Value {
    let mut rows = data
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    rows.sort_by(|left, right| {
        let left_created = parse_created_at(left);
        let right_created = parse_created_at(right);
        right_created
            .cmp(&left_created)
            .then_with(|| value_string(right, "import_id").cmp(&value_string(left, "import_id")))
    });

    Value::Array(rows)
}

fn render_import_undo_json(data: &Value) -> Value {
    json!({
        "ok": true,
        "version": JSON_VERSION,
        "data": data.clone()
    })
}

fn render_import_duplicates_json(data: &Value) -> Value {
    json!({
        "ok": true,
        "version": JSON_VERSION,
        "data": data.clone()
    })
}

fn render_import_keys_uniq_json(data: &Value) -> Value {
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

fn parse_created_at(row: &Value) -> i64 {
    if let Some(raw) = row.get("created_at") {
        if let Some(value) = raw.as_i64() {
            return value;
        }
        if let Some(text) = raw.as_str() {
            return text.parse::<i64>().unwrap_or(0);
        }
    }
    0
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
                    {"import_id": "imp_1", "created_at": "1", "status": "committed"}
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
            }
        }
    }

    #[test]
    fn runtime_error_json_uses_universal_shape() {
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
                assert!(value.get("ok").is_none());
            }
        }
    }

    #[test]
    fn import_duplicates_json_uses_structured_envelope() {
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
                assert_eq!(value["ok"], Value::Bool(true));
                assert_eq!(value["version"], Value::String("v1".to_string()));
                assert_eq!(
                    value["data"]["import_id"],
                    Value::String("imp_1".to_string())
                );
            }
        }
    }

    use serde_json::Value;
}
