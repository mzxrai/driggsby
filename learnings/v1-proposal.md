# Driggsby V1 Proposal (Python-First, Agent-Oriented)

Last updated: 2026-02-20

## What Driggsby Is

Driggsby is a local finance data layer for agents.

- The agent handles parsing and interpretation.
- Driggsby handles persistence, schema, validation, and consistency.
- Data stays local in SQLite (`~/.driggsby/ledger.db`).

## Current Direction (Updated)

Driggsby v1 is now Python-first (not Rust-first).

```
driggsby (Python package)
  ├── CLI commands          init, schema, import, accounts, transactions
  ├── SQLite ledger         ~/.driggsby/ledger.db
  ├── Strict typing         pyright (strict)
  └── Tests                 pytest
```

No embedded dashboard is in v1 scope right now.

## Target User

A developer already using an agent (Codex, Claude Code, etc.) who wants a stable local ledger that survives across sessions.

## Skill Model (How Imports Work)

The user’s agent is responsible for turning statements into JSON. Driggsby does not parse PDFs itself.

```
statement file(s)
   ↓
agent writes/runs parser code
   ↓
structured JSON payload
   ↓
driggsby import --dry-run   (validate + normalize + preview)
   ↓
driggsby import             (write path, next phase)
```

### Why this split

- Keeps Driggsby deterministic and simple.
- Keeps LLM-specific logic out of the core CLI.
- Makes schema and validation behavior stable over time.

## V1 Data Model Direction

Core tables in scope:

- `accounts`
- `imports`
- `transactions`
- `source_account_links`
- `schema_migrations`

Recent v1 decisions:

- Removed transfer-only fields from `transactions` for now.
- Added source identity tracking (`source_provider`, `source_account_ref`).
- Added provider normalization + deterministic dedupe fingerprinting.

## CLI Direction

Current command surface:

- `driggsby init`
- `driggsby schema`
- `driggsby import --format json [FILE|-] [--dry-run]`
- `driggsby accounts`
- `driggsby transactions [--account] [--category] [--start] [--end]`

Current import behavior:

- `--dry-run` validates payloads and returns structured JSON preview.
- non-dry-run import remains placeholder for now (full insert pipeline is next phase).

## Boundaries (Important)

Driggsby does:

- Stable schema + migration control
- Validation contract for imports
- Source identity normalization
- Deterministic dedupe support
- Local persistence primitives

Driggsby does not do (in v1):

- Native PDF parsing
- LLM calling/orchestration
- Financial advice
- Full dashboard/web app

## Distribution Direction

Short-term: Python package and CLI workflow via `uv`/local dev.

Long-term packaging (brew, binaries, etc.) is deferred until core import + ledger behavior is complete and stable.

## Immediate Next Milestones

1. Implement non-dry-run import write path.
2. Persist imports/transactions using the same validated contract.
3. Apply normalization consistently on write.
4. Add safe dedupe behavior during insert.
5. Expand query/read commands beyond placeholders.
