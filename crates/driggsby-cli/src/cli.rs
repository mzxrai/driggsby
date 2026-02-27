use chrono::NaiveDate;
use clap::{Parser, Subcommand};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsoDate(pub String);

impl IsoDate {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn parse_iso_date(value: &str) -> Result<IsoDate, String> {
    if value.len() != 10 {
        return Err("date must use YYYY-MM-DD format".to_string());
    }

    let bytes = value.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err("date must use YYYY-MM-DD format".to_string());
    }

    for index in [0usize, 1, 2, 3, 5, 6, 8, 9] {
        if !bytes[index].is_ascii_digit() {
            return Err("date must use YYYY-MM-DD format".to_string());
        }
    }

    if NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() {
        return Err("date must use valid calendar values".to_string());
    }

    Ok(IsoDate(value.to_string()))
}

pub fn parse_import_key_property(value: &str) -> Result<String, String> {
    match value {
        "account_key" | "account_type" | "currency" | "merchant" | "category" => {
            Ok(value.to_string())
        }
        _ => Err(
            "property must be one of: account_key, account_type, currency, merchant, category"
                .to_string(),
        ),
    }
}

/// Extended help shown after `driggsby import create --help`.
/// Contains workflow guidance, schema, and next-step instructions.
pub const IMPORT_CREATE_AFTER_HELP: &str = "\
How import works:
  Driggsby does not parse raw bank PDFs or provider-specific CSVs.
  You parse each statement into a normalized file, then import it.

  Accepted formats:
    JSON — one top-level array of transaction objects
    CSV  — one header row with schema field names

  <path> is a local file path.
  To read stdin explicitly, use `-` as the path.
  Example: cat rows.json | driggsby import create --dry-run -
  One import call takes one file. For multiple files, combine
  first or run multiple import commands.

What to do next:
  1. If you have made previous imports and are unsure about canonical ledger keys,
     run `driggsby import keys uniq`.
  2. Parse your source into normalized JSON or schema-matching CSV.
  3. Run `driggsby import create --dry-run <path>` and fix any reported issues.
  4. Run `driggsby import create <path>` once dry-run passes.

Import schema:
  JSON example (one top-level array):
  [
    {
      \"account_key\": \"chase_checking_1234\",
      \"account_type\": \"checking\",
      \"posted_at\": \"2026-01-15\",
      \"amount\": -42.15,
      \"currency\": \"USD\",
      \"description\": \"WHOLE FOODS\",
      \"statement_id\": \"chase_checking_1234_2026-01-31\",
      \"external_id\": \"txn_12345\",
      \"merchant\": \"Whole Foods\",
      \"category\": \"Groceries\"
    }
  ]

  CSV example (header + rows):
  account_key,account_type,posted_at,amount,currency,description,statement_id,external_id,merchant,category
  chase_checking_1234,checking,2026-01-15,-42.15,USD,WHOLE FOODS,chase_checking_1234_2026-01-31,txn_12345,Whole Foods,Groceries
  chase_checking_1234,checking,2026-01-16,42.15,USD,REFUND,chase_checking_1234_2026-01-31,txn_12346,Whole Foods,Groceries

Stability rule (important):
  Keep canonical identifiers and labels exactly the same across imports.
  This includes `account_key`, `currency`, `merchant`, and `category`.
  When known, keep `account_type` stable too.
  If these drift over time, your ledger analysis will drift too.
  Before mapping new files, run `driggsby import keys uniq` and copy those canonical values.

Field rules (very explicit):
  account_key (required):
    A stable account name. Pick one value and keep it the same forever.
    Example: `chase_checking_1234`

  account_type (optional but recommended):
    Canonical values:
      checking, savings, credit_card, loan, brokerage, retirement, hsa, other
    Common aliases are accepted and normalized automatically, including:
      creditcard, credit-card, retirement_401k, 401k_retirement, investment_taxable
    If provided for an account_key, keep it consistent forever.

  posted_at (required):
    Date only, exactly `YYYY-MM-DD`.
    Example: `2026-01-15`

  amount (required):
    A number, not text.
    Signed amount rules (strict):
    - negative = money out (`spend`, `card charge`)
    - positive = money in (`refund`, `payment`, `credit`)
    Use exactly one sign convention everywhere. Do not flip signs between imports.
    Use at most 2 decimal places.
    Example charge: `-42.15`
    Example refund/payment: `42.15`

  currency (required):
    3-letter ISO code.
    Example: `USD`

  description (required):
    Raw transaction text from the source.

  statement_id (optional):
    Statement grouping key when you have statement boundaries.
    Use `statement_id` = `<account_key>_<statement_end_YYYY-MM-DD>` when available.
    Never reuse the same `statement_id` across different imports/statements.
    Imports with reused provided `statement_id` values are rejected.
    Example: `chase_checking_1234_2026-01-31`

  external_id (optional):
    Upstream transaction ID if your bank/export provides one.
    If present in your source, keep it exactly as given.
    Helps stronger dedupe across imports.

  merchant (optional):
    Clean merchant name if you know it.
    If you do not know it, omit it.

  category (optional):
    Clean category label if you know it.
    If you do not know it, omit it.
";

#[derive(Debug, Parser)]
#[command(
    name = "driggsby",
    version,
    about = "personal finance intelligence layer",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Manage account-level ledger orientation commands
    #[command(arg_required_else_help = true)]
    Account {
        #[command(subcommand)]
        command: AccountCommand,
    },
    /// Database discovery and query commands
    #[command(arg_required_else_help = true)]
    Db {
        #[command(subcommand)]
        command: DbCommand,
    },
    /// Manage transaction imports
    #[command(arg_required_else_help = true)]
    Import {
        #[command(subcommand)]
        command: ImportCommand,
    },
    /// Preview Driggsby capabilities using bundled sample data
    #[command(arg_required_else_help = true)]
    Demo {
        #[command(subcommand)]
        command: DemoCommand,
    },
    /// Detect unusual spending patterns in your imported transactions
    Anomalies {
        /// Start date filter (YYYY-MM-DD)
        #[arg(long, value_parser = parse_iso_date)]
        from: Option<IsoDate>,
        /// End date filter (YYYY-MM-DD)
        #[arg(long, value_parser = parse_iso_date)]
        to: Option<IsoDate>,
        /// Emit structured JSON object output for machine parsing
        #[arg(long)]
        json: bool,
    },
    /// Detect recurring transaction patterns in your imported data
    Recurring {
        /// Start date filter (YYYY-MM-DD)
        #[arg(long, value_parser = parse_iso_date)]
        from: Option<IsoDate>,
        /// End date filter (YYYY-MM-DD)
        #[arg(long, value_parser = parse_iso_date)]
        to: Option<IsoDate>,
        /// Emit structured JSON object output for machine parsing
        #[arg(long)]
        json: bool,
    },
    /// Open the Driggsby web dashboard in your browser
    Dash,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DbCommand {
    /// Show your local database path, connection URI, and public view contracts
    Schema {
        #[command(subcommand)]
        command: Option<SchemaCommand>,
    },
    /// Run a read-only SQL query against public v1_* views
    Sql {
        /// Inline SQL query to execute
        query: Option<String>,
        /// Read SQL from a file path, or `-` for stdin
        #[arg(long)]
        file: Option<String>,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum AccountCommand {
    /// Show account-level orientation for your current ledger
    List {
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum SchemaCommand {
    /// Show column details for a specific public view
    View {
        /// Name of the view to inspect (e.g. v1_transactions)
        view_name: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ImportCommand {
    /// Import normalized transaction data into your local Driggsby ledger
    #[command(after_long_help = IMPORT_CREATE_AFTER_HELP)]
    Create {
        /// Validate import data without writing to the ledger
        #[arg(long)]
        dry_run: bool,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
        /// Path to a normalized JSON or CSV file (use `-` for stdin)
        path: Option<String>,
    },
    /// List all past imports with their status and row counts
    List {
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Inspect rows this import was deduped against, with match context
    Duplicates {
        /// The import ID to inspect (e.g. imp_abc123)
        import_id: String,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// List canonical unique values for high-drift import properties
    #[command(arg_required_else_help = true)]
    Keys {
        #[command(subcommand)]
        command: ImportKeysCommand,
    },
    /// Revert a previously committed import and restore overwritten transactions
    Undo {
        /// The import ID to revert (e.g. imp_abc123)
        import_id: String,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ImportKeysCommand {
    /// List canonical unique values for one tracked property or all tracked properties
    Uniq {
        /// Optional property filter: account_key, account_type, currency, merchant, or category
        #[arg(value_parser = parse_import_key_property)]
        property: Option<String>,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum DemoCommand {
    /// Open the demo dashboard with bundled sample data
    Dash,
    /// Preview sample recurring transaction patterns
    Recurring,
    /// Preview sample spending anomaly detection
    Anomalies,
}

#[cfg(test)]
pub fn parse_from<I, T>(itr: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::try_parse_from(itr)
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::{AccountCommand, Commands, DemoCommand, ImportCommand, parse_from};

    #[test]
    fn parse_command_paths() {
        let cases: [Vec<&str>; 26] = [
            vec!["driggsby", "account", "list"],
            vec!["driggsby", "account", "list", "--json"],
            vec!["driggsby", "db", "schema"],
            vec!["driggsby", "db", "schema", "view", "v1_transactions"],
            vec!["driggsby", "db", "sql", "SELECT 1"],
            vec!["driggsby", "db", "sql", "SELECT 1", "--json"],
            vec!["driggsby", "db", "sql", "--file", "./query.sql"],
            vec!["driggsby", "db", "sql", "--file", "-"],
            vec!["driggsby", "import", "create"],
            vec![
                "driggsby",
                "import",
                "create",
                "--dry-run",
                "./statement.csv",
            ],
            vec!["driggsby", "import", "create", "./statement.csv", "--json"],
            vec!["driggsby", "import", "list", "--json"],
            vec!["driggsby", "import", "undo", "imp_1", "--json"],
            vec!["driggsby", "import", "undo", "imp_1"],
            vec!["driggsby", "import", "duplicates", "imp_1"],
            vec!["driggsby", "import", "duplicates", "imp_1", "--json"],
            vec!["driggsby", "import", "list"],
            vec!["driggsby", "import", "keys", "uniq"],
            vec![
                "driggsby",
                "import",
                "keys",
                "uniq",
                "account_key",
                "--json",
            ],
            vec!["driggsby", "demo", "dash"],
            vec!["driggsby", "demo", "anomalies"],
            vec![
                "driggsby",
                "anomalies",
                "--from",
                "2026-01-01",
                "--to",
                "2026-02-01",
            ],
            vec!["driggsby", "anomalies", "--json"],
            vec!["driggsby", "recurring", "--from", "2026-01-01"],
            vec!["driggsby", "recurring", "--json"],
            vec!["driggsby", "dash"],
        ];

        for case in cases {
            let parsed = parse_from(case.clone());
            assert!(parsed.is_ok(), "failed to parse: {case:?}");
        }
    }

    #[test]
    fn parse_db_schema_view_path() {
        let parsed = parse_from(["driggsby", "db", "schema", "view", "v1_transactions"]);
        assert!(parsed.is_ok());
    }

    #[test]
    fn parse_account_list_subcommand() {
        let parsed = parse_from(["driggsby", "account", "list", "--json"]);
        assert!(parsed.is_ok());
        if let Ok(cli) = parsed {
            assert!(matches!(
                cli.command,
                Commands::Account {
                    command: AccountCommand::List { json: true }
                }
            ));
        }
    }

    #[test]
    fn parse_import_subcommands() {
        let parsed = parse_from(["driggsby", "import", "undo", "imp_1"]);
        assert!(parsed.is_ok());
        if let Ok(cli) = parsed {
            assert!(matches!(
                cli.command,
                Commands::Import {
                    command: ImportCommand::Undo { .. },
                }
            ));
        }

        let parsed_list = parse_from(["driggsby", "import", "list"]);
        assert!(parsed_list.is_ok());

        let parsed_duplicates = parse_from(["driggsby", "import", "duplicates", "imp_1"]);
        assert!(parsed_duplicates.is_ok());

        let parsed_json = parse_from(["driggsby", "import", "list", "--json"]);
        assert!(parsed_json.is_ok());
        if let Ok(cli) = parsed_json {
            assert!(matches!(
                cli.command,
                Commands::Import {
                    command: ImportCommand::List { json: true },
                }
            ));
        }

        let parsed_keys = parse_from(["driggsby", "import", "keys", "uniq"]);
        assert!(parsed_keys.is_ok());

        let parsed_keys_with_property =
            parse_from(["driggsby", "import", "keys", "uniq", "merchant", "--json"]);
        assert!(parsed_keys_with_property.is_ok());

        let parsed_account_type = parse_from([
            "driggsby",
            "import",
            "keys",
            "uniq",
            "account_type",
            "--json",
        ]);
        assert!(parsed_account_type.is_ok());
    }

    #[test]
    fn parse_demo_subcommands() {
        let dash = parse_from(["driggsby", "demo", "dash"]);
        assert!(dash.is_ok());
        if let Ok(cli) = dash {
            assert!(matches!(
                cli.command,
                Commands::Demo {
                    command: DemoCommand::Dash
                }
            ));
        }

        let recurring = parse_from(["driggsby", "demo", "recurring"]);
        assert!(recurring.is_ok());
        if let Ok(cli) = recurring {
            assert!(matches!(
                cli.command,
                Commands::Demo {
                    command: DemoCommand::Recurring
                }
            ));
        }

        let anomalies = parse_from(["driggsby", "demo", "anomalies"]);
        assert!(anomalies.is_ok());
        if let Ok(cli) = anomalies {
            assert!(matches!(
                cli.command,
                Commands::Demo {
                    command: DemoCommand::Anomalies
                }
            ));
        }
    }

    #[test]
    fn bare_demo_shows_help() {
        let parsed = parse_from(["driggsby", "demo"]);
        assert!(parsed.is_err());
        if let Err(err) = parsed {
            assert_eq!(
                err.kind(),
                ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            );
        }
    }

    #[test]
    fn invalid_date_is_rejected() {
        let parsed = parse_from(["driggsby", "anomalies", "--from", "2026-99-01"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn parse_intelligence_json_flags() {
        let anomalies = parse_from(["driggsby", "anomalies", "--json"]);
        assert!(anomalies.is_ok());
        if let Ok(cli) = anomalies {
            assert!(matches!(
                cli.command,
                Commands::Anomalies { json: true, .. }
            ));
        }

        let recurring = parse_from(["driggsby", "recurring", "--json"]);
        assert!(recurring.is_ok());
        if let Ok(cli) = recurring {
            assert!(matches!(
                cli.command,
                Commands::Recurring { json: true, .. }
            ));
        }
    }

    #[test]
    fn parse_import_json_flags() {
        let run = parse_from(["driggsby", "import", "create", "./rows.csv", "--json"]);
        assert!(run.is_ok());
        if let Ok(cli) = run {
            assert!(matches!(
                cli.command,
                Commands::Import {
                    command: ImportCommand::Create {
                        json: true,
                        path: Some(_),
                        ..
                    },
                }
            ));
        }

        let list = parse_from(["driggsby", "import", "list", "--json"]);
        assert!(list.is_ok());
        if let Ok(cli) = list {
            assert!(matches!(
                cli.command,
                Commands::Import {
                    command: ImportCommand::List { json: true },
                }
            ));
        }

        let undo = parse_from(["driggsby", "import", "undo", "imp_1", "--json"]);
        assert!(undo.is_ok());
        if let Ok(cli) = undo {
            assert!(matches!(
                cli.command,
                Commands::Import {
                    command: ImportCommand::Undo { json: true, .. },
                }
            ));
        }

        let duplicates = parse_from(["driggsby", "import", "duplicates", "imp_1", "--json"]);
        assert!(duplicates.is_ok());

        let keys_uniq = parse_from(["driggsby", "import", "keys", "uniq", "--json"]);
        assert!(keys_uniq.is_ok());
    }

    #[test]
    fn parse_unsupported_json_flags_are_rejected() {
        let schema = parse_from(["driggsby", "db", "schema", "--json"]);
        assert!(schema.is_err());

        let schema_view = parse_from([
            "driggsby",
            "db",
            "schema",
            "view",
            "v1_transactions",
            "--json",
        ]);
        assert!(schema_view.is_err());

        let dash = parse_from(["driggsby", "dash", "--json"]);
        assert!(dash.is_err());
    }

    #[test]
    fn parse_removed_top_level_schema_command_is_rejected() {
        let schema = parse_from(["driggsby", "schema"]);
        assert!(schema.is_err());

        let schema_view = parse_from(["driggsby", "schema", "view", "v1_transactions"]);
        assert!(schema_view.is_err());
    }

    #[test]
    fn parse_import_keys_invalid_property_is_rejected() {
        let parsed = parse_from(["driggsby", "import", "keys", "uniq", "acct_key"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn bare_import_shows_help() {
        let parsed = parse_from(["driggsby", "import"]);
        assert!(parsed.is_err());
        if let Err(err) = parsed {
            assert_eq!(
                err.kind(),
                ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            );
        }
    }

    #[test]
    fn bare_account_shows_help() {
        let parsed = parse_from(["driggsby", "account"]);
        assert!(parsed.is_err());
        if let Err(err) = parsed {
            assert_eq!(
                err.kind(),
                ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            );
        }
    }

    #[test]
    fn help_command_is_rejected() {
        let parsed = parse_from(["driggsby", "help"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn subcommand_help_uses_clap_display_help() {
        let parsed = parse_from(["driggsby", "import", "--help"]);
        assert!(parsed.is_err());
        if let Err(err) = parsed {
            assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        }
    }

    #[test]
    fn import_create_help_uses_clap_display_help() {
        let parsed = parse_from(["driggsby", "import", "create", "--help"]);
        assert!(parsed.is_err());
        if let Err(err) = parsed {
            assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        }
    }

    #[test]
    fn guide_command_is_rejected() {
        let parsed = parse_from(["driggsby", "guide"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn invalid_demo_subcommand_is_rejected() {
        let parsed = parse_from(["driggsby", "demo", "unknown"]);
        assert!(parsed.is_err());

        let parsed_schema = parse_from(["driggsby", "demo", "schema"]);
        assert!(parsed_schema.is_err());
    }

    #[test]
    fn dry_run_and_json_both_accepted_on_import_create() {
        let parsed = parse_from([
            "driggsby",
            "import",
            "create",
            "--dry-run",
            "rows.csv",
            "--json",
        ]);
        assert!(parsed.is_ok());
        if let Ok(cli) = parsed {
            assert!(matches!(
                cli.command,
                Commands::Import {
                    command: ImportCommand::Create {
                        dry_run: true,
                        json: true,
                        ..
                    },
                }
            ));
        }
    }
}
