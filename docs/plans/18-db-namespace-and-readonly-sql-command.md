# 18 - `db` Namespace + Read-Only SQL Command for Agents

## Summary

This phase introduces a dedicated database namespace and a first-class SQL query command for agents.

Primary outcomes:
- replace top-level `schema` command path with `db schema`
- add `db sql` for executing read-only SQL queries against Driggsby public views
- enforce strict read-only + public-view-only query rules at runtime
- provide deterministic plaintext default output and machine-safe JSON opt-in for `db sql`

This is a greenfield project and breaking changes are explicitly allowed.

## Why This Phase

Current guidance tells agents to leave the CLI and use `sqlite3` directly for custom SQL. That creates avoidable friction and inconsistent safety behavior.

A dedicated `db` namespace with a built-in query command gives agents:
- better command discoverability
- one trusted execution path
- consistent error handling and output contracts
- safer defaults for financial data access

## Locked Decisions

- [x] Introduce `driggsby db` namespace for database discovery/query workflows.
- [x] Hard-break top-level `driggsby schema` immediately (no alias period).
- [x] `driggsby db schema` must preserve the existing schema summary/view behavior and prose style.
- [x] Add `driggsby db sql` as read-only query surface.
- [x] SQL input modes: inline arg, `--file <path>`, and stdin via `--file -`.
- [x] SQL source conflicts are rejected deterministically (no implicit precedence).
- [x] Default output for `db sql` is plaintext; JSON is opt-in via `--json`.
- [x] `db sql --json` returns a `columns + rows` object (no `ok/version` envelope).
- [x] Runtime enforcement is mandatory: single statement, read-only statement, public-view-only access.
- [x] Query path must not permit access to `internal_*` tables or SQLite administration surfaces.

## Goals and Acceptance Criteria

- [x] Agents can discover DB surfaces via `driggsby db --help` and execute SQL in <=3 calls.
  - Acceptance: top-level help and db help explicitly show `db schema` and `db sql` workflows.
  - Acceptance: `driggsby db schema` includes ready-to-run query examples.

- [x] Existing schema functionality is preserved under `db schema`.
  - Acceptance: `driggsby db schema` output remains data-access-focused and semantically equivalent to prior output.
  - Acceptance: `driggsby db schema view <name>` continues to return the same contract for known views.

- [x] `driggsby db sql` provides secure read-only query execution.
  - Acceptance: valid SELECT queries against `v1_*` views succeed.
  - Acceptance: writes, non-readonly statements, multi-statement input, PRAGMA/ATTACH, and internal-object reads fail with deterministic errors.

- [x] `db sql` output is deterministic and machine-friendly.
  - Acceptance: plaintext output includes query summary + tabular results.
  - Acceptance: JSON output returns stable `columns`, `rows`, `row_count`, `truncated`, and source metadata.

- [x] Breaking change to old path is explicit and tested.
  - Acceptance: `driggsby schema` fails with deterministic `invalid_argument` guidance that points to `driggsby db schema`.

## Scope

- [x] CLI command tree migration from top-level `schema` to `db schema`
- [x] New CLI command: `db sql`
- [x] Client command module for SQL execution
- [x] Read-only connection helper and SQL safety checks
- [x] SQL result contract types
- [x] Plaintext and JSON renderers for SQL results
- [x] Help text and command-hint updates
- [x] Contract tests + security-focused client tests

## Out of Scope

- [ ] Arbitrary SQL write support
- [ ] Query plan/perf optimization features (EXPLAIN, hints, caching)
- [ ] New public view definitions or schema migrations for this phase
- [ ] Backward compatibility for `driggsby schema`

## Public API / Interface Changes

### 1) New `db` command family

CLI surface:
- `driggsby db schema`
- `driggsby db schema view <view_name>`
- `driggsby db sql <query>`
- `driggsby db sql --file <path>`
- `driggsby db sql --file -` (stdin)

### 2) Removed top-level schema path

- `driggsby schema` is removed.
- `driggsby schema view <name>` is removed.

Parse/runtime guidance for removed path:
- deterministic `invalid_argument` with recovery steps:
  - `Run \`driggsby db schema\` for DB discovery.`
  - `Run \`driggsby db --help\` for DB command usage.`

### 3) `db sql` output contracts

#### Plaintext (default)

Shape requirements:
- opening success sentence
- summary block with row counts/truncation/source mode
- deterministic table/row-block rendering using existing formatting utilities
- explicit message when zero rows returned

#### JSON (`--json`)

```json
{
  "columns": [
    { "name": "account_key", "type": "text", "nullable": false },
    { "name": "txn_count", "type": "integer", "nullable": false }
  ],
  "rows": [
    ["chase_checking_1234", 184],
    ["amex_gold_9999", 122]
  ],
  "row_count": 2,
  "truncated": false,
  "max_rows": 1000,
  "source": "inline"
}
```

Contract rules:
- preserve column order exactly as returned by SQLite
- `rows` are positional arrays aligned to `columns`
- scalar JSON values only (`null`, boolean, number, string)
- no top-level envelope for success JSON

### 4) `--json` policy

- `db sql --json` is supported.
- `db schema --json` and `db schema view ... --json` remain unsupported (retain current schema JSON policy posture).

## Security Model for `db sql`

### Enforcement Layers (all required)

- [x] Input guardrails:
  - non-empty SQL required
  - max SQL length enforced
  - reject NUL bytes

- [x] Source guardrails:
  - exactly one input source (inline OR file OR stdin)
  - deterministic invalid_argument on conflicts/missing source

- [x] Connection guardrails:
  - open dedicated read-only SQLite connection (`SQLITE_OPEN_READ_ONLY`)
  - apply busy-timeout and existing sqlite error mapping

- [x] Statement guardrails:
  - single statement only (reject multi-statement)
  - require `stmt.readonly() == true`

- [x] Object-access guardrails:
  - enforce public-view-only reads (`v1_*` allowlist from canonical contract)
  - deny internal tables and SQLite administration/DDL/DML surfaces
  - implement using SQLite authorizer hook via `rusqlite` hooks feature

- [x] Output guardrails:
  - bounded row return count with truncation flag
  - default cap `1000`, hard max `10000`

## Error Contract Behavior

Use existing unified error envelope logic.

`db sql` user-facing failures should map to deterministic `invalid_argument` variants with explicit next steps, including:
- missing SQL
- conflicting SQL sources
- multi-statement query
- non-readonly statement
- forbidden object access
- malformed SQL syntax

System failures continue using existing mapped codes (`ledger_locked`, `ledger_corrupt`, etc.).

## Architecture and Implementation Design

### Workstream A - CLI command tree and parsing

- [x] Add `Db` command family in `crates/driggsby-cli/src/cli.rs`.
- [x] Add `DbCommand::Schema` and `DbCommand::Sql` subcommands.
- [x] Add parser tests for full `db` path matrix.
- [x] Remove legacy parse paths for top-level `schema`.

### Workstream B - CLI dispatch + mode routing

- [x] Route `db schema` and `db schema view` to existing schema client commands.
- [x] Route `db sql` to new client SQL command.
- [x] Extend output mode router for `db sql --json`.
- [x] Keep parse-time JSON error inference behavior consistent.

### Workstream C - Client SQL execution module

- [x] Add `crates/driggsby-client/src/commands/sql.rs` (or `db_sql.rs`) with single responsibility.
- [x] Resolve SQL source, validate source rules, and normalize query text.
- [x] Execute query through strict safety pipeline.
- [x] Return typed SQL result contract.

### Workstream D - Read-only connection + authorizer guardrails

- [x] Add read-only connection helper in `crates/driggsby-client/src/state.rs`.
- [x] Enable and wire authorizer-based allow/deny logic.
- [x] Use canonical public view allowlist from shared contract source.

### Workstream E - Output rendering

- [x] Add `crates/driggsby-cli/src/output/sql_text.rs` for plaintext SQL results.
- [x] Wire `sql` branch in `crates/driggsby-cli/src/output/mod.rs`.
- [x] Add `sql` JSON branch in `crates/driggsby-cli/src/output/json.rs`.
- [x] Ensure renderer gracefully handles empty results and mixed value types.

### Workstream F - Help text and command guidance updates

- [x] Update root/top-level help copy in `crates/driggsby-cli/src/main.rs` to reference `driggsby db schema`.
- [x] Update schema-rendered “inspect/detail” guidance commands to `driggsby db schema view <name>`.
- [x] Update command hint mapping in `main.rs` for `db`, `db schema`, `db schema view`, and `db sql`.
- [x] Update any recovery-step command strings that currently point to `driggsby schema`.

### Workstream G - Tests and hardening

- [x] Add/adjust CLI contract tests in `crates/driggsby-cli/tests/contract_scaffold.rs`.
- [x] Add client-level SQL safety tests under `crates/driggsby-client/tests/`.
- [x] Add unit tests for authorizer rules and input-source resolution.
- [x] Add regression tests to prove no-write behavior for sql command path.

## TDD Execution Plan

### Step 0 - Red tests first

- [x] Add failing parse/contract tests for new `db` command paths.
- [x] Add failing SQL safety tests (allowed + blocked query classes).
- [x] Add failing tests for removed top-level `schema` path behavior.

### Step 1 - Minimal implementation

- [x] Implement CLI tree migration and dispatch routing.
- [x] Implement client SQL module with minimal safe execution path.
- [x] Implement SQL output renderers and mode wiring.

### Step 2 - Iterate to green

- [x] Run targeted test subsets and fix implementation defects.
- [x] Ensure behavior is deterministic across text and JSON output.

### Step 3 - Full verification

- [x] Run `cargo test --all-features`.
- [x] Run `just required-check`.

### Step 4 - Multi-agent review loops

- [x] Stage 1 `agentic_ux` review (primary + adversarial) for public interface quality.
- [x] Stage 2 `verification` review (primary + adversarial) for defects/security/maintainability.
- [x] Fix all `high_friction+` UX issues and `medium+` verification issues.

### Step 5 - Final gates and closeout

- [x] Run final sweep review with 1-2 subagents.
- [x] Run smoke checks with real CLI invocations and representative SQL queries.
- [x] Update this plan file with completed checkboxes + executive summary.
- [x] Run `just rust-verify`.

## Test Matrix

- [x] `T-01` `driggsby db schema` returns expected plaintext schema summary.
- [x] `T-02` `driggsby db schema view v1_transactions` returns expected plaintext detail output.
- [x] `T-03` `driggsby schema` fails with deterministic `invalid_argument` + migration guidance.
- [x] `T-04` `driggsby db sql \"SELECT * FROM v1_transactions LIMIT 1\"` succeeds.
- [x] `T-05` `driggsby db sql --file query.sql` succeeds.
- [x] `T-06` `driggsby db sql --file -` with piped SQL succeeds; empty stdin fails clearly.
- [x] `T-07` `db sql` missing input source fails deterministically.
- [x] `T-08` `db sql` conflicting sources (inline + file, file + stdin) fail deterministically.
- [x] `T-09` multi-statement SQL fails.
- [x] `T-10` non-readonly statement (`INSERT/UPDATE/DELETE`) fails.
- [x] `T-11` direct internal-table query fails.
- [x] `T-12` PRAGMA/ATTACH/DETACH style statements fail.
- [x] `T-13` CTE over allowed `v1_*` views succeeds.
- [x] `T-14` `db sql --json` returns `columns + rows` contract with deterministic ordering.
- [x] `T-15` row cap behavior is deterministic (`truncated` true when cap exceeded).
- [x] `T-16` query path is no-write (pre/post row counts unchanged).
- [x] `T-17` parse/runtime errors with `--json` return universal JSON error shape.

## Risks and Mitigations

- [x] Risk: hard-breaking `schema` may surprise existing agent scripts.
  - Mitigation: explicit error guidance and top-level help updates pointing to `db schema`.

- [x] Risk: SQL allow/deny logic may accidentally over-block legitimate read queries.
  - Mitigation: broaden positive tests (including CTEs/aliases/subqueries) before finalizing allowlist decisions.

- [x] Risk: SQL safety checks drift from public view contract source-of-truth.
  - Mitigation: derive allowlist from shared canonical view contracts rather than duplicate constants.

- [x] Risk: large query outputs overwhelm agents.
  - Mitigation: enforce bounded row cap with explicit truncation metadata and clear guidance to add `LIMIT`.

## Formal Acceptance Checklist

- [x] New `db` command tree is fully wired and documented in help output.
- [x] `db schema` functionality parity is preserved.
- [x] `db sql` supports inline/file/stdin with deterministic source resolution.
- [x] SQL execution is proven read-only and public-view-only by tests.
- [x] Plaintext and JSON contracts for `db sql` are stable and covered.
- [x] Legacy `schema` path removal behavior is explicit and user-guided.
- [x] Required checks/tests pass and review findings are resolved.

## Assumptions and Defaults

- [x] Default SQL result cap is `1000` rows; hard cap is `10000`.
- [x] SQL text length cap is `65536` bytes.
- [x] `db schema` remains plaintext-only in this phase.
- [x] No DB migration is required; this phase is command/output/security-layer work.

## Executive Summary
- Migrated the CLI from top-level `schema` to a dedicated `db` namespace, with `db schema`, `db schema view`, and the new `db sql` command wired end-to-end through parsing, dispatch, output mode routing, and help text.
- Implemented `db sql` with strict guardrails: single-source SQL resolution (inline/file/stdin), input validation, read-only SQLite connection, single-statement + readonly enforcement, authorizer-backed object/function restrictions, and bounded row output with truncation metadata.
- Hardened security after adversarial review by adding canonical required-view SQL verification during setup, preventing tampered view definitions from being used to exfiltrate `internal_*` data through `db sql`.
- Stabilized output contracts for agent consumption: plaintext summary/table rendering and raw JSON payload (`columns`, `rows`, `row_count`, `truncated`, `max_rows`, `source`, optional `source_ref`) without success envelopes.
- Improved agent UX around common failure paths, including deterministic migration guidance for removed `driggsby schema` and actionable recovery text when inline SQL is not quoted correctly.
- Added and expanded tests across CLI and client layers, including safety regressions (no-write behavior, tampered-view rejection, malformed/user SQL error mapping, metadata consistency), then re-ran full verification gates (`cargo test --all-features`, `just required-check`, `just rust-verify`) successfully.
- Follow-up reviewer pass confirmed prior `high_friction+` and `medium+` findings were resolved; one deliberate decision remains unchanged by plan design: `db schema --json` stays unsupported in this phase.
