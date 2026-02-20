"""CLI entrypoint for driggsby."""

from datetime import date
import json
from pathlib import Path

import click

from driggsby.db import connect_database, initialize_database
from driggsby.import_contract import DryRunResult, ValidationIssue, build_dry_run_result
from driggsby.import_json import read_json_input
from driggsby.models import TransactionFilters
from driggsby.migrations import apply_pending_migrations
from driggsby.schema import build_schema_payload


@click.group()
def main() -> None:
    """Driggsby command line interface."""


def _validate_yyyy_mm_dd(
    _context: click.Context, _parameter: click.Parameter, value: str | None
) -> str | None:
    if value is None:
        return None

    try:
        date.fromisoformat(value)
    except ValueError as exc:
        raise click.BadParameter("Expected date format YYYY-MM-DD.") from exc

    return value


@main.command()
def init() -> None:
    """Create the local driggsby sqlite file if needed."""
    result = initialize_database()
    click.echo(
        f"Driggsby initialized at {result.path}. "
        f"Applied {len(result.applied_versions)} migration(s). "
        f"Schema version: {result.current_version}"
    )


@main.command()
def schema() -> None:
    """Print canonical schema JSON from live sqlite metadata."""
    connection, _, _ = connect_database()
    try:
        apply_pending_migrations(connection)
        payload = build_schema_payload(connection)
    finally:
        connection.close()
    click.echo(json.dumps(payload, sort_keys=True))


def _resolve_import_file(file: str | None) -> Path | None:
    if file is None or file == "-":
        return None

    path = Path(file)
    if not path.exists():
        raise click.ClickException(f"File does not exist: {path}")
    if path.is_dir():
        raise click.ClickException(f"Expected a file path but got directory: {path}")
    return path


def _invalid_json_dry_run_result(message: str) -> DryRunResult:
    return DryRunResult(
        valid=False,
        normalized_source_provider=None,
        source_account_ref=None,
        transaction_count=0,
        errors=(ValidationIssue(path="$", message=message),),
        fingerprints=(),
    )


@main.command(name="import")
@click.option(
    "--format",
    "import_format",
    default="json",
    show_default=True,
    type=click.Choice(["json"], case_sensitive=False),
)
@click.option("--dry-run", is_flag=True, help="Validate input and print preview JSON.")
@click.argument(
    "file",
    required=False,
    type=str,
)
def import_command(import_format: str, dry_run: bool, file: str | None) -> None:
    """Import JSON from a file or stdin (placeholder only)."""
    if import_format.lower() != "json":
        raise click.ClickException("Only JSON is supported in this phase.")

    file_path = _resolve_import_file(file)
    stdin_text = click.get_text_stream("stdin").read() if file_path is None else None
    try:
        import_input = read_json_input(file_path=file_path, stdin_text=stdin_text)
    except json.JSONDecodeError as exc:
        if dry_run:
            summary = _invalid_json_dry_run_result(f"Invalid JSON input: {exc.msg}")
            click.echo(json.dumps(summary.to_dict(), sort_keys=True))
            raise click.exceptions.Exit(1) from exc
        raise click.ClickException(f"Invalid JSON input: {exc.msg}") from exc

    if dry_run:
        summary = build_dry_run_result(import_input.payload)
        click.echo(json.dumps(summary.to_dict(), sort_keys=True))
        if not summary.valid:
            raise click.exceptions.Exit(1)
        return

    click.echo(
        f"Placeholder import complete (toy). source={import_input.source} "
        f"bytes={import_input.bytes_read}"
    )


@main.command()
def accounts() -> None:
    """List accounts (placeholder only)."""
    click.echo("No data yet. Accounts command is a placeholder.")


@main.command()
@click.option("--account", type=str, default=None)
@click.option("--category", type=str, default=None)
@click.option("--start", type=str, callback=_validate_yyyy_mm_dd, default=None)
@click.option("--end", type=str, callback=_validate_yyyy_mm_dd, default=None)
def transactions(
    account: str | None, category: str | None, start: str | None, end: str | None
) -> None:
    """List transactions (placeholder only)."""
    filters = TransactionFilters(
        account=account,
        category=category,
        start=start,
        end=end,
    )
    click.echo(f"No data yet. Transactions command is a placeholder. filters={filters}")
