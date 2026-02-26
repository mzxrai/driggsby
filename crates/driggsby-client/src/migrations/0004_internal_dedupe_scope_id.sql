PRAGMA foreign_keys = OFF;

DROP VIEW IF EXISTS v1_transactions;
DROP VIEW IF EXISTS v1_accounts;

ALTER TABLE internal_transactions RENAME TO internal_transactions_old;

CREATE TABLE internal_transactions (
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

INSERT INTO internal_transactions (
    txn_id,
    import_id,
    statement_id,
    dedupe_scope_id,
    account_key,
    posted_at,
    amount,
    currency,
    description,
    external_id,
    merchant,
    category
)
SELECT
    txn_id,
    import_id,
    statement_id,
    CASE
        WHEN statement_id IS NULL OR TRIM(statement_id) = '' THEN 'gen|legacy_backfill|' || account_key
        ELSE 'stmt|' || account_key || '|' || statement_id
    END AS dedupe_scope_id,
    account_key,
    posted_at,
    amount,
    currency,
    description,
    external_id,
    merchant,
    category
FROM internal_transactions_old;

DROP TABLE internal_transactions_old;

ALTER TABLE internal_transaction_dedupe_candidates RENAME TO internal_transaction_dedupe_candidates_old;

CREATE TABLE internal_transaction_dedupe_candidates (
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

INSERT INTO internal_transaction_dedupe_candidates (
    candidate_id,
    import_id,
    dedupe_key,
    statement_id,
    dedupe_scope_id,
    account_key,
    posted_at,
    amount,
    currency,
    description,
    external_id,
    merchant,
    category,
    source_row_index,
    dedupe_reason,
    matched_txn_id,
    matched_import_id,
    matched_batch_row_index,
    created_at,
    promoted_txn_id
)
SELECT
    candidate_id,
    import_id,
    dedupe_key,
    statement_id,
    CASE
        WHEN statement_id IS NULL OR TRIM(statement_id) = '' THEN 'gen|legacy_backfill|' || account_key
        ELSE 'stmt|' || account_key || '|' || statement_id
    END AS dedupe_scope_id,
    account_key,
    posted_at,
    amount,
    currency,
    description,
    external_id,
    merchant,
    category,
    source_row_index,
    dedupe_reason,
    matched_txn_id,
    matched_import_id,
    matched_batch_row_index,
    created_at,
    promoted_txn_id
FROM internal_transaction_dedupe_candidates_old;

DROP TABLE internal_transaction_dedupe_candidates_old;

DROP INDEX IF EXISTS idx_internal_transactions_import_id;
DROP INDEX IF EXISTS idx_internal_transactions_account_posted_at;
DROP INDEX IF EXISTS idx_internal_transactions_account_external_id;
DROP INDEX IF EXISTS idx_internal_transactions_fallback_dedupe;
DROP INDEX IF EXISTS idx_internal_transaction_dedupe_candidates_dedupe_key;
DROP INDEX IF EXISTS idx_internal_transaction_dedupe_candidates_import_id;

CREATE VIEW IF NOT EXISTS v1_transactions AS
SELECT
    txn_id,
    import_id,
    statement_id,
    account_key,
    posted_at,
    amount,
    currency,
    description,
    external_id,
    merchant,
    category
FROM internal_transactions;

CREATE VIEW IF NOT EXISTS v1_accounts AS
SELECT
    account_key,
    currency,
    MIN(posted_at) AS first_posted_at,
    MAX(posted_at) AS last_posted_at,
    COUNT(*) AS txn_count
FROM internal_transactions
GROUP BY account_key, currency;

CREATE INDEX IF NOT EXISTS idx_internal_transactions_import_id
ON internal_transactions(import_id);

CREATE INDEX IF NOT EXISTS idx_internal_transactions_account_posted_at
ON internal_transactions(account_key, posted_at);

CREATE INDEX IF NOT EXISTS idx_internal_transactions_account_external_id
ON internal_transactions(account_key, external_id);

CREATE INDEX IF NOT EXISTS idx_internal_transactions_fallback_dedupe
ON internal_transactions(account_key, posted_at, amount, currency, description);

CREATE INDEX IF NOT EXISTS idx_internal_transaction_dedupe_candidates_dedupe_key
ON internal_transaction_dedupe_candidates(dedupe_key, promoted_txn_id, source_row_index);

CREATE INDEX IF NOT EXISTS idx_internal_transaction_dedupe_candidates_import_id
ON internal_transaction_dedupe_candidates(import_id);

PRAGMA foreign_keys = ON;

SELECT 1;
