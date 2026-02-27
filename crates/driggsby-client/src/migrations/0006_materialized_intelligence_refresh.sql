DROP VIEW IF EXISTS v1_recurring;
DROP VIEW IF EXISTS v1_anomalies;

DROP TABLE IF EXISTS internal_recurring_materialized;
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

DROP TABLE IF EXISTS internal_anomalies_materialized;
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

CREATE INDEX IF NOT EXISTS idx_internal_recurring_materialized_last_seen_at
ON internal_recurring_materialized(last_seen_at);

CREATE INDEX IF NOT EXISTS idx_internal_anomalies_materialized_posted_at
ON internal_anomalies_materialized(posted_at);

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
