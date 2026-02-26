# Plaid Integration — Learnings & Architecture Guide

Last updated: 2026-02-19

---

## Key Takeaways

1. **Plaid bills per Item, not per account.** An Item = one bank login. Chase checking + savings + credit card under one login = 1 Item. Fidelity brokerage + IRA under one login = 1 Item.

2. **You don't need the Balance API.** `/accounts/get` returns cached balances for free (no per-call fee). Cached data updates 1-4x/day when the Item has Transactions, Investments, or Liabilities enabled. The paid Balance API (`/accounts/balance/get`) is only needed for real-time balance checks (payment verification). Driggsby does not need real-time balances.

3. **Use "Additional Consented Products" to defer billing.** Initialize Link with only `transactions` in the `products` array. Put `liabilities` and `investments` in `additional_consented_products`. Billing only starts when you actually call those endpoints, not when the Item is created. This prevents accidental charges on Items where you don't need those products.

4. **Liabilities is not needed for basic credit card PFM.** The Transactions product already gives you `balances.current`, `balances.available`, and `balances.limit` (credit limit) on credit accounts via cached balance data. Enough for spending tracking, utilization, and "you owe $X." Only enable Liabilities when you need APR, minimum payment amount, due dates, and statement details.

5. **Investments has two separate subscriptions.** Investments Holdings and Investments Transactions are billed independently. The Holdings subscription starts when you call `/investments/holdings/get`. The Transactions subscription starts only when you call `/investments/transactions/get`. **Important caveat:** calling `/investments/transactions/get` also activates the Holdings subscription if it isn't already active, so you get billed for both. For portfolio snapshots and net worth, Holdings alone is sufficient. Never call the transactions endpoint unless you explicitly intend to pay for it.

6. **Webhook-driven architecture is the correct pattern.** Listen for `SYNC_UPDATES_AVAILABLE` webhooks, then call `/transactions/sync`. Don't poll. The sync response includes both new transactions and cached account balances at no additional cost.

7. **Re-authentication is free.** When a user needs to re-link (password change, MFA expiry), use Link Update Mode. This does not incur a new connection fee. Only creates a charge if the user creates an entirely new Item instead of re-linking the existing one.

---

## Pricing Reference

All prices are estimates based on research (Feb 2026). Plaid does not publish a public rate card. Confirm with your account rep.

### Subscription Products (per Item per month)

| Product | Rack Rate | At Scale (1000+) | Billing Trigger |
|---|---|---|---|
| Transactions | ~$1.50 | $0.40-$0.80 | Item created with product enabled, OR first API call if using additional_consented_products |
| Liabilities | ~$1.00-$2.00 | negotiable | First call to `/liabilities/get` on that Item |
| Investments Holdings | ~$1.00-$1.50 | negotiable | First call to `/investments/holdings/get` on that Item |
| Investments Transactions | ~$1.00-$1.50 | negotiable | First call to `/investments/transactions/get` on that Item |

### One-Time Products (per event)

| Product | Rack Rate | At Scale | Notes |
|---|---|---|---|
| Auth | $0.50-$1.50 | $0.30-$0.45 | Per successful link |
| Identity | $1.00-$1.50 | $0.50-$0.90 | Per verification |
| Assets | $2.00-$5.00 | negotiable | Per report |

### Free Endpoints (no per-call fee)

| Endpoint | What It Returns | Freshness |
|---|---|---|
| `/accounts/get` | Cached balances (current, available, limit, currency) for all accounts on an Item | As of last sync (1-4x/day) |
| Balance data in `/transactions/sync` response | Same cached balances, but only for accounts with activity in that sync batch | Same |

### Paid Per-Call Endpoints (avoid these)

| Endpoint | Cost | When You'd Need It |
|---|---|---|
| `/accounts/balance/get` | $0.10-$0.50/call | Real-time balance for payment verification. Driggsby does NOT need this. |
| `/transactions/refresh` | Per-call (add-on) | Force an on-demand sync. Rate limited: 2/min per Item, 100/min per client. Must request access. |

---

## Cost Modeling for Driggsby

### Typical User Profile: 4 Items

```
Item 1: Primary bank (checking + savings + credit card) — 1 Item
Item 2: Standalone credit card (Discover, Amex, etc.)   — 1 Item
Item 3: Brokerage + retirement (Fidelity, Schwab, etc.) — 1 Item
Item 4: Loan servicer (student loans, mortgage)          — 1 Item
```

### Feature Tiers and What to Enable

**Tier A — Basic PFM + Net Worth (cheapest)**
- Spending categories, transaction history, balances, credit utilization, portfolio value, net worth
- No debt management (APR, due dates, min payments)

| Item | Product Enabled | Monthly Cost (Rack) |
|---|---|---|
| Bank (checking+savings+CC) | Transactions | $1.50 |
| Credit card | Transactions | $1.50 |
| Brokerage | Investments Holdings only | $1.00-$1.50 |
| Loan servicer | Liabilities | $1.00-$1.50 |
| **Total** | | **$5.00-$6.00** |
| **At scale** | | **$2.50-$3.50** |

**Tier B — PFM + Debt Management (adds credit card APR, due dates)**

| Item | Product Enabled | Monthly Cost (Rack) |
|---|---|---|
| Bank (checking+savings+CC) | Transactions | $1.50 |
| Credit card | Transactions + Liabilities | $2.50-$3.50 |
| Brokerage | Investments Holdings only | $1.00-$1.50 |
| Loan servicer | Liabilities | $1.00-$1.50 |
| **Total** | | **$6.00-$8.00** |
| **At scale** | | **$3.50-$5.50** |

**Tier C — Full (adds investment trade history)**

Same as Tier B, plus call `/investments/transactions/get` on the brokerage Item. Adds another ~$1.00-$1.50/month.

**Simpler user (just bank + credit card, no investments/loans):**

| Item | Product | Cost |
|---|---|---|
| Bank + CC | Transactions | $1.50 |
| **Total** | | **$1.50** |

### One-Time Costs (Amortized)

```
4 Items x ~$0.75 avg Auth fee = $3.00 one-time
Amortized over 12 months retention = ~$0.25/month
```

---

## API Reference for Driggsby Integration

### Link Initialization

```json
POST /link/token/create
{
  "client_id": "...",
  "secret": "...",
  "client_name": "Driggsby",
  "country_codes": ["US"],
  "language": "en",
  "user": {
    "client_user_id": "driggsby-user-123",
    "phone_number": "+1...",       // optional, enables Returning User Experience
    "email_address": "..."         // optional, same
  },
  "products": ["transactions"],
  "additional_consented_products": ["liabilities", "investments"]
}
```

Key points:
- `balance` is NOT a valid product string — it's automatically included with any product
- Only `transactions` in `products` array — this is what we always need and what starts billing
- `liabilities` and `investments` in `additional_consented_products` — defers billing until we call those endpoints
- Pre-filling `phone_number` and `email_address` enables faster re-linking (~10% conversion lift)

### Valid Product Strings

```
transactions, investments, liabilities, auth, identity, assets,
transfer, payment_initiation, identity_verification,
income_verification, employment, standing_orders, signal,
cra_base_report
```

### Token Exchange Flow

```
1. Backend:  POST /link/token/create  →  link_token (short-lived)
2. Frontend: Initialize Plaid Link with link_token
3. User:     Selects bank, authenticates
4. Frontend: onSuccess callback receives public_token
5. Backend:  POST /item/public_token/exchange  →  access_token + item_id (permanent, store securely)
```

### Transaction Sync (Core Data Loop)

```
Webhook received: SYNC_UPDATES_AVAILABLE
    │
    ▼
POST /transactions/sync
{
  "access_token": "...",
  "cursor": "last_saved_cursor"    // empty string on first sync
}
    │
    ▼
Response:
{
  "added": [...],           // new transactions
  "modified": [...],        // changed transactions (tips, corrections)
  "removed": [...],         // reversed/deleted transactions
  "accounts": [{            // cached balance data (FREE)
    "account_id": "...",
    "balances": {
      "current": 4521.00,
      "available": 4200.00,
      "limit": null,          // non-null for credit accounts
      "iso_currency_code": "USD",
      "last_updated_datetime": null          // only populated for Capital One; null for most institutions
    }
  }],
  "next_cursor": "...",     // MUST save this
  "has_more": true/false    // loop until false
}
```

Critical implementation notes:
- First sync (empty cursor): returns up to 2 years of history. Loop through `has_more: true` pages.
- Save `next_cursor` after every successful sync. This is your bookmark.
- Transactions move from pending to posted via `removed` (old pending ID) + `added` (new posted ID), or `modified` (amount change, e.g., tip added).
- Always deduplicate on `transaction_id` — webhooks may fire more than once.
- Verify webhook authenticity via the `Plaid-Verification` JWT header.

### Balance Data (Free, Cached)

For on-demand balance checks (e.g., user opens dashboard), use:

```
POST /accounts/get
{
  "access_token": "..."
}
```

Returns cached balances for ALL accounts on the Item. No per-call fee. Freshness: as of last Plaid sync (1-4x/day).

Balance fields for credit accounts:
- `current`: amount owed (positive = debt)
- `available`: remaining credit to spend
- `limit`: credit limit (nullable — depends on institution)
- `iso_currency_code`: "USD", etc.
- `last_updated_datetime`: ISO 8601 (nullable — only reliably populated for Capital One; null for most institutions)

### Investments (Holdings Only for V1)

Only call this endpoint. Never call `/investments/transactions/get` unless you intend to pay for the second subscription.

```
POST /investments/holdings/get
{
  "access_token": "..."
}
```

Returns: holdings (quantity, value, cost basis), security details (ticker, CUSIP, type). Sufficient for net worth, allocation, portfolio overview.

Listen for webhook: `webhook_type: "HOLDINGS"`, `webhook_code: "DEFAULT_UPDATE"`.

### Liabilities

```
POST /liabilities/get
{
  "access_token": "..."
}
```

Returns per account type:
- **Credit cards**: APRs (purchase, balance transfer, cash advance), minimum payment, next due date, last statement balance/date
- **Student loans**: interest rate, loan term, origination date, outstanding balance, repayment plan, expected payoff date
- **Mortgages** (limited, mainly Canadian institutions): interest rate, term, property address

Listen for webhook: `webhook_type: "LIABILITIES"`, `webhook_code: "DEFAULT_UPDATE"`. Data refreshes ~1x/day.

---

## Webhook Reference

| Webhook Type | Code | When It Fires | Your Response |
|---|---|---|---|
| `TRANSACTIONS` | `SYNC_UPDATES_AVAILABLE` | New/modified/removed transactions detected (1-4x/day) | Call `/transactions/sync` with saved cursor |
| `TRANSACTIONS` | `INITIAL_UPDATE` | First ~30 days of transactions ready after new link | Call `/transactions/sync` |
| `TRANSACTIONS` | `HISTORICAL_UPDATE` | Full history (up to 2 years) ready after new link | Call `/transactions/sync` |
| `HOLDINGS` | `DEFAULT_UPDATE` | Holdings data updated (typically daily, after market close) | Call `/investments/holdings/get` |
| `LIABILITIES` | `DEFAULT_UPDATE` | Liabilities data updated (~1x/day) | Call `/liabilities/get` |
| `ITEM` | `ERROR` | Item entered error state (login required, etc.) | Trigger Link Update Mode for user |
| `ITEM` | `NEW_ACCOUNTS_AVAILABLE` | User added new accounts at their bank | Call `/accounts/get`, optionally re-sync |
| `ITEM` | `PENDING_EXPIRATION` | Access consent expiring soon (primarily UK/EU under PSD2; US equivalent is `PENDING_DISCONNECT`) | Trigger Link Update Mode |

Webhook payload structure:
```json
{
  "webhook_type": "TRANSACTIONS",
  "webhook_code": "SYNC_UPDATES_AVAILABLE",
  "item_id": "...",
  "environment": "production"
}
```

Always verify webhook authenticity via the `Plaid-Verification` header (JWT signed by Plaid).

---

## Architecture: Optimal Event Loop

```
Plaid Cloud                        Driggsby Backend
───────────                        ────────────────

Bank detects activity
        │
Plaid syncs (1-4x/day)
        │
Fires webhook ──────────────────►  POST /webhooks/plaid
                                        │
                                   Verify JWT signature
                                        │
                                   Route by webhook_type:
                                        │
                   ┌────────────────────┼────────────────────┐
                   │                    │                     │
              TRANSACTIONS         HOLDINGS            LIABILITIES
                   │                    │                     │
           /transactions/sync    /investments/         /liabilities/get
           (loop has_more)        holdings/get               │
                   │                    │                     │
                   └────────────────────┼─────────────────────┘
                                        │
                                   Write to DB:
                                   - transactions table
                                   - holdings table
                                   - liabilities table
                                   - account_balances table
                                   (from sync/accounts response)
                                        │
                                   Update derived views:
                                   - net worth snapshots
                                   - spending summaries
                                   - category breakdowns
                                        │
                                   AI Caretaker check:
                                   - duplicate transactions?
                                   - unusual amounts?
                                   - new recurring charges?
                                   - spending anomalies?
                                        │
                                   If flagged → queue email
```

### Cost-Critical Implementation Rules

1. **NEVER call `/accounts/balance/get`.** Always use cached balances from `/accounts/get` or the `accounts[]` in `/transactions/sync`.

2. **NEVER call `/investments/transactions/get` unless the user has opted into trade history.** This triggers a second subscription on the Item. Gate this behind an explicit feature flag in your codebase.

3. **NEVER put `liabilities` or `investments` in the `products` array of `/link/token/create`.** Always use `additional_consented_products`. Only call those endpoints when you've confirmed the Item has a relevant account type (credit card, loan, brokerage).

4. **Always use Link Update Mode for re-authentication.** Creating a new Item instead of re-linking incurs a new Auth fee.

5. **Don't request `/transactions/refresh` access unless necessary.** The webhook-driven sync (1-4x/day) is sufficient for PFM. If you add a "refresh now" button, it should call `/transactions/sync` with the existing cursor (free) and display whatever data is already cached — not force a paid on-demand refresh.

---

## Backend SDKs

| Language | Package | Status |
|---|---|---|
| Node.js | `plaid-node` | Official |
| Python | `plaid-python` | Official |
| Ruby | `plaid-ruby` | Official |
| Java | `plaid-java` | Official |
| Go | `plaid-go` | Official |
| .NET | `Going.Plaid` | Community |
| Elixir | `plaid-elixir` | Community |
| **Rust** | **None usable** | See below |

Frontend SDKs (Plaid Link):
| Platform | Package | Status |
|---|---|---|
| React | `react-plaid-link` | Official |
| Vanilla JS | Plaid Link JS | Official |
| iOS (Swift) | `plaid-link-ios` | Official |
| Android (Kotlin) | `plaid-link-android` | Official |
| React Native | `react-native-plaid-link-sdk` | Official |
| Flutter | `plaid_flutter` | Community |

### Rust Client Strategy

There is no official Plaid Rust SDK. The community `plaid` crate on crates.io (auto-generated via Libninja) has not been updated in over a year and is effectively useless for production use.

**Our approach: build a thin custom client.** Driggsby only uses ~6 Plaid endpoints. A custom client with `reqwest` + `serde` is straightforward and keeps dependencies minimal.

Resources for building it:
- **OpenAPI spec**: Plaid publishes their full spec at [github.com/plaid/plaid-openapi](https://github.com/plaid/plaid-openapi). Use this as the canonical reference for request/response shapes.
- **Go client as reference**: `plaid-go` is the closest official SDK to Rust idiomatically. Use it as a structural reference for how endpoints, error handling, and pagination work.
- **Progenitor** (by Oxide Computer): If we ever want to auto-generate a full client from the OpenAPI spec, Progenitor produces much more idiomatic Rust than openapi-generator. But for 6 endpoints, hand-written is simpler.

Endpoints we need to implement:
```
POST /link/token/create          — generate Link token for frontend
POST /item/public_token/exchange — exchange public_token for access_token
POST /transactions/sync          — incremental transaction sync (cursor-based)
POST /accounts/get               — cached balances (free)
POST /investments/holdings/get   — portfolio holdings
POST /liabilities/get            — debt details
```

Key implementation notes:
- All endpoints are POST with JSON body, authenticated via `client_id` + `secret` headers (or in body).
- Plaid uses extensible enums — new values can appear without notice. Use `#[serde(other)]` on all enum variants to handle unknown values gracefully instead of failing deserialization.
- Error responses follow a consistent shape: `{ error_type, error_code, error_message, display_message }`. Deserialize into a common error type.
- The `/transactions/sync` endpoint uses cursor-based pagination. Loop until `has_more: false`.

---

## Development & Testing

### Environments

| Environment | Endpoint | Data | Cost | Limits |
|---|---|---|---|---|
| Sandbox | `sandbox.plaid.com` | Mock only (test credentials: `user_good`/`pass_good`) | Free | Unlimited |
| Limited Production | `production.plaid.com` | Real banks | 200 free API calls per product (lifetime) | Some OAuth banks (Chase, BofA) require separate registration |
| Production | `production.plaid.com` | Real banks | Pay as you go or contracted | Requires production approval (1-5 business days) |

Note: The old "Development" environment was deprecated June 2024. "Limited Production" replaced it. The 200 free calls are lifetime, not monthly, and both successful and failed calls count.

### Production Approval Timeline

| Product | Approval Time |
|---|---|
| Transactions, Balance, Auth | 1-3 business days |
| Identity | Instant once account approved |
| Transfer (ACH) | 2-3 weeks (underwriting) |

Tip: Start the security questionnaire while still building in sandbox. The review runs in parallel.

### Security Questionnaire Requirements
- Encryption: AES-256 at rest, TLS 1.2+ in transit
- Access controls documented
- Privacy policy on your website
- Use case description

---

## Negotiation Notes (for Plaid Rep Meeting)

### Your Position
- PFM use case — high Items-per-user (3-4 avg), but lean product mix (mostly Transactions)
- Using `additional_consented_products` to keep billable product count low
- Developer-first audience, growing to consumer

### Key Questions
1. Bundle rate for Transactions + Investments Holdings + Liabilities? Or each priced independently?
2. Is there a PFM-specific package or startup credits program?
3. Webhook-driven only (no paid Balance or Refresh) — does that affect pricing tier?
4. Volume discount thresholds — what user count unlocks Growth pricing?
5. What does the Enrich product add over the default PFCv2 categorization in Transactions?

### Leverage Points
- MX has higher categorization accuracy (95%+)
- Finicity has Mastercard backing and subscription-based pricing
- Teller is free for first 100 connections
- Yodlee under new ownership (STG), potentially aggressive pricing
- You don't need to be aggressive — just let them know you're evaluating

### Startup Programs
- Fintech Sandbox offers Plaid credits (per endpoint/month, minimums waived)
- Ask rep directly about startup credits / waived minimums for first 6-12 months

---

## Competitors Quick Reference

| Provider | Best For | Pricing Model | Key Advantage |
|---|---|---|---|
| Plaid | Consumer fintech, developer experience | Usage-based (per Item per product) | Largest network, best DX, fastest time-to-market |
| MX | Data quality, bank partnerships | Enterprise licensing | 95%+ categorization accuracy |
| Finicity (Mastercard) | Regulated lending, enterprise | Enterprise/subscription | Mastercard brand, regulatory expertise |
| Teller | Simple bank connectivity | Free first 100, then per-enrollment | Cheapest entry point |
| Yodlee (STG) | Global coverage | Enterprise subscription | 19,000+ sources, APAC/EMEA |

Recommendation: Start with Plaid only. Log connection failures and data quality issues by institution. Add a fallback provider only for specific hotspots — don't split by data type across providers (creates reconciliation nightmares).
