# 15 - Import ID Unification + Agent-Safe Next Actions

## Summary

This phase simplifies `import create` success output so agents have one canonical identifier and one obvious safe default next move.

Primary outcomes:
- remove `undo_id` from `import create` output (JSON + plaintext)
- keep only `import_id` as the single canonical id
- add explicit next-action guidance that reduces accidental destructive behavior
- make duplicate preview wording unambiguous about whether it is full or truncated

This is a greenfield project and breaking changes are explicitly allowed.

## Why This Phase

- Live dynamic agent testing surfaced repeated first-shot failures from identifier/path confusion:
  - agents read wrong keys (`id`, `summary.import_id`) instead of `data.import_id`
  - agents skipped or mis-sequenced follow-up commands due to brittle extraction logic
- Current output includes both `import_id` and `undo_id`, even though they are currently the same value, which adds avoidable ambiguity.
- Output currently risks nudging lazy agents toward optional or destructive actions too early.

## Locked Decisions

- [x] `import_id` is the only post-import identifier in `import create` success output.
- [x] Remove `undo_id` from `import create` success output everywhere.
- [x] Keep plaintext-first UX; do not force `--json` in recommended follow-up commands.
- [x] Default immediate next step after successful commit import is:
  - `driggsby schema`
- [x] Follow-up commands are grouped under an explicit `Other actions` section in plaintext.
- [x] `Undo this import` remains visible but is explicitly marked `(destructive)`.
- [x] Show duplicates inspection action only when duplicates were actually flagged.
- [x] Duplicate preview wording must explicitly state whether it is full or truncated.
- [x] For dry-run success, primary next step is commit:
  - file input: `driggsby import create <path>`
  - stdin input: `driggsby import create`

## Goals and Acceptance Criteria

- [x] `import create` success payloads contain `data.import_id` and never `data.undo_id`.
  - Acceptance: contract tests assert `undo_id` absence for commit success.
- [x] Agents receive explicit, low-risk next actions without reading source.
  - Acceptance: output contains `Next step` + `Other actions` groupings with deterministic command strings.
- [x] Duplicate preview semantics are obvious at a glance.
  - Acceptance:
    - non-truncated wording: "showing all N duplicate rows"
    - truncated wording: "showing first 50 of M duplicate rows; truncated"
- [x] Duplicates follow-up action appears only when relevant.
  - Acceptance: no duplicates command/action when `duplicate_summary.total == 0`.
- [x] Dry-run guidance is action-oriented and non-destructive.
  - Acceptance: primary next step is commit command, no undo action for dry-run output.

## Scope

- [x] Update client contracts for `import create` success payload shape.
- [x] Update import execution assembly for next-action metadata.
- [x] Update plaintext rendering for import success output.
- [x] Update JSON rendering expectations/tests.
- [x] Update docs/examples that show `undo_id` in create success output.
- [x] Run full verification and refresh plan-13 dynamic scenario checks focused on U-03/U-05 failure class.

## Out of Scope

- [ ] Reworking `import undo` command semantics.
- [ ] Reworking `import duplicates` data model semantics.
- [ ] Broad CLI copy redesign outside import create success surfaces.
- [ ] Schema/view-level behavior changes.

## Public API / Interface Changes

### Breaking contract changes

- [x] Remove `undo_id` from `ImportData` (`import create` success).

### New success metadata

- [x] Add structured next-action metadata to `ImportData`:
  - `next_step` (single immediate safe default command)
  - `other_actions` (ordered optional commands with labels and risk hints)

Proposed JSON shape (illustrative):

```json
{
  "ok": true,
  "version": "v1",
  "data": {
    "import_id": "imp_...",
    "next_step": {
      "label": "Connect and query your data",
      "command": "driggsby schema"
    },
    "other_actions": [
      {
        "label": "View import list",
        "command": "driggsby import list"
      },
      {
        "label": "View duplicates",
        "command": "driggsby import duplicates imp_..."
      },
      {
        "label": "Undo this import (destructive)",
        "command": "driggsby import undo imp_...",
        "risk": "destructive"
      }
    ]
  }
}
```

Behavior rules:
- [x] Omit `"View duplicates"` action when `duplicate_summary.total == 0`.
- [x] For dry-run success:
  - `next_step.command = "driggsby import create <path>"` for file input, or `driggsby import create` for stdin input
  - no undo action.

## Plaintext Output Contract (Import Create)

- [x] Keep top summary concise and deterministic.
- [x] Show one identifier:
  - `Import ID: imp_...`
- [x] Remove `Undo ID:` line entirely.
- [x] Render clear preview scope wording:
  - non-truncated: `Duplicates Preview (showing all N duplicate rows):`
  - truncated: `Duplicates Preview (showing first 50 of M duplicate rows; truncated):`
- [x] Render action guidance in this structure:
  - `Next step:`
  - `Other actions:`
- [x] `Other actions` ordered for safety/readability:
  1. View import list
  2. View duplicates (conditional)
  3. Undo this import `(destructive)`

## Architecture and Implementation Design

### 1) Contracts/types update

- [x] Edit `crates/driggsby-client/src/contracts/types.rs`:
  - remove `undo_id` from `ImportData`
  - add typed next-action structs for deterministic serialization

### 2) Import execution output assembly

- [x] Edit `crates/driggsby-client/src/import/mod.rs`:
  - build `next_step` and `other_actions` for commit and dry-run paths
  - conditionally include duplicates action based on duplicate totals
  - preserve existing summary/duplicates payload data

### 3) Persistence plumbing cleanup

- [x] Edit `crates/driggsby-client/src/import/persist.rs` and related call sites:
  - remove unnecessary `undo_id` plumbing in create response path
  - keep undo functionality keyed by `import_id` unchanged

### 4) Plaintext renderer update

- [x] Edit `crates/driggsby-cli/src/output/import_text.rs`:
  - remove `Undo ID` rendering
  - render new preview scope wording
  - render `Next step` and `Other actions` sections with destructive label on undo

### 5) JSON-mode tests and scaffold updates

- [x] Update `crates/driggsby-cli/tests/contract_scaffold.rs`:
  - remove assertions that require `data.undo_id` on import create
  - add assertions for `next_step` and `other_actions` presence/shape
  - add duplicate-action conditional coverage

### 6) Client flow tests

- [x] Update `crates/driggsby-client/tests/import_flow.rs`:
  - remove equality assertions tying `import_id` and `undo_id`
  - assert only canonical `data.import_id`
  - assert dry-run vs commit next-action behavior

### 7) Docs/examples alignment

- [x] Update docs/spec examples under `docs/` and command examples that show both ids.
- [x] Ensure examples show plaintext `Other actions` style and explicit duplicate preview scope wording.

## TDD Execution Plan

### Step 0 - Red tests first

- [x] Write/update tests for:
  - `undo_id` removal from create success payload
  - next-action fields/sections
  - duplicates-action conditional visibility
  - preview scope wording clarity

### Step 1 - Minimal implementation

- [x] Implement type and assembly updates in client.
- [x] Implement renderer updates in CLI output layer.

### Step 2 - Iterate to green

- [x] Run targeted tests and fix only necessary implementation gaps.

### Step 3 - Full verification

- [x] `cargo test --all-features`
- [x] `just required-check`

### Step 4 - Agent-focused verification

- [x] Re-run dynamic agent scenarios focused on previous failure modes:
  - U-03/U-05 parsing/extraction path safety
  - verify first-shot behavior without source spelunking

### Step 5 - Final gate and closeout

- [x] `just rust-verify`
- [x] Update this plan with completion checkmarks and executive summary.
- [ ] Commit with descriptive message and required `Authored by:` footer.

## Test Matrix

- [x] Import create commit JSON success:
  - has `data.import_id`
  - does not have `data.undo_id`
  - has `next_step` and ordered `other_actions`
- [x] Import create commit plaintext:
  - shows only `Import ID`
  - no `Undo ID`
  - includes `Next step` and `Other actions`
  - undo line includes `(destructive)`
- [x] Duplicate preview wording:
  - non-truncated and truncated variants each asserted exactly
- [x] Conditional duplicates action:
  - present only when duplicates total > 0
- [x] Dry-run outputs:
  - next step is commit command
  - no undo action
- [x] Regression:
  - `import undo <id>` works unchanged
  - `import list` and `import duplicates` contracts remain deterministic

## Risks and Mitigations

- [x] Risk: downstream tests/tools rely on `undo_id` in create payload.
  - Mitigation: broad test updates and explicit breaking-change docs note.
- [x] Risk: action guidance still nudges destructive behavior.
  - Mitigation: demote undo into `Other actions` and require `(destructive)` label.
- [x] Risk: preview wording drift across branches.
  - Mitigation: exact string tests for both truncated/non-truncated variants.
- [x] Risk: contract expansion adds noise.
  - Mitigation: keep one immediate `next_step` and short ordered `other_actions`.

## Formal Acceptance Checklist

- [x] All targeted tests pass.
- [x] `cargo test --all-features` passes.
- [x] `just required-check` passes.
- [ ] Agent-focused rerun confirms U-03/U-05 class is prevented first-shot.
- [x] `just rust-verify` passes.
- [x] Plan closeout and executive summary completed.

## Executive Summary (closeout)

- Completed the import-id unification: `import create` success now exposes canonical `data.import_id` only, with `data.undo_id` fully removed from client contracts, response assembly, and CLI rendering/tests.
- Added structured, agent-safe next-action metadata to import-create success payloads:
  - `next_step` for one safe default
  - ordered `other_actions` with conditional duplicates action and explicit destructive risk for undo.
- Updated plaintext import-create UX to match agent-safe guidance:
  - one identifier (`Import ID`)
  - explicit duplicate preview scope wording (`showing all ...` vs `showing first ...; truncated`)
  - deterministic `Next step` / `Other actions` sections.
- Implemented dry-run next-step guidance that remains actionable in both source modes:
  - file dry-run -> `driggsby import create <path>`
  - stdin dry-run -> `driggsby import create`
  - no undo action for dry-run.
- Updated docs examples to remove stale `undo_id` references and align command/output examples with new action guidance.
- Verification completed successfully:
  - targeted red/green test cycle
  - `cargo test --all-features`
  - `just required-check`
  - `just rust-verify`.
- Re-ran live dynamic Plan 13 agent-lab in waves with one subagent per scenario and isolated homes; consolidated report written at:
  - `/Users/mbm-gsc/driggsby/tmp/plan13-live-agent-lab/run-live-agent-lab-20260226-104117/final_report.md`.
- Outcome note for next agent:
  - U-05 and U-06 improved to first-shot pass.
  - U-03 remained inconclusive due harness execution drift (subagent wrote to prior run root), so the formal U-03/U-05 “both prevented first-shot” confirmation remains open.
