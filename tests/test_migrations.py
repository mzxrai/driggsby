from pathlib import Path
import sqlite3

import pytest

from driggsby.db import initialize_database


def _table_names(connection: sqlite3.Connection) -> set[str]:
    rows = connection.execute(
        """
        SELECT name
        FROM sqlite_master
        WHERE type = 'table'
          AND name NOT LIKE 'sqlite_%';
        """
    ).fetchall()
    return {str(row[0]) for row in rows}


def _column_names(connection: sqlite3.Connection, table_name: str) -> set[str]:
    rows = connection.execute(f"PRAGMA table_info('{table_name}');").fetchall()
    return {str(row[1]) for row in rows}


def _index_names(connection: sqlite3.Connection, table_name: str) -> set[str]:
    rows = connection.execute(f"PRAGMA index_list('{table_name}');").fetchall()
    return {str(row[1]) for row in rows}


def _has_unique_index_on_columns(
    connection: sqlite3.Connection, table_name: str, columns: tuple[str, ...]
) -> bool:
    rows = connection.execute(f"PRAGMA index_list('{table_name}');").fetchall()
    for row in rows:
        index_name = str(row[1])
        is_unique = bool(row[2])
        if not is_unique:
            continue

        index_info_rows = connection.execute(
            f"PRAGMA index_info('{index_name}');"
        ).fetchall()
        index_columns = tuple(
            str(index_info_row[2]) for index_info_row in index_info_rows
        )
        if index_columns == columns:
            return True

    return False


def test_initialize_database_applies_core_migration(fake_home: Path) -> None:
    first = initialize_database()
    second = initialize_database()

    assert first.created is True
    assert first.applied_versions == ("001_core_ledger",)
    assert first.current_version == "001_core_ledger"

    assert second.created is False
    assert second.applied_versions == ()
    assert second.current_version == "001_core_ledger"


def test_expected_tables_and_indexes_exist(fake_home: Path) -> None:
    result = initialize_database()
    connection = sqlite3.connect(result.path)
    try:
        tables = _table_names(connection)
        assert {
            "schema_migrations",
            "accounts",
            "imports",
            "transactions",
            "source_account_links",
        } <= tables

        imports_columns = _column_names(connection, "imports")
        assert "source_provider" in imports_columns
        assert "source_account_ref" in imports_columns

        accounts_indexes = _index_names(connection, "accounts")
        assert "idx_accounts_type" in accounts_indexes
        assert "idx_accounts_institution" in accounts_indexes

        imports_indexes = _index_names(connection, "imports")
        assert "idx_imports_account_id" in imports_indexes
        assert "idx_imports_imported_at" in imports_indexes
        assert "idx_imports_provider_ref" in imports_indexes

        transaction_indexes = _index_names(connection, "transactions")
        assert "idx_transactions_account_posted" in transaction_indexes
        assert "idx_transactions_import_id" in transaction_indexes
        assert "idx_transactions_category_posted" in transaction_indexes
        assert "idx_transactions_account_external_id" in transaction_indexes
        assert "idx_transactions_transfer_pair" not in transaction_indexes

        transaction_columns = _column_names(connection, "transactions")
        assert "transfer_pair_id" not in transaction_columns
        assert "transfer_role" not in transaction_columns

        assert _has_unique_index_on_columns(
            connection,
            "source_account_links",
            ("source_provider", "source_account_ref"),
        )
    finally:
        connection.close()


def test_constraints_enforced(fake_home: Path) -> None:
    result = initialize_database()
    connection = sqlite3.connect(result.path)
    try:
        account_id = connection.execute(
            """
            INSERT INTO accounts (name, account_type)
            VALUES ('Primary Checking', 'bank')
            RETURNING id;
            """
        ).fetchone()
        assert account_id is not None
        account_row_id = int(account_id[0])

        import_id = connection.execute(
            """
            INSERT INTO imports (
                account_id,
                source_name,
                source_type,
                source_provider,
                source_account_ref,
                statement_hash,
                parser_name,
                parser_version,
                metadata_json
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id;
            """,
            (
                account_row_id,
                "sample.csv",
                "csv",
                "chase",
                "acct-1234",
                "abc123",
                "driggsby-skill-parser",
                "v1",
                '{"ok": true}',
            ),
        ).fetchone()
        assert import_id is not None
        import_row_id = int(import_id[0])

        with pytest.raises(sqlite3.IntegrityError):
            connection.execute(
                """
                INSERT INTO imports (
                    account_id,
                    source_name,
                    source_type,
                    source_provider,
                    source_account_ref,
                    statement_hash,
                    parser_name,
                    parser_version
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?);
                """,
                (
                    account_row_id,
                    "sample-duplicate.csv",
                    "csv",
                    "chase",
                    "acct-1234",
                    "abc123",
                    "driggsby-skill-parser",
                    "v1",
                ),
            )

        with pytest.raises(sqlite3.IntegrityError):
            connection.execute(
                """
                INSERT INTO imports (
                    account_id,
                    source_name,
                    source_type,
                    source_provider,
                    source_account_ref,
                    statement_hash,
                    parser_name,
                    parser_version,
                    metadata_json
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);
                """,
                (
                    account_row_id,
                    "bad-metadata.csv",
                    "csv",
                    "chase",
                    "acct-1234",
                    "hash-2",
                    "driggsby-skill-parser",
                    "v1",
                    "not-json",
                ),
            )

        connection.execute(
            """
            INSERT INTO transactions (
                account_id,
                import_id,
                dedupe_fingerprint,
                posted_date,
                description,
                amount_cents
            )
            VALUES (?, ?, ?, ?, ?, ?);
            """,
            (
                account_row_id,
                import_row_id,
                "fp-1",
                "2026-01-01",
                "Coffee Shop",
                -650,
            ),
        )

        with pytest.raises(sqlite3.IntegrityError):
            connection.execute(
                """
                INSERT INTO transactions (
                    account_id,
                    import_id,
                    dedupe_fingerprint,
                    posted_date,
                    description,
                    amount_cents
                )
                VALUES (?, ?, ?, ?, ?, ?);
                """,
                (
                    account_row_id,
                    import_row_id,
                    "fp-1",
                    "2026-01-01",
                    "Coffee Shop duplicate",
                    -650,
                ),
            )

        connection.execute(
            """
            INSERT INTO source_account_links (
                source_provider,
                source_account_ref,
                account_id
            )
            VALUES (?, ?, ?);
            """,
            ("apple_card", "apple-card-0001", account_row_id),
        )

        with pytest.raises(sqlite3.IntegrityError):
            connection.execute(
                """
                INSERT INTO source_account_links (
                    source_provider,
                    source_account_ref,
                    account_id
                )
                VALUES (?, ?, ?);
                """,
                ("apple_card", "apple-card-0001", account_row_id),
            )
    finally:
        connection.close()
