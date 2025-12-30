//! Async database context providing ergonomic database access.
//!
//! The `DbContext` wraps the connection pool and handles `spawn_blocking`
//! internally, providing an async-friendly API for database operations.
//!
//! # Example
//!
//! ```ignore
//! let ctx = DbContext::new(pool);
//!
//! // Query one row
//! let settings = ctx.query_one(|conn| {
//!     conn.query_row(
//!         queries::settings::GetSettings::SQL,
//!         [],
//!         AppConfiguration::from_row,
//!     )
//! }).await?;
//!
//! // Query optional row
//! let java = ctx.query_opt(|conn| {
//!     conn.query_row(
//!         queries::java::FindJavaById::SQL,
//!         [&id],
//!         Java::from_row,
//!     )
//! }).await?;
//! ```

use crate::{DatabaseError, DbPool};

/// Async-friendly database context that wraps `spawn_blocking` internally.
///
/// Clone is cheap as it only clones the underlying `Arc<Pool>`.
#[derive(Clone)]
pub struct DbContext {
    pool: DbPool,
}

impl DbContext {
    /// Creates a new database context from a connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Executes a query expecting exactly one row.
    ///
    /// Returns error if zero or multiple rows are returned.
    pub async fn query_one<T, F>(&self, f: F) -> Result<T, DatabaseError>
    where
        F: FnOnce(&rusqlite::Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            f(&conn).map_err(DatabaseError::Query)
        })
        .await
        .map_err(|e| DatabaseError::Custom(e.to_string()))?
    }

    /// Executes a query expecting zero or one row.
    ///
    /// Returns `Ok(None)` if no rows are found.
    pub async fn query_opt<T, F>(&self, f: F) -> Result<Option<T>, DatabaseError>
    where
        F: FnOnce(&rusqlite::Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            match f(&conn) {
                Ok(v) => Ok(Some(v)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(DatabaseError::Query(e)),
            }
        })
        .await
        .map_err(|e| DatabaseError::Custom(e.to_string()))?
    }

    /// Executes a query returning multiple rows.
    ///
    /// The closure should prepare the statement, execute query_map,
    /// and collect results.
    pub async fn query_all<T, F>(&self, f: F) -> Result<Vec<T>, DatabaseError>
    where
        F: FnOnce(&rusqlite::Connection) -> rusqlite::Result<Vec<T>> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            f(&conn).map_err(DatabaseError::Query)
        })
        .await
        .map_err(|e| DatabaseError::Custom(e.to_string()))?
    }

    /// Executes an INSERT, UPDATE, or DELETE statement.
    ///
    /// Returns the number of rows affected.
    pub async fn execute<F>(&self, f: F) -> Result<usize, DatabaseError>
    where
        F: FnOnce(&rusqlite::Connection) -> rusqlite::Result<usize> + Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            f(&conn).map_err(DatabaseError::Query)
        })
        .await
        .map_err(|e| DatabaseError::Custom(e.to_string()))?
    }

    /// Executes multiple operations within a transaction.
    ///
    /// The transaction is automatically committed on success or rolled
    /// back on error.
    pub async fn with_transaction<T, F>(&self, f: F) -> Result<T, DatabaseError>
    where
        F: FnOnce(&rusqlite::Transaction<'_>) -> Result<T, DatabaseError> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            crate::with_transaction(&mut conn, f)
        })
        .await
        .map_err(|e| DatabaseError::Custom(e.to_string()))?
    }

    /// Returns a reference to the underlying connection pool.
    ///
    /// Use this for advanced operations not covered by the context methods.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }
}

impl From<DbPool> for DbContext {
    fn from(pool: DbPool) -> Self {
        Self::new(pool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_pool_default, run_migrations};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_query_one() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = create_pool_default(&db_path).unwrap();

        {
            let mut conn = pool.get().unwrap();
            run_migrations(&mut conn).unwrap();
            conn.execute(
                "INSERT INTO AppConfiguration (id, releaseChannel, xmx) VALUES (0, 'stable', 4096)",
                [],
            )
            .unwrap();
        }

        let ctx = DbContext::new(pool);
        let count: i32 = ctx
            .query_one(|conn| {
                conn.query_row("SELECT COUNT(*) FROM AppConfiguration", [], |row| {
                    row.get(0)
                })
            })
            .await
            .unwrap();

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_query_opt_found() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = create_pool_default(&db_path).unwrap();

        {
            let mut conn = pool.get().unwrap();
            run_migrations(&mut conn).unwrap();
            conn.execute(
                "INSERT INTO AppConfiguration (id, releaseChannel, xmx) VALUES (0, 'stable', 4096)",
                [],
            )
            .unwrap();
        }

        let ctx = DbContext::new(pool);
        let result: Option<String> = ctx
            .query_opt(|conn| {
                conn.query_row(
                    "SELECT releaseChannel FROM AppConfiguration WHERE id = 0",
                    [],
                    |row| row.get(0),
                )
            })
            .await
            .unwrap();

        assert_eq!(result, Some("stable".to_string()));
    }

    #[tokio::test]
    async fn test_query_opt_not_found() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = create_pool_default(&db_path).unwrap();

        {
            let mut conn = pool.get().unwrap();
            run_migrations(&mut conn).unwrap();
        }

        let ctx = DbContext::new(pool);
        let result: Option<String> = ctx
            .query_opt(|conn| {
                conn.query_row(
                    "SELECT releaseChannel FROM AppConfiguration WHERE id = 999",
                    [],
                    |row| row.get(0),
                )
            })
            .await
            .unwrap();

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_execute() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = create_pool_default(&db_path).unwrap();

        {
            let mut conn = pool.get().unwrap();
            run_migrations(&mut conn).unwrap();
        }

        let ctx = DbContext::new(pool);
        let rows_affected = ctx
            .execute(|conn| {
                conn.execute(
                    "INSERT INTO AppConfiguration (id, releaseChannel, xmx) VALUES (0, 'stable', 4096)",
                    [],
                )
            })
            .await
            .unwrap();

        assert_eq!(rows_affected, 1);
    }
}
