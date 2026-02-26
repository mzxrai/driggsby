use std::io;

use serde_json::Value;

pub fn render_demo_or_dash(command: &str, data: &Value) -> io::Result<String> {
    let url = data
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::other("demo/dash output requires url"))?;

    let topic = data.get("topic").and_then(Value::as_str).unwrap_or("dash");

    match command {
        "demo" => match topic {
            "recurring" => render_demo_stub(url, "recurring", "recurring transaction patterns"),
            "anomalies" => render_demo_stub(url, "anomalies", "spending anomaly detection"),
            _ => Ok([
                format!("Opening demo at {url}"),
                String::new(),
                "This demo uses bundled sample data so you can explore Driggsby".to_string(),
                "without importing your own transactions first.".to_string(),
                String::new(),
                "If the browser did not open automatically, copy the URL above.".to_string(),
            ]
            .join("\n")),
        },
        "dash" => Ok([
            format!("Opening your dashboard at {url}"),
            String::new(),
            "Your dashboard displays live data from your local Driggsby database.".to_string(),
            String::new(),
            "If the browser did not open automatically, copy the URL above.".to_string(),
        ]
        .join("\n")),
        _ => Err(io::Error::other("unsupported demo renderer command")),
    }
}

fn render_demo_stub(url: &str, topic: &str, description: &str) -> io::Result<String> {
    Ok([
        format!("Demo: {description}"),
        String::new(),
        format!("Sample {topic} output is not yet available in this version."),
        format!("Visit {url} to preview {topic} with bundled sample data."),
        String::new(),
        "To see real output, import your transactions first:".to_string(),
        "  driggsby import create --help".to_string(),
    ]
    .join("\n"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::render_demo_or_dash;

    #[test]
    fn demo_dash_renders_opening_message() {
        let demo = render_demo_or_dash(
            "demo",
            &json!({ "url": "http://127.0.0.1:8787/demo/dash", "topic": "dash" }),
        );
        assert!(demo.is_ok());
        if let Ok(text) = demo {
            assert!(text.starts_with("Opening demo at"));
        }
    }

    #[test]
    fn demo_recurring_renders_stub() {
        let demo = render_demo_or_dash(
            "demo",
            &json!({ "url": "http://127.0.0.1:8787/demo/recurring", "topic": "recurring" }),
        );
        assert!(demo.is_ok());
        if let Ok(text) = demo {
            assert!(text.starts_with("Demo: recurring transaction patterns"));
            assert!(text.contains("Sample recurring output"));
            assert!(text.contains("driggsby import create --help"));
        }
    }

    #[test]
    fn demo_anomalies_renders_stub() {
        let demo = render_demo_or_dash(
            "demo",
            &json!({ "url": "http://127.0.0.1:8787/demo/anomalies", "topic": "anomalies" }),
        );
        assert!(demo.is_ok());
        if let Ok(text) = demo {
            assert!(text.starts_with("Demo: spending anomaly detection"));
            assert!(text.contains("Sample anomalies output"));
        }
    }

    #[test]
    fn dash_renders_opening_message() {
        let dash = render_demo_or_dash("dash", &json!({ "url": "http://127.0.0.1:8787" }));
        assert!(dash.is_ok());
        if let Ok(text) = dash {
            assert!(text.starts_with("Opening your dashboard at"));
        }
    }
}
