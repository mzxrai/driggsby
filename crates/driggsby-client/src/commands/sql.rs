use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, Error as SqliteError, ffi::ErrorCode};
use serde_json::Value;

use crate::contracts::envelope::{SuccessEnvelope, success};
use crate::contracts::types::{SqlColumn, SqlQueryData};
use crate::setup::{ensure_initialized, ensure_initialized_at};
use crate::state::open_readonly_connection;
use crate::{ClientError, ClientResult};

const DEFAULT_MAX_ROWS: usize = 1000;
const HARD_MAX_ROWS: usize = 10000;
const MAX_SQL_LENGTH: usize = 65_536;
const ALLOWED_SQL_FUNCTIONS: [&str; 17] = [
    "abs", "avg", "coalesce", "count", "date", "datetime", "ifnull", "length", "lower", "max",
    "min", "nullif", "printf", "round", "strftime", "substr", "sum",
];

#[derive(Debug, Clone)]
pub struct SqlQueryOptions<'a> {
    pub query: Option<String>,
    pub file: Option<String>,
    pub home_override: Option<&'a Path>,
    pub stdin_override: Option<String>,
    pub max_rows: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SqlSource {
    Inline,
    File { path: String },
    Stdin,
}

pub fn run(query: Option<String>, file: Option<String>) -> ClientResult<SuccessEnvelope> {
    run_with_options(SqlQueryOptions {
        query,
        file,
        home_override: None,
        stdin_override: None,
        max_rows: None,
    })
}

#[doc(hidden)]
pub fn run_with_options(options: SqlQueryOptions<'_>) -> ClientResult<SuccessEnvelope> {
    let setup = if let Some(home) = options.home_override {
        ensure_initialized_at(home)?
    } else {
        ensure_initialized()?
    };

    let (sql, source) = resolve_sql_source(
        options.query,
        options.file,
        options.stdin_override.as_deref(),
    )?;
    validate_sql_input(&sql)?;

    let max_rows = normalize_max_rows(options.max_rows)?;

    let db_path = PathBuf::from(&setup.db_path);
    let connection = open_readonly_connection(&db_path)?;
    let allowed_views = setup
        .public_views
        .iter()
        .map(|view| view.name.clone())
        .collect::<Vec<String>>();

    install_readonly_authorizer(&connection, &allowed_views)
        .map_err(|error| map_query_error(&db_path, &error))?;

    let data = execute_query(&connection, &db_path, &sql, &source, max_rows)?;
    success("db sql", data)
}

fn resolve_sql_source(
    query: Option<String>,
    file: Option<String>,
    stdin_override: Option<&str>,
) -> ClientResult<(String, SqlSource)> {
    let has_inline = query.is_some();
    let has_file = file.is_some();

    if has_inline && has_file {
        return Err(sql_source_error());
    }

    if let Some(inline_query) = query {
        return Ok((inline_query, SqlSource::Inline));
    }

    if let Some(file_path) = file {
        if file_path == "-" {
            let stdin_body = if let Some(override_body) = stdin_override {
                override_body.to_string()
            } else {
                let mut buffer = String::new();
                std::io::stdin()
                    .read_to_string(&mut buffer)
                    .map_err(|error| {
                        ClientError::invalid_argument_with_recovery(
                            &format!("Failed to read SQL from stdin: {error}"),
                            vec!["Pass an inline SQL query, or provide --file <path>.".to_string()],
                        )
                    })?;
                buffer
            };
            return Ok((stdin_body, SqlSource::Stdin));
        }

        let file_body = fs::read_to_string(&file_path).map_err(|error| {
            ClientError::invalid_argument_with_recovery(
                &format!("Failed to read SQL file `{file_path}`: {error}"),
                vec![
                    "Check the file path and read permissions, then retry.".to_string(),
                    "Or pass an inline SQL query directly to `driggsby db sql`.".to_string(),
                ],
            )
        })?;

        return Ok((file_body, SqlSource::File { path: file_path }));
    }

    Err(sql_source_error())
}

fn sql_source_error() -> ClientError {
    ClientError::invalid_argument_with_recovery(
        "Provide exactly one SQL source: inline query arg, --file <path>, or --file - for stdin.",
        vec![
            "Use `driggsby db sql \"SELECT * FROM v1_transactions LIMIT 5;\"`.".to_string(),
            "Or use `driggsby db sql --file <path-to-query.sql>`.".to_string(),
        ],
    )
}

fn validate_sql_input(sql: &str) -> ClientResult<()> {
    if sql.is_empty() || sql.trim().is_empty() {
        return Err(ClientError::invalid_argument_with_recovery(
            "SQL query cannot be empty.",
            vec!["Provide a non-empty SQL query and retry.".to_string()],
        ));
    }

    if sql.as_bytes().contains(&0) {
        return Err(ClientError::invalid_argument_with_recovery(
            "SQL query contains unsupported NUL bytes.",
            vec!["Remove NUL bytes and retry the query.".to_string()],
        ));
    }

    if sql.len() > MAX_SQL_LENGTH {
        return Err(ClientError::invalid_argument_with_recovery(
            &format!("SQL query exceeds max length ({MAX_SQL_LENGTH} characters)."),
            vec![
                "Shorten the query and rerun `driggsby db sql`.".to_string(),
                "For long workflows, split your query into smaller statements.".to_string(),
            ],
        ));
    }

    Ok(())
}

fn normalize_max_rows(max_rows: Option<usize>) -> ClientResult<usize> {
    let resolved = max_rows.unwrap_or(DEFAULT_MAX_ROWS);
    if resolved == 0 || resolved > HARD_MAX_ROWS {
        return Err(ClientError::invalid_argument_with_recovery(
            &format!("max_rows must be between 1 and {HARD_MAX_ROWS}."),
            vec!["Retry with a valid max_rows value.".to_string()],
        ));
    }
    Ok(resolved)
}

fn install_readonly_authorizer(
    connection: &Connection,
    allowed_views: &[String],
) -> rusqlite::Result<()> {
    let allowed = allowed_views
        .iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<String>>();

    connection.authorizer(Some(move |context: AuthContext<'_>| {
        if authorize_action(context, &allowed) {
            Authorization::Allow
        } else {
            Authorization::Deny
        }
    }))
}

fn authorize_action(context: AuthContext<'_>, allowed_views: &[String]) -> bool {
    match context.action {
        AuthAction::Select => true,
        AuthAction::Read { table_name, .. } => {
            is_allowed_read_access(table_name, context.accessor, allowed_views)
        }
        AuthAction::Function { function_name } => is_allowed_function(function_name),
        _ => false,
    }
}

fn is_allowed_read_access(
    table_name: &str,
    accessor: Option<&str>,
    allowed_views: &[String],
) -> bool {
    let table_lower = table_name.to_lowercase();
    if table_lower.starts_with("sqlite_") {
        return false;
    }

    if allowed_views.iter().any(|view| view == &table_lower) {
        return true;
    }

    if let Some(accessor_name) = accessor {
        let accessor_lower = accessor_name.to_lowercase();
        if allowed_views.iter().any(|view| view == &accessor_lower) {
            return true;
        }
    }

    false
}

fn is_allowed_function(function_name: &str) -> bool {
    let normalized = function_name.to_ascii_lowercase();
    ALLOWED_SQL_FUNCTIONS
        .iter()
        .any(|allowed| allowed == &normalized)
}

fn execute_query(
    connection: &Connection,
    db_path: &Path,
    sql: &str,
    source: &SqlSource,
    max_rows: usize,
) -> ClientResult<SqlQueryData> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| map_query_error(db_path, &error))?;

    if !statement.readonly() {
        return Err(ClientError::invalid_argument_with_recovery(
            "SQL statement must be read-only.",
            vec![
                "Use SELECT-only queries against public `v1_*` views.".to_string(),
                "Run `driggsby db schema` to inspect supported view contracts.".to_string(),
            ],
        ));
    }

    let column_names = statement
        .column_names()
        .into_iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>();
    let mut inferred_types = vec!["unknown".to_string(); column_names.len()];
    let mut inferred_nullable = vec![false; column_names.len()];

    let mut rows_cursor = statement
        .query([])
        .map_err(|error| map_query_error(db_path, &error))?;

    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut truncated = false;
    while let Some(row) = rows_cursor
        .next()
        .map_err(|error| map_query_error(db_path, &error))?
    {
        if rows.len() >= max_rows {
            truncated = true;
            break;
        }

        let mut output_row = Vec::with_capacity(column_names.len());
        for column_index in 0..column_names.len() {
            let raw_value = row
                .get_ref(column_index)
                .map_err(|error| map_query_error(db_path, &error))?;
            let json_value = value_ref_to_json(raw_value);
            infer_column_contract(
                &json_value,
                &mut inferred_types[column_index],
                &mut inferred_nullable[column_index],
            );
            output_row.push(json_value);
        }

        rows.push(output_row);
    }

    let columns = column_names
        .iter()
        .enumerate()
        .map(|(index, name)| SqlColumn {
            name: name.clone(),
            column_type: finalize_inferred_type(&inferred_types[index], inferred_nullable[index]),
            nullable: inferred_nullable[index],
        })
        .collect::<Vec<SqlColumn>>();

    Ok(SqlQueryData {
        columns,
        row_count: rows.len() as i64,
        rows,
        truncated,
        max_rows: max_rows as i64,
        source: source_label(source).to_string(),
        source_ref: source_ref(source),
    })
}

fn infer_column_contract(value: &Value, inferred_type: &mut String, inferred_nullable: &mut bool) {
    if value.is_null() {
        *inferred_nullable = true;
        return;
    }

    let observed = infer_scalar_type(value);
    if inferred_type == "unknown" {
        *inferred_type = observed;
        return;
    }

    if inferred_type != &observed {
        *inferred_type = "mixed".to_string();
    }
}

fn infer_scalar_type(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(number) => {
            if number.is_i64() || number.is_u64() {
                "integer".to_string()
            } else {
                "real".to_string()
            }
        }
        Value::String(_) => "text".to_string(),
        _ => "unknown".to_string(),
    }
}

fn finalize_inferred_type(inferred_type: &str, inferred_nullable: bool) -> String {
    if inferred_type == "unknown" && inferred_nullable {
        "null".to_string()
    } else {
        inferred_type.to_string()
    }
}

fn value_ref_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => Value::from(number),
        ValueRef::Real(number) => Value::from(number),
        ValueRef::Text(bytes) => Value::String(String::from_utf8_lossy(bytes).to_string()),
        ValueRef::Blob(bytes) => Value::String(encode_blob_hex(bytes)),
    }
}

fn encode_blob_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn source_label(source: &SqlSource) -> &'static str {
    match source {
        SqlSource::Inline => "inline",
        SqlSource::File { .. } => "file",
        SqlSource::Stdin => "stdin",
    }
}

fn source_ref(source: &SqlSource) -> Option<String> {
    match source {
        SqlSource::File { path } => Some(path.clone()),
        _ => None,
    }
}

fn map_query_error(db_path: &Path, error: &SqliteError) -> ClientError {
    if matches!(error, SqliteError::MultipleStatement) {
        return ClientError::invalid_argument_with_recovery(
            "SQL query must contain a single statement.",
            vec![
                "Run one statement per `driggsby db sql` command.".to_string(),
                "If needed, split multi-statement SQL into separate calls.".to_string(),
            ],
        );
    }

    if let SqliteError::SqlInputError { .. } = error {
        return ClientError::invalid_argument_with_recovery(
            "Malformed SQL query.",
            vec![
                "Fix SQL syntax and retry.".to_string(),
                "Run `driggsby db schema` to inspect available views and columns.".to_string(),
            ],
        );
    }

    let code = error.sqlite_error_code();
    if matches!(code, Some(ErrorCode::AuthorizationForStatementDenied)) {
        return ClientError::invalid_argument_with_recovery(
            "SQL is restricted to read-only queries over public v1_* views.",
            vec![
                "Query only `v1_*` views and avoid internal/SQLite administration objects."
                    .to_string(),
                "Run `driggsby db schema` for view discovery.".to_string(),
                "Use built-in SQL functions only (for example count, sum, avg, min, max)."
                    .to_string(),
            ],
        );
    }

    if matches!(
        code,
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    ) {
        return ClientError::ledger_locked(db_path);
    }

    if matches!(code, Some(ErrorCode::NotADatabase)) {
        return ClientError::ledger_corrupt(db_path);
    }

    if matches!(code, Some(ErrorCode::CannotOpen | ErrorCode::ReadOnly)) {
        return ClientError::ledger_init_permission_denied(db_path, &error.to_string());
    }

    ClientError::invalid_argument_with_recovery(
        &format!("SQL query could not be executed: {error}"),
        vec![
            "Verify SQL syntax and supported built-in functions, then retry.".to_string(),
            "Run `driggsby db schema` to inspect available views and columns.".to_string(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::{SqlSource, is_allowed_function, is_allowed_read_access, resolve_sql_source};

    #[test]
    fn resolve_source_accepts_inline_file_and_stdin() {
        let inline = resolve_sql_source(Some("SELECT 1".to_string()), None, None);
        assert!(inline.is_ok());
        if let Ok((sql, source)) = inline {
            assert_eq!(sql, "SELECT 1");
            assert_eq!(source, SqlSource::Inline);
        }

        let file = resolve_sql_source(None, Some("query.sql".to_string()), None);
        assert!(file.is_err());

        let stdin = resolve_sql_source(None, Some("-".to_string()), Some("SELECT 1"));
        assert!(stdin.is_ok());
        if let Ok((sql, source)) = stdin {
            assert_eq!(sql, "SELECT 1");
            assert_eq!(source, SqlSource::Stdin);
        }
    }

    #[test]
    fn resolve_source_rejects_conflicts_and_missing_input() {
        let conflict = resolve_sql_source(
            Some("SELECT 1".to_string()),
            Some("query.sql".to_string()),
            None,
        );
        assert!(conflict.is_err());

        let missing = resolve_sql_source(None, None, None);
        assert!(missing.is_err());
    }

    #[test]
    fn allowed_read_access_enforces_view_only_policy() {
        let allowed = vec!["v1_transactions".to_string(), "v1_accounts".to_string()];

        assert!(is_allowed_read_access("v1_transactions", None, &allowed));
        assert!(is_allowed_read_access(
            "internal_transactions",
            Some("v1_transactions"),
            &allowed,
        ));
        assert!(!is_allowed_read_access(
            "internal_transactions",
            None,
            &allowed
        ));
        assert!(!is_allowed_read_access("sqlite_master", None, &allowed));
    }

    #[test]
    fn function_allowlist_is_explicit() {
        assert!(is_allowed_function("count"));
        assert!(is_allowed_function("SUM"));
        assert!(!is_allowed_function("sqlite_version"));
        assert!(!is_allowed_function("load_extension"));
    }
}
