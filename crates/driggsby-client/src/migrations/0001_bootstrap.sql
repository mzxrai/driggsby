CREATE TABLE IF NOT EXISTS internal_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS internal_import_runs (
    import_id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    committed_at TEXT,
    reverted_at TEXT,
    rows_read INTEGER NOT NULL DEFAULT 0,
    rows_valid INTEGER NOT NULL DEFAULT 0,
    rows_invalid INTEGER NOT NULL DEFAULT 0,
    inserted INTEGER NOT NULL DEFAULT 0,
    deduped INTEGER NOT NULL DEFAULT 0,
    source_kind TEXT,
    source_ref TEXT
);

CREATE TABLE IF NOT EXISTS internal_transactions (
    txn_id TEXT PRIMARY KEY,
    import_id TEXT NOT NULL,
    statement_id TEXT,
    dedupe_scope_id TEXT NOT NULL,
    account_key TEXT NOT NULL,
    posted_at TEXT NOT NULL,
    amount REAL NOT NULL,
    currency TEXT NOT NULL,
    description TEXT NOT NULL,
    external_id TEXT,
    merchant TEXT,
    category TEXT
);

CREATE TABLE IF NOT EXISTS internal_accounts (
    account_key TEXT PRIMARY KEY,
    account_type TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS internal_import_account_stats (
    import_id TEXT NOT NULL,
    account_key TEXT NOT NULL,
    rows_read INTEGER NOT NULL DEFAULT 0,
    inserted INTEGER NOT NULL DEFAULT 0,
    deduped INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (import_id, account_key)
);

CREATE TABLE IF NOT EXISTS internal_transaction_dedupe_candidates (
    candidate_id TEXT PRIMARY KEY,
    import_id TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    statement_id TEXT,
    dedupe_scope_id TEXT NOT NULL,
    account_key TEXT NOT NULL,
    posted_at TEXT NOT NULL,
    amount REAL NOT NULL,
    currency TEXT NOT NULL,
    description TEXT NOT NULL,
    external_id TEXT,
    merchant TEXT,
    category TEXT,
    source_row_index INTEGER NOT NULL,
    dedupe_reason TEXT NOT NULL CHECK (dedupe_reason IN ('batch', 'existing_ledger')),
    matched_txn_id TEXT,
    matched_import_id TEXT,
    matched_batch_row_index INTEGER,
    created_at TEXT NOT NULL,
    promoted_txn_id TEXT
);

CREATE TABLE IF NOT EXISTS internal_recurring_materialized (
    group_key TEXT PRIMARY KEY,
    account_key TEXT NOT NULL,
    merchant TEXT NOT NULL,
    cadence TEXT NOT NULL,
    typical_amount REAL NOT NULL,
    currency TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    next_expected_at TEXT,
    occurrence_count INTEGER NOT NULL,
    score REAL NOT NULL,
    is_active INTEGER NOT NULL CHECK (is_active IN (0, 1))
);

CREATE TABLE IF NOT EXISTS internal_anomalies_materialized (
    txn_id TEXT PRIMARY KEY,
    account_key TEXT NOT NULL,
    posted_at TEXT NOT NULL,
    merchant TEXT NOT NULL,
    amount REAL NOT NULL,
    currency TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    reason TEXT NOT NULL,
    score REAL NOT NULL,
    severity TEXT NOT NULL
);

INSERT OR IGNORE INTO internal_meta (key, value) VALUES ('schema_version', 'v1');
INSERT OR IGNORE INTO internal_meta (key, value) VALUES ('public_views_version', 'v1');
INSERT OR IGNORE INTO internal_meta (key, value) VALUES ('import_contract_version', 'v1');

-- driggsby:safe_repair:start:v1_transactions
CREATE VIEW v1_transactions AS
SELECT
    t.txn_id,
    t.import_id,
    t.statement_id,
    t.account_key,
    a.account_type,
    t.posted_at,
    t.amount,
    t.currency,
    t.description,
    t.external_id,
    t.merchant,
    t.category
FROM internal_transactions t
LEFT JOIN internal_accounts a ON a.account_key = t.account_key;
-- driggsby:safe_repair:end:v1_transactions

-- driggsby:safe_repair:start:v1_accounts
CREATE VIEW v1_accounts AS
SELECT
    t.account_key,
    a.account_type,
    t.currency,
    MIN(t.posted_at) AS first_posted_at,
    MAX(t.posted_at) AS last_posted_at,
    COUNT(*) AS txn_count,
    ROUND(SUM(t.amount), 2) AS net_amount
FROM internal_transactions t
LEFT JOIN internal_accounts a ON a.account_key = t.account_key
GROUP BY t.account_key, a.account_type, t.currency;
-- driggsby:safe_repair:end:v1_accounts

-- driggsby:safe_repair:start:v1_imports
CREATE VIEW v1_imports AS
SELECT
    import_id,
    status,
    created_at,
    committed_at,
    reverted_at,
    rows_read,
    rows_valid,
    rows_invalid,
    inserted,
    deduped,
    source_kind,
    source_ref
FROM internal_import_runs;
-- driggsby:safe_repair:end:v1_imports

-- driggsby:safe_repair:start:v1_recurring
CREATE VIEW v1_recurring AS
SELECT
    group_key,
    account_key,
    merchant,
    cadence,
    typical_amount,
    currency,
    last_seen_at,
    next_expected_at,
    occurrence_count,
    score,
    is_active
FROM internal_recurring_materialized;
-- driggsby:safe_repair:end:v1_recurring

-- driggsby:safe_repair:start:v1_anomalies
CREATE VIEW v1_anomalies AS
SELECT
    txn_id,
    account_key,
    posted_at,
    merchant,
    amount,
    currency,
    reason_code,
    reason,
    score,
    severity
FROM internal_anomalies_materialized;
-- driggsby:safe_repair:end:v1_anomalies

-- driggsby:safe_repair:start:idx_internal_transactions_import_id
CREATE INDEX idx_internal_transactions_import_id
ON internal_transactions(import_id);
-- driggsby:safe_repair:end:idx_internal_transactions_import_id

-- driggsby:safe_repair:start:idx_internal_transactions_account_posted_at
CREATE INDEX idx_internal_transactions_account_posted_at
ON internal_transactions(account_key, posted_at);
-- driggsby:safe_repair:end:idx_internal_transactions_account_posted_at

-- driggsby:safe_repair:start:idx_internal_transactions_account_external_id
CREATE INDEX idx_internal_transactions_account_external_id
ON internal_transactions(account_key, external_id);
-- driggsby:safe_repair:end:idx_internal_transactions_account_external_id

-- driggsby:safe_repair:start:idx_internal_transactions_fallback_dedupe
CREATE INDEX idx_internal_transactions_fallback_dedupe
ON internal_transactions(account_key, posted_at, amount, currency, description);
-- driggsby:safe_repair:end:idx_internal_transactions_fallback_dedupe

-- driggsby:safe_repair:start:idx_internal_import_runs_created_at_desc
CREATE INDEX idx_internal_import_runs_created_at_desc
ON internal_import_runs(created_at DESC);
-- driggsby:safe_repair:end:idx_internal_import_runs_created_at_desc

-- driggsby:safe_repair:start:idx_internal_transaction_dedupe_candidates_dedupe_key
CREATE INDEX idx_internal_transaction_dedupe_candidates_dedupe_key
ON internal_transaction_dedupe_candidates(dedupe_key, promoted_txn_id, source_row_index);
-- driggsby:safe_repair:end:idx_internal_transaction_dedupe_candidates_dedupe_key

-- driggsby:safe_repair:start:idx_internal_transaction_dedupe_candidates_import_id
CREATE INDEX idx_internal_transaction_dedupe_candidates_import_id
ON internal_transaction_dedupe_candidates(import_id);
-- driggsby:safe_repair:end:idx_internal_transaction_dedupe_candidates_import_id

-- driggsby:safe_repair:start:idx_internal_import_account_stats_import_id
CREATE INDEX idx_internal_import_account_stats_import_id
ON internal_import_account_stats(import_id);
-- driggsby:safe_repair:end:idx_internal_import_account_stats_import_id

-- driggsby:safe_repair:start:idx_internal_import_account_stats_account_key
CREATE INDEX idx_internal_import_account_stats_account_key
ON internal_import_account_stats(account_key);
-- driggsby:safe_repair:end:idx_internal_import_account_stats_account_key

-- driggsby:safe_repair:start:idx_internal_recurring_materialized_last_seen_at
CREATE INDEX idx_internal_recurring_materialized_last_seen_at
ON internal_recurring_materialized(last_seen_at);
-- driggsby:safe_repair:end:idx_internal_recurring_materialized_last_seen_at

-- driggsby:safe_repair:start:idx_internal_anomalies_materialized_posted_at
CREATE INDEX idx_internal_anomalies_materialized_posted_at
ON internal_anomalies_materialized(posted_at);
-- driggsby:safe_repair:end:idx_internal_anomalies_materialized_posted_at
