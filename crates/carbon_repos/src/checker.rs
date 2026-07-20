//! Test-time query checker ("SQLx at test time", fixture-free) — checker v1.
//!
//! Verifies every registered `QueryCheck` against a real migrated schema:
//! the SQL prepares (catches unknown tables/columns/syntax), declared param
//! names resolve, multi-param queries use named params (never positional
//! `?`), and — when row metadata is present — every expected column shows up
//! in the result set. Exported so later plans/tasks can call it directly
//! instead of redefining it per test file.

use crate::registry::QueryCheck;
use rusqlite::Connection;

/// Checks every `QueryCheck` in `queries` against `conn`'s schema, returning
/// one human-readable violation string per problem found. An empty result
/// means every query passed.
pub fn check_module(conn: &Connection, queries: &[QueryCheck]) -> Vec<String> {
    let mut violations = Vec::new();
    for q in queries {
        // 1. prepare: syntax, tables, columns, params must exist
        let st = match conn.prepare(q.sql) {
            Ok(st) => st,
            Err(e) => {
                violations.push(format!("{}: does not prepare: {e}", q.name));
                continue;
            }
        };
        // 2. declared param names must all resolve
        for p in q.params {
            if st.parameter_index(p).ok().flatten().is_none() {
                violations.push(format!("{}: param {p} not present in SQL", q.name));
            }
        }
        // 3. multi-param queries must use named params (no bare '?')
        if q.params.len() > 1 && q.sql.contains('?') {
            violations.push(format!("{}: multi-param query uses positional '?'", q.name));
        }
        // 4. result shape vs COLUMNS metadata
        if let Some(cols) = q.columns {
            let actual: Vec<String> = st.column_names().iter().map(|s| s.to_string()).collect();
            for spec in cols {
                if !actual.iter().any(|a| a == spec.name) {
                    violations.push(format!(
                        "{}: column '{}' missing from result set {actual:?}",
                        q.name, spec.name
                    ));
                }
            }
        }
    }
    violations
}
