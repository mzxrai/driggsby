# Plan 2 — Import Contract + Identity + `--dry-run`

## Status
- [x] Plan updated and persisted in `plans/2-import-contract-identity.md`
- [x] Approved for implementation

## Goal
- [x] Keep import dead-simple for the calling agent
- [x] Remove transfer-only schema fields from v1
- [x] Add clean source identity tracking for long-term consistency
- [x] Validate imports with `driggsby import --dry-run` (no separate command)

## Locked Decisions
- [x] Use `driggsby import --dry-run`, not `validate-import`
- [x] Remove `transfer_pair_id` and `transfer_role` from `transactions`
- [x] Edit base migration (`001`) directly, then reset local pre-prod DB
- [x] Keep this round focused on contract + validation preview, not full insert

## Scope

### In Scope
- [x] Update `src/driggsby/migrations.py` (`001_core_ledger`) to remove transfer fields/index
- [x] Add source identity support in schema (`source_provider`, `source_account_ref`, link table)
- [x] Add import contract validation models (required vs optional fields)
- [x] Add source/provider normalization logic (alias-safe canonical values)
- [x] Add server-side dedupe fingerprint generation for preview output
- [x] Add `--dry-run` behavior to `driggsby import --format json [FILE|-]`
- [x] Reset local DB and re-initialize from migrations

### Out of Scope
- [ ] Full transaction insert pipeline
- [ ] Automatic/fuzzy account matching
- [ ] Holdings/positions schema work
- [ ] Plaid-first field expansion

## Import Contract (v1, small and explicit)

### Required top-level
- [x] `source_provider` (string)
- [x] `source_account_ref` (string, non-empty)
- [x] `transactions` (array, non-empty)

### Required per transaction
- [x] `posted_date` (`YYYY-MM-DD`)
- [x] `description` (string, non-empty)
- [x] `amount_cents` (signed integer)
- [x] `currency` (3-letter code; default `USD` when omitted)

### Optional top-level
- [x] `source_name`
- [x] `source_type` (`pdf|csv|json|api`)
- [x] `parser_name`
- [x] `parser_version`
- [x] `period_start`
- [x] `period_end`
- [x] `metadata_json`

### Optional per transaction
- [x] `settled_date`
- [x] `merchant`
- [x] `normalized_merchant`
- [x] `category`
- [x] `transaction_type`
- [x] `status`
- [x] `owner_name`
- [x] `external_id`
- [x] `metadata_json`

## Schema Changes

### `001_core_ledger` cleanup (pre-prod edit)
- [x] Remove `transactions.transfer_pair_id`
- [x] Remove `transactions.transfer_role`
- [x] Remove `idx_transactions_transfer_pair`

### Source identity support
- [x] Add `imports.source_provider TEXT NOT NULL DEFAULT 'other'`
- [x] Add `imports.source_account_ref TEXT NOT NULL DEFAULT ''`
- [x] Add index on `(source_provider, source_account_ref)`
- [x] Add `source_account_links` table with unique `(source_provider, source_account_ref)`

## CLI Behavior

### `driggsby import --format json [FILE|-] --dry-run`
- [x] Parse JSON from file or stdin (`-` / omitted file path uses stdin)
- [x] Validate payload contract
- [x] Normalize `source_provider` to canonical slug
- [x] Compute dedupe fingerprints (preview only)
- [x] Output summary JSON:
- [x] `valid`
- [x] `normalized_source_provider`
- [x] `source_account_ref`
- [x] `transaction_count`
- [x] `errors` (path + message)
- [x] Exit `0` if valid, non-zero if invalid

### `driggsby import` (without `--dry-run`)
- [x] Keep current placeholder behavior for this round

## TDD Execution

### Red phase
- [x] Add/adjust tests for `001` transfer-field removal
- [x] Add tests for source identity schema objects
- [x] Add tests for import contract validation
- [x] Add tests for provider normalization aliases
- [x] Add tests for dedupe fingerprint determinism
- [x] Add CLI tests for `import --dry-run` success + failure
- [x] Run `uv run pyright`
- [x] Run `uv run python -m pytest -q` and confirm failing tests

### Green phase
- [x] Implement smallest code needed to pass tests
- [x] Run `uv run pyright`
- [x] Run `uv run python -m pytest -q`

### Verify + review
- [x] Reset DB by moving old file aside: `mv ~/.driggsby/ledger.db ~/.driggsby/ledger.db.preplan2.bak`
- [x] Re-init: `uv run driggsby init`
- [x] Smoke-check: `uv run driggsby schema`
- [x] Run code review subagent
- [x] Run adversarial review subagent
- [x] Fix medium+ issues
- [x] Final `uv run pyright`
- [x] Final `uv run python -m pytest -q`

## Acceptance Criteria
- [x] Contract is small enough for reliable agent mapping
- [x] Transfer fields are removed from v1 schema
- [x] Source identity tracking is present and indexed
- [x] `import --dry-run` gives clear validation/preview output
- [x] Local DB has been reset and rebuilt from updated migrations
- [x] Tests pass and `pyright` is clean

## Executive Summary
- [x] Added a minimal import validation contract with explicit required/optional fields and clear field-level errors for dry-run responses.
- [x] Added source identity normalization and deterministic dedupe fingerprinting for import previews.
- [x] Updated core migration `001_core_ledger` to remove transfer-only transaction fields and add source identity schema support.
- [x] Extended CLI import flow with `--dry-run`, including valid/invalid structured JSON output and non-zero exit on invalid payloads.
- [x] Expanded tests for migration shape, import contract validation, normalization, dedupe behavior, and dry-run CLI coverage (including malformed JSON).
- [x] Local pre-production DB was reset and re-initialized so live schema now matches the updated base migration.

### Notes for Next Agent
- [x] The project intentionally edits migration `001` in pre-production mode; if production compatibility becomes required later, add forward migrations instead of rewriting `001`.
- [x] `source_provider` normalization currently maps known aliases and falls back to `other`; update `src/driggsby/source_identity.py` when onboarding new providers.
- [x] Current `import` without `--dry-run` remains a placeholder by design; persistence/linking logic is still out of scope for this phase.
