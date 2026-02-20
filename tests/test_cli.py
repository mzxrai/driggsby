import json
from pathlib import Path

from click.testing import CliRunner, Result

from driggsby.cli import main


def run_cli(args: list[str], input_text: str | None = None) -> Result:
    runner = CliRunner()
    return runner.invoke(main, args, input=input_text)


def test_help_lists_commands() -> None:
    result = run_cli(["--help"])
    assert result.exit_code == 0
    assert "init" in result.output
    assert "schema" in result.output
    assert "import" in result.output
    assert "accounts" in result.output
    assert "transactions" in result.output


def test_init_creates_db(fake_home: Path) -> None:
    db_path = fake_home / ".driggsby" / "ledger.db"

    result = run_cli(["init"])

    assert result.exit_code == 0
    assert db_path.exists()
    assert db_path.is_file()
    assert "applied 1 migration" in result.output.lower()


def test_init_is_idempotent(fake_home: Path) -> None:
    first = run_cli(["init"])
    second = run_cli(["init"])

    assert first.exit_code == 0
    assert second.exit_code == 0
    assert "applied 0 migration" in second.output.lower()


def test_schema_returns_canonical_json(fake_home: Path) -> None:
    result = run_cli(["schema"])

    assert result.exit_code == 0
    payload = json.loads(result.output)
    assert payload["schema_version"] == "001_core_ledger"
    tables = payload["tables"]
    table_names = {table["name"] for table in tables}
    assert {"accounts", "imports", "transactions", "schema_migrations"} <= table_names


def test_import_accepts_json_file(tmp_path: Path) -> None:
    statement_path = tmp_path / "statement.json"
    statement_path.write_text("{}", encoding="utf-8")

    result = run_cli(["import", "--format", "json", str(statement_path)])

    assert result.exit_code == 0
    assert "placeholder" in result.output.lower()


def test_import_accepts_json_stdin() -> None:
    result = run_cli(["import", "--format", "json"], input_text="{}")

    assert result.exit_code == 0
    assert "placeholder" in result.output.lower()


def test_import_accepts_json_stdin_with_dash_file_argument() -> None:
    result = run_cli(["import", "--format", "json", "-"], input_text="{}")

    assert result.exit_code == 0
    assert "placeholder" in result.output.lower()


def test_import_dry_run_successful_payload() -> None:
    payload = {
        "source_provider": "Apple Card",
        "source_account_ref": "apple-card-1234",
        "transactions": [
            {
                "posted_date": "2026-01-10",
                "description": "Coffee Shop",
                "amount_cents": -650,
            }
        ],
    }
    result = run_cli(
        ["import", "--format", "json", "--dry-run"],
        input_text=json.dumps(payload),
    )

    assert result.exit_code == 0
    summary = json.loads(result.output)
    assert summary["valid"] is True
    assert summary["normalized_source_provider"] == "apple_card"
    assert summary["source_account_ref"] == "apple-card-1234"
    assert summary["transaction_count"] == 1
    assert summary["errors"] == []
    assert len(summary["fingerprints"]) == 1


def test_import_dry_run_invalid_payload() -> None:
    payload = {
        "source_provider": "Apple Card",
        "transactions": [
            {
                "posted_date": "2026/01/10",
                "description": " ",
                "amount_cents": -650,
                "currency": "US",
            }
        ],
    }
    result = run_cli(
        ["import", "--format", "json", "--dry-run"],
        input_text=json.dumps(payload),
    )

    assert result.exit_code == 1
    summary = json.loads(result.output)
    assert summary["valid"] is False
    assert summary["normalized_source_provider"] is None
    assert summary["source_account_ref"] is None
    assert summary["transaction_count"] == 0
    assert summary["errors"]

    error_paths = {error["path"] for error in summary["errors"]}
    assert "source_account_ref" in error_paths
    assert "transactions[0].posted_date" in error_paths
    assert "transactions[0].description" in error_paths
    assert "transactions[0].currency" in error_paths


def test_import_dry_run_invalid_json() -> None:
    result = run_cli(["import", "--format", "json", "--dry-run"], input_text="{")

    assert result.exit_code == 1
    summary = json.loads(result.output)
    assert summary["valid"] is False
    assert summary["errors"]
    assert summary["errors"][0]["path"] == "$"
    assert "invalid json input" in summary["errors"][0]["message"].lower()


def test_import_rejects_missing_file(tmp_path: Path) -> None:
    missing_path = tmp_path / "missing.json"

    result = run_cli(["import", "--format", "json", str(missing_path)])

    assert result.exit_code != 0
    assert "does not exist" in result.output.lower()


def test_accounts_returns_no_data_message() -> None:
    result = run_cli(["accounts"])

    assert result.exit_code == 0
    assert "no data yet" in result.output.lower()


def test_transactions_with_valid_dates() -> None:
    result = run_cli(["transactions", "--start", "2026-01-01", "--end", "2026-01-31"])

    assert result.exit_code == 0
    assert "no data yet" in result.output.lower()


def test_transactions_rejects_invalid_start_format() -> None:
    result = run_cli(["transactions", "--start", "01/01/2026"])

    assert result.exit_code != 0
    assert "yyyy-mm-dd" in result.output.lower()
