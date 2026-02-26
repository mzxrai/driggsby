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
  - `driggsby import create <path>`

## Goals and Acceptance Criteria

- [ ] `import create` success payloads contain `data.import_id` and never `data.undo_id`.
  - Acceptance: contract tests assert `undo_id` absence for commit success.
- [ ] Agents receive explicit, low-risk next actions without reading source.
  - Acceptance: output contains `Next step` + `Other actions` groupings with deterministic command strings.
- [ ] Duplicate preview semantics are obvious at a glance.
  - Acceptance:
    - non-truncated wording: "showing all N duplicate rows"
    - truncated wording: "showing first 50 of M duplicate rows; truncated"
- [ ] Duplicates follow-up action appears only when relevant.
  - Acceptance: no duplicates command/action when `duplicate_summary.total == 0`.
- [ ] Dry-run guidance is action-oriented and non-destructive.
  - Acceptance: primary next step is commit command, no undo action for dry-run output.

## Scope

- [ ] Update client contracts for `import create` success payload shape.
- [ ] Update import execution assembly for next-action metadata.
- [ ] Update plaintext rendering for import success output.
- [ ] Update JSON rendering expectations/tests.
- [ ] Update docs/examples that show `undo_id` in create success output.
- [ ] Run full verification and refresh plan-13 dynamic scenario checks focused on U-03/U-05 failure class.

## Out of Scope

- [ ] Reworking `import undo` command semantics.
- [ ] Reworking `import duplicates` data model semantics.
- [ ] Broad CLI copy redesign outside import create success surfaces.
- [ ] Schema/view-level behavior changes.

## Public API / Interface Changes

### Breaking contract changes

- [ ] Remove `undo_id` from `ImportData` (`import create` success).

### New success metadata

- [ ] Add structured next-action metadata to `ImportData`:
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
        "label": "Undo this import",
        "command": "driggsby import undo imp_...",
        "risk": "destructive"
      }
    ]
  }
}
```

Behavior rules:
- [ ] Omit `"View duplicates"` action when `duplicate_summary.total == 0`.
- [ ] For dry-run success:
  - `next_step.command = "driggsby import create <path>"`
  - no undo action.

## Plaintext Output Contract (Import Create)

- [ ] Keep top summary concise and deterministic.
- [ ] Show one identifier:
  - `Import ID: imp_...`
- [ ] Remove `Undo ID:` line entirely.
- [ ] Render clear preview scope wording:
  - non-truncated: `Duplicates Preview (showing all N duplicate rows):`
  - truncated: `Duplicates Preview (showing first 50 of M duplicate rows; truncated):`
- [ ] Render action guidance in this structure:
  - `Next step:`
  - `Other actions:`
- [ ] `Other actions` ordered for safety/readability:
  1. View import list
  2. View duplicates (conditional)
  3. Undo this import `(destructive)`

## Architecture and Implementation Design

### 1) Contracts/types update

- [ ] Edit `crates/driggsby-client/src/contracts/types.rs`:
  - remove `undo_id` from `ImportData`
  - add typed next-action structs for deterministic serialization

### 2) Import execution output assembly

- [ ] Edit `crates/driggsby-client/src/import/mod.rs`:
  - build `next_step` and `other_actions` for commit and dry-run paths
  - conditionally include duplicates action based on duplicate totals
  - preserve existing summary/duplicates payload data

### 3) Persistence plumbing cleanup

- [ ] Edit `crates/driggsby-client/src/import/persist.rs` and related call sites:
  - remove unnecessary `undo_id` plumbing in create response path
  - keep undo functionality keyed by `import_id` unchanged

### 4) Plaintext renderer update

- [ ] Edit `crates/driggsby-cli/src/output/import_text.rs`:
  - remove `Undo ID` rendering
  - render new preview scope wording
  - render `Next step` and `Other actions` sections with destructive label on undo

### 5) JSON-mode tests and scaffold updates

- [ ] Update `crates/driggsby-cli/tests/contract_scaffold.rs`:
  - remove assertions that require `data.undo_id` on import create
  - add assertions for `next_step` and `other_actions` presence/shape
  - add duplicate-action conditional coverage

### 6) Client flow tests

- [ ] Update `crates/driggsby-client/tests/import_flow.rs`:
  - remove equality assertions tying `import_id` and `undo_id`
  - assert only canonical `data.import_id`
  - assert dry-run vs commit next-action behavior

### 7) Docs/examples alignment

- [ ] Update docs/spec examples under `docs/` and command examples that show both ids.
- [ ] Ensure examples show plaintext `Other actions` style and explicit duplicate preview scope wording.

## TDD Execution Plan

### Step 0 - Red tests first

- [ ] Write/update tests for:
  - `undo_id` removal from create success payload
  - next-action fields/sections
  - duplicates-action conditional visibility
  - preview scope wording clarity

### Step 1 - Minimal implementation

- [ ] Implement type and assembly updates in client.
- [ ] Implement renderer updates in CLI output layer.

### Step 2 - Iterate to green

- [ ] Run targeted tests and fix only necessary implementation gaps.

### Step 3 - Full verification

- [ ] `cargo test --all-features`
- [ ] `just required-check`

### Step 4 - Agent-focused verification

- [ ] Re-run dynamic agent scenarios focused on previous failure modes:
  - U-03/U-05 parsing/extraction path safety
  - verify first-shot behavior without source spelunking

### Step 5 - Final gate and closeout

- [ ] `just rust-verify`
- [ ] Update this plan with completion checkmarks and executive summary.
- [ ] Commit with descriptive message and required `Authored by:` footer.

## Test Matrix

- [ ] Import create commit JSON success:
  - has `data.import_id`
  - does not have `data.undo_id`
  - has `next_step` and ordered `other_actions`
- [ ] Import create commit plaintext:
  - shows only `Import ID`
  - no `Undo ID`
  - includes `Next step` and `Other actions`
  - undo line includes `(destructive)`
- [ ] Duplicate preview wording:
  - non-truncated and truncated variants each asserted exactly
- [ ] Conditional duplicates action:
  - present only when duplicates total > 0
- [ ] Dry-run outputs:
  - next step is commit command
  - no undo action
- [ ] Regression:
  - `import undo <id>` works unchanged
  - `import list` and `import duplicates` contracts remain deterministic

## Risks and Mitigations

- [ ] Risk: downstream tests/tools rely on `undo_id` in create payload.
  - Mitigation: broad test updates and explicit breaking-change docs note.
- [ ] Risk: action guidance still nudges destructive behavior.
  - Mitigation: demote undo into `Other actions` and require `(destructive)` label.
- [ ] Risk: preview wording drift across branches.
  - Mitigation: exact string tests for both truncated/non-truncated variants.
- [ ] Risk: contract expansion adds noise.
  - Mitigation: keep one immediate `next_step` and short ordered `other_actions`.

## Formal Acceptance Checklist

- [ ] All targeted tests pass.
- [ ] `cargo test --all-features` passes.
- [ ] `just required-check` passes.
- [ ] Agent-focused rerun confirms U-03/U-05 class is prevented first-shot.
- [ ] `just rust-verify` passes.
- [ ] Plan closeout and executive summary completed.
