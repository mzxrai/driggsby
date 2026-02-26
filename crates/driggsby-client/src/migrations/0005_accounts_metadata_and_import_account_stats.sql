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

INSERT INTO internal_accounts (account_key, account_type, created_at, updated_at)
SELECT DISTINCT
    t.account_key,
    NULL AS account_type,
    CAST(strftime('%s', 'now') AS TEXT) AS created_at,
    CAST(strftime('%s', 'now') AS TEXT) AS updated_at
FROM internal_transactions t
WHERE t.account_key IS NOT NULL
  AND TRIM(t.account_key) <> ''
ON CONFLICT(account_key) DO NOTHING;

INSERT INTO internal_import_account_stats (import_id, account_key, rows_read, inserted, deduped)
SELECT
    import_id,
    account_key,
    COUNT(*) AS rows_read,
    COUNT(*) AS inserted,
    0 AS deduped
FROM internal_transactions
GROUP BY import_id, account_key
ON CONFLICT(import_id, account_key) DO UPDATE SET
    rows_read = excluded.rows_read,
    inserted = excluded.inserted;

INSERT INTO internal_import_account_stats (import_id, account_key, rows_read, inserted, deduped)
SELECT
    import_id,
    account_key,
    COUNT(*) AS rows_read,
    0 AS inserted,
    COUNT(*) AS deduped
FROM internal_transaction_dedupe_candidates
GROUP BY import_id, account_key
ON CONFLICT(import_id, account_key) DO UPDATE SET
    deduped = excluded.deduped,
    rows_read = internal_import_account_stats.inserted + excluded.deduped;

DROP VIEW IF EXISTS v1_transactions;
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

DROP VIEW IF EXISTS v1_accounts;
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

CREATE INDEX IF NOT EXISTS idx_internal_import_account_stats_import_id
ON internal_import_account_stats(import_id);

CREATE INDEX IF NOT EXISTS idx_internal_import_account_stats_account_key
ON internal_import_account_stats(account_key);
