"""Schema inspection helpers."""

from typing import Any
import sqlite3

from driggsby.migrations import get_current_schema_version


def _table_names(connection: sqlite3.Connection) -> list[str]:
    rows = connection.execute(
        """
        SELECT name
        FROM sqlite_master
        WHERE type = 'table'
          AND name NOT LIKE 'sqlite_%'
        ORDER BY name ASC;
        """
    ).fetchall()
    return [str(row[0]) for row in rows]


def _table_columns(
    connection: sqlite3.Connection, table_name: str
) -> list[dict[str, Any]]:
    rows = connection.execute(f"PRAGMA table_info('{table_name}');").fetchall()
    columns: list[dict[str, Any]] = []
    for row in rows:
        columns.append(
            {
                "name": str(row[1]),
                "type": str(row[2]),
                "nullable": not bool(row[3]),
                "primary_key": bool(row[5]),
                "default": row[4],
            }
        )
    return columns


def _table_indexes(
    connection: sqlite3.Connection, table_name: str
) -> list[dict[str, Any]]:
    rows = connection.execute(f"PRAGMA index_list('{table_name}');").fetchall()
    indexes: list[dict[str, Any]] = []
    for row in rows:
        index_name = str(row[1])
        index_columns_rows = connection.execute(
            f"PRAGMA index_info('{index_name}');"
        ).fetchall()
        index_columns = [str(column_row[2]) for column_row in index_columns_rows]
        indexes.append(
            {
                "name": index_name,
                "unique": bool(row[2]),
                "origin": str(row[3]),
                "partial": bool(row[4]),
                "columns": index_columns,
            }
        )

    indexes.sort(key=lambda index: str(index["name"]))
    return indexes


def build_schema_payload(connection: sqlite3.Connection) -> dict[str, Any]:
    tables: list[dict[str, Any]] = []
    for table_name in _table_names(connection):
        tables.append(
            {
                "name": table_name,
                "columns": _table_columns(connection, table_name),
                "indexes": _table_indexes(connection, table_name),
            }
        )

    return {
        "schema_version": get_current_schema_version(connection),
        "tables": tables,
    }
