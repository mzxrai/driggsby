"""SQLite schema migration helpers."""

from dataclasses import dataclass
from datetime import datetime, timezone
import sqlite3


@dataclass(frozen=True, slots=True)
class Migration:
    version: str
    description: str
    statements: tuple[str, ...]


MIGRATIONS: tuple[Migration, ...] = (
    Migration(
        version="001_core_ledger",
        description="Create core ledger tables and indexes.",
        statements=(
            """
            CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                external_ref TEXT UNIQUE,
                name TEXT NOT NULL,
                institution TEXT,
                account_type TEXT NOT NULL
                    CHECK (account_type IN (
                        'bank',
                        'credit_card',
                        'brokerage',
                        'loan',
                        'retirement',
                        'other'
                    )),
                account_subtype TEXT,
                is_liability INTEGER NOT NULL DEFAULT 0
                    CHECK (is_liability IN (0, 1)),
                currency TEXT NOT NULL DEFAULT 'USD'
                    CHECK (length(currency) = 3),
                created_at TEXT NOT NULL
                    DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at TEXT NOT NULL
                    DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            );
            """,
            """
            CREATE TABLE IF NOT EXISTS imports (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id INTEGER
                    REFERENCES accounts(id) ON DELETE SET NULL,
                source_name TEXT NOT NULL,
                source_type TEXT NOT NULL
                    CHECK (source_type IN ('pdf', 'csv', 'json', 'api')),
                source_provider TEXT NOT NULL DEFAULT 'other',
                source_account_ref TEXT NOT NULL DEFAULT '',
                statement_hash TEXT NOT NULL,
                parser_name TEXT NOT NULL,
                parser_version TEXT NOT NULL,
                period_start TEXT,
                period_end TEXT,
                transaction_count INTEGER NOT NULL DEFAULT 0,
                imported_at TEXT NOT NULL
                    DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                metadata_json TEXT
                    CHECK (
                        metadata_json IS NULL OR json_valid(metadata_json)
                    ),
                UNIQUE(source_type, statement_hash)
            );
            """,
            """
            CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id INTEGER NOT NULL
                    REFERENCES accounts(id) ON DELETE CASCADE,
                import_id INTEGER
                    REFERENCES imports(id) ON DELETE SET NULL,
                external_id TEXT,
                dedupe_fingerprint TEXT NOT NULL,
                posted_date TEXT NOT NULL,
                settled_date TEXT,
                description TEXT NOT NULL,
                merchant TEXT,
                normalized_merchant TEXT,
                category TEXT,
                transaction_type TEXT,
                amount_cents INTEGER NOT NULL,
                currency TEXT NOT NULL DEFAULT 'USD'
                    CHECK (length(currency) = 3),
                status TEXT NOT NULL DEFAULT 'posted'
                    CHECK (status IN ('pending', 'posted', 'cleared')),
                owner_name TEXT,
                metadata_json TEXT
                    CHECK (
                        metadata_json IS NULL OR json_valid(metadata_json)
                    ),
                created_at TEXT NOT NULL
                    DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at TEXT NOT NULL
                    DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                UNIQUE(account_id, dedupe_fingerprint)
            );
            """,
            """
            CREATE INDEX IF NOT EXISTS idx_accounts_type
                ON accounts(account_type);
            """,
            """
            CREATE INDEX IF NOT EXISTS idx_accounts_institution
                ON accounts(institution);
            """,
            """
            CREATE INDEX IF NOT EXISTS idx_imports_account_id
                ON imports(account_id);
            """,
            """
            CREATE INDEX IF NOT EXISTS idx_imports_imported_at
                ON imports(imported_at);
            """,
            """
            CREATE INDEX IF NOT EXISTS idx_imports_provider_ref
                ON imports(source_provider, source_account_ref);
            """,
            """
            CREATE INDEX IF NOT EXISTS idx_transactions_account_posted
                ON transactions(account_id, posted_date DESC);
            """,
            """
            CREATE INDEX IF NOT EXISTS idx_transactions_import_id
                ON transactions(import_id);
            """,
            """
            CREATE INDEX IF NOT EXISTS idx_transactions_category_posted
                ON transactions(category, posted_date DESC);
            """,
            """
            CREATE TABLE IF NOT EXISTS source_account_links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_provider TEXT NOT NULL,
                source_account_ref TEXT NOT NULL,
                account_id INTEGER NOT NULL
                    REFERENCES accounts(id) ON DELETE CASCADE,
                created_at TEXT NOT NULL
                    DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at TEXT NOT NULL
                    DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                UNIQUE(source_provider, source_account_ref)
            );
            """,
            """
            CREATE INDEX IF NOT EXISTS idx_source_account_links_account_id
                ON source_account_links(account_id);
            """,
            """
            CREATE UNIQUE INDEX IF NOT EXISTS
                idx_transactions_account_external_id
                ON transactions(account_id, external_id)
                WHERE external_id IS NOT NULL;
            """,
        ),
    ),
)


def ensure_migrations_table(connection: sqlite3.Connection) -> None:
    connection.execute(
        """
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );
        """
    )


def get_applied_versions(connection: sqlite3.Connection) -> set[str]:
    rows = connection.execute("SELECT version FROM schema_migrations;").fetchall()
    return {str(row[0]) for row in rows}


def get_current_schema_version(connection: sqlite3.Connection) -> str | None:
    row = connection.execute(
        """
        SELECT version
        FROM schema_migrations
        ORDER BY applied_at DESC, version DESC
        LIMIT 1;
        """
    ).fetchone()
    if row is None:
        return None
    return str(row[0])


def apply_pending_migrations(connection: sqlite3.Connection) -> list[str]:
    ensure_migrations_table(connection)
    applied_versions = get_applied_versions(connection)
    applied_now: list[str] = []

    for migration in MIGRATIONS:
        if migration.version in applied_versions:
            continue

        with connection:
            for statement in migration.statements:
                connection.execute(statement)
            connection.execute(
                """
                INSERT INTO schema_migrations (version, description, applied_at)
                VALUES (?, ?, ?);
                """,
                (
                    migration.version,
                    migration.description,
                    datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                ),
            )

        applied_now.append(migration.version)

    return applied_now
