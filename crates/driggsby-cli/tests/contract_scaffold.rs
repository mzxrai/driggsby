use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const EXPECTED_TOP_LEVEL_HELP: &str = "Driggsby — personal finance intelligence layer

USAGE: driggsby <command>

Try it:
  driggsby demo dash                            Open sample dashboard with bundled data
  driggsby demo recurring                       Preview sample recurring patterns
  driggsby demo anomalies                       Preview sample anomaly detection

Import your transactions:
  1. driggsby import create --help              Read import schema and workflow details
  2. driggsby import create --dry-run <path>    Safely validate import without data writes
  3. driggsby import create <path>              Import transactions

View Driggsby analysis (refreshed on each new import):
  driggsby recurring                            Detect recurring transactions
  driggsby anomalies                            Detect spending anomalies
  driggsby dash                                 Open web dashboard (prints URL, attempts browser open)

Need to do custom analysis? Run SQL against our views:
  1. driggsby schema                            Get DB path and view names
  2. Query `v1_*` views with sqlite3 or any SQL client

Other commands:
  driggsby import keys uniq                     List canonical import identifiers
  driggsby import duplicates <id>               Inspect duplicate rows from one import
  driggsby import list                          List past imports
  driggsby import undo <id>                     Undo an import
  driggsby schema view <name>                   Inspect one view's columns

Want to ensure a clean first run, or having issues/errors?
  Run `driggsby import create --help` for import workflow guidance,
  or `driggsby <command> --help` for command usage.
";

const EXPECTED_ROOT_HELP: &str = "Driggsby - personal finance intelligence layer

Usage:
  driggsby <command>

Start here:
  driggsby import create --help
  driggsby schema
";

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn unique_test_home() -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_nanos(),
        Err(_) => 0,
    };
    let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.push(format!(
        "driggsby-cli-test-{}-{stamp}-{sequence}",
        std::process::id()
    ));
    path
}

fn run_cli_in_home_with_input(
    home: &std::path::Path,
    args: &[&str],
    input: Option<&str>,
) -> (bool, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_driggsby"));
    for arg in args {
        command.arg(arg);
    }
    command.env("DRIGGSBY_HOME", home);
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let child_spawn = command.spawn();
    assert!(child_spawn.is_ok());
    if let Ok(mut child) = child_spawn {
        if let Some(body) = input {
            let mut stdin = child.stdin.take();
            assert!(stdin.is_some());
            if let Some(mut pipe) = stdin.take() {
                let write_result = pipe.write_all(body.as_bytes());
                assert!(write_result.is_ok());
            }
        }

        let output = child.wait_with_output();
        assert!(output.is_ok());
        if let Ok(result) = output {
            let stdout = String::from_utf8(result.stdout);
            assert!(stdout.is_ok());
            if let Ok(stdout_text) = stdout {
                return (result.status.success(), stdout_text);
            }
        }
    }

    (false, String::new())
}

fn run_cli_with_input(args: &[&str], input: Option<&str>) -> (bool, String, std::path::PathBuf) {
    let home = unique_test_home();
    let (ok, body) = run_cli_in_home_with_input(&home, args, input);
    (ok, body, home)
}

fn run_cli(args: &[&str]) -> (bool, String, std::path::PathBuf) {
    run_cli_with_input(args, None)
}

fn write_source_file(home: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let create_home = fs::create_dir_all(home);
    assert!(create_home.is_ok());

    let source_path = home.join(name);
    let write = fs::write(&source_path, body);
    assert!(write.is_ok());
    source_path
}

fn parse_json(body: &str) -> Value {
    let parsed = serde_json::from_str::<Value>(body);
    assert!(parsed.is_ok());
    if let Ok(value) = parsed {
        return value;
    }
    Value::Null
}

fn assert_text_error_contract(body: &str, code: &str) {
    assert!(body.contains("Something went wrong, but it's easy to fix."));
    assert!(body.contains(&format!("  Error:    {code}")));
    assert!(body.contains("  Details:"));
    assert!(body.contains("What to do next:"));
}

#[test]
fn root_command_uses_short_plaintext_help() {
    let (ok, body, _) = run_cli(&[]);
    assert!(ok);
    assert_eq!(body, EXPECTED_ROOT_HELP);
}

#[test]
fn help_and_version_return_success_output() {
    let (help_ok, help_body, _) = run_cli(&["--help"]);
    assert!(help_ok);
    assert_eq!(help_body, EXPECTED_TOP_LEVEL_HELP);

    let (version_ok, version_body, _) = run_cli(&["--version"]);
    assert!(version_ok);
    assert_eq!(version_body.trim(), "driggsby 0.1.0");
}

#[test]
fn import_help_shows_subcommand_descriptions() {
    let (ok, body, _) = run_cli(&["import", "--help"]);
    assert!(ok);
    assert!(body.contains("create"));
    assert!(body.contains("list"));
    assert!(body.contains("keys"));
    assert!(body.contains("duplicates"));
    assert!(body.contains("undo"));
    assert!(body.contains("Import normalized transaction data"));
    assert!(body.contains("List all past imports"));
    assert!(body.contains("List canonical unique values"));
    assert!(body.contains("deduped against"));
    assert!(body.contains("Revert a previously committed import"));
}

#[test]
fn import_create_help_shows_workflow_and_schema() {
    let (ok, body, _) = run_cli(&["import", "create", "--help"]);
    assert!(ok);
    assert!(body.contains("How import works:"));
    assert!(body.contains("What to do next:"));
    assert!(body.contains("driggsby import keys uniq"));
    assert!(body.contains("Import schema:"));
    assert!(body.contains("account_key"));
    assert!(body.contains("posted_at"));
    assert!(body.contains("YYYY-MM-DD"));
    assert!(body.contains("negative = money out"));
    assert!(body.contains("positive = money in"));
    assert!(body.contains("Credit card"));
}

#[test]
fn bare_import_shows_help_with_subcommands() {
    let (ok, body, _) = run_cli(&["import"]);
    assert!(ok);
    assert!(body.contains("create"));
    assert!(body.contains("keys"));
    assert!(body.contains("list"));
    assert!(body.contains("undo"));
}

#[test]
fn schema_output_is_plaintext_and_data_access_focused() {
    let (ok, body, _) = run_cli(&["schema"]);
    assert!(ok);
    assert!(body.starts_with("Your ledger database is stored locally"));
    assert!(body.contains("Summary:"));
    assert!(body.contains("Database path:"));
    assert!(body.contains("Readonly URI:"));
    assert!(body.contains("Connect with sqlite3:"));
    assert!(body.contains("Public Views:"));
    assert!(body.contains("View: v1_transactions"));
    assert!(body.contains("View: v1_accounts"));
    assert!(body.contains("View: v1_imports"));
    assert!(body.contains("Inspect one view in detail:"));
    assert!(!body.contains("# "));
    assert!(!body.contains("| Column | Type | Nullable |"));
}

#[test]
fn schema_view_output_is_plaintext() {
    let (ok, body, _) = run_cli(&["schema", "view", "v1_transactions"]);
    assert!(ok);
    assert!(body.starts_with("View details for v1_transactions."));
    assert!(body.contains("Columns:"));
    assert!(body.contains("txn_id"));
    assert!(body.contains("not null"));
    assert!(!body.contains("\"ok\""));
}

#[test]
fn unknown_schema_view_uses_plaintext_error_contract() {
    let (ok, body, _) = run_cli(&["schema", "view", "v1_missing"]);
    assert!(!ok);
    assert_text_error_contract(&body, "unknown_view");
}

#[test]
fn import_dry_run_default_is_plaintext_summary() {
    let home = unique_test_home();
    let source_path = write_source_file(
        &home,
        "import.csv",
        "statement_id,account_key,posted_at,amount,currency,description\nchase_checking_1234_2026-01-31,chase_checking_1234,2026-01-01,-5.00,USD,COFFEE\n",
    );
    let source_arg = source_path.display().to_string();
    let (ok, body) =
        run_cli_in_home_with_input(&home, &["import", "create", "--dry-run", &source_arg], None);
    assert!(ok);
    assert!(body.starts_with("Dry-run validation completed successfully."));
    assert!(body.contains("Summary:"));
    assert!(body.contains("Rows read:"));
    assert!(body.contains("Inserted:"));
    assert!(body.contains("Canonical existing values:"));
    assert!(body.contains("Per-account sign profile:"));
    assert!(body.contains("Drift warnings:"));
    assert!(body.contains("No rows were written because this was a dry run."));
    assert!(!body.contains("\"ok\""));
}

#[test]
fn import_plaintext_success_shows_import_and_undo_ids() {
    let home = unique_test_home();
    let source_path = write_source_file(
        &home,
        "import.json",
        r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-01","amount":-7.50,"currency":"USD","description":"COFFEE"}
]"#,
    );
    let source_arg = source_path.display().to_string();
    let (ok, body) = run_cli_in_home_with_input(&home, &["import", "create", &source_arg], None);
    assert!(ok);
    assert!(body.starts_with("Import completed successfully."));
    assert!(body.contains("Import ID:"));
    assert!(body.contains("Undo ID:"));
    assert!(body.contains("Summary:"));
    assert!(body.contains("Duplicate Summary:"));
    assert!(body.contains("Duplicates Preview"));
    assert!(!body.contains("Deduped:"));
    assert!(!body.contains("\"ok\""));
}

#[test]
fn import_json_success_uses_structured_envelope_without_command_field() {
    let home = unique_test_home();
    let source_path = write_source_file(
        &home,
        "import.json",
        r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-01","amount":-7.50,"currency":"USD","description":"COFFEE"}
]"#,
    );
    let source_arg = source_path.display().to_string();
    let (ok, body) =
        run_cli_in_home_with_input(&home, &["import", "create", &source_arg, "--json"], None);
    assert!(ok);
    let payload = parse_json(&body);
    assert_eq!(payload["ok"], Value::Bool(true));
    assert_eq!(payload["version"], Value::String("v1".to_string()));
    assert!(payload["data"]["import_id"].is_string());
    assert!(payload["data"]["undo_id"].is_string());
    assert!(payload["data"]["summary"].is_object());
    assert!(payload["data"]["issues"].is_array());
    assert!(payload["data"]["query_context"].is_object());
    assert!(payload["data"]["message"].is_string());
    assert!(payload.get("command").is_none());
}

#[test]
fn import_list_plaintext_and_json_contracts_are_both_supported() {
    let home = unique_test_home();
    let source_path = write_source_file(
        &home,
        "import-list.json",
        r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-03-01","amount":-11.00,"currency":"USD","description":"LIST-ROW"}
]"#,
    );
    let source_arg = source_path.display().to_string();
    let (import_ok, import_body) =
        run_cli_in_home_with_input(&home, &["import", "create", &source_arg, "--json"], None);
    assert!(import_ok);
    let import_payload = parse_json(&import_body);
    let import_id = import_payload["data"]["import_id"].as_str();
    assert!(import_id.is_some());

    if let Some(id) = import_id {
        let (undo_ok, _undo_body) =
            run_cli_in_home_with_input(&home, &["import", "undo", id], None);
        assert!(undo_ok);
    }

    let (list_ok, list_body) = run_cli_in_home_with_input(&home, &["import", "list"], None);
    assert!(list_ok);
    assert!(list_body.contains("Imports:"));
    assert!(list_body.contains("Created (local)"));
    assert!(list_body.contains("Import ID"));
    assert!(!list_body.contains("\"ok\""));

    let (json_ok, json_body) =
        run_cli_in_home_with_input(&home, &["import", "list", "--json"], None);
    assert!(json_ok);
    let json_payload = parse_json(&json_body);
    assert!(json_payload.is_array());
    if let Some(rows) = json_payload.as_array() {
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["status"], Value::String("reverted".to_string()));
        assert!(rows[0]["import_id"].is_string());
    }
}

#[test]
fn import_undo_plaintext_and_json_contracts_are_both_supported() {
    let home = unique_test_home();
    let source_path = write_source_file(
        &home,
        "undo.json",
        r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-03-02","amount":-88.00,"currency":"USD","description":"UNDO"}
]"#,
    );
    let source_arg = source_path.display().to_string();

    let (import_ok, import_body) =
        run_cli_in_home_with_input(&home, &["import", "create", &source_arg, "--json"], None);
    assert!(import_ok);
    let import_payload = parse_json(&import_body);
    let import_id = import_payload["data"]["import_id"].as_str();
    assert!(import_id.is_some());

    if let Some(id) = import_id {
        let (undo_ok, undo_body) = run_cli_in_home_with_input(&home, &["import", "undo", id], None);
        assert!(undo_ok);
        assert!(undo_body.starts_with("Import reverted successfully."));
        assert!(undo_body.contains("Rows reverted:"));
        assert!(undo_body.contains("Rows promoted:"));

        let source_path_2 = write_source_file(
            &home,
            "undo2.json",
            r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-03-03","amount":-14.00,"currency":"USD","description":"UNDO2"}
]"#,
        );
        let source_arg_2 = source_path_2.display().to_string();
        let (second_import_ok, second_import_body) =
            run_cli_in_home_with_input(&home, &["import", "create", &source_arg_2, "--json"], None);
        assert!(second_import_ok);
        let second_import_payload = parse_json(&second_import_body);
        let second_import_id = second_import_payload["data"]["import_id"].as_str();
        assert!(second_import_id.is_some());
        if let Some(second_id) = second_import_id {
            let (undo_json_ok, undo_json_body) =
                run_cli_in_home_with_input(&home, &["import", "undo", second_id, "--json"], None);
            assert!(undo_json_ok);
            let undo_payload = parse_json(&undo_json_body);
            assert_eq!(undo_payload["ok"], Value::Bool(true));
            assert_eq!(undo_payload["version"], Value::String("v1".to_string()));
            assert_eq!(
                undo_payload["data"]["import_id"],
                Value::String(second_id.to_string())
            );
            assert_eq!(
                undo_payload["data"]["summary"]["rows_reverted"],
                Value::from(1)
            );
            assert!(undo_payload["data"]["message"].is_string());
            assert!(undo_payload.get("command").is_none());
        }
    }
}

#[test]
fn import_undo_json_runtime_error_uses_universal_error_shape() {
    let home = unique_test_home();
    let (ok, body) =
        run_cli_in_home_with_input(&home, &["import", "undo", "imp_missing", "--json"], None);
    assert!(!ok);
    let payload = parse_json(&body);
    assert!(payload["error"].is_object());
    assert_eq!(
        payload["error"]["code"],
        Value::String("import_id_not_found".to_string())
    );
    assert!(payload["error"]["message"].is_string());
    assert!(payload["error"]["recovery_steps"].is_array());
    assert_eq!(
        payload["error"]["data"]["import_id"],
        Value::String("imp_missing".to_string())
    );
    assert_eq!(
        payload["error"]["data"]["help_command"],
        Value::String("driggsby import create --help".to_string())
    );
    assert_eq!(
        payload["error"]["data"]["help_section_title"],
        Value::String("Import Troubleshooting".to_string())
    );
    assert!(payload.get("ok").is_none());
    assert!(payload.get("data").is_none());
}

#[test]
fn import_create_json_validation_error_uses_nested_error_data() {
    let home = unique_test_home();
    let source_path = write_source_file(
        &home,
        "missing-statement-id.json",
        r#"[
  {"account_key":"chase_checking_1234","posted_at":"2026-03-02","amount":-88.00,"currency":"USD","description":"UNDO"}
]"#,
    );
    let source_arg = source_path.display().to_string();
    let (ok, body) = run_cli_in_home_with_input(
        &home,
        &["import", "create", "--dry-run", &source_arg, "--json"],
        None,
    );
    assert!(!ok);
    let payload = parse_json(&body);
    assert_eq!(
        payload["error"]["code"],
        Value::String("import_validation_failed".to_string())
    );
    assert!(payload["error"]["data"]["summary"].is_object());
    assert!(payload["error"]["data"]["issues"].is_array());
    assert_eq!(
        payload["error"]["data"]["issues"][0]["field"],
        Value::String("statement_id".to_string())
    );
    assert_eq!(
        payload["error"]["data"]["issues"][0]["code"],
        Value::String("missing_required_field".to_string())
    );
    assert!(payload.get("data").is_none());
}

#[test]
fn import_create_json_schema_mismatch_error_uses_nested_error_data() {
    let home = unique_test_home();
    let source_path = write_source_file(
        &home,
        "schema-mismatch.csv",
        "account,posted_date,amount_usd,description\nx,2026-01-01,-1.00,Test\n",
    );
    let source_arg = source_path.display().to_string();
    let (ok, body) = run_cli_in_home_with_input(
        &home,
        &["import", "create", "--dry-run", &source_arg, "--json"],
        None,
    );
    assert!(!ok);
    let payload = parse_json(&body);
    assert_eq!(
        payload["error"]["code"],
        Value::String("import_schema_mismatch".to_string())
    );
    assert!(payload["error"]["data"]["expected_headers"].is_array());
    assert!(payload["error"]["data"]["actual_headers"].is_array());
    assert!(payload.get("data").is_none());
}

#[test]
fn import_duplicates_plaintext_and_json_contracts_are_supported() {
    let home = unique_test_home();
    let source_path = write_source_file(
        &home,
        "dupes.json",
        r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-01","amount":-5.00,"currency":"USD","description":"COFFEE"},
  {"statement_id":"chase_checking_1234_2026-02-28","account_key":"chase_checking_1234","posted_at":"2026-01-01","amount":-5.00,"currency":"USD","description":"COFFEE"}
]"#,
    );
    let source_arg = source_path.display().to_string();
    let (import_ok, import_body) =
        run_cli_in_home_with_input(&home, &["import", "create", &source_arg, "--json"], None);
    assert!(import_ok);
    let import_payload = parse_json(&import_body);
    let import_id = import_payload["data"]["import_id"].as_str();
    assert!(import_id.is_some());

    if let Some(id) = import_id {
        let (text_ok, text_body) =
            run_cli_in_home_with_input(&home, &["import", "duplicates", id], None);
        assert!(text_ok);
        assert!(text_body.starts_with("Duplicate rows for import"));
        assert!(text_body.contains("Total duplicates:"));

        let (json_ok, json_body) =
            run_cli_in_home_with_input(&home, &["import", "duplicates", id, "--json"], None);
        assert!(json_ok);
        let payload = parse_json(&json_body);
        assert_eq!(payload["ok"], Value::Bool(true));
        assert_eq!(payload["version"], Value::String("v1".to_string()));
        assert_eq!(payload["data"]["import_id"], Value::String(id.to_string()));
        assert!(payload["data"]["rows"].is_array());
    }
}

#[test]
fn import_dry_run_with_json_returns_json_output() {
    let home = unique_test_home();
    let source_path = write_source_file(
        &home,
        "import.csv",
        "statement_id,account_key,posted_at,amount,currency,description\nchase_checking_1234_2026-01-31,chase_checking_1234,2026-01-01,-5.00,USD,COFFEE\n",
    );
    let source_arg = source_path.display().to_string();
    let (ok, body) = run_cli_in_home_with_input(
        &home,
        &["import", "create", "--dry-run", &source_arg, "--json"],
        None,
    );
    assert!(ok);
    let payload = parse_json(&body);
    assert_eq!(payload["ok"], Value::Bool(true));
    assert_eq!(payload["data"]["dry_run"], Value::Bool(true));
    assert!(payload["data"]["key_inventory"].is_object());
    assert!(payload["data"]["sign_profiles"].is_array());
    assert!(payload["data"]["drift_warnings"].is_array());
}

#[test]
fn import_keys_uniq_plaintext_and_json_contracts_are_supported() {
    let home = unique_test_home();
    let source_path = write_source_file(
        &home,
        "keys.json",
        r#"[
  {"statement_id":"chase_checking_1234_2026-01-31","account_key":"chase_checking_1234","posted_at":"2026-01-01","amount":-5.00,"currency":"USD","description":"COFFEE","merchant":"Blue Bottle","category":"Coffee"},
  {"statement_id":"amex_gold_9999_2026-01-31","account_key":"amex_gold_9999","posted_at":"2026-01-02","amount":-15.00,"currency":"USD","description":"LUNCH","merchant":"Sweetgreen","category":"Dining"}
]"#,
    );
    let source_arg = source_path.display().to_string();
    let (import_ok, _import_body) =
        run_cli_in_home_with_input(&home, &["import", "create", &source_arg], None);
    assert!(import_ok);

    let (text_ok, text_body) = run_cli_in_home_with_input(&home, &["import", "keys", "uniq"], None);
    assert!(text_ok);
    assert!(text_body.starts_with("Canonical unique values"));
    assert!(text_body.contains("account_key"));
    assert!(text_body.contains("currency"));
    assert!(text_body.contains("merchant"));
    assert!(text_body.contains("category"));

    let (json_ok, json_body) = run_cli_in_home_with_input(
        &home,
        &["import", "keys", "uniq", "merchant", "--json"],
        None,
    );
    assert!(json_ok);
    let payload = parse_json(&json_body);
    assert_eq!(payload["ok"], Value::Bool(true));
    assert_eq!(
        payload["data"]["property"],
        Value::String("merchant".to_string())
    );
    assert!(payload["data"]["inventories"].is_array());
    assert!(payload["data"]["inventories"][0]["existing_values"].is_array());
    assert!(payload["data"]["inventories"][0]["value_counts"].is_array());
}

#[test]
fn import_keys_uniq_invalid_property_is_plaintext_error_even_with_json_flag() {
    let (ok, body, _) = run_cli(&["import", "keys", "uniq", "acct", "--json"]);
    assert!(!ok);
    assert_text_error_contract(&body, "invalid_argument");
    assert!(!body.trim_start().starts_with('{'));
}

#[test]
fn unsupported_json_flag_on_plaintext_only_command_is_rejected() {
    let (ok, body, _) = run_cli(&["schema", "--json"]);
    assert!(!ok);
    assert_text_error_contract(&body, "invalid_argument");
}

#[test]
fn anomalies_and_recurring_default_to_plaintext() {
    let (anomalies_ok, anomalies_body, _) = run_cli(&["anomalies"]);
    assert!(anomalies_ok);
    assert!(anomalies_body.starts_with("No anomalies found."));
    assert!(!anomalies_body.contains("\"rows\""));

    let (recurring_ok, recurring_body, _) = run_cli(&["recurring"]);
    assert!(recurring_ok);
    assert!(recurring_body.starts_with("No recurring patterns found."));
    assert!(!recurring_body.contains("\"rows\""));
}

#[test]
fn anomalies_and_recurring_json_use_structured_objects() {
    let (anomalies_ok, anomalies_body, _) = run_cli(&["anomalies", "--json"]);
    assert!(anomalies_ok);
    let anomalies_payload = parse_json(&anomalies_body);
    assert!(anomalies_payload["rows"].is_array());
    assert!(anomalies_payload["data_covers"].is_object());
    assert!(anomalies_payload["data_covers"]["from"].is_null());
    assert!(anomalies_payload["data_covers"]["to"].is_null());

    let (recurring_ok, recurring_body, _) = run_cli(&["recurring", "--json"]);
    assert!(recurring_ok);
    let recurring_payload = parse_json(&recurring_body);
    assert!(recurring_payload["rows"].is_array());
    assert!(recurring_payload["from"].is_null());
    assert!(recurring_payload["to"].is_null());
}

#[test]
fn parse_and_argument_errors_are_always_plaintext_even_when_json_is_present() {
    let (parse_ok, parse_body, _) = run_cli(&["anomalies", "--json", "--from", "2026-99-01"]);
    assert!(!parse_ok);
    assert_text_error_contract(&parse_body, "invalid_argument");
    assert!(!parse_body.trim_start().starts_with('{'));

    let (arg_ok, arg_body, _) = run_cli(&["import", "create", "--json"]);
    assert!(!arg_ok);
    assert_text_error_contract(&arg_body, "invalid_argument");
    assert!(!arg_body.trim_start().starts_with('{'));

    let (keys_ok, keys_body, _) = run_cli(&["import", "keys", "uniq", "acct", "--json"]);
    assert!(!keys_ok);
    assert_text_error_contract(&keys_body, "invalid_argument");
    assert!(keys_body.contains("driggsby import keys uniq --help"));
    assert!(!keys_body.trim_start().starts_with('{'));
}

#[test]
fn demo_and_dash_outputs_follow_plaintext_contract() {
    let (dash_ok, dash_body, _) = run_cli(&["dash"]);
    assert!(dash_ok);
    assert!(dash_body.starts_with("Opening your dashboard at http://127.0.0.1:8787"));
    assert!(dash_body.contains("If the browser did not open automatically"));

    let (demo_ok, demo_body, _) = run_cli(&["demo", "dash"]);
    assert!(demo_ok);
    assert!(demo_body.starts_with("Opening demo at http://127.0.0.1:8787/demo/dash"));
    assert!(demo_body.contains("This demo uses bundled sample data"));

    let (recurring_ok, recurring_body, _) = run_cli(&["demo", "recurring"]);
    assert!(recurring_ok);
    assert!(recurring_body.starts_with("Demo: recurring transaction patterns"));

    let (anomalies_ok, anomalies_body, _) = run_cli(&["demo", "anomalies"]);
    assert!(anomalies_ok);
    assert!(anomalies_body.starts_with("Demo: spending anomaly detection"));
}

#[test]
fn help_and_guide_commands_are_rejected_with_plaintext_invalid_argument() {
    let (help_ok, help_body, _) = run_cli(&["help"]);
    assert!(!help_ok);
    assert_text_error_contract(&help_body, "invalid_argument");

    let (guide_ok, guide_body, _) = run_cli(&["guide"]);
    assert!(!guide_ok);
    assert_text_error_contract(&guide_body, "invalid_argument");
}
