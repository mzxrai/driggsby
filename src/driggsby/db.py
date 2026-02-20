"""Database helpers for local db initialization."""

from dataclasses import dataclass
from pathlib import Path
import sqlite3

from driggsby.migrations import apply_pending_migrations, get_current_schema_version

APP_DIR_NAME = ".driggsby"
DB_FILE_NAME = "ledger.db"


@dataclass(frozen=True, slots=True)
class InitializationResult:
    path: Path
    created: bool
    applied_versions: tuple[str, ...]
    current_version: str | None


def get_default_db_path(home: Path | None = None) -> Path:
    root = home if home is not None else Path.home()
    return root / APP_DIR_NAME / DB_FILE_NAME


def ensure_db_file(db_path: Path | None = None) -> tuple[Path, bool]:
    path = db_path if db_path is not None else get_default_db_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    already_exists = path.exists()

    with sqlite3.connect(path):
        pass

    return path, (not already_exists)


def connect_database(db_path: Path | None = None) -> tuple[sqlite3.Connection, Path, bool]:
    path, created = ensure_db_file(db_path)
    connection = sqlite3.connect(path)
    connection.execute("PRAGMA foreign_keys = ON;")
    return connection, path, created


def initialize_database(db_path: Path | None = None) -> InitializationResult:
    connection, path, created = connect_database(db_path)
    try:
        applied_versions = tuple(apply_pending_migrations(connection))
        current_version = get_current_schema_version(connection)
    finally:
        connection.close()

    return InitializationResult(
        path=path,
        created=created,
        applied_versions=applied_versions,
        current_version=current_version,
    )
