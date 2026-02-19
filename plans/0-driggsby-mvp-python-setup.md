# Driggsby MVP (Python-First) — Project Setup & CLI Stubs

## Status
- [ ] Approved for implementation
- [x] Persisted to `plans/0-driggsby-mvp-python-setup.md`

## Summary
- [ ] Create standalone Python repo at `~/driggsby/skill/`
- [ ] Use `uv` + strict `pyright` + `pytest`
- [ ] Implement CLI stubs
- [ ] Implement real `init` that creates `~/.driggsby/ledger.db`
- [ ] Keep DB file-only for this phase (no tables yet)

## Locked Decisions
- [x] `init` is idempotent (`already initialized`, exit `0`)
- [x] Hardcoded DB path: `~/.driggsby/ledger.db`
- [x] `schema` emits placeholder JSON marked toy/dev
- [x] `import` reads stdin when `[FILE]` is omitted
- [x] Transactions flags use `--start` and `--end`
- [x] Date format is `YYYY-MM-DD`
- [x] `schema` placeholder shape is fixed
- [x] Normal output uses `stdout`, errors use `stderr`
- [x] `accounts`/`transactions` stubs return clear `no data yet` output

## Public CLI Interface
- [ ] `driggsby init`
- [ ] `driggsby schema`
- [ ] `driggsby import --format json [FILE]`
- [ ] `driggsby accounts`
- [ ] `driggsby transactions [--account TEXT] [--category TEXT] [--start YYYY-MM-DD] [--end YYYY-MM-DD]`

## Project Structure
- [ ] `skill/pyproject.toml`
- [ ] `skill/pyrightconfig.json`
- [ ] `skill/src/driggsby/__init__.py`
- [ ] `skill/src/driggsby/cli.py`
- [ ] `skill/src/driggsby/models.py`
- [ ] `skill/src/driggsby/schema.py`
- [ ] `skill/src/driggsby/db.py`
- [ ] `skill/src/driggsby/import_json.py`
- [ ] `skill/tests/conftest.py`
- [ ] `skill/tests/test_cli.py`
- [ ] `skill/docs/v1-proposal.md`
- [ ] `skill/.gitignore`

## Placeholder JSON Contract (`driggsby schema`)
- [ ] Output is valid JSON
- [ ] Includes toy marker
- [ ] Includes version marker
- [ ] Includes message
- [ ] Includes entities list

Example shape:

```json
{
  "toy": true,
  "version": "0.1.0-dev",
  "message": "Toy schema placeholder. Not production-ready.",
  "entities": []
}
```

## TDD Execution Checklist

### Phase 1 — Setup
- [ ] Initialize project with `uv` under `skill/`
- [ ] Add runtime dependencies: `click`, `pydantic`
- [ ] Add dev dependencies: `pytest`, `pytest-cov`, `pyright`
- [ ] Add script entrypoint: `driggsby = "driggsby.cli:main"`
- [ ] Configure strict pyright

### Phase 2 — Tests First
- [ ] Write CLI help/command discovery tests
- [ ] Write `init` creates DB file test
- [ ] Write `init` idempotency test
- [ ] Write `schema` placeholder JSON contract test
- [ ] Write `import` file input test
- [ ] Write `import` stdin input test
- [ ] Write `transactions --start/--end` test
- [ ] Run tests and confirm initial failure

### Phase 3 — Minimal Implementation
- [ ] Implement `cli.py` stubs
- [ ] Implement `init` directory + DB creation behavior
- [ ] Implement idempotent `init` status messaging
- [ ] Implement `schema` placeholder JSON output
- [ ] Implement stdin handling for `import`
- [ ] Implement basic date argument handling (`YYYY-MM-DD` expectation)

### Phase 4 — Verification
- [ ] `uv run pytest` passes
- [ ] `uv run pyright` is clean
- [ ] `uv run driggsby --help` shows all commands
- [ ] `uv run driggsby init` creates DB / reports already initialized
- [ ] `uv run driggsby schema` prints valid placeholder JSON

### Phase 5 — Repo Hygiene
- [ ] Update `skill/docs/v1-proposal.md` to Python-first direction
- [ ] Initialize git in `skill/`
- [ ] Add initial commit with clear message

## Acceptance Criteria
- [ ] New standalone repo exists at `~/driggsby/skill/`
- [ ] Strict typing and tests are configured and passing
- [ ] CLI surface exactly matches locked interface
- [ ] `init` behavior matches idempotent local DB contract
- [ ] No business logic/tables implemented yet
- [ ] Proposal doc exists at `skill/docs/v1-proposal.md`

## Out of Scope (This Phase)
- [ ] SQLite tables/indexes
- [ ] Deduplication/normalization logic
- [ ] Real import parsing
- [ ] Dashboard/API implementation
- [ ] Configurable DB path (`--db-path` / env var)
