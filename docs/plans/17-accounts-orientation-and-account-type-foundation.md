# 17 - Accounts Orientation Surface + Account Type Foundation

## Summary

This phase delivers the account-orientation and account-typing foundation requested in user interviews and planning notes.

Primary outcomes:
- add a standalone `driggsby accounts` command with plaintext and `--json` support
- add per-import account coverage to `import list` (including `--json`) so agents can see touched account keys and outcomes in one call
- append post-import ledger/account summary to `import create` success output
- add optional `account_type` to import schema with validated canonical values + aliases
- enforce account-type consistency for existing account keys
- add account metadata persistence with migration/backfill support for existing ledgers
- expose `account_type` through `import keys uniq` and help/schema surfaces

This is a greenfield project and breaking changes are explicitly allowed.

## Why This Phase

Interview evidence in `/Users/mbm-gsc/docs/driggsby/user_interviews` repeatedly showed:
- agents need a first-call orientation command (`driggsby accounts`) every session
- import success currently lacks a confidence checkpoint (“what is now in the ledger?”)
- `import list` currently lacks account visibility, causing cross-referencing friction
- `account_type` is required before reliable snapshot/forecast intelligence can be built

## Locked Decisions

- [x] `driggsby accounts` will be a top-level command (not `account list`).
- [x] `driggsby accounts --json` will return an object with `summary` + `rows`.
- [x] `import list` per-row account details will represent attempted+outcome coverage (`rows_read`, `inserted`, `deduped`) per account.
- [x] `account_type` will be validated text with canonical values + aliases (not unrestricted free-form and not a hard Rust enum in the persisted contract).
- [x] Missing `account_type` is allowed; conflicting provided `account_type` for an already-typed account is a hard validation error.
- [x] Canonical account metadata will live in a new `internal_accounts` table.
- [x] `account_type` will be discoverable via `import keys uniq`.
- [x] `import create --help` will document `account_type` as optional but recommended.

## Canonical `account_type` Contract

Canonical values accepted in this phase:
- `checking`
- `savings`
- `credit_card`
- `loan`
- `brokerage`
- `retirement`
- `hsa`
- `other`

Alias normalization (input -> canonical):
- `credit`, `creditcard`, `card` -> `credit_card`
- `investment`, `taxable` -> `brokerage`
- `401k`, `ira`, `roth` -> `retirement`

Validation behavior:
- unrecognized non-empty values fail validation with deterministic issue code
- normalization occurs before consistency checks
- consistency checks compare canonical normalized values only

## Goals and Acceptance Criteria

- [x] Agents can run `driggsby accounts` and immediately see ledger-wide orientation + per-account stats.
  - Acceptance: plaintext includes one summary line and deterministic per-account rows.
  - Acceptance: `--json` includes `summary` and `rows` with stable field names.

- [x] `import create` success includes post-import ledger/account summary in both plaintext and JSON.
  - Acceptance: commit success payload includes a structured ledger summary block.
  - Acceptance: dry-run behavior remains unchanged (no misleading "ledger now" output).

- [x] `import list` surfaces touched accounts and per-account outcomes.
  - Acceptance: each import row in `import list --json` includes `accounts[]` with `account_key`, `account_type`, `rows_read`, `inserted`, `deduped`.
  - Acceptance: plaintext `import list` visibly surfaces account keys per import without requiring SQL.

- [x] `account_type` is supported as optional import input with consistency enforcement.
  - Acceptance: first typed import sets canonical type for account.
  - Acceptance: later conflicting type fails validation with row-level issue + guidance.
  - Acceptance: later omitted type succeeds.

- [x] Existing ledgers are migrated safely and backfilled for account metadata.
  - Acceptance: migration adds required schema objects and passes setup integrity checks.
  - Acceptance: existing distinct account keys are represented in `internal_accounts` with null `account_type` unless typed by future imports.

- [x] `import keys uniq` includes `account_type` inventory.
  - Acceptance: property parser/help/output all support `account_type` deterministically.

## Scope

- [x] New command: `driggsby accounts` (`--json` supported)
- [x] Import create success contract/renderer additions for ledger/account summary
- [x] Import list contract/renderer additions for account coverage per import
- [x] Import schema + validation support for optional `account_type`
- [x] Account metadata storage + migration/backfill for existing ledgers
- [x] `import keys uniq` extension to include `account_type`
- [x] Help text and schema contract updates (`import create --help`, top-level help, parser guidance)
- [x] Full test coverage updates across client/CLI/unit/integration

## Out of Scope

- [ ] Transfer detection and transfer netting logic
- [ ] Snapshot/forecast command implementation
- [ ] Merchant/category normalization redesign
- [ ] Retrofitting exact per-account outcome metrics for every historic reverted import where source data was already deleted pre-phase

## Public API / Interface Changes

### 1) New top-level command: `accounts`

CLI:
- `driggsby accounts`
- `driggsby accounts --json`

Success command key:
- `accounts`

JSON shape (contract):

```json
{
  "summary": {
    "account_count": 8,
    "transaction_count": 644,
    "earliest_posted_at": "2025-11-01",
    "latest_posted_at": "2026-03-31",
    "typed_account_count": 5,
    "untyped_account_count": 3,
    "net_amount": 10345.22
  },
  "rows": [
    {
      "account_key": "operating_checking_7314",
      "account_type": "checking",
      "currency": "USD",
      "txn_count": 42,
      "first_posted_at": "2026-01-01",
      "last_posted_at": "2026-03-31",
      "net_amount": 3359.60
    }
  ]
}
```

Sorting:
- `rows` sorted by `account_key ASC, currency ASC`.

### 2) `import create` success payload expansion

Add structured post-import ledger summary under `data.ledger_accounts` for committed imports only.

```json
{
  "data": {
    "import_id": "imp_...",
    "ledger_accounts": {
      "summary": { "...": "same shape as accounts.summary" },
      "rows": [ { "...": "same shape as accounts.rows item" } ]
    }
  }
}
```

Plaintext:
- append a `Your ledger now:` section after import mechanics and duplicate summary, reusing the same account-summary model.

### 3) `import list` row expansion

Maintain top-level JSON array shape for `import list --json`.

Per row additive fields:

```json
{
  "import_id": "imp_...",
  "status": "committed",
  "...": "existing fields preserved",
  "accounts": [
    {
      "account_key": "operating_checking_7314",
      "account_type": "checking",
      "rows_read": 120,
      "inserted": 110,
      "deduped": 10
    }
  ]
}
```

### 4) `import keys uniq` extension

- `account_type` added to supported properties in parser/help/errors.
- inventory output includes `account_type` alongside existing properties.

### 5) Import schema extension

`account_type` becomes an optional input field in JSON/CSV import schema.

`import create --help` updates:
- field description
- canonical values
- alias examples
- optional-but-recommended guidance

## Data Model / Persistence Changes

### 1) New table: `internal_accounts`

Columns:
- `account_key TEXT PRIMARY KEY`
- `account_type TEXT` (nullable)
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Purpose:
- canonical account metadata source of truth
- consistency checks for incoming `account_type`

### 2) New table: `internal_import_account_stats`

Columns:
- `import_id TEXT NOT NULL`
- `account_key TEXT NOT NULL`
- `rows_read INTEGER NOT NULL DEFAULT 0`
- `inserted INTEGER NOT NULL DEFAULT 0`
- `deduped INTEGER NOT NULL DEFAULT 0`
- `PRIMARY KEY (import_id, account_key)`

Purpose:
- stable per-import account coverage snapshot, preserved across undo/revert

### 3) Migration and backfill strategy

- Add migration `0005_accounts_metadata_and_import_account_stats.sql`.
- Backfill `internal_accounts` from distinct `account_key` in `internal_transactions`.
- Backfill `internal_import_account_stats` best-effort:
  - `inserted` from `internal_transactions` grouped by `(import_id, account_key)`
  - `deduped` from `internal_transaction_dedupe_candidates` grouped by `(import_id, account_key)`
  - `rows_read = inserted + deduped`
- Known limitation (explicit): historic reverted imports created before this phase may have partial account coverage if inserted rows were deleted before stats existed.

### 4) View and schema contract updates

- `v1_transactions`: add `account_type` via join on `internal_accounts.account_key`.
- `v1_accounts`: include `account_type` and keep existing aggregate fields.
- Update public view contracts in code and schema text output accordingly.

## Architecture and Implementation Design

### Workstream A - Contracts and shared types

- [x] Add account summary contracts in `crates/driggsby-client/src/contracts/types.rs`:
  - `AccountsSummary`
  - `AccountRow`
  - `AccountsData`
  - `ImportListAccountStat`
- [x] Add `ledger_accounts` field to `ImportData` (optional).
- [x] Add `accounts: Vec<ImportListAccountStat>` to `ImportListItem`.
- [x] Extend `ImportKeyInventory` for `account_type`.

### Workstream B - Client command surfaces

- [x] Create `crates/driggsby-client/src/commands/accounts.rs`.
- [x] Wire module export in `commands/mod.rs`.
- [x] Add dispatch path + CLI variant for `accounts` in `crates/driggsby-cli/src/cli.rs` and `dispatch.rs`.
- [x] Update top-level help copy in `main.rs` to include `driggsby accounts`.

### Workstream C - Import create / list data assembly

- [x] During commit import flow, compute and persist per-import account stats in `internal_import_account_stats`.
- [x] After commit, compute ledger account summary and attach to `ImportExecutionResult`.
- [x] Update `commands::import::list_with_options` to load `accounts` stats per import row.
- [x] Join account stats to `internal_accounts` to include `account_type` in list outputs.

### Workstream D - Account type parsing + validation + consistency

- [x] Add `account_type` to optional import schema fields (`commands/common.rs`).
- [x] Extend parser row struct in `import/parse.rs` (JSON + CSV).
- [x] Extend canonical row model and validation in `import/validate.rs`:
  - normalization
  - canonical value validation
  - in-file consistency
- [x] Add ledger consistency check against `internal_accounts`.
- [x] Add deterministic issue codes:
  - `invalid_account_type`
  - `account_type_conflict_in_import`
  - `account_type_conflicts_with_ledger`

### Workstream E - Persistence + migration

- [x] Add migration SQL `0005_accounts_metadata_and_import_account_stats.sql`.
- [x] Update `migrations.rs` to register migration and update safe-repair coverage as needed.
- [x] Update setup integrity checks in `setup.rs`:
  - new required table(s)
  - new required columns
  - user_version expectation (4 -> 5)
- [x] Update bootstrap SQL for fresh installs to include new tables and updated views.

### Workstream F - Rendering and output-mode wiring

- [x] Add plaintext renderer for `accounts` (new file `output/accounts_text.rs`).
- [x] Wire `accounts` in `output/mod.rs` and `output/json.rs`.
- [x] Add output mode support in `output/mode.rs` for `accounts --json`.
- [x] Update `import_text.rs`:
  - import create ledger summary section
  - import list account visibility section
- [x] Keep deterministic ordering and no markdown tables.

### Workstream G - Help and parser guidance

- [x] Update `IMPORT_CREATE_AFTER_HELP` with `account_type` docs and examples.
- [x] Update `import keys uniq` property parser/help to include `account_type`.
- [x] Update parse-error command hint resolver in `main.rs` for `accounts` command path.

## TDD Execution Plan

### Step 0 - Red tests first

- [x] Add failing tests for new `accounts` command parse/dispatch/output.
- [x] Add failing tests for `import list` account stats in plaintext and JSON.
- [x] Add failing tests for post-import ledger summary in `import create`.
- [x] Add failing tests for `account_type` parsing, validation, and conflict handling.
- [x] Add failing migration/setup tests for new tables/views/user_version.
- [x] Add failing `import keys uniq account_type` tests.

### Step 1 - Minimal implementation

- [x] Implement contracts + command plumbing.
- [x] Implement migration and persistence updates.
- [x] Implement validation and account metadata write path.
- [x] Implement renderers/help updates.

### Step 2 - Iterate to green

- [x] Run targeted tests and resolve only behavior gaps.

### Step 3 - Full verification

- [x] `cargo test --all-features`
- [x] `just required-check`

### Step 4 - Agentic review stages

- [x] Stage 1 `agentic_ux` review (primary + adversarial)
- [x] Stage 2 `verification` review (primary + adversarial)
- [x] Fix `high_friction+` and `medium+` findings

### Step 5 - Final gate

- [x] `just rust-verify`
- [x] Closeout updates in this plan + executive summary section
- [x] Commit with descriptive message ending in `Authored by:` footer

## Test Matrix

- [x] `T-01` `driggsby accounts` plaintext shows ledger summary and per-account rows.
- [x] `T-02` `driggsby accounts --json` returns `{ summary, rows }`.
- [x] `T-03` empty-ledger `accounts` output provides clear first-step guidance.
- [x] `T-04` `import create` commit output includes `ledger_accounts` JSON block.
- [x] `T-05` `import create` plaintext includes `Your ledger now` section.
- [x] `T-06` dry-run `import create` does not include misleading ledger-now block.
- [x] `T-07` `import list --json` rows include additive `accounts[]` with counts.
- [x] `T-08` `import list` plaintext visibly surfaces account keys per import.
- [x] `T-09` import with first-time valid `account_type` persists canonical type.
- [x] `T-10` import omitting `account_type` for typed account succeeds.
- [x] `T-11` conflicting `account_type` vs canonical ledger value fails with deterministic issue code.
- [x] `T-12` conflicting `account_type` values in the same batch fail deterministically.
- [x] `T-13` unknown `account_type` alias/value fails with guidance.
- [x] `T-14` `import keys uniq account_type` works in plaintext and JSON.
- [x] `T-15` setup migration upgrades to user_version `5` and validates new required objects.
- [x] `T-16` backfilled existing ledger exposes account keys in `internal_accounts`.
- [x] `T-17` undo/reverted import still retains per-import account stats snapshot for new imports.

## Risks and Mitigations

- [x] Risk: ambiguous historic import account stats for old reverted imports.
  - Mitigation: explicit best-effort backfill and documented limitation in output/help text.

- [x] Risk: account-type alias sprawl causes contract drift.
  - Mitigation: centralized normalization map + explicit canonical output values only.

- [x] Risk: output becomes too verbose in `import list` plaintext.
  - Mitigation: keep compact primary table and deterministic secondary account-details section.

- [x] Risk: migration misses setup integrity updates and causes false `ledger_corrupt`.
  - Mitigation: update `setup.rs` + setup tests in same TDD cycle before implementation is considered complete.

## Assumptions and Defaults

- [x] Breaking changes are allowed in this phase.
- [x] `account_type` remains optional input for now; strict requirement can be revisited in snapshot phase.
- [x] Canonical account metadata authority is `internal_accounts`.
- [x] `import list --json` remains a top-level array; account coverage is additive per-row data.
- [x] New `accounts --json` uses object shape (`summary` + `rows`) for deterministic agent parsing.

## Progress Update (In-Flight)

- Completed foundations:
  - Contracts/types for accounts summary rows, import list account stats, and `ledger_accounts`.
  - Client account query command (`commands/accounts.rs`) and dispatch wiring.
  - Account metadata persistence + migration path (`internal_accounts`, `internal_import_account_stats`) with setup integrity upgrades to user_version `5`.
  - `account_type` parsing/normalization/validation with conflict checks (in-file and against ledger), including expanded alias support for common agent-generated variants (for example `retirement_401k`, `401k_retirement`, `credit-card`).
  - `import keys uniq` property surface extended with `account_type`.
  - CLI JSON policy refined per user feedback: edit commands retain envelope (`ok/version`), while read-only surfaces return command data directly.
- Additional closeout fixes completed after review:
  - `import create` now rejects conflicting dual-source input (file + stdin) instead of silently ignoring stdin.
  - Dry-run next-step guidance now emits concrete file commit command when path is known, and explicit stdin replay template for stdin-based dry-runs.
  - Undo now reconciles account metadata for touched keys and clears stale account types when no canonical transactions remain.
  - Safe-repair/verification coverage now includes `internal_import_account_stats` indexes.
  - Shared account summary/table renderer extracted to reduce duplication and drift between `accounts` and `import` text surfaces.
- Remaining before full completion:
  - Create commit with descriptive message ending in `Authored by:` footer.

## Executive Summary

- Implemented the full accounts-orientation and account-type foundation: new `accounts` command, richer import outputs, per-import account coverage in `import list`, and optional validated `account_type` input with alias normalization and conflict enforcement.
- Added persistent account metadata/state with migration and setup integrity upgrades, including safe-repair coverage for new account-stat indexes.
- Improved first-shot UX and safety after review: conflicting file+stdin import sources now hard-fail, dry-run next-step commands are more actionable, and root help now surfaces `driggsby accounts`.
- Fixed undo correctness bug by reconciling account metadata for touched keys, preventing stale `account_type` state from blocking valid future imports after revert.
- Reduced renderer drift risk by extracting shared account summary/table rendering logic used by both `accounts` and `import` output paths.
- Verified with full Rust gates: targeted suites, `cargo test --all-features`, `just required-check`, and `just rust-verify` all passed.
