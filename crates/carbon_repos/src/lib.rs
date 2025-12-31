//! Database access layer for GDLauncher.
//!
//! This crate provides:
//! - Connection pooling via r2d2-sqlite
//! - Database migrations via rusqlite_migration
//! - Type-safe model structs with `FromRow` derive macro
//! - Validated SQL queries
//!
//! # Example
//!
//! ```ignore
//! use carbon_repos::{create_pool_default, migrations, queries, models};
//! use std::path::Path;
//!
//! // Create connection pool
//! let pool = create_pool_default(Path::new("gdl_conf.db"))?;
//!
//! // Run migrations
//! let mut conn = pool.get()?;
//! migrations::run_migrations(&mut conn)?;
//!
//! // Query data
//! let settings = conn.query_row(
//!     queries::settings::GetSettings::SQL,
//!     [],
//!     |row| models::AppConfiguration::from_row(row),
//! )?;
//! ```

pub mod connection;
pub mod context;
pub mod error;
pub mod migrations;
pub mod models;
pub mod queries;

// Re-export commonly used types at crate root
pub use connection::{
    AsConnection, DbConn, DbPool, OptionalExt, PoolConfig, batch_execute, create_pool,
    create_pool_default, with_transaction,
};
pub use context::DbContext;
pub use error::DatabaseError;
pub use migrations::{
    get_current_version, get_migrations, has_pending_migrations, migration_count, run_migrations,
};

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_full_database_setup() {
        // Create in-memory database
        let mut conn = Connection::open_in_memory().unwrap();

        // Run migrations
        run_migrations(&mut conn).unwrap();

        // Verify version
        assert_eq!(get_current_version(&conn), migration_count());

        // Verify we can query
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM AppConfiguration", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0); // No data seeded yet
    }

    #[test]
    fn test_models_and_queries_integration() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();

        // Seed a settings row
        conn.execute(
            "INSERT INTO AppConfiguration (id, releaseChannel, xmx) VALUES (0, 'stable', 4096)",
            [],
        )
        .unwrap();

        // Query using our typed query
        let settings = queries::settings::GetSettings::fetch_one(&conn).unwrap();

        assert_eq!(settings.id, 0);
        assert_eq!(settings.release_channel, "stable");
        assert_eq!(settings.xmx, 4096);
    }
}
