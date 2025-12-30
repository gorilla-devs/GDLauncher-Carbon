//! Database migration management.
//!
//! This module handles database schema migrations using rusqlite_migration.
//! Migrations are auto-discovered from the migrations/ folder at compile time.

use rusqlite::Connection;

use crate::error::DatabaseError;

// Include auto-generated migration definitions (get_migrations, MIGRATION_COUNT)
include!(concat!(env!("OUT_DIR"), "/migrations_generated.rs"));

/// Returns the total number of migrations.
pub fn migration_count() -> i32 {
    MIGRATION_COUNT
}

/// Runs all pending migrations on the given connection.
///
/// This function also handles the one-time transition from legacy migration tracking
/// (the `_prisma_migrations` table used by older versions) to rusqlite_migration's
/// tracking (`user_version` pragma).
pub fn run_migrations(conn: &mut Connection) -> Result<(), DatabaseError> {
    // Handle legacy -> rusqlite_migration transition (one-time for existing databases)
    handle_legacy_migration_table(conn)?;

    // Check for backwards migration before attempting to migrate
    check_backwards_migration(conn)?;

    // Run pending migrations
    let migrations = get_migrations();
    migrations.to_latest(conn)?;

    Ok(())
}

/// Converts legacy migration tracking to rusqlite_migration format.
///
/// Older versions used a `_prisma_migrations` table to track applied migrations.
/// Current versions use the SQLite `user_version` pragma via rusqlite_migration.
/// This function converts between the two formats on first run for existing databases.
fn handle_legacy_migration_table(conn: &mut Connection) -> Result<(), DatabaseError> {
    // Check if the legacy migrations table exists
    let legacy_table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_prisma_migrations'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if legacy_table_exists {
        // Count applied legacy migrations
        let legacy_count: i32 = conn
            .query_row("SELECT COUNT(*) FROM _prisma_migrations", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        // Set user_version to match legacy migration count
        conn.pragma_update(None, "user_version", legacy_count)
            .map_err(DatabaseError::Query)?;

        // Drop the legacy tracking table
        conn.execute("DROP TABLE IF EXISTS _prisma_migrations", [])
            .map_err(DatabaseError::Query)?;

        tracing::info!(
            "Converted {} legacy migrations to rusqlite_migration format",
            legacy_count
        );
    }

    Ok(())
}

/// Checks if the database version is newer than what the app supports.
///
/// This prevents data loss from running an older app version against a
/// database that was migrated by a newer version.
fn check_backwards_migration(conn: &Connection) -> Result<(), DatabaseError> {
    let current_version = get_current_version(conn);
    let expected_version = migration_count();

    if current_version > expected_version {
        return Err(DatabaseError::BackwardsMigration {
            db_version: current_version,
            app_version: expected_version,
        });
    }

    Ok(())
}

/// Gets the current migration version from the database.
pub fn get_current_version(conn: &Connection) -> i32 {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap_or(0)
}

/// Returns true if there are pending migrations to apply.
pub fn has_pending_migrations(conn: &Connection) -> bool {
    get_current_version(conn) < migration_count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_count() {
        // Migration count is auto-discovered, just verify it's non-zero
        assert!(migration_count() > 0, "Should have at least one migration");
    }

    #[test]
    fn test_run_migrations_fresh_db() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();

        // Verify migrations were applied
        let version = get_current_version(&conn);
        assert_eq!(version, migration_count());

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(
            tables.contains(&"AppConfiguration".to_string()),
            "AppConfiguration table should exist"
        );
        assert!(
            tables.contains(&"Account".to_string()),
            "Account table should exist"
        );
        assert!(
            tables.contains(&"Instance".to_string()),
            "Instance table should exist"
        );
    }

    #[test]
    fn test_idempotent_migrations() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Run migrations twice
        run_migrations(&mut conn).unwrap();
        run_migrations(&mut conn).unwrap();

        // Should still have correct version
        assert_eq!(get_current_version(&conn), migration_count());
    }

    #[test]
    fn test_backwards_migration_detection() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();

        // Simulate a newer database version
        conn.pragma_update(None, "user_version", migration_count() + 5)
            .unwrap();

        // Should fail with backwards migration error
        let result = check_backwards_migration(&conn);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_backwards_migration());
    }

    #[test]
    fn test_legacy_table_conversion() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Create a fake legacy migrations table
        conn.execute(
            "CREATE TABLE _prisma_migrations (
                id TEXT PRIMARY KEY,
                migration_name TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        // Insert some fake migrations
        conn.execute(
            "INSERT INTO _prisma_migrations (id, migration_name) VALUES ('1', 'migration_1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO _prisma_migrations (id, migration_name) VALUES ('2', 'migration_2')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO _prisma_migrations (id, migration_name) VALUES ('3', 'migration_3')",
            [],
        )
        .unwrap();

        // Convert legacy table
        handle_legacy_migration_table(&mut conn).unwrap();

        // Check user_version was set
        assert_eq!(get_current_version(&conn), 3);

        // Check legacy table was dropped
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_prisma_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!table_exists);
    }
}
