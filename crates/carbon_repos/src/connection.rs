//! Database connection pool and transaction helpers.
//!
//! This module provides r2d2-based SQLite connection pooling with proper
//! WAL mode configuration and helper functions for transactions.

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::Path;
use std::time::Duration;

use crate::error::DatabaseError;

/// Type alias for the database connection pool.
pub type DbPool = Pool<SqliteConnectionManager>;

/// Type alias for a pooled database connection.
pub type DbConn = PooledConnection<SqliteConnectionManager>;

/// Trait for types that can be used as a database connection.
///
/// This allows query methods to accept both `&Connection` and `&Transaction`,
/// as well as pooled connections.
///
/// # Example
///
/// ```ignore
/// fn do_query(conn: &impl AsConnection) -> rusqlite::Result<i32> {
///     conn.as_connection().query_row("SELECT 1", [], |r| r.get(0))
/// }
///
/// // Works with Connection
/// do_query(&conn)?;
///
/// // Works with Transaction
/// with_transaction(&mut conn, |tx| {
///     do_query(tx)?;
///     Ok(())
/// })?;
/// ```
pub trait AsConnection {
    /// Returns a reference to the underlying connection.
    fn as_connection(&self) -> &Connection;
}

impl AsConnection for Connection {
    fn as_connection(&self) -> &Connection {
        self
    }
}

impl AsConnection for rusqlite::Transaction<'_> {
    fn as_connection(&self) -> &Connection {
        self // Transaction derefs to Connection
    }
}

impl AsConnection for PooledConnection<SqliteConnectionManager> {
    fn as_connection(&self) -> &Connection {
        self // PooledConnection derefs to Connection
    }
}

impl<T: AsConnection + ?Sized> AsConnection for &T {
    fn as_connection(&self) -> &Connection {
        (*self).as_connection()
    }
}

/// Configuration for the database pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool.
    pub max_size: u32,
    /// Minimum number of idle connections to maintain.
    pub min_idle: Option<u32>,
    /// Connection timeout duration.
    pub connection_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 4,
            min_idle: Some(1),
            connection_timeout: Duration::from_secs(30),
        }
    }
}

/// SQLite pragma configuration applied to each connection.
const SQLITE_PRAGMAS: &str = r#"
    PRAGMA journal_mode = WAL;
    PRAGMA synchronous = NORMAL;
    PRAGMA temp_store = MEMORY;
    PRAGMA mmap_size = 30000000000;
    PRAGMA foreign_keys = ON;
    PRAGMA busy_timeout = 5000;
"#;

/// Creates a new connection pool with the given configuration.
///
/// Each connection is initialized with SQLite pragmas for optimal performance:
/// - WAL journal mode for better concurrency
/// - NORMAL synchronous mode for balance between safety and speed
/// - Memory temp store for faster temporary operations
/// - Memory mapping for improved I/O performance
/// - Foreign key enforcement enabled
/// - 5 second busy timeout for lock contention
pub fn create_pool(db_path: &Path, config: PoolConfig) -> Result<DbPool, DatabaseError> {
    let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
        conn.execute_batch(SQLITE_PRAGMAS)?;
        Ok(())
    });

    let pool = Pool::builder()
        .max_size(config.max_size)
        .min_idle(config.min_idle)
        .connection_timeout(config.connection_timeout)
        .build(manager)
        .map_err(DatabaseError::Pool)?;

    Ok(pool)
}

/// Creates a new connection pool with default configuration.
pub fn create_pool_default(db_path: &Path) -> Result<DbPool, DatabaseError> {
    create_pool(db_path, PoolConfig::default())
}

/// Executes a function within a transaction, automatically committing on success
/// or rolling back on error.
///
/// # Example
///
/// ```ignore
/// with_transaction(&mut conn, |tx| {
///     tx.execute("UPDATE users SET name = ?1 WHERE id = ?2", ["Alice", "1"])?;
///     tx.execute("INSERT INTO logs (message) VALUES (?1)", ["Updated user"])?;
///     Ok(())
/// })?;
/// ```
pub fn with_transaction<T, F>(conn: &mut Connection, f: F) -> Result<T, DatabaseError>
where
    F: FnOnce(&rusqlite::Transaction<'_>) -> Result<T, DatabaseError>,
{
    let tx = conn.transaction().map_err(DatabaseError::Query)?;
    let result = f(&tx)?;
    tx.commit().map_err(DatabaseError::Query)?;
    Ok(result)
}

/// Executes multiple independent SQL statements in a single transaction.
///
/// All statements are executed in order. If any statement fails, the entire
/// transaction is rolled back.
pub fn batch_execute(conn: &Connection, queries: &[&str]) -> Result<(), DatabaseError> {
    let tx = conn.unchecked_transaction().map_err(DatabaseError::Query)?;
    for query in queries {
        tx.execute(query, []).map_err(DatabaseError::Query)?;
    }
    tx.commit().map_err(DatabaseError::Query)?;
    Ok(())
}

/// Helper trait for optional query results.
pub trait OptionalExt<T> {
    /// Converts a query result to an Option, treating "no rows" as None.
    fn optional(self) -> Result<Option<T>, DatabaseError>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, DatabaseError> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::Query(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_pool() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let pool = create_pool_default(&db_path).unwrap();
        let conn = pool.get().unwrap();

        // Verify pragmas are set
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let foreign_keys: i32 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);
    }

    #[test]
    fn test_with_transaction_commit() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let pool = create_pool_default(&db_path).unwrap();
        let mut conn = pool.get().unwrap();

        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", [])
            .unwrap();

        with_transaction(&mut conn, |tx| {
            tx.execute("INSERT INTO test (value) VALUES (?1)", ["hello"])
                .map_err(DatabaseError::Query)?;
            Ok(())
        })
        .unwrap();

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_with_transaction_rollback() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let pool = create_pool_default(&db_path).unwrap();
        let mut conn = pool.get().unwrap();

        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", [])
            .unwrap();

        let result: Result<(), DatabaseError> = with_transaction(&mut conn, |tx| {
            tx.execute("INSERT INTO test (value) VALUES (?1)", ["hello"])
                .map_err(DatabaseError::Query)?;
            Err(DatabaseError::Custom("intentional error".into()))
        });

        assert!(result.is_err());

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
