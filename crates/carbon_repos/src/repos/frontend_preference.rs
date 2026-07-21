//! Repository queries for the `FrontendPreference` KV table.

use crate::dbtypes::DbDateTime;
use crate::queries;
use chrono::{DateTime, FixedOffset};

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct FrontendPreferenceRow {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<FixedOffset>,
}

queries! {
    fn get_preference(key: &str) -> Option<FrontendPreferenceRow> =
        "SELECT key, value, updatedAt FROM FrontendPreference WHERE key = :key";
    // `updatedAt` is written explicitly on every upsert (the freshness lint
    // guards this column).
    fn upsert_preference(key: &str, value: &str, updated_at: DbDateTime) -> usize =
        "INSERT INTO FrontendPreference (key, value, updatedAt) VALUES (:key, :value, :updated_at)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updatedAt = excluded.updatedAt";
    fn delete_preference(key: &str) -> usize =
        "DELETE FROM FrontendPreference WHERE key = :key";
}

/// Every checkable query in this module.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    QUERIES.to_vec()
}
