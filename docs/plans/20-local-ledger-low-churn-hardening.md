# 20 - Local Ledger Low-Churn Hardening

## Summary

This phase adds minimal, high-impact hardening for Driggsby local ledger storage.

Primary outcomes:
- enforce strict local filesystem permissions for ledger artifacts on Unix
- set pragmatic SQLite safety defaults with low implementation churn
- add focused tests to prevent regressions
- document what is protected now and what remains out of scope

## Why This Phase

Driggsby is local-first and handles financial transaction data.

Current behavior protects the ledger directory best-effort, but does not explicitly enforce private permissions on `ledger.db` and sidecar files. In practice this can leave sensitive data too readable (`0644` depending on host defaults). We also do not currently set `secure_delete`, which leaves deleted row remnants more recoverable.

This phase fixes those gaps without introducing major architectural change.

## Locked Decisions

- [x] Keep scope minimal and low churn.
- [x] Do not adopt SQLCipher in this phase.
- [x] Add Unix permission hardening for ledger files (`0600`) and ledger home (`0700`).
- [x] Treat permission-hardening failures as actionable errors on Unix (not silent best-effort).
- [x] Set connection PRAGMAs for writable connections: `foreign_keys=ON`, `secure_delete=ON`.
- [x] Avoid adding new CLI commands in this phase.

## Goals and Acceptance Criteria

- [ ] Ledger filesystem permissions are private by default on Unix.
  - [x] Acceptance: ledger home resolves to mode `0700`.
  - [x] Acceptance: `ledger.db` resolves to mode `0600`.
  - [x] Acceptance: sidecar files (`-wal`, `-shm`, `-journal`) are hardened to `0600` when present.

- [ ] Permission hardening failures are surfaced clearly.
  - [x] Acceptance: failed permission hardening maps to deterministic ledger init permission errors.

- [ ] SQLite writable connections apply low-churn safety defaults.
  - [x] Acceptance: writable `open_connection` enables `foreign_keys` and `secure_delete`.

- [ ] Behavior is covered by focused tests.
  - [x] Acceptance: tests validate Unix mode hardening and PRAGMA behavior.
  - [x] Acceptance: tests remain deterministic and do not require broad fixture changes.

- [ ] Documentation is explicit and agent-friendly.
  - [x] Acceptance: README and `docs/security.md` describe local ledger hardening defaults and current limits.

## Scope

- [x] Update `crates/driggsby-client/src/state.rs` to:
  - [x] enforce and verify directory permissions on Unix (`0700`)
  - [x] enforce and verify database file/sidecar permissions on Unix (`0600`)
  - [x] surface hardening errors instead of ignoring them
  - [x] apply writable connection PRAGMAs (`foreign_keys=ON`, `secure_delete=ON`)

- [x] Add/adjust tests under `crates/driggsby-client/tests/`:
  - [x] verify Unix ledger home + db mode expectations
  - [x] verify `secure_delete` is enabled on writable connections
  - [x] verify repeated initialization remains idempotent with hardening in place

- [x] Update docs:
  - [x] README local data model/security notes
  - [x] `docs/security.md` local ledger hardening section

## Out of Scope

- [ ] SQLCipher integration or encrypted-at-rest key management
- [ ] new `driggsby security status` command
- [ ] runtime policy broker for agent capability scoping

## TDD Execution Plan

### Step 1: Tests first (red)

- [x] add failing tests for Unix permission expectations
- [x] add failing test for `secure_delete` PRAGMA

### Step 2: Minimal implementation (green)

- [x] implement permission hardening and error handling in `state.rs`
- [x] implement PRAGMA setup in writable connection open path

### Step 3: Verify

- [x] run targeted tests
- [x] run `just rust-verify`

## Review and Closeout

- [x] perform final code review sweep for duplication/code smells
- [x] update this plan with completion checkboxes and executive summary
- [ ] commit with descriptive message and `Authored by:` footer

## Executive Summary

- Hardened Unix ledger storage permissions in the client state layer and removed silent best-effort chmod behavior.
- Added explicit file permission enforcement for `ledger.db` and SQLite sidecar files, plus explicit refusal of symlink ledger paths for safer permission handling.
- Added writable connection PRAGMA hardening (`foreign_keys=ON`, `secure_delete=ON`) to reduce integrity and remanence risk with minimal churn.
- Added focused setup tests for permission mode expectations and secure-delete behavior, keeping existing setup idempotence coverage intact.
- Updated README and `docs/security.md` so agent and human users can discover what local hardening is applied and what remains out of scope (not encrypted at rest yet).
