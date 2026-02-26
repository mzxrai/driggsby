use std::fs;
use std::io::{IsTerminal, Read};

use crate::contracts::types::ImportWarning;
use crate::import::invalid_input_error;
use crate::{ClientError, ClientResult};

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum SourceKind {
    File,
    Stdin,
}

impl SourceKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Stdin => "stdin",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedSource {
    pub(crate) source_kind: SourceKind,
    pub(crate) source_ref: Option<String>,
    pub(crate) content: String,
    pub(crate) source_used: Option<String>,
    pub(crate) source_ignored: Option<String>,
    pub(crate) source_conflict: bool,
    pub(crate) warnings: Vec<ImportWarning>,
}

pub(crate) fn resolve_source(
    path: Option<String>,
    stdin_override: Option<String>,
) -> ClientResult<ResolvedSource> {
    let stdin_body = read_stdin(stdin_override)?;
    let has_stdin = stdin_body
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    if let Some(path_value) = path {
        if path_value == "-" {
            if let Some(stdin_value) = stdin_body
                && !stdin_value.trim().is_empty()
            {
                return Ok(ResolvedSource {
                    source_kind: SourceKind::Stdin,
                    source_ref: None,
                    content: stdin_value,
                    source_used: Some("stdin".to_string()),
                    source_ignored: None,
                    source_conflict: false,
                    warnings: Vec::new(),
                });
            }

            return Err(invalid_input_error(
                "Path `-` means stdin input, but stdin was empty. Pipe JSON/CSV input or pass a file path.",
            ));
        }

        let file_body = fs::read_to_string(&path_value).map_err(|error| {
            ClientError::invalid_argument_with_recovery(
                &format!("Could not read import file `{path_value}`: {error}"),
                vec![
                    "Verify the path exists and is readable.".to_string(),
                    "Rerun driggsby import create <path>.".to_string(),
                ],
            )
        })?;

        if has_stdin {
            return Err(invalid_input_error(
                "Both stdin and file input were provided. Pass exactly one source: either a file path or piped stdin.",
            ));
        }

        return Ok(ResolvedSource {
            source_kind: SourceKind::File,
            source_ref: Some(path_value),
            content: file_body,
            source_used: Some("file".to_string()),
            source_ignored: None,
            source_conflict: false,
            warnings: Vec::<ImportWarning>::new(),
        });
    }

    if let Some(stdin_value) = stdin_body
        && !stdin_value.trim().is_empty()
    {
        return Ok(ResolvedSource {
            source_kind: SourceKind::Stdin,
            source_ref: None,
            content: stdin_value,
            source_used: Some("stdin".to_string()),
            source_ignored: None,
            source_conflict: false,
            warnings: Vec::new(),
        });
    }

    Err(invalid_input_error(
        "No import source provided. Pass a file path or pipe input via stdin.",
    ))
}

fn read_stdin(stdin_override: Option<String>) -> ClientResult<Option<String>> {
    if let Some(value) = stdin_override {
        return Ok(Some(value));
    }

    if std::io::stdin().is_terminal() {
        return Ok(None);
    }

    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|error| {
            ClientError::invalid_argument_with_recovery(
                &format!("Could not read stdin: {error}"),
                vec![
                    "Retry with an explicit file path argument.".to_string(),
                    "Or rerun with valid stdin content.".to_string(),
                ],
            )
        })?;

    if buffer.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(buffer))
}
