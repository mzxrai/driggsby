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


def test_init_is_idempotent(fake_home: Path) -> None:
    first = run_cli(["init"])
    second = run_cli(["init"])

    assert first.exit_code == 0
    assert second.exit_code == 0
    assert "already initialized" in second.output.lower()


def test_schema_returns_placeholder_json() -> None:
    result = run_cli(["schema"])

    assert result.exit_code == 0
    payload = json.loads(result.output)
    assert payload["toy"] is True
    assert payload["version"] == "0.1.0-dev"
    assert isinstance(payload["message"], str)
    assert payload["entities"] == []


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
