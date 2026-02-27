use crate::ClientResult;
use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::{SchemaSummaryData, SchemaViewData};
use crate::setup::ensure_initialized;

pub fn summary() -> ClientResult<SuccessEnvelope> {
    let setup = ensure_initialized()?;
    let data = SchemaSummaryData {
        db_path: setup.db_path,
        readonly_uri: setup.readonly_uri,
        public_views: setup.public_views,
    };
    success("db schema", data)
}

pub fn view(view_name: &str) -> ClientResult<SuccessEnvelope> {
    let setup = ensure_initialized()?;
    let view = setup
        .public_views
        .into_iter()
        .find(|candidate| candidate.name == view_name);
    if let Some(known_view) = view {
        let data = SchemaViewData {
            view_name: view_name.to_string(),
            columns: known_view.columns,
        };
        success("db schema view", data)
    } else {
        Err(crate::ClientError::new(
            "unknown_view",
            &format!(
                "Unknown view `{view_name}`. Run `driggsby db schema` to list available views."
            ),
            vec![
                "Run `driggsby db schema` to list available views.".to_string(),
                "Use `driggsby db schema view v1_transactions` as a known-good example."
                    .to_string(),
            ],
        ))
    }
}
