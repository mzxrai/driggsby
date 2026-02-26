use crate::ClientResult;
use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::DemoData;
use crate::setup::ensure_initialized;

pub fn run() -> ClientResult<SuccessEnvelope> {
    let _setup = ensure_initialized()?;
    let data = DemoData {
        topic: "dash".to_string(),
        url: "http://127.0.0.1:8787".to_string(),
        fallback_steps: vec!["Copy the URL into your browser manually.".to_string()],
        source: "local-runtime".to_string(),
    };

    success("dash", data)
}
