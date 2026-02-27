mod cli;
mod dispatch;
mod output;
mod stdout_io;

use std::process::ExitCode;

use clap::{Parser, error::ErrorKind};
use driggsby_client::ClientError;
use stdout_io::write_stdout_text;

const ROOT_HELP: &str = "Driggsby - personal finance intelligence layer

Usage:
  driggsby <command>

Start here:
  driggsby account list
  driggsby import create --help
  driggsby db schema
";

const TOP_LEVEL_HELP: &str = "Driggsby — personal finance intelligence layer

USAGE: driggsby <command>

Try it:
  driggsby demo dash                                      Open sample dashboard with bundled data
  driggsby demo recurring                                 Preview sample recurring patterns
  driggsby demo anomalies                                 Preview sample anomaly detection

Import your transactions:
  1. driggsby import create --help                        Read import schema and workflow details
  2. driggsby import create --dry-run <path>              Safely validate import without data writes
  3. driggsby import create <path>                        Import transactions

View Driggsby analysis (refreshed on each new import):
  driggsby recurring                                      Detect recurring transactions
  driggsby anomalies                                      Detect spending anomalies
  driggsby dash                                           Open web dashboard (prints URL, attempts browser open)

Need to do custom analysis? Run SQL against our views:
  1. driggsby db schema                                   Get DB path and view names
  2. driggsby db sql \"SELECT * FROM v1_transactions LIMIT 5;\"

Other commands:
  driggsby account list                                   Show account-level ledger orientation
  driggsby import list                                    List past imports
  driggsby import keys uniq                               List canonical import identifiers
  driggsby import undo <import-id>                        Undo an import

Want to ensure a clean first run, or having issues/errors?
  Run `driggsby import create --help` for import workflow guidance,
  or `driggsby <command> --help` for command usage.
";

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(code) => code,
    }
}

fn run() -> Result<ExitCode, ExitCode> {
    let raw_args = std::env::args().collect::<Vec<String>>();
    if raw_args.len() == 1 {
        if write_stdout_text(ROOT_HELP).is_err() {
            return Err(ExitCode::from(2));
        }
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(error) = removed_schema_command_error(&raw_args) {
        let mode = infer_requested_output_mode(&raw_args);
        if output::print_failure(&error, mode).is_err() {
            return Err(ExitCode::from(2));
        }
        return Err(ExitCode::from(1));
    }
    let parsed = cli::Cli::try_parse();
    let cli = match parsed {
        Ok(value) => value,
        Err(err) => {
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp
                    | ErrorKind::DisplayVersion
                    | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            ) {
                if matches!(
                    err.kind(),
                    ErrorKind::DisplayHelp | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                ) {
                    if is_top_level_help_request(&raw_args) {
                        if write_stdout_text(TOP_LEVEL_HELP).is_err() {
                            return Err(ExitCode::from(2));
                        }
                    } else if write_stdout_text(&err.to_string()).is_err() {
                        return Err(ExitCode::from(2));
                    }
                } else if write_stdout_text(&err.to_string()).is_err() {
                    return Err(ExitCode::from(2));
                }
                return Ok(ExitCode::SUCCESS);
            }
            let command_hint = if matches!(
                err.kind(),
                ErrorKind::MissingRequiredArgument
                    | ErrorKind::InvalidValue
                    | ErrorKind::ValueValidation
                    | ErrorKind::WrongNumberOfValues
                    | ErrorKind::UnknownArgument
                    | ErrorKind::InvalidSubcommand
            ) {
                command_path_from_args(&raw_args)
            } else {
                None
            };
            let clean_message = strip_clap_boilerplate(&err.to_string());
            let parse_error =
                parse_error_with_command_hint(&clean_message, command_hint.as_deref());
            let mode = infer_requested_output_mode(&raw_args);
            if output::print_failure(&parse_error, mode).is_err() {
                return Err(ExitCode::from(2));
            }
            return Err(ExitCode::from(1));
        }
    };
    let mode = output::mode_for_command(&cli.command);

    let dispatched = dispatch::dispatch(&cli);
    match dispatched {
        Ok(success) => {
            if output::print_success(&success, mode).is_err() {
                return Err(ExitCode::from(2));
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            if output::print_failure(&error, mode).is_err() {
                return Err(ExitCode::from(2));
            }
            Err(exit_code_for_error(&error))
        }
    }
}

fn is_top_level_help_request(raw_args: &[String]) -> bool {
    raw_args.len() == 2 && matches!(raw_args[1].as_str(), "--help" | "-h")
}

/// Strips clap's trailing boilerplate (Usage line, "For more information" hint)
/// so our "What to do next" section is the single source of guidance.
fn strip_clap_boilerplate(message: &str) -> String {
    let trimmed = if let Some(pos) = message.find("\n\nUsage:") {
        &message[..pos]
    } else if let Some(pos) = message.find("\nFor more information") {
        &message[..pos]
    } else {
        message
    };
    trimmed.trim_end().to_string()
}

/// Builds the subcommand path from raw CLI args for use in help hints.
///
/// Collects non-flag, non-path-like arguments after the binary name to form
/// a command string like "import undo" or "db schema view".
fn command_path_from_args(raw_args: &[String]) -> Option<String> {
    let non_flags: Vec<&str> = raw_args
        .iter()
        .skip(1)
        .filter(|value| !value.starts_with('-'))
        .map(String::as_str)
        .collect();
    if non_flags.is_empty() {
        return None;
    }

    let hint = match non_flags.as_slice() {
        ["account", "list", ..] => Some("account list"),
        ["account", ..] => Some("account"),
        ["db", "schema", "view", ..] => Some("db schema view"),
        ["db", "schema", ..] => Some("db schema"),
        ["db", "sql", ..] => Some("db sql"),
        ["db", ..] => Some("db"),
        ["import", "keys", "uniq", ..] => Some("import keys uniq"),
        ["import", "create", ..] => Some("import create"),
        ["import", "list", ..] => Some("import list"),
        ["import", "duplicates", ..] => Some("import duplicates"),
        ["import", "undo", ..] => Some("import undo"),
        ["import", "keys", ..] => Some("import keys"),
        ["import", ..] => Some("import"),
        ["intelligence", "refresh", ..] => Some("intelligence refresh"),
        ["intelligence", ..] => Some("intelligence"),
        ["demo", "dash", ..] => Some("demo dash"),
        ["demo", "recurring", ..] => Some("demo recurring"),
        ["demo", "anomalies", ..] => Some("demo anomalies"),
        ["demo", ..] => Some("demo"),
        ["anomalies", ..] => Some("anomalies"),
        ["recurring", ..] => Some("recurring"),
        ["dash", ..] => Some("dash"),
        _ => None,
    };
    hint.map(std::string::ToString::to_string)
}

fn removed_schema_command_error(raw_args: &[String]) -> Option<ClientError> {
    if raw_args.len() < 2 || raw_args[1] != "schema" {
        return None;
    }

    Some(ClientError::invalid_argument_with_recovery(
        "Top-level `schema` commands were removed.",
        vec![
            "Run `driggsby db schema` for DB discovery.".to_string(),
            "Run `driggsby db --help` for DB command usage.".to_string(),
        ],
    ))
}

fn parse_error_with_command_hint(clean_message: &str, command_hint: Option<&str>) -> ClientError {
    if command_hint == Some("intelligence") {
        return ClientError::invalid_argument_with_recovery(
            clean_message,
            vec![
                "Run `driggsby intelligence refresh` to rebuild recurring/anomaly materializations."
                    .to_string(),
                "Run `driggsby intelligence --help` for maintenance command usage.".to_string(),
            ],
        );
    }

    if command_hint == Some("db sql") && clean_message.contains("unexpected argument") {
        return ClientError::invalid_argument_with_recovery(
            "SQL must be provided as one quoted argument, or via --file/--file -.",
            vec![
                "Quote inline SQL: `driggsby db sql \"SELECT * FROM v1_transactions LIMIT 5;\"`."
                    .to_string(),
                "Use a file path: `driggsby db sql --file query.sql`.".to_string(),
                "Use stdin: `cat query.sql | driggsby db sql --file -`.".to_string(),
            ],
        );
    }

    ClientError::invalid_argument_for_command(clean_message, command_hint)
}

fn exit_code_for_error(error: &ClientError) -> ExitCode {
    if is_internal_error(error) {
        ExitCode::from(2)
    } else {
        ExitCode::from(1)
    }
}

fn infer_requested_output_mode(raw_args: &[String]) -> output::OutputMode {
    if raw_args.iter().skip(1).any(|value| value == "--json") {
        return output::OutputMode::Json;
    }
    output::OutputMode::Text
}

fn is_internal_error(error: &ClientError) -> bool {
    error.code.starts_with("internal_")
        || matches!(
            error.code.as_str(),
            "ledger_init_permission_denied"
                | "ledger_locked"
                | "ledger_corrupt"
                | "migration_failed"
                | "ledger_init_failed"
        )
}
