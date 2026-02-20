# Plan 1 — V1 Core Ledger Migrations

## Status
- [x] Plan file created before further implementation

## Goal
- [x] Implement the v1 core SQLite migration system and schema (`accounts`, `imports`, `transactions`, `schema_migrations`)
- [x] Replace toy `schema` output with canonical live DB metadata output
- [x] Keep implementation small, typed, and fully test-backed

## Locked Scope
- [x] No holdings tables in this stage
- [x] `account_subtype` is free text (not enum)
- [x] `parser_name` = `driggsby-skill-parser`, `parser_version` = `v1`
- [x] Signed `amount_cents` model
- [x] JSON metadata fields must pass `json_valid`

## Execution Checklist

### 1) Tests first (red phase)
- [x] Update `tests/test_cli.py` for new `init` and `schema` behavior
- [x] Add `tests/test_migrations.py` for schema objects, indexes, and constraints
- [x] Run `uv run pyright`
- [x] Run `uv run python -m pytest -q` and confirm failures

### 2) Implementation (green phase)
- [x] Add migration module `src/driggsby/migrations.py`
- [x] Update `src/driggsby/db.py` for connection + migration initialization helpers
- [x] Update `src/driggsby/schema.py` to emit canonical schema payload
- [x] Update `src/driggsby/cli.py` (`init` and `schema` commands)

### 3) Verification
- [x] Run `uv run pyright` clean
- [x] Run `uv run python -m pytest -q` all green
- [x] Run CLI smoke checks (`driggsby init`, `driggsby schema`)

### 4) Final pass
- [x] Run subagent review for medium+ blockers
- [x] Fix any medium+ findings
- [x] Re-run `pyright` + tests

## Notes
- [x] Keep updates in this file as boxes are completed.
