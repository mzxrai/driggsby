"""CLI entrypoint for driggsby."""

from datetime import date
import json
from pathlib import Path

import click

from driggsby.db import ensure_db_file
from driggsby.import_json import read_json_input
from driggsby.models import TransactionFilters
from driggsby.schema import build_schema_placeholder


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
    db_path, created = ensure_db_file()
    if created:
        click.echo(f"Initialized toy Driggsby database at {db_path}")
        return

    click.echo(f"Driggsby is already initialized at {db_path}")


@main.command()
def schema() -> None:
    """Print a placeholder schema JSON document."""
    payload = build_schema_placeholder().model_dump()
    click.echo(json.dumps(payload))


@main.command(name="import")
@click.option(
    "--format",
    "import_format",
    default="json",
    show_default=True,
    type=click.Choice(["json"], case_sensitive=False),
)
@click.argument(
    "file",
    required=False,
    type=click.Path(exists=True, dir_okay=False, readable=True, path_type=Path),
)
def import_command(import_format: str, file: Path | None) -> None:
    """Import JSON from a file or stdin (placeholder only)."""
    if import_format.lower() != "json":
        raise click.ClickException("Only JSON is supported in this phase.")

    stdin_text = click.get_text_stream("stdin").read() if file is None else None
    try:
        import_input = read_json_input(file_path=file, stdin_text=stdin_text)
    except json.JSONDecodeError as exc:
        raise click.ClickException(f"Invalid JSON input: {exc.msg}") from exc

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
    click.echo(
        "No data yet. Transactions command is a placeholder. "
        f"filters={filters}"
    )
