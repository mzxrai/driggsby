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

- [ ] `--json` mode returns JSON-formatted errors for parse-time argument failures.
  - Acceptance: invalid `import keys uniq` property with `--json` yields JSON envelope, not plaintext prose.
- [ ] `import create --dry-run -` reads stdin as source.
  - Acceptance: piped input succeeds with `-`, empty stdin with `-` fails with explicit guidance.
- [ ] Plaintext output clearly surfaces mixed-source conflict (`file + stdin`).
  - Acceptance: plaintext includes explicit stdin-ignored warning message.
- [ ] `import duplicates` no longer returns stale non-existent active match IDs after undo/promotion.
  - Acceptance: live `matched_*` fields resolve to active rows when present; historical pointers remain available.
- [ ] Amount precision policy is enforced.
  - Acceptance: values over scale fail validation deterministically with row-level issue code.
- [ ] `import list --json` includes local-friendly + machine-safe timestamp structure.
  - Acceptance: each row includes `timestamps.{created,committed,reverted}` objects (or null) with `local`, `utc`, and `epoch_s`.
- [ ] Schema output clarifies semantic contract boundaries.
  - Acceptance: `schema` and `schema view` text explicitly state semantic-contract nature.

## Scope

- [ ] CLI parse-failure output mode handling for `--json`
- [ ] Import source resolution (`-` alias, stdin behavior, conflict communication)
- [ ] Duplicates API shape and SQL query semantics for live + historical pointers
- [ ] Amount scale validation policy
- [ ] Import list JSON timestamp enrichment
- [ ] Schema text clarity updates
- [ ] Regression tests for ordering, contracts, and edge cases

## Out of Scope

- [ ] Introducing a new global duplicate history command
- [ ] Back-compat preservation for old JSON consumers (greenfield allows breakage)
- [ ] Multi-currency minor-unit policy engine (this phase applies a global max scale rule)
- [ ] Large migration of timestamp storage format at DB level

## Public API / Interface Changes

### 1) `import duplicates` row shape expansion

- [ ] Add fields to `ImportDuplicateRow`:
  - `matched_txn_id_at_dedupe: Option<String>`
  - `matched_import_id_at_dedupe: Option<String>`
- [ ] Keep existing `matched_txn_id` / `matched_import_id` but redefine semantics as current live match pointers.

### 2) `import list --json` additive timestamp object

- [ ] For each row, add:
  - `timestamps.created`
  - `timestamps.committed`
  - `timestamps.reverted`
- [ ] Each non-null timestamp object includes:
  - `epoch_s` (integer)
  - `utc` (RFC3339 UTC string)
  - `local` (friendly local timezone string)

### 3) Validation issue contract for amount scale

- [ ] Add deterministic validation issue code for over-scale amounts (for example `invalid_amount_scale`).

## Architecture and Implementation Design

### Workstream A - Parse-time `--json` correctness

- [ ] Update parse-error handling in `crates/driggsby-cli/src/main.rs` to infer requested output mode from raw args when clap parsing fails.
- [ ] Update failure rendering in `crates/driggsby-cli/src/output/mod.rs` to allow JSON error output for `invalid_argument` parse failures when JSON mode is requested.
- [ ] Preserve existing plaintext fallback behavior when JSON mode is not requested.

### Workstream B - Source handling and visibility

- [ ] Update `crates/driggsby-client/src/import/input.rs`:
  - [ ] interpret `path == "-"` as stdin input
  - [ ] return explicit guidance if `-` is provided without stdin content
  - [ ] keep file-over-stdin precedence for non-`-` file paths
- [ ] Update plaintext render path in `crates/driggsby-cli/src/output/import_text.rs` to emit source-conflict warning from `warnings` payload.
- [ ] Update help text in `crates/driggsby-cli/src/cli.rs` to explicitly document `-` stdin usage.

### Workstream C - Duplicate pointer correctness after undo/promotion

- [ ] Extend duplicate row types in `crates/driggsby-client/src/contracts/types.rs`.
- [ ] Update `duplicates_with_options` query in `crates/driggsby-client/src/commands/import.rs`:
  - [ ] resolve live match via joins to active `internal_transactions`
  - [ ] preserve original `matched_*` values as historical-at-dedupe fields
  - [ ] retain deterministic ordering (`source_row_index`, `dedupe_reason`, `candidate_id`)
- [ ] Update duplicate projection logic in `crates/driggsby-client/src/import/mod.rs` as needed for preview consistency.
- [ ] Optionally enrich plaintext duplicate rendering to show “originally matched” when live vs historical differs.

### Workstream D - Amount precision guardrails

- [ ] Refactor validation in `crates/driggsby-client/src/import/validate.rs` so amount-scale checks can run with currency context.
- [ ] Enforce max 2 decimal places for this phase.
- [ ] Return row-level validation issues with deterministic code and clear expected/received messaging.
- [ ] Update import help copy in `crates/driggsby-cli/src/cli.rs` to state precision expectations.

### Workstream E - Timestamp contract enrichment for `import list --json`

- [ ] Keep DB/client row storage and query unchanged in `crates/driggsby-client/src/commands/import.rs`.
- [ ] Implement additive timestamp shaping in `crates/driggsby-cli/src/output/json.rs` (`render_import_list_json`):
  - [ ] parse epoch-like fields
  - [ ] produce `utc` + `local` + numeric `epoch_s`
  - [ ] keep existing fields (`created_at`, `committed_at`, `reverted_at`) for now
- [ ] Preserve local-friendly formatting consistency with `crates/driggsby-cli/src/output/import_text.rs`.

### Workstream F - Schema wording clarity

- [ ] Update `crates/driggsby-cli/src/output/schema_text.rs` to clearly label schema output as Driggsby semantic contract.
- [ ] Clarify that SQLite `PRAGMA table_info(view)` may differ for view physical metadata.

## TDD Execution Plan

### Step 0 - Red tests first

- [ ] Add failing tests for each bug fix before implementation:
  - [ ] parse-error JSON contract
  - [ ] `-` stdin alias behavior
  - [ ] plaintext source-conflict warning rendering
  - [ ] stale duplicate match pointer remediation
  - [ ] amount precision rejection
  - [ ] additive timestamp object in `import list --json`
  - [ ] schema wording clarity output checks

### Step 1 - Minimal implementation

- [ ] Implement workstreams A-F with smallest coherent code changes.

### Step 2 - Iterate to green

- [ ] Run focused tests and patch only defects that block intended behavior.

### Step 3 - Full verification

- [ ] `cargo +stable test --all-features`
- [ ] `just required-check`

### Step 4 - Rogue-rat replay

- [ ] Re-run a reduced ad-hoc CLI smoke matrix in isolated `/tmp` homes:
  - [ ] mixed source runs
  - [ ] JSON parse-failure runs
  - [ ] undo/promotion + duplicates inspection runs
  - [ ] schema/view wording checks
  - [ ] SQL verification via `schema` db path + `sqlite3`

### Step 5 - Final gate

- [ ] `just rust-verify`
- [ ] Update this plan with completion checkmarks and executive summary

## Test Matrix

- [ ] `T-01` `import keys uniq --json ACCOUNT_KEY` returns JSON error envelope.
- [ ] `T-02` `import create --dry-run -` with piped valid input succeeds.
- [ ] `T-03` `import create --dry-run -` with no stdin content fails with explicit guidance.
- [ ] `T-04` mixed stdin+file source surfaces warning in plaintext and JSON.
- [ ] `T-05` duplicate pointer after undo/promotion resolves to active live txn/import IDs.
- [ ] `T-06` duplicate row still includes historical-at-dedupe match IDs.
- [ ] `T-07` over-scale amount fails validation with deterministic issue code.
- [ ] `T-08` valid 2-decimal and integer amounts continue to pass.
- [ ] `T-09` `import list --json` rows include additive `timestamps.*` objects.
- [ ] `T-10` `import list` plaintext remains local-friendly and deterministic.
- [ ] `T-11` schema outputs include semantic-contract clarification note.
- [ ] `T-12` import ordering tie-case remains deterministic with equal `created_at`.

## Risks and Mitigations

- [ ] Risk: downstream consumers depend on prior parse-error plaintext despite `--json`.
  - Mitigation: contract tests + release note in plan closeout.
- [ ] Risk: precision rejection blocks existing loose data pipelines.
  - Mitigation: explicit validation messages and docs guidance before commit flows.
- [ ] Risk: duplicates query joins add subtle null/live edge cases.
  - Mitigation: targeted promotion/reversion tests with SQL assertions.
- [ ] Risk: timestamp enrichment causes accidental contract drift.
  - Mitigation: additive-only shape + snapshot-style JSON assertions.

## Formal Acceptance Checklist

- [ ] All targeted new tests pass.
- [ ] Full test suite passes.
- [ ] Lint/safety checks pass.
- [ ] Rogue-rat replay confirms fixed behavior against SQL state.
- [ ] No open `high` severity findings from this defect set remain.
- [ ] Plan closeout executive summary completed.

## Executive Summary (closeout)

- Pending implementation.
