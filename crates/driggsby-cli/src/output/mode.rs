use crate::cli::{AccountCommand, Commands, ImportCommand, ImportKeysCommand};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OutputMode {
    Text,
    Json,
}

pub fn mode_for_command(command: &Commands) -> OutputMode {
    match command {
        Commands::Account { command } => match command {
            AccountCommand::List { json } => {
                if *json {
                    OutputMode::Json
                } else {
                    OutputMode::Text
                }
            }
        },
        Commands::Import { command } => match command {
            ImportCommand::Create { json, .. }
            | ImportCommand::List { json }
            | ImportCommand::Duplicates { json, .. }
            | ImportCommand::Keys {
                command: ImportKeysCommand::Uniq { json, .. },
            }
            | ImportCommand::Undo { json, .. } => {
                if *json {
                    OutputMode::Json
                } else {
                    OutputMode::Text
                }
            }
        },
        Commands::Anomalies { json, .. } | Commands::Recurring { json, .. } => {
            if *json {
                OutputMode::Json
            } else {
                OutputMode::Text
            }
        }
        _ => OutputMode::Text,
    }
}

#[cfg(test)]
mod tests {
    use super::{OutputMode, mode_for_command};
    use crate::cli::parse_from;

    #[test]
    fn mode_uses_json_for_import_create_with_json_flag() {
        let import = parse_from(["driggsby", "import", "create", "rows.csv", "--json"]);
        assert!(import.is_ok());
        if let Ok(cli) = import {
            assert_eq!(mode_for_command(&cli.command), OutputMode::Json);
        }
    }

    #[test]
    fn mode_uses_json_for_import_create_dry_run_with_json_flag() {
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
            assert_eq!(mode_for_command(&cli.command), OutputMode::Json);
        }
    }

    #[test]
    fn mode_uses_json_for_import_list_with_json_flag() {
        let parsed = parse_from(["driggsby", "import", "list", "--json"]);
        assert!(parsed.is_ok());
        if let Ok(cli) = parsed {
            assert_eq!(mode_for_command(&cli.command), OutputMode::Json);
        }
    }

    #[test]
    fn mode_uses_json_for_import_undo_with_json_flag() {
        let parsed = parse_from(["driggsby", "import", "undo", "imp_1", "--json"]);
        assert!(parsed.is_ok());
        if let Ok(cli) = parsed {
            assert_eq!(mode_for_command(&cli.command), OutputMode::Json);
        }
    }

    #[test]
    fn mode_uses_json_for_import_duplicates_with_json_flag() {
        let parsed = parse_from(["driggsby", "import", "duplicates", "imp_1", "--json"]);
        assert!(parsed.is_ok());
        if let Ok(cli) = parsed {
            assert_eq!(mode_for_command(&cli.command), OutputMode::Json);
        }
    }

    #[test]
    fn mode_uses_json_for_import_keys_uniq_with_json_flag() {
        let parsed = parse_from(["driggsby", "import", "keys", "uniq", "--json"]);
        assert!(parsed.is_ok());
        if let Ok(cli) = parsed {
            assert_eq!(mode_for_command(&cli.command), OutputMode::Json);
        }
    }

    #[test]
    fn mode_uses_json_for_accounts_with_json_flag() {
        let parsed = parse_from(["driggsby", "account", "list", "--json"]);
        assert!(parsed.is_ok());
        if let Ok(cli) = parsed {
            assert_eq!(mode_for_command(&cli.command), OutputMode::Json);
        }
    }

    #[test]
    fn mode_uses_text_for_commands_without_json_flag() {
        let schema = parse_from(["driggsby", "schema"]);
        assert!(schema.is_ok());
        if let Ok(cli) = schema {
            assert_eq!(mode_for_command(&cli.command), OutputMode::Text);
        }

        let import_create = parse_from(["driggsby", "import", "create", "rows.csv"]);
        assert!(import_create.is_ok());
        if let Ok(cli) = import_create {
            assert_eq!(mode_for_command(&cli.command), OutputMode::Text);
        }

        let keys_uniq = parse_from(["driggsby", "import", "keys", "uniq"]);
        assert!(keys_uniq.is_ok());
        if let Ok(cli) = keys_uniq {
            assert_eq!(mode_for_command(&cli.command), OutputMode::Text);
        }
    }
}
