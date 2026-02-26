use std::collections::HashMap;

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

const BOOTSTRAP_SQL: &str = include_str!("migrations/0001_bootstrap.sql");
const ADD_TRANSACTION_DEDUPE_CANDIDATES_SQL: &str =
    include_str!("migrations/0002_add_transaction_dedupe_candidates.sql");
const ADD_STATEMENT_ID_AND_DUPLICATE_METADATA_SQL: &str =
    include_str!("migrations/0003_add_statement_id_and_duplicate_metadata.sql");
const ADD_INTERNAL_DEDUPE_SCOPE_ID_SQL: &str =
    include_str!("migrations/0004_internal_dedupe_scope_id.sql");

pub const REQUIRED_VIEW_NAMES: [&str; 5] = [
    "v1_transactions",
    "v1_accounts",
    "v1_imports",
    "v1_recurring",
    "v1_anomalies",
];

pub const REQUIRED_INDEX_NAMES: [&str; 7] = [
    "idx_internal_transactions_import_id",
    "idx_internal_transactions_account_posted_at",
    "idx_internal_transactions_account_external_id",
    "idx_internal_transactions_fallback_dedupe",
    "idx_internal_import_runs_created_at_desc",
    "idx_internal_transaction_dedupe_candidates_dedupe_key",
    "idx_internal_transaction_dedupe_candidates_import_id",
];

pub const REQUIRED_META_KEYS: [(&str, &str); 3] = [
    ("schema_version", "v1"),
    ("public_views_version", "v1"),
    ("import_contract_version", "v1"),
];

pub fn run_pending(conn: &mut Connection) -> rusqlite_migration::Result<()> {
    let migrations = Migrations::new(vec![
        M::up(BOOTSTRAP_SQL),
        M::up(ADD_TRANSACTION_DEDUPE_CANDIDATES_SQL),
        M::up(ADD_STATEMENT_ID_AND_DUPLICATE_METADATA_SQL),
        M::up(ADD_INTERNAL_DEDUPE_SCOPE_ID_SQL),
    ]);
    migrations.to_latest(conn)
}

pub fn safe_repair_statement(statement_name: &str) -> Option<String> {
    parse_safe_repair_statements().remove(statement_name)
}

fn parse_safe_repair_statements() -> HashMap<String, String> {
    let mut blocks: HashMap<String, String> = HashMap::new();
    let mut active_name: Option<String> = None;
    let mut active_sql = String::new();

    for line in BOOTSTRAP_SQL.lines() {
        let trimmed = line.trim();

        if let Some(name) = trimmed.strip_prefix("-- driggsby:safe_repair:start:") {
            active_name = Some(name.to_string());
            active_sql.clear();
            continue;
        }

        if let Some(name) = trimmed.strip_prefix("-- driggsby:safe_repair:end:") {
            if let Some(active) = &active_name
                && active == name
            {
                blocks.insert(name.to_string(), active_sql.trim().to_string());
            }
            active_name = None;
            active_sql.clear();
            continue;
        }

        if active_name.is_some() {
            active_sql.push_str(line);
            active_sql.push('\n');
        }
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::safe_repair_statement;

    #[test]
    fn safe_repair_statement_exists_for_views_and_indexes() {
        for name in [
            "v1_transactions",
            "v1_accounts",
            "v1_imports",
            "v1_recurring",
            "v1_anomalies",
            "idx_internal_transactions_import_id",
            "idx_internal_transactions_account_posted_at",
            "idx_internal_transactions_account_external_id",
            "idx_internal_transactions_fallback_dedupe",
            "idx_internal_import_runs_created_at_desc",
            "idx_internal_transaction_dedupe_candidates_dedupe_key",
            "idx_internal_transaction_dedupe_candidates_import_id",
        ] {
            let sql = safe_repair_statement(name);
            assert!(sql.is_some());
        }
    }
}
