"""Database helpers for local db initialization."""

from pathlib import Path
import sqlite3

APP_DIR_NAME = ".driggsby"
DB_FILE_NAME = "ledger.db"


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
