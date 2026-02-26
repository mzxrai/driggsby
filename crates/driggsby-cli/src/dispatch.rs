use driggsby_client::commands;
use driggsby_client::{ClientResult, SuccessEnvelope};

use crate::cli::{Cli, Commands, DemoCommand, ImportCommand, ImportKeysCommand, SchemaCommand};

pub fn dispatch(cli: &Cli) -> ClientResult<SuccessEnvelope> {
    match &cli.command {
        Commands::Schema { command } => match command {
            Some(SchemaCommand::View { view_name }) => commands::schema::view(view_name),
            None => commands::schema::summary(),
        },
        Commands::Import { command } => match command {
            ImportCommand::Create {
                dry_run,
                json: _,
                path,
            } => commands::import::run(path.clone(), *dry_run),
            ImportCommand::List { .. } => commands::import::list(),
            ImportCommand::Duplicates { import_id, .. } => commands::import::duplicates(import_id),
            ImportCommand::Keys { command } => match command {
                ImportKeysCommand::Uniq { property, .. } => {
                    commands::import::keys_uniq(property.clone())
                }
            },
            ImportCommand::Undo { import_id, .. } => commands::import::undo(import_id),
        },
        Commands::Demo { command } => {
            let topic = demo_command_to_str(command);
            commands::demo::run(topic)
        }
        Commands::Anomalies { from, to, .. } => {
            let from_value = from.as_ref().map(|value| value.as_str());
            let to_value = to.as_ref().map(|value| value.as_str());
            commands::anomalies::run(from_value, to_value)
        }
        Commands::Recurring { from, to, .. } => {
            let from_value = from.as_ref().map(|value| value.as_str());
            let to_value = to.as_ref().map(|value| value.as_str());
            commands::recurring::run(from_value, to_value)
        }
        Commands::Dash => commands::dash::run(),
    }
}

fn demo_command_to_str(command: &DemoCommand) -> &'static str {
    match command {
        DemoCommand::Dash => "dash",
        DemoCommand::Recurring => "recurring",
        DemoCommand::Anomalies => "anomalies",
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::parse_from;

    use super::dispatch;

    #[test]
    fn dispatches_to_expected_command_names() {
        let cases: [(&[&str], &str); 2] = [
            (&["driggsby", "demo", "dash"], "demo"),
            (&["driggsby", "schema"], "schema"),
        ];

        for (args, expected_command) in cases {
            let parsed = parse_from(args);
            assert!(parsed.is_ok());
            if let Ok(cli) = parsed {
                let response = dispatch(&cli);
                assert!(response.is_ok());
                if let Ok(success) = response {
                    assert_eq!(success.command, expected_command);
                }
            }
        }
    }

    #[test]
    fn import_list_dispatches_successfully() {
        let parsed = parse_from(["driggsby", "import", "list"]);
        assert!(parsed.is_ok());
    }

    #[test]
    fn import_duplicates_dispatches_successfully() {
        let parsed = parse_from(["driggsby", "import", "duplicates", "imp_1"]);
        assert!(parsed.is_ok());
    }

    #[test]
    fn guide_command_is_not_dispatchable() {
        let parsed = parse_from(["driggsby", "guide"]);
        assert!(parsed.is_err());
    }
}
