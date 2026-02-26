CREATE TABLE IF NOT EXISTS internal_transaction_dedupe_candidates (
    candidate_id TEXT PRIMARY KEY,
    import_id TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    statement_id TEXT NOT NULL,
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

CREATE INDEX IF NOT EXISTS idx_internal_transaction_dedupe_candidates_dedupe_key
ON internal_transaction_dedupe_candidates(dedupe_key, promoted_txn_id, source_row_index);

CREATE INDEX IF NOT EXISTS idx_internal_transaction_dedupe_candidates_import_id
ON internal_transaction_dedupe_candidates(import_id);
