# 19 - Recurring Command: Deterministic Intelligence Core (Recurring-First, Anomalies-Ready)

## Summary

This phase implements `driggsby recurring` as a real, deterministic, precision-first intelligence command.

Primary outcomes:
- replace placeholder recurring output with real recurring classification over imported transactions
- introduce a shared `intelligence` module foundation that recurring uses now and anomalies can reuse next
- produce auditable recurring results with explicit evidence fields (cadence fit, amount fit, score, counts, expected next date)
- keep output deterministic and understandable for both agents and humans
- add a deep synthetic verification battery with fixtures created under `/tmp`

## Why This Phase

`recurring` is one of the highest-value first intelligence features in Driggsby.

Today, `recurring` is wired but placeholder-level. We need to make it reliable enough that an agent can trust first-shot output without reading source code, while keeping implementation simple, inspectable, and secure.

At the same time, this is the right point to establish a reusable intelligence architecture so future commands (`anomalies` and others) do not duplicate date parsing, normalization, query filtering, and testing patterns.

## Locked Decisions

- [x] Storage mode: compute recurring on demand from ledger transactions (no recurring materialization refresh pipeline in this phase).
- [x] Classifier bias: precision-first (minimize false positives).
- [x] Cadence scope (v1): `weekly`, `biweekly`, `monthly` only.
- [x] Shared architecture now: build reusable intelligence core used by recurring immediately and by anomalies later.
- [x] Keep CLI surface unchanged: `driggsby recurring [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--json]`.
- [x] Include additive recurring evidence fields in JSON output while preserving compatibility fields used by current text renderer.
- [x] Require strict and deterministic filtering/ordering behavior.
- [x] Add a required synthetic fixture battery under `/tmp` as part of implementation verification.

## Goals and Acceptance Criteria

- [ ] `driggsby recurring` returns real recurring classifications from imported transactions.
  - Acceptance: recurring rows include deterministic cadence and evidence fields.
  - Acceptance: results are stable across repeated runs on identical data.

- [ ] Classification is auditable and precision-first.
  - Acceptance: each row includes deterministic evidence metrics (`cadence_fit`, `amount_fit`, `score`, `occurrence_count`).
  - Acceptance: synthetic negative cases (frequent discretionary spend, weak merchant/description quality) do not over-classify.

- [ ] Output remains agent-friendly and human-readable.
  - Acceptance: plaintext output is concise but explicit about cadence, amount, and next expected date.
  - Acceptance: JSON output remains deterministic and machine-safe with stable types.

- [ ] Shared intelligence architecture is established for future commands.
  - Acceptance: recurring uses shared date/filter/normalization/query primitives.
  - Acceptance: anomalies command remains functional and can adopt the same primitives without refactor churn.

- [ ] Deep synthetic validation is implemented.
  - Acceptance: battery tests generate synthetic transaction files under `/tmp` and assert recurring classification correctness across positive/negative/edge scenarios.

## Scope

- [ ] Add shared client module tree for intelligence logic (query/filter/date/normalization + recurring detector).
- [ ] Implement recurring command logic end-to-end using shared intelligence primitives.
- [ ] Add recurring-specific data contracts/types (split from anomaly-shaped shared row struct).
- [ ] Update recurring plaintext and JSON output handling to include richer deterministic fields.
- [ ] Add unit, client integration, CLI contract, and synthetic battery tests.
- [ ] Add documentation notes for recurring behavior and intelligence architecture extension path.

## Out of Scope

- [ ] Full anomaly detection implementation.
- [ ] Quarterly/annual recurring cadence support.
- [ ] New persisted recurring materialization/recompute pipeline.
- [ ] Dashboard feature implementation changes.

## Public Interface / Contract Changes

### 1) Command surface

No path or flag changes:
- `driggsby recurring`
- `driggsby recurring --from <YYYY-MM-DD>`
- `driggsby recurring --to <YYYY-MM-DD>`
- `driggsby recurring --json`

### 2) Client-side validation behavior

- [ ] Validate `from` and `to` as real calendar dates (not just shape checks).
- [ ] Reject invalid ranges (`from > to`) with deterministic `invalid_argument` + recovery steps.

### 3) Recurring JSON row contract (additive)

Recurring rows will include (minimum):
- `group_key`
- `account_key`
- `merchant` (compat alias for counterparty label)
- `counterparty`
- `counterparty_source` (`merchant|description`)
- `cadence` (`weekly|biweekly|monthly`)
- `typical_amount`
- `currency`
- `first_seen_at`
- `last_seen_at`
- `next_expected_at`
- `occurrence_count`
- `cadence_fit`
- `amount_fit`
- `score`
- `amount_min`
- `amount_max`
- `sample_description`
- `quality_flags` (array of deterministic diagnostics)
- `is_active`

### 4) Plaintext recurring output

- [ ] Keep existing recurring heading and table flow.
- [ ] Ensure deterministic sorting and display.
- [ ] Add concise confidence context so agents/humans can understand why a row was classified recurring.

## Architecture and Design

### Workstream A - Shared intelligence foundation

- [ ] Create `crates/driggsby-client/src/intelligence/mod.rs`.
- [ ] Add shared submodules:
  - [ ] `types.rs` (shared normalized transaction and filter input types)
  - [ ] `date.rs` (strict parsing + cadence date math + month clamping)
  - [ ] `normalize.rs` (merchant + description normalization/fingerprint helpers)
  - [ ] `query.rs` (transaction loading + range filtering)
- [ ] Expose minimal clear interfaces for command modules.

### Workstream B - Recurring detector module

- [ ] Add `crates/driggsby-client/src/intelligence/recurring.rs`.
- [ ] Implement deterministic grouping key:
  - [ ] `account_key + currency + sign(amount) + counterparty_key`
  - [ ] merchant-first counterparty key, description-fingerprint fallback.
- [ ] Implement cadence hypotheses and scoring:
  - [ ] weekly: target 7d, tolerance +/-1d
  - [ ] biweekly: target 14d, tolerance +/-2d
  - [ ] monthly: month-step with day clamping, tolerance +/-3d
- [ ] Implement minimum-occurrence gates:
  - [ ] monthly >= 3
  - [ ] weekly/biweekly >= 4
- [ ] Implement precision-first thresholds:
  - [ ] `cadence_fit >= 0.75`
  - [ ] `score >= 0.78`
- [ ] Implement deterministic tie-break logic:
  - [ ] higher `cadence_fit`
  - [ ] lower median interval error
  - [ ] higher occurrence count
  - [ ] cadence priority `monthly > biweekly > weekly`
- [ ] Implement deterministic final row sort:
  - [ ] `next_expected_at` ASC (nulls last)
  - [ ] `score` DESC
  - [ ] `counterparty` ASC
  - [ ] `group_key` ASC

### Workstream C - Command integration

- [ ] Refactor `commands/recurring.rs` to thin-command orchestration over shared intelligence modules.
- [ ] Keep setup/init behavior and data range hint behavior consistent.
- [ ] Ensure `from/to` filters are applied exactly once at the shared query layer.

### Workstream D - Contracts and output integration

- [ ] Split existing shared intelligence row contract so recurring has dedicated typed payload structs.
- [ ] Keep anomalies command contract stable for now.
- [ ] Update recurring row normalization in CLI output layer for new evidence fields.
- [ ] Add/update JSON renderer expectations for recurring richer rows.
- [ ] Preserve empty-state contract (`No recurring patterns found.` when no rows).

### Workstream E - Anomalies readiness

- [ ] Route anomalies command through shared intelligence filter/query/date primitives where practical without changing anomaly behavior.
- [ ] Add comments/docs that define the standard pattern for new intelligence commands.
- [ ] Keep anomalies output/contract tests passing unchanged unless intentionally updated.

## Deterministic Classification Spec (v1)

### Input prefilter

- [ ] Ignore zero-amount rows.
- [ ] Ignore rows with invalid real calendar dates.
- [ ] Keep sign-separated streams (debits and credits never mix in one recurring group).

### Counterparty quality

- [ ] Merchant normalization:
  - [ ] trim + uppercase
  - [ ] non-alphanumeric collapsed to spaces
  - [ ] repeated spaces collapsed
- [ ] Description fallback fingerprint:
  - [ ] normalized description
  - [ ] remove numeric-only tokens
  - [ ] remove generic noise tokens (`POS`, `DEBIT`, `CARD`, `PURCHASE`, `ACH`, `ONLINE`, `PAYMENT`, etc.)
  - [ ] retain first N stable non-noise tokens
- [ ] Enforce minimum fallback fingerprint quality before classifying recurring.

### Fit metrics

- [ ] `cadence_fit = matched_intervals / total_intervals`
- [ ] `median_abs_amount = median(abs(amount))`
- [ ] `amount_tol = max(1.00, median_abs_amount * 0.15)`
- [ ] `amount_fit = in-tolerance-occurrences / total_occurrences`
- [ ] `score = 0.65*cadence_fit + 0.25*amount_fit + 0.10*counterparty_quality`

## Testing Plan (TDD + Synthetic Battery)

### 1) Unit tests (detector internals)

- [ ] date math and month-end clamping
- [ ] merchant normalization and description fallback fingerprinting
- [ ] grouping key determinism
- [ ] cadence-fit calculations
- [ ] amount-fit calculations
- [ ] score and threshold behavior
- [ ] tie-break determinism

### 2) Client integration tests

- [ ] add new recurring command integration tests under `crates/driggsby-client/tests/`
- [ ] verify from/to filtering behavior
- [ ] verify invalid date range errors
- [ ] verify deterministic ordering and field presence

### 3) CLI contract tests

- [ ] extend `crates/driggsby-cli/tests/contract_scaffold.rs` for recurring non-empty plaintext output
- [ ] extend recurring JSON contract assertions for evidence fields and types
- [ ] keep empty-state recurring behavior assertions
- [ ] verify recurring parse/argument errors return JSON envelope when `--json` is present

### 4) Required synthetic battery under `/tmp`

All synthetic fixtures are created under `/tmp` during tests (no repo-tracked fixture files required).

- [ ] Create dedicated synthetic battery test module (client and/or CLI level).
- [ ] Build helper to write synthetic JSON files under `/tmp/driggsby-recurring-*`.
- [ ] For each dataset, import via real import command path, then run recurring and assert expected outcomes.

Battery matrix (minimum required):
- [ ] monthly fixed amount (positive)
- [ ] monthly end-of-month clamp behavior (positive)
- [ ] monthly slight amount variance (positive)
- [ ] weekly fixed cadence (positive)
- [ ] weekly shifted by one day once (positive)
- [ ] biweekly fixed cadence (positive)
- [ ] biweekly with holiday shift (positive)
- [ ] merchant missing + strong description fingerprint (positive)
- [ ] merchant missing + weak generic descriptions (negative)
- [ ] opposite sign streams for same merchant (separation expected)
- [ ] same merchant across multiple currencies (separation expected)
- [ ] only two occurrences (negative)
- [ ] high amount volatility (negative)
- [ ] mixed frequent discretionary spend (negative)
- [ ] cadence switch within one group (negative unless one cadence strongly dominates)
- [ ] shuffled input order invariance (same deterministic output)
- [ ] from/to scoped window behavior

## Review, Verification, and Quality Gates

- [ ] Run targeted tests while iterating.
- [ ] Run full suite: `cargo test --all-features`.
- [ ] Run lint/safety gate: `just required-check`.
- [ ] Run Stage 1 `agentic_ux` review (primary + adversarial).
- [ ] Run Stage 2 `verification` review (primary + adversarial).
- [ ] Fix all `high_friction+` and `medium+` findings.
- [ ] Perform final sweep review and regression rerun.
- [ ] Run final Rust gate before implementation commit: `just rust-verify`.

## Risks and Mitigations

- [ ] Risk: false positives on frequent discretionary spend.
  - Mitigation: precision-first thresholds, quality gating, sign separation, min occurrence gates.

- [ ] Risk: date edge-case regressions (month ends/leap behavior).
  - Mitigation: strict date parsing and explicit month-clamp tests.

- [ ] Risk: intelligence code duplication as new commands arrive.
  - Mitigation: shared intelligence modules and explicit extension pattern docs in this phase.

- [ ] Risk: contract drift between client rows and CLI renderers.
  - Mitigation: contract tests in both client and CLI layers plus deterministic field/type assertions.

## Assumptions and Defaults

- [x] This phase is allowed to make breaking internal contract refactors where needed.
- [x] No back-compat preservation is required for undeployed greenfield surfaces.
- [x] Recurring on-demand computation is acceptable performance-wise for current project stage.
- [x] Precision-first recurring behavior is preferred for initial trust-building.

