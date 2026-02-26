use std::path::Path;

use serde_json::{Value, json};
use thiserror::Error;

use crate::contracts::types::{ImportIssue, ImportSummary};

pub(crate) const IMPORT_HELP_COMMAND: &str = "driggsby import create --help";
pub(crate) const IMPORT_HELP_SECTION_TITLE: &str = "Import Troubleshooting";

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct ClientError {
    pub code: String,
    pub message: String,
    pub recovery_steps: Vec<String>,
    pub data: Option<Value>,
}

impl ClientError {
    pub fn new(code: &str, message: &str, recovery_steps: Vec<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            recovery_steps,
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_import_help(self) -> Self {
        self.with_import_help_data(json!({}))
    }

    pub fn with_import_help_data(self, data: Value) -> Self {
        self.with_data(merge_import_help_data(data))
    }

    pub fn invalid_argument(message: &str) -> Self {
        Self::invalid_argument_for_command(message, None)
    }

    pub fn invalid_argument_for_command(message: &str, command: Option<&str>) -> Self {
        let help_hint = match command {
            Some(cmd) => format!("Run `driggsby {cmd} --help` for usage."),
            None => "Run `driggsby --help` for usage.".to_string(),
        };
        let error = Self::new("invalid_argument", message, vec![help_hint]);
        if let Some(cmd) = command {
            return error.with_data(json!({
                "command_hint": cmd,
            }));
        }
        error
    }

    pub fn invalid_argument_with_recovery(message: &str, recovery_steps: Vec<String>) -> Self {
        Self::new("invalid_argument", message, recovery_steps)
    }

    pub fn invalid_import_format(message: &str, received_format: &str) -> Self {
        Self::invalid_argument_with_recovery(
            message,
            vec![
                "Provide a supported import format (JSON array or CSV).".to_string(),
                "Run `driggsby import create --help` to confirm field requirements.".to_string(),
            ],
        )
        .with_import_help_data(json!({
            "received_format": received_format,
            "supported_formats": ["json_array", "csv"],
        }))
    }

    pub fn import_schema_mismatch(
        required_headers: Vec<String>,
        optional_headers: Vec<String>,
        actual_headers: Vec<String>,
    ) -> Self {
        let mut expected_headers = required_headers.clone();
        expected_headers.extend(optional_headers.clone());

        Self::new(
            "import_schema_mismatch",
            "CSV headers do not satisfy the import schema.",
            vec![
                "Include all required headers; optional headers may be omitted.".to_string(),
                "Do not include unknown headers.".to_string(),
                "Run `driggsby import create --help` to review required and optional fields."
                    .to_string(),
                "Rerun `driggsby import create --dry-run <path>`.".to_string(),
            ],
        )
        .with_import_help_data(json!({
            "required_headers": required_headers,
            "optional_headers": optional_headers,
            "expected_headers": expected_headers,
            "actual_headers": actual_headers,
        }))
    }

    pub fn import_validation_failed(summary: ImportSummary, issues: Vec<ImportIssue>) -> Self {
        let issue_count = summary.rows_invalid;
        Self::new(
            "import_validation_failed",
            &format!(
                "Import failed validation: {issue_count} rows need fixes. No rows were written."
            ),
            vec![
                "Fix the listed issues in your source file.".to_string(),
                "Rerun driggsby import create --dry-run <path>.".to_string(),
                "Then rerun driggsby import create <path>.".to_string(),
            ],
        )
        .with_import_help_data(json!({
            "summary": summary,
            "issues": issues,
        }))
    }

    pub fn import_id_not_found(import_id: &str) -> Self {
        Self::new(
            "import_id_not_found",
            &format!("Import id `{import_id}` was not found."),
            vec![
                "Run driggsby import list to find a valid import id.".to_string(),
                "Retry with driggsby import undo <import_id>.".to_string(),
            ],
        )
        .with_import_help_data(json!({
            "import_id": import_id,
        }))
    }

    pub fn import_duplicates_id_not_found(import_id: &str) -> Self {
        Self::new(
            "import_id_not_found",
            &format!("Import id `{import_id}` was not found."),
            vec![
                "Run driggsby import list to find a valid import id.".to_string(),
                "Retry with driggsby import duplicates <import_id>.".to_string(),
            ],
        )
        .with_import_help_data(json!({
            "import_id": import_id,
        }))
    }

    pub fn import_already_reverted(import_id: &str) -> Self {
        Self::new(
            "import_already_reverted",
            &format!("Import id `{import_id}` was already reverted."),
            vec![
                "Run driggsby import list to inspect import statuses.".to_string(),
                "Choose a committed import id and retry undo.".to_string(),
            ],
        )
        .with_import_help_data(json!({
            "import_id": import_id,
        }))
    }

    pub fn internal_serialization(message: &str) -> Self {
        Self::new("internal_serialization_error", message, Vec::new())
    }

    pub fn ledger_init_permission_denied(path: &Path, detail: &str) -> Self {
        let location = path.display().to_string();
        Self::new(
            "ledger_init_permission_denied",
            &format!("Cannot initialize ledger at `{location}`: {detail}"),
            vec![format!(
                "Grant write access to `{location}` or set `DRIGGSBY_HOME` to a writable directory."
            )],
        )
    }

    pub fn ledger_locked(path: &Path) -> Self {
        let location = path.display().to_string();
        Self::new(
            "ledger_locked",
            &format!("Ledger database is locked at `{location}`."),
            vec![format!(
                "Close other processes using `{location}` so the lock is released."
            )],
        )
    }

    pub fn ledger_corrupt(path: &Path) -> Self {
        let location = path.display().to_string();
        Self::new(
            "ledger_corrupt",
            &format!("Ledger database appears corrupt at `{location}`."),
            vec![format!(
                "Replace `{location}` with a valid SQLite ledger file or restore from backup."
            )],
        )
    }

    pub fn migration_failed(path: &Path, detail: &str) -> Self {
        let location = path.display().to_string();
        Self::new(
            "migration_failed",
            &format!("Ledger migration failed at `{location}`: {detail}"),
            vec!["Resolve conflicting schema objects referenced in the error details.".to_string()],
        )
    }

    pub fn ledger_init_failed(path: &Path, detail: &str) -> Self {
        let location = path.display().to_string();
        Self::new(
            "ledger_init_failed",
            &format!("Ledger initialization failed at `{location}`: {detail}"),
            Vec::new(),
        )
    }
}

fn merge_import_help_data(mut data: Value) -> Value {
    if !data.is_object() {
        data = json!({});
    }

    if let Some(object) = data.as_object_mut() {
        object.insert(
            "help_command".to_string(),
            Value::String(IMPORT_HELP_COMMAND.to_string()),
        );
        object.insert(
            "help_section_title".to_string(),
            Value::String(IMPORT_HELP_SECTION_TITLE.to_string()),
        );
    }

    data
}

pub type ClientResult<T> = Result<T, ClientError>;
