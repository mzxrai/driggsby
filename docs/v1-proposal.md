# Driggsby V1 Proposal (Python-First)

Last updated: 2026-02-19

## Purpose

Build an initial Driggsby CLI in Python that gives us a strict, testable foundation for local financial data tooling.  
This phase focuses on project setup, contracts, and safe stubs, not full ledger behavior.

## Scope for This Phase

1. Python package scaffold using `uv`
2. Strict typing via `pyright`
3. Test-first CLI development with `pytest`
4. Command surface and stable placeholder outputs
5. Real local initialization behavior for SQLite file creation

## Command Surface

- `driggsby init`
- `driggsby schema`
- `driggsby import --format json [FILE]`
- `driggsby accounts`
- `driggsby transactions [--account TEXT] [--category TEXT] [--start YYYY-MM-DD] [--end YYYY-MM-DD]`

## Behavioral Contracts

### `init`

- DB location is hardcoded for now: `~/.driggsby/ledger.db`
- Command is idempotent:
  - first run creates directory and DB file
  - later runs return success with `already initialized`
- No table creation in this phase

### `schema`

- Returns valid JSON placeholder output
- Explicitly marked toy/dev
- Current shape:

```json
{
  "toy": true,
  "version": "0.1.0-dev",
  "message": "Toy schema placeholder. Not production-ready.",
  "entities": []
}
```

### `import`

- Supports `--format json`
- Accepts file input or stdin when file is omitted
- Validates JSON structure (placeholder handling only)
- Emits placeholder status output and exits cleanly

### `accounts` and `transactions`

- Return clear `no data yet` placeholder responses
- `transactions` validates date format as `YYYY-MM-DD`

## Non-Goals (This Phase)

- Real transaction persistence/import pipeline
- Deduplication and normalization logic
- Dashboard and API
- Configurable DB path (`--db-path` or env override)

## Why This Approach

This gives us a reliable execution contract for agent workflows now, while keeping implementation small and easy to evolve.  
Future phases can add schema/tables, import logic, and query behavior without reworking the foundation.
