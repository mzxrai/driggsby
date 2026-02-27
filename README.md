# Driggsby

Driggsby is a local-first personal finance data and intelligence layer for coding agents (and humans).

It gives you:
- a stable local ledger database
- an import pipeline for normalized transaction files
- a CLI that is explicit and machine-friendly
- public SQL views (`v1_*`) you can query directly

## Current State (February 27, 2026)

Implemented and usable now:
- Local ledger setup + migrations (`SQLite`, auto-initialized on first command)
- Transaction import flow with dry-run validation:
  - `import create`
  - `import list`
  - `import duplicates`
  - `import undo`
  - `import keys uniq`
- Account orientation command:
  - `account list`
- Schema discovery commands:
  - `db schema`
  - `db schema view <name>`
- Dashboard/demo URL commands:
  - `dash`
  - `demo dash`
  - `demo recurring`
  - `demo anomalies`
  - these commands currently return local URLs (default `http://127.0.0.1:8787...`) and assume a dashboard runtime is available there

Implemented intelligence command:
- `recurring` and `anomalies` are SQL-backed intelligence commands
- both commands read from materialized intelligence tables/views (`v1_recurring`, `v1_anomalies`)
- intelligence materialization refreshes automatically on committed `import create` and successful `import undo`
- hidden maintenance escape hatch is available: `driggsby intelligence refresh`
- policy versions are explicit (`recurring/v1`, `anomalies/v1`)
- recurring rows are intentionally concise (`group_key`, `merchant`, cadence/amount/timing/score fields)
- anomaly rows are intentionally concise (`txn_id`, timing, amount, reason, severity, score fields)

## Quick Start

### 1) Build and run

```bash
cargo run -p driggsby-cli -- --help
```

Or install a local binary:

```bash
cargo install --path crates/driggsby-cli
driggsby --help
```

### 2) First-run workflow

```bash
# Show local DB path + semantic query contract
driggsby db schema

# Read the import contract and examples
driggsby import create --help

# Validate import file without writing
driggsby import create --dry-run /path/to/normalized.json

# Commit import
driggsby import create /path/to/normalized.json

# Verify ledger orientation
driggsby account list

# Optional maintenance: force intelligence rebuild
driggsby intelligence refresh
```

## Import Contract (Normalized Input)

Driggsby imports **normalized JSON or CSV** (not raw bank exports).  
Run `driggsby import create --help` for the full contract and examples.

Required fields:
- `account_key`
- `posted_at` (`YYYY-MM-DD`)
- `amount` (numeric, max 2 decimal places)
- `currency` (ISO-3 like `USD`)
- `description`

Optional fields:
- `account_type` (recommended)
- `statement_id`
- `external_id`
- `merchant`
- `category`

## JSON Output Mode

`--json` is currently supported on:
- `account list`
- `import create`
- `import list`
- `import duplicates`
- `import keys uniq`
- `import undo`
- `recurring`
- `anomalies`

## Local Data Model

Default ledger path:
- `~/.driggsby/ledger.db`

Override ledger home:
- set `DRIGGSBY_HOME`

Public semantic views:
- `v1_transactions`
- `v1_accounts`
- `v1_imports`
- `v1_recurring`
- `v1_anomalies`

## Development

Useful commands:

```bash
# Lint/safety gate
just required-check

# Full Rust gate (fmt + clippy + tests + build)
just rust-verify

# Full tests
cargo test --all-features
```

## Docs

- Security automation: [`docs/security.md`](docs/security.md)
- Plaid architecture notes: [`docs/plaid.md`](docs/plaid.md)
- Recent implementation plans: [`docs/plans/`](docs/plans/)
