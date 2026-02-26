use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::Connection;

use crate::ClientResult;
use crate::contracts::types::ImportSignProfile;
use crate::import::CanonicalTransaction;
use crate::state::map_sqlite_error;

#[derive(Debug, Clone, Default)]
pub(crate) struct SignCounts {
    pub(crate) negative_count: i64,
    pub(crate) positive_count: i64,
}

impl SignCounts {
    pub(crate) fn total_count(&self) -> i64 {
        self.negative_count + self.positive_count
    }

    pub(crate) fn negative_ratio(&self) -> f64 {
        let total = self.total_count();
        if total <= 0 {
            return 0.0;
        }
        (self.negative_count as f64) / (total as f64)
    }

    fn positive_ratio(&self) -> f64 {
        let total = self.total_count();
        if total <= 0 {
            return 0.0;
        }
        (self.positive_count as f64) / (total as f64)
    }
}

pub(crate) fn existing_sign_count_map(
    connection: &Connection,
    db_path: &Path,
) -> ClientResult<BTreeMap<String, SignCounts>> {
    let mut statement = connection
        .prepare(
            "SELECT
                account_key,
                SUM(CASE WHEN amount < 0 THEN 1 ELSE 0 END) AS negative_count,
                SUM(CASE WHEN amount > 0 THEN 1 ELSE 0 END) AS positive_count
             FROM internal_transactions
             GROUP BY account_key
             ORDER BY account_key ASC",
        )
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                SignCounts {
                    negative_count: row.get::<_, i64>(1)?,
                    positive_count: row.get::<_, i64>(2)?,
                },
            ))
        })
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut map = BTreeMap::new();
    for row in rows {
        let (account_key, counts) = row.map_err(|error| map_sqlite_error(db_path, &error))?;
        map.insert(account_key, counts);
    }

    Ok(map)
}

pub(crate) fn incoming_sign_count_map<'a, I>(rows: I) -> BTreeMap<String, SignCounts>
where
    I: IntoIterator<Item = &'a CanonicalTransaction>,
{
    let mut map = BTreeMap::new();

    for row in rows {
        let entry = map
            .entry(row.account_key.clone())
            .or_insert_with(SignCounts::default);
        if row.amount < 0.0 {
            entry.negative_count += 1;
        } else if row.amount > 0.0 {
            entry.positive_count += 1;
        }
    }

    map
}

pub(crate) fn profiles_from_sign_counts(
    counts_by_account: &BTreeMap<String, SignCounts>,
) -> Vec<ImportSignProfile> {
    counts_by_account
        .iter()
        .map(|(account_key, counts)| ImportSignProfile {
            account_key: account_key.clone(),
            negative_count: counts.negative_count,
            positive_count: counts.positive_count,
            negative_ratio: counts.negative_ratio(),
            positive_ratio: counts.positive_ratio(),
            total_count: counts.total_count(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::import::CanonicalTransaction;

    use super::{SignCounts, incoming_sign_count_map, profiles_from_sign_counts};

    #[test]
    fn incoming_sign_count_map_tracks_positive_and_negative_amounts() {
        let rows = [
            CanonicalTransaction {
                statement_id: Some("acct_1_2026-01-31".to_string()),
                dedupe_scope_id: "stmt|acct_1|acct_1_2026-01-31".to_string(),
                account_key: "acct_1".to_string(),
                posted_at: "2026-01-01".to_string(),
                amount: -10.0,
                currency: "USD".to_string(),
                description: "a".to_string(),
                external_id: None,
                merchant: None,
                category: None,
            },
            CanonicalTransaction {
                statement_id: Some("acct_1_2026-01-31".to_string()),
                dedupe_scope_id: "stmt|acct_1|acct_1_2026-01-31".to_string(),
                account_key: "acct_1".to_string(),
                posted_at: "2026-01-02".to_string(),
                amount: 8.0,
                currency: "USD".to_string(),
                description: "b".to_string(),
                external_id: None,
                merchant: None,
                category: None,
            },
            CanonicalTransaction {
                statement_id: Some("acct_2_2026-01-31".to_string()),
                dedupe_scope_id: "stmt|acct_2|acct_2_2026-01-31".to_string(),
                account_key: "acct_2".to_string(),
                posted_at: "2026-01-03".to_string(),
                amount: 0.0,
                currency: "USD".to_string(),
                description: "c".to_string(),
                external_id: None,
                merchant: None,
                category: None,
            },
        ];

        let counts = incoming_sign_count_map(rows.iter());
        assert_eq!(counts.get("acct_1").map(|row| row.negative_count), Some(1));
        assert_eq!(counts.get("acct_1").map(|row| row.positive_count), Some(1));
        assert_eq!(counts.get("acct_2").map(|row| row.negative_count), Some(0));
        assert_eq!(counts.get("acct_2").map(|row| row.positive_count), Some(0));
    }

    #[test]
    fn profiles_are_sorted_and_ratios_are_derived_from_counts() {
        let counts_by_account = BTreeMap::from([
            (
                "acct_1".to_string(),
                SignCounts {
                    negative_count: 3,
                    positive_count: 1,
                },
            ),
            (
                "acct_2".to_string(),
                SignCounts {
                    negative_count: 0,
                    positive_count: 2,
                },
            ),
        ]);

        let profiles = profiles_from_sign_counts(&counts_by_account);
        assert_eq!(profiles[0].account_key, "acct_1");
        assert_eq!(profiles[0].negative_ratio, 0.75);
        assert_eq!(profiles[0].positive_ratio, 0.25);
        assert_eq!(profiles[1].account_key, "acct_2");
        assert_eq!(profiles[1].negative_ratio, 0.0);
        assert_eq!(profiles[1].positive_ratio, 1.0);
    }
}
