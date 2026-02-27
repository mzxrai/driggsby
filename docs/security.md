# Security Automation

This repo uses a layered security model:

1. Local git hooks to block risky commits before they leave your machine.
2. CI security workflows to catch issues in PRs and on `main`.
3. Dependabot to open update PRs automatically.

## Local Setup

Run once on macOS:

```bash
just security-bootstrap-macos
```

That will:

1. Install `gitleaks`, `cargo-deny`, and `trivy`.
2. Configure git hooks via `core.hooksPath=.githooks`.

Run a full local security scan:

```bash
just security-local
```

## Hooks

`pre-commit`:

1. Runs `gitleaks` on staged changes.

`pre-push`:

1. Runs `gitleaks` across git history.
2. Runs `cargo-deny check advisories`.

## CI Workflows

1. `Gitleaks` (`.github/workflows/gitleaks.yml`):
   1. Scans git history.
   2. Scans current working tree.
2. `Security Suite` (`.github/workflows/security.yml`):
   1. Dependency Review action on PRs.
   2. `cargo-deny` advisories scan.
   3. Trivy filesystem scan (vulns + secrets + misconfig).
3. `CodeQL` (GitHub default setup in repository settings):
   1. Rust static analysis.

All GitHub Actions are pinned to full commit SHAs.

## Dependabot

`/.github/dependabot.yml` enables weekly update PRs for:

1. Cargo dependencies.
2. GitHub Actions dependencies.

## Local Ledger Hardening

Driggsby also applies local runtime hardening for ledger storage.

On Unix systems:

1. Ledger home is enforced to `0700`.
2. Ledger database artifacts are enforced to `0600` when present:
   1. `ledger.db`
   2. `ledger.db-wal`
   3. `ledger.db-shm`
   4. `ledger.db-journal`
3. Writable SQLite connections enable:
   1. `PRAGMA foreign_keys = ON`
   2. `PRAGMA secure_delete = ON`

Notes:

1. Driggsby is local-first but currently does not encrypt the SQLite database at rest.
2. These controls reduce accidental local exposure and data remanence risk, but they do not replace host-level malware defenses or full-disk encryption.
