# Driggsby V1 — Open Source Proposal

Last updated: 2026-02-19

---

## What Driggsby Is

A local financial ledger with a dashboard. It stores structured financial data on your machine, lets you query it, and shows it in a browser. It assumes you have an AI agent (Claude Code, Codex, etc.) available — the agent provides the intelligence, Driggsby provides the memory.

## Target Audience

Developers who already have an AI coding agent running. This is the sole audience for v1. They have Claude Code or Codex installed. They're comfortable with a CLI. They want to understand their finances without shipping their data to a cloud service.

## Why It Exists

1. **Agents are stateless.** Your agent can read a bank statement and answer questions about it, but next session it's gone. Driggsby is the persistent layer that accumulates financial data across sessions.

2. **Onramp to the paid product.** The open source CLI is the funnel. Users experience "query my finances with an AI agent," feel the pull toward automation, and upgrade to the Plaid tier when they're tired of manual imports.

3. **Things agents can't easily do on their own:**
   - Maintain a stable, tested schema across months of data
   - Deduplicate transactions across imports
   - Normalize merchant names consistently (AMZN MKTP US → Amazon)
   - Serve a persistent dashboard you can bookmark
   - Upgrade to live Plaid data (requires account approval, webhooks, infrastructure)

---

## Architecture

### One Rust Binary

Everything ships in a single compiled binary. No runtime dependencies. The dashboard's static assets are embedded.

```
driggsby (binary)
  ├── CLI commands         import, query, export, schema, etc.
  ├── SQLite database      ~/.driggsby/ledger.db
  ├── Web server           localhost dashboard + JSON API
  └── Embedded dashboard   basic HTML/CSS/JS, baked into the binary
```

### Distribution

```
brew install driggsby
npm install -g driggsby
cargo install driggsby
GitHub releases (prebuilt binaries)
```

---

## How Data Gets In

### Primary Path: Agent + Skill (/driggsby-import)

99% of bank statements are PDFs. Parsing PDFs reliably requires an LLM. The user's agent does this work via a skill (slash command) that Driggsby ships.

```
/driggsby-import ~/Downloads/statements/

  Skill loads → Claude follows controlled instructions →
  For each PDF:
    1. Reads the PDF (Claude reads PDFs natively)
    2. Identifies bank, account type, period
    3. Writes a Python parser script (ALWAYS code, never inline)
    4. Executes the script → structured JSON output
    5. Pipes JSON into `driggsby import --json -`
    6. Driggsby stores, deduplicates, normalizes
```

Key design decisions for the skill:
- **Always writes code.** Never parses transactions inline. Code runs deterministically against every line — no hallucination, no skipped transactions.
- **Queries Driggsby for its schema at runtime** (`driggsby schema`). If the schema evolves, the skill adapts automatically. The CLI is the source of truth.
- **Runs in a forked context** (`context: fork`). Keeps parser scripts out of the user's main conversation.
- **Verifies its own work.** Spot-checks transaction count, first/last entries, and closing balance against the PDF before importing.

### Secondary Path: CSV Import

For users who can export CSV from their bank's website. Fully deterministic, no LLM needed. Column mapping handled via flags — the agent reads any errors and retries with the right mapping.

### Future Path: Plaid (Paid Tier)

Automatic, live data. Webhook-driven sync. No manual imports. This is the paid product.

---

## The Dashboard

### What Ships (Embedded, Default)

A minimal, clean, functional dashboard. Not everything — just the basics. Shows accounts, transactions, spending by category, balances. This is the "it works out of the box" experience and what makes the README compelling.

### The JSON API (Extensible)

The dashboard server exposes a JSON API. The embedded dashboard is just one consumer. Users can point their agent at the API and build any custom view they want — "redesign the dashboard," "add a subscription chart," "group by merchant." The default dashboard is the starting point. The API makes it infinitely extensible.

---

## What Driggsby Does NOT Do (Boundary Line)

| Driggsby's Job | Not Driggsby's Job |
|---|---|
| Store structured financial data | Parse PDFs (agent does this) |
| Deduplicate transactions | Understand statement formats (agent) |
| Normalize merchant names | Categorize intelligently (agent / rules) |
| Serve dashboard + API | Generate custom charts (agent) |
| Accept structured input (JSON, CSV) | Give financial advice |
| Maintain stable schema | Call an LLM |
| Provide Plaid upgrade path | Talk to banks directly (until Plaid tier) |

The line: structured data in, structured data out. If it requires intelligence, the agent does it. If it requires persistence, Driggsby does it.

---

## Upgrade Path

```
Free (open source)              Paid (Plaid tier)
─────────────────               ─────────────────
Manual import (PDF via skill,   Automatic. Live data.
CSV via CLI)                    Webhook-driven sync.
Local SQLite                    Cloud-synced + local
Basic dashboard                 Enhanced dashboard + analytics
                                AI caretaker (background monitoring,
                                duplicate detection, fraud alerts,
                                spending anomaly emails)
```

The free tier lets people experience "query my finances with an AI agent." The paid tier removes the friction of manual imports and adds background intelligence.

---

## README Pitch

> **Driggsby** — a local financial ledger with a beautiful dashboard.
>
> Works with your AI agent (Claude Code, Codex, etc.) to turn bank statements into structured, queryable data. Your data stays on your machine.
>
> `brew install driggsby && driggsby init && driggsby dashboard`
