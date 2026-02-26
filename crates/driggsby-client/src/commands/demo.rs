use crate::ClientResult;
use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::DemoData;

pub fn run(topic: &str) -> ClientResult<SuccessEnvelope> {
    let (url, fallback_steps): (&str, Vec<String>) = match topic {
        "recurring" => (
            "http://127.0.0.1:8787/demo/recurring",
            vec!["Copy the URL into your browser manually.".to_string()],
        ),
        "anomalies" => (
            "http://127.0.0.1:8787/demo/anomalies",
            vec!["Copy the URL into your browser manually.".to_string()],
        ),
        _ => (
            "http://127.0.0.1:8787/demo/dash",
            vec!["Copy the URL into your browser manually.".to_string()],
        ),
    };

    let data = DemoData {
        topic: topic.to_string(),
        url: url.to_string(),
        fallback_steps,
        source: "sample-snapshot".to_string(),
    };

    success("demo", data)
}
