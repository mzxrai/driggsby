# Driggsby V1 Proposal (Python-First)

Last updated: 2026-02-20

## Purpose

Build a strict, testable local ledger foundation for agent-driven finance workflows.

Driggsby owns:

- SQLite schema + migrations
- import validation contract
- normalization + dedupe primitives
- local CLI contracts

The user’s agent owns:

- parsing statements/files into structured JSON

## Current Implemented Surface

- `driggsby init`
- `driggsby schema`
- `driggsby import --format json [FILE|-] [--dry-run]`
- `driggsby accounts`
- `driggsby transactions [--account TEXT] [--category TEXT] [--start YYYY-MM-DD] [--end YYYY-MM-DD]`

## Current Behavior

### `init`

- DB path is currently fixed: `~/.driggsby/ledger.db`
- Applies migrations and reports schema version
- Idempotent on repeated runs

### `schema`

- Returns canonical schema metadata from live SQLite

### `import --dry-run`

- Accepts JSON from file or stdin
- Validates required/optional contract fields
- Normalizes `source_provider`
- Computes deterministic dedupe fingerprints
- Returns structured summary JSON with `valid`, `errors`, and counts
- Exits non-zero when invalid

### `import` (without `--dry-run`)

- Still placeholder in this phase

## Current Schema Direction

Core tables:

- `accounts`
- `imports`
- `transactions`
- `source_account_links`
- `schema_migrations`

Recent decisions:

- removed transfer fields from `transactions` for v1 simplicity
- added source identity fields on `imports`
- added source-account linking table

## Non-Goals (Current Phase)

- Full insert/write pipeline for transactions
- Fuzzy account matching
- Dashboard/API product surface
- Plaid-native ingestion

## Next Steps

1. Implement non-dry-run import persistence path.
2. Apply normalization at write time consistently.
3. Add insert-time dedupe behavior.
4. Expand read/query commands beyond placeholders.
