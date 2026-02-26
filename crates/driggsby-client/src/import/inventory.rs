use std::collections::BTreeSet;
use std::path::Path;

use rusqlite::Connection;

use crate::ClientResult;
use crate::contracts::types::{ImportKeyInventory, ImportPropertyInventory, ImportValueCount};
use crate::import::CanonicalTransaction;
use crate::state::map_sqlite_error;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum TrackedProperty {
    AccountKey,
    AccountType,
    Currency,
    Merchant,
    Category,
}

impl TrackedProperty {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AccountKey => "account_key",
            Self::AccountType => "account_type",
            Self::Currency => "currency",
            Self::Merchant => "merchant",
            Self::Category => "category",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "account_key" => Some(Self::AccountKey),
            "account_type" => Some(Self::AccountType),
            "currency" => Some(Self::Currency),
            "merchant" => Some(Self::Merchant),
            "category" => Some(Self::Category),
            _ => None,
        }
    }

    fn column_name(self) -> &'static str {
        self.as_str()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct IncomingUniqueValues {
    pub(crate) account_key: BTreeSet<String>,
    pub(crate) account_type: BTreeSet<String>,
    pub(crate) currency: BTreeSet<String>,
    pub(crate) merchant: BTreeSet<String>,
    pub(crate) category: BTreeSet<String>,
}

pub(crate) fn query_key_inventory(
    connection: &Connection,
    db_path: &Path,
) -> ClientResult<ImportKeyInventory> {
    let total_rows = query_total_rows(connection, db_path)?;
    Ok(ImportKeyInventory {
        account_key: query_property_inventory(
            connection,
            db_path,
            TrackedProperty::AccountKey,
            total_rows,
        )?,
        account_type: query_property_inventory(
            connection,
            db_path,
            TrackedProperty::AccountType,
            total_rows,
        )?,
        currency: query_property_inventory(
            connection,
            db_path,
            TrackedProperty::Currency,
            total_rows,
        )?,
        merchant: query_property_inventory(
            connection,
            db_path,
            TrackedProperty::Merchant,
            total_rows,
        )?,
        category: query_property_inventory(
            connection,
            db_path,
            TrackedProperty::Category,
            total_rows,
        )?,
    })
}

pub(crate) fn query_property_inventory(
    connection: &Connection,
    db_path: &Path,
    property: TrackedProperty,
    total_rows: i64,
) -> ClientResult<ImportPropertyInventory> {
    let (null_sql, values_sql) = match property {
        TrackedProperty::AccountType => (
            "SELECT COUNT(*)
             FROM internal_transactions t
             LEFT JOIN internal_accounts a ON a.account_key = t.account_key
             WHERE a.account_type IS NULL OR TRIM(a.account_type) = ''"
                .to_string(),
            "SELECT a.account_type, COUNT(*)
             FROM internal_transactions t
             LEFT JOIN internal_accounts a ON a.account_key = t.account_key
             WHERE a.account_type IS NOT NULL AND TRIM(a.account_type) <> ''
             GROUP BY a.account_type
             ORDER BY a.account_type ASC"
                .to_string(),
        ),
        _ => {
            let column = property.column_name();
            (
                format!(
                    "SELECT COUNT(*) FROM internal_transactions WHERE {column} IS NULL OR TRIM({column}) = ''"
                ),
                format!(
                    "SELECT {column}, COUNT(*)
                     FROM internal_transactions
                     WHERE {column} IS NOT NULL AND TRIM({column}) <> ''
                     GROUP BY {column}
                     ORDER BY {column} ASC"
                ),
            )
        }
    };

    let null_count = connection
        .query_row(null_sql.as_str(), [], |row| row.get::<_, i64>(0))
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut statement = connection
        .prepare(values_sql.as_str())
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let rows = statement
        .query_map([], |row| {
            Ok(ImportValueCount {
                value: row.get::<_, String>(0)?,
                count: row.get::<_, i64>(1)?,
            })
        })
        .map_err(|error| map_sqlite_error(db_path, &error))?;

    let mut existing_values = Vec::new();
    let mut value_counts = Vec::new();
    for row in rows {
        let value_count = row.map_err(|error| map_sqlite_error(db_path, &error))?;
        existing_values.push(value_count.value.clone());
        value_counts.push(value_count);
    }

    Ok(ImportPropertyInventory {
        property: property.as_str().to_string(),
        unique_count: existing_values.len() as i64,
        existing_values,
        value_counts,
        null_count,
        total_rows,
    })
}

pub(crate) fn inventory_to_vec(inventory: &ImportKeyInventory) -> Vec<ImportPropertyInventory> {
    vec![
        inventory.account_key.clone(),
        inventory.account_type.clone(),
        inventory.currency.clone(),
        inventory.merchant.clone(),
        inventory.category.clone(),
    ]
}

pub(crate) fn incoming_unique_values<'a, I>(rows: I) -> IncomingUniqueValues
where
    I: IntoIterator<Item = &'a CanonicalTransaction>,
{
    let mut values = IncomingUniqueValues::default();

    for row in rows {
        values.account_key.insert(row.account_key.clone());
        if let Some(account_type) = row.account_type.as_ref()
            && !account_type.trim().is_empty()
        {
            values.account_type.insert(account_type.clone());
        }
        values.currency.insert(row.currency.clone());

        if let Some(merchant) = row.merchant.as_ref()
            && !merchant.trim().is_empty()
        {
            values.merchant.insert(merchant.clone());
        }

        if let Some(category) = row.category.as_ref()
            && !category.trim().is_empty()
        {
            values.category.insert(category.clone());
        }
    }

    values
}

fn query_total_rows(connection: &Connection, db_path: &Path) -> ClientResult<i64> {
    connection
        .query_row("SELECT COUNT(*) FROM internal_transactions", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|error| map_sqlite_error(db_path, &error))
}

#[cfg(test)]
mod tests {
    use crate::import::CanonicalTransaction;

    use super::{TrackedProperty, incoming_unique_values};

    #[test]
    fn tracked_property_parser_accepts_only_supported_values() {
        assert_eq!(
            TrackedProperty::parse("account_key"),
            Some(TrackedProperty::AccountKey)
        );
        assert_eq!(
            TrackedProperty::parse("currency"),
            Some(TrackedProperty::Currency)
        );
        assert_eq!(
            TrackedProperty::parse("account_type"),
            Some(TrackedProperty::AccountType)
        );
        assert_eq!(
            TrackedProperty::parse("merchant"),
            Some(TrackedProperty::Merchant)
        );
        assert_eq!(
            TrackedProperty::parse("category"),
            Some(TrackedProperty::Category)
        );
        assert_eq!(TrackedProperty::parse("acct"), None);
    }

    #[test]
    fn incoming_unique_values_collects_expected_sets() {
        let rows = [
            CanonicalTransaction {
                statement_id: Some("acct_1_2026-01-31".to_string()),
                dedupe_scope_id: "stmt|acct_1|acct_1_2026-01-31".to_string(),
                account_key: "acct_1".to_string(),
                account_type: Some("checking".to_string()),
                posted_at: "2026-01-01".to_string(),
                amount: -1.0,
                currency: "USD".to_string(),
                description: "a".to_string(),
                external_id: None,
                merchant: Some("Coffee".to_string()),
                category: Some("Food".to_string()),
            },
            CanonicalTransaction {
                statement_id: Some("acct_1_2026-01-31".to_string()),
                dedupe_scope_id: "stmt|acct_1|acct_1_2026-01-31".to_string(),
                account_key: "acct_1".to_string(),
                account_type: Some("checking".to_string()),
                posted_at: "2026-01-02".to_string(),
                amount: -2.0,
                currency: "USD".to_string(),
                description: "b".to_string(),
                external_id: None,
                merchant: Some("Coffee".to_string()),
                category: None,
            },
            CanonicalTransaction {
                statement_id: Some("acct_2_2026-01-31".to_string()),
                dedupe_scope_id: "stmt|acct_2|acct_2_2026-01-31".to_string(),
                account_key: "acct_2".to_string(),
                account_type: None,
                posted_at: "2026-01-03".to_string(),
                amount: 3.0,
                currency: "EUR".to_string(),
                description: "c".to_string(),
                external_id: None,
                merchant: None,
                category: Some("Travel".to_string()),
            },
        ];

        let values = incoming_unique_values(rows.iter());
        assert_eq!(
            values.account_key.into_iter().collect::<Vec<String>>(),
            vec!["acct_1".to_string(), "acct_2".to_string()]
        );
        assert_eq!(
            values.account_type.into_iter().collect::<Vec<String>>(),
            vec!["checking".to_string()]
        );
        assert_eq!(
            values.currency.into_iter().collect::<Vec<String>>(),
            vec!["EUR".to_string(), "USD".to_string()]
        );
        assert_eq!(
            values.merchant.into_iter().collect::<Vec<String>>(),
            vec!["Coffee".to_string()]
        );
        assert_eq!(
            values.category.into_iter().collect::<Vec<String>>(),
            vec!["Food".to_string(), "Travel".to_string()]
        );
    }
}
