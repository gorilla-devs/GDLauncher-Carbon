//! Database error types.

use thiserror::Error;

/// Database-related errors.
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Error from rusqlite query execution.
    #[error("database query error: {0}")]
    Query(#[from] rusqlite::Error),

    /// Error from connection pool operations.
    #[error("connection pool error: {0}")]
    Pool(#[source] r2d2::Error),

    /// Error getting a connection from the pool.
    #[error("failed to get connection from pool: {0}")]
    GetConnection(#[source] r2d2::Error),

    /// Error during database migration.
    #[error("migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    /// Database version is newer than the application supports.
    /// This happens when trying to run an older app version against a
    /// database that was migrated by a newer version.
    #[error(
        "database version {db_version} is newer than app version {app_version} (backwards migration not supported)"
    )]
    BackwardsMigration { db_version: i32, app_version: i32 },

    /// Custom error for application-specific database errors.
    #[error("{0}")]
    Custom(String),

    /// Record not found.
    #[error("record not found: {0}")]
    NotFound(String),

    /// Constraint violation (unique, foreign key, etc.).
    #[error("constraint violation: {0}")]
    Constraint(String),
}

impl DatabaseError {
    /// Creates a new custom error.
    pub fn custom<S: Into<String>>(msg: S) -> Self {
        Self::Custom(msg.into())
    }

    /// Creates a not found error.
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        Self::NotFound(msg.into())
    }

    /// Returns true if this error indicates no rows were returned.
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            DatabaseError::NotFound(_) | DatabaseError::Query(rusqlite::Error::QueryReturnedNoRows)
        )
    }

    /// Returns true if this is a backwards migration error.
    pub fn is_backwards_migration(&self) -> bool {
        matches!(self, DatabaseError::BackwardsMigration { .. })
    }
}

/// Extension trait for converting r2d2 pool errors.
impl From<r2d2::Error> for DatabaseError {
    fn from(err: r2d2::Error) -> Self {
        DatabaseError::GetConnection(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DatabaseError::custom("test error");
        assert_eq!(err.to_string(), "test error");

        let err = DatabaseError::not_found("user 123");
        assert_eq!(err.to_string(), "record not found: user 123");

        let err = DatabaseError::BackwardsMigration {
            db_version: 10,
            app_version: 5,
        };
        assert!(err.to_string().contains("backwards migration"));
    }

    #[test]
    fn test_is_not_found() {
        assert!(DatabaseError::not_found("test").is_not_found());
        assert!(DatabaseError::Query(rusqlite::Error::QueryReturnedNoRows).is_not_found());
        assert!(!DatabaseError::custom("other error").is_not_found());
    }
}
