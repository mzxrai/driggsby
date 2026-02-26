# 16 - Import Surface Reliability Remediation (Rogue-Rat Findings)

## Summary

This phase remediates the confirmed import-surface defects from the rogue-rat testing blitz. The goal is to improve agent safety, contract determinism, and data integrity across `import create`, `import list`, `import duplicates`, `import undo`, and `schema` guidance.

Primary outcomes:
- make `--json` deterministic even for parse-time argument failures
- improve source handling (`-` for stdin) and plaintext source-conflict visibility
- fix stale duplicate match pointers after undo/promotion while preserving historical context
- enforce amount precision guardrails to reduce financial data drift risk
- align timestamp UX with local-friendly display while preserving machine-safe fields
- clarify semantic schema contract wording vs raw SQLite physical metadata

## Why This Phase

- The ad-hoc subagent testing run surfaced several deterministic production-risk defects.
- The highest-risk items affect agent first-shot reliability and machine contract correctness.
- This project targets lazy coding agents first; ambiguous output contracts and stale pointers increase breakage risk.

## Locked Decisions

- [x] Timestamp UX should be friendly local-time for human-facing views and include machine-safe timestamp fields for JSON.
- [x] Amount precision policy: reject over-scale amounts in validation (initial policy: max 2 decimal places).
- [x] Duplicates contract should include both current live match pointers and historical-at-dedupe pointers.
- [x] Dedupe history retention remains keep-forever with import-scoped query behavior.
- [x] `import list --json` remains a top-level array in this phase; shape changes are additive.
- [x] Schema mismatch finding is treated as contract wording clarity, not a physical schema redesign.

## Goals and Acceptance Criteria

- [x] `--json` mode returns JSON-formatted errors for parse-time argument failures.
  - Acceptance: invalid `import keys uniq` property with `--json` yields JSON envelope, not plaintext prose.
- [x] `import create --dry-run -` reads stdin as source.
  - Acceptance: piped input succeeds with `-`, empty stdin with `-` fails with explicit guidance.
- [x] Plaintext output clearly surfaces mixed-source conflict (`file + stdin`).
  - Acceptance: plaintext includes explicit stdin-ignored warning message.
- [x] `import duplicates` no longer returns stale non-existent active match IDs after undo/promotion.
  - Acceptance: live `matched_*` fields resolve to active rows when present; historical pointers remain available.
- [x] Amount precision policy is enforced.
  - Acceptance: values over scale fail validation deterministically with row-level issue code.
- [x] `import list --json` includes local-friendly + machine-safe timestamp structure.
  - Acceptance: each row includes `timestamps.{created,committed,reverted}` objects (or null) with `local`, `utc`, and `epoch_s`.
- [x] Schema output clarifies semantic contract boundaries.
  - Acceptance: `schema` and `schema view` text explicitly state semantic-contract nature.

## Scope

- [x] CLI parse-failure output mode handling for `--json`
- [x] Import source resolution (`-` alias, stdin behavior, conflict communication)
- [x] Duplicates API shape and SQL query semantics for live + historical pointers
- [x] Amount scale validation policy
- [x] Import list JSON timestamp enrichment
- [x] Schema text clarity updates
- [x] Regression tests for ordering, contracts, and edge cases

## Out of Scope

- [x] Introducing a new global duplicate history command
- [x] Back-compat preservation for old JSON consumers (greenfield allows breakage)
- [x] Multi-currency minor-unit policy engine (this phase applies a global max scale rule)
- [x] Large migration of timestamp storage format at DB level

## Public API / Interface Changes

### 1) `import duplicates` row shape expansion

- [x] Add fields to `ImportDuplicateRow`:
  - `matched_txn_id_at_dedupe: Option<String>`
  - `matched_import_id_at_dedupe: Option<String>`
- [x] Keep existing `matched_txn_id` / `matched_import_id` but redefine semantics as current live match pointers.

### 2) `import list --json` additive timestamp object

- [x] For each row, add:
  - `timestamps.created`
  - `timestamps.committed`
  - `timestamps.reverted`
- [x] Each non-null timestamp object includes:
  - `epoch_s` (integer)
  - `utc` (RFC3339 UTC string)
  - `local` (friendly local timezone string)

### 3) Validation issue contract for amount scale

- [x] Add deterministic validation issue code for over-scale amounts (for example `invalid_amount_scale`).

## Architecture and Implementation Design

### Workstream A - Parse-time `--json` correctness

- [x] Update parse-error handling in `crates/driggsby-cli/src/main.rs` to infer requested output mode from raw args when clap parsing fails.
- [x] Update failure rendering in `crates/driggsby-cli/src/output/mod.rs` to allow JSON error output for `invalid_argument` parse failures when JSON mode is requested.
- [x] Preserve existing plaintext fallback behavior when JSON mode is not requested.

### Workstream B - Source handling and visibility

- [x] Update `crates/driggsby-client/src/import/input.rs`:
  - [x] interpret `path == "-"` as stdin input
  - [x] return explicit guidance if `-` is provided without stdin content
  - [x] keep file-over-stdin precedence for non-`-` file paths
- [x] Update plaintext render path in `crates/driggsby-cli/src/output/import_text.rs` to emit source-conflict warning from `warnings` payload.
- [x] Update help text in `crates/driggsby-cli/src/cli.rs` to explicitly document `-` stdin usage.

### Workstream C - Duplicate pointer correctness after undo/promotion

- [x] Extend duplicate row types in `crates/driggsby-client/src/contracts/types.rs`.
- [x] Update `duplicates_with_options` query in `crates/driggsby-client/src/commands/import.rs`:
  - [x] resolve live match via joins to active `internal_transactions`
  - [x] preserve original `matched_*` values as historical-at-dedupe fields
  - [x] retain deterministic ordering (`source_row_index`, `dedupe_reason`, `candidate_id`)
- [x] Update duplicate projection logic in `crates/driggsby-client/src/import/mod.rs` as needed for preview consistency.
- [x] Optionally enrich plaintext duplicate rendering to show “originally matched” when live vs historical differs.

### Workstream D - Amount precision guardrails

- [x] Refactor validation in `crates/driggsby-client/src/import/validate.rs` so amount-scale checks can run with currency context.
- [x] Enforce max 2 decimal places for this phase.
- [x] Return row-level validation issues with deterministic code and clear expected/received messaging.
- [x] Update import help copy in `crates/driggsby-cli/src/cli.rs` to state precision expectations.

### Workstream E - Timestamp contract enrichment for `import list --json`

- [x] Keep DB/client row storage and query unchanged in `crates/driggsby-client/src/commands/import.rs`.
- [x] Implement additive timestamp shaping in `crates/driggsby-cli/src/output/json.rs` (`render_import_list_json`):
  - [x] parse epoch-like fields
  - [x] produce `utc` + `local` + numeric `epoch_s`
  - [x] keep existing fields (`created_at`, `committed_at`, `reverted_at`) for now
- [x] Preserve local-friendly formatting consistency with `crates/driggsby-cli/src/output/import_text.rs`.

### Workstream F - Schema wording clarity

- [x] Update `crates/driggsby-cli/src/output/schema_text.rs` to clearly label schema output as Driggsby semantic contract.
- [x] Clarify that SQLite `PRAGMA table_info(view)` may differ for view physical metadata.

## TDD Execution Plan

### Step 0 - Red tests first

- [x] Add failing tests for each bug fix before implementation:
  - [x] parse-error JSON contract
  - [x] `-` stdin alias behavior
  - [x] plaintext source-conflict warning rendering
  - [x] stale duplicate match pointer remediation
  - [x] amount precision rejection
  - [x] additive timestamp object in `import list --json`
  - [x] schema wording clarity output checks

### Step 1 - Minimal implementation

- [x] Implement workstreams A-F with smallest coherent code changes.

### Step 2 - Iterate to green

- [x] Run focused tests and patch only defects that block intended behavior.

### Step 3 - Full verification

- [x] `cargo +stable test --all-features`
- [x] `just required-check`

### Step 4 - Rogue-rat replay

- [x] Re-run a reduced ad-hoc CLI smoke matrix in isolated `/tmp` homes:
  - [x] mixed source runs
  - [x] JSON parse-failure runs
  - [x] undo/promotion + duplicates inspection runs
  - [x] schema/view wording checks
  - [x] SQL verification via `schema` db path + `sqlite3`

### Step 5 - Final gate

- [x] `just rust-verify`
- [x] Update this plan with completion checkmarks and executive summary

## Test Matrix

- [x] `T-01` `import keys uniq --json ACCOUNT_KEY` returns JSON error envelope.
- [x] `T-02` `import create --dry-run -` with piped valid input succeeds.
- [x] `T-03` `import create --dry-run -` with no stdin content fails with explicit guidance.
- [x] `T-04` mixed stdin+file source surfaces warning in plaintext and JSON.
- [x] `T-05` duplicate pointer after undo/promotion resolves to active live txn/import IDs.
- [x] `T-06` duplicate row still includes historical-at-dedupe match IDs.
- [x] `T-07` over-scale amount fails validation with deterministic issue code.
- [x] `T-08` valid 2-decimal and integer amounts continue to pass.
- [x] `T-09` `import list --json` rows include additive `timestamps.*` objects.
- [x] `T-10` `import list` plaintext remains local-friendly and deterministic.
- [x] `T-11` schema outputs include semantic-contract clarification note.
- [x] `T-12` import ordering tie-case remains deterministic with equal `created_at`.

## Risks and Mitigations

- [x] Risk: downstream consumers depend on prior parse-error plaintext despite `--json`.
  - Mitigation: contract tests + release note in plan closeout.
- [x] Risk: precision rejection blocks existing loose data pipelines.
  - Mitigation: explicit validation messages and docs guidance before commit flows.
- [x] Risk: duplicates query joins add subtle null/live edge cases.
  - Mitigation: targeted promotion/reversion tests with SQL assertions.
- [x] Risk: timestamp enrichment causes accidental contract drift.
  - Mitigation: additive-only shape + snapshot-style JSON assertions.

## Formal Acceptance Checklist

- [x] All targeted new tests pass.
- [x] Full test suite passes.
- [x] Lint/safety checks pass.
- [x] Rogue-rat replay confirms fixed behavior against SQL state.
- [x] No open `high` severity findings from this defect set remain.
- [x] Plan closeout executive summary completed.

## Executive Summary (closeout)

- Implemented parse-time `--json` error determinism in the CLI by inferring output mode from raw args and rendering JSON error envelopes for invalid-argument parse failures; command hints are now constrained to known command paths so fallback guidance stays executable.
- Added explicit stdin alias support for `import create --dry-run -` with clear empty-stdin guidance, documented `-` usage in help text, and surfaced mixed file+stdin conflicts in plaintext under a warnings section without duplicate warning lines.
- Remediated duplicate pointer staleness by expanding duplicate row contracts with historical-at-dedupe fields (`matched_*_at_dedupe`) while recalculating live `matched_*` pointers from active transactions (including post-undo promotions).
- Enforced amount precision guardrails with deterministic `invalid_amount_scale` issues for values beyond 2 decimal places, including scientific-notation and leading-dot forms that previously slipped through; added focused regression tests.
- Enriched `import list --json` rows with additive `timestamps.{created,committed,reverted}` objects containing `epoch_s`, UTC RFC3339, and local-friendly strings while preserving legacy timestamp fields and array top-level shape.
- Updated schema plaintext output (`schema`, `schema view`) to explicitly frame view metadata as the Driggsby semantic contract and clarify potential differences from SQLite physical `PRAGMA` metadata.
- Added/updated targeted and integration tests across CLI and client for all scoped defects, including deterministic tie-ordering when `created_at` values are equal.
- Verification and closeout: ran `cargo +stable test --all-features`, `just required-check`, reduced rogue-rat smoke replay in isolated `/tmp` homes (including SQL checks via `sqlite3`), follow-up focused review, and final hard gate `just rust-verify`.
