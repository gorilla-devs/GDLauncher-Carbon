//! Database initialization and management.
//!
//! This module provides rusqlite-based database initialization and
//! connection pool management.

use crate::app_version::APP_VERSION;
use carbon_repos::{
    DbPool, PoolConfig, create_pool, migrations, models::AppConfiguration, queries,
};
use std::path::PathBuf;
use sysinfo::System;
use thiserror::Error;
use tracing::{debug, info, instrument, trace};

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("database connection pool error: {0}")]
    Pool(#[from] carbon_repos::DatabaseError),

    #[error("database query error: {0}")]
    Query(#[from] rusqlite::Error),

    #[error("database migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    #[error("database version is newer than app version (backwards migration)")]
    BackwardsMigration,

    #[error("failed to seed database: {0}")]
    Seed(String),

    #[error("error while fetching latest terms and privacy checksum: {0}")]
    TermsAndPrivacy(#[source] anyhow::Error),
}

/// Loads the database, runs migrations, and returns a connection pool.
#[instrument(skip(runtime_path, latest_consent_sha))]
pub fn load_and_migrate(
    runtime_path: PathBuf,
    latest_consent_sha: Option<String>,
) -> Result<DbPool, anyhow::Error> {
    let runtime_path = dunce::simplified(&runtime_path);
    let db_path = runtime_path.join("gdl_conf.db");

    debug!("Database path: {:?}", db_path);
    debug!("Starting migration procedure");

    // Create connection pool with proper pragmas
    let config = PoolConfig {
        max_size: 4,
        min_idle: Some(1),
        ..Default::default()
    };

    let pool = create_pool(&db_path, config)?;

    // Run migrations
    {
        let mut conn = pool.get()?;

        // Check for backwards migration before attempting to migrate
        let current_version = migrations::get_current_version(&conn);
        let expected_version = migrations::migration_count();

        if current_version > expected_version {
            debug!(
                "Backwards migration detected: database version {} > app migrations {}",
                current_version, expected_version
            );
            println!("_STATUS_:BACKWARDS_MIGRATION");
            return Err(DatabaseError::BackwardsMigration.into());
        }

        debug!(
            "Running migrations (current: {}, target: {})",
            current_version, expected_version
        );
        migrations::run_migrations(&mut conn)?;
        debug!("Migrations complete");
    }

    // Seed initial data
    seed_init_db(&pool, latest_consent_sha)?;

    Ok(pool)
}

/// Find appropriate default XMX based on system memory.
fn find_appropriate_default_xmx() -> i32 {
    let mut sys = System::new();
    sys.refresh_memory();

    match sys.total_memory() / 1024 / 1024 {
        0..=4096 => 1024,
        4097..=6144 => 2048,
        6145..=8192 => 3072,
        _ => 4096,
    }
}

/// Seeds initial database data if needed.
fn seed_init_db(pool: &DbPool, latest_consent_sha: Option<String>) -> Result<(), anyhow::Error> {
    let conn = pool.get()?;

    let release_channel = match APP_VERSION {
        v if v.contains("alpha") => "alpha",
        v if v.contains("beta") => "beta",
        _ => "stable",
    };

    // Check if app configuration exists
    let count: i32 = conn.query_row(queries::settings::CountSettings::SQL, [], |row| row.get(0))?;

    if count == 0 {
        trace!("No app configuration found. Creating default one");

        let xmx = find_appropriate_default_xmx();
        conn.execute(
            queries::settings::CreateSettings::SQL,
            rusqlite::params![release_channel, xmx, APP_VERSION],
        )?;

        trace!("Created default app configuration");
    }

    // Get current app configuration
    let app_config = conn.query_row(queries::settings::GetSettings::SQL, [], |row| {
        AppConfiguration::from_row(row)
    })?;

    // Determine what updates are needed
    let is_equal_to_current_version = app_config
        .last_app_version
        .as_ref()
        .map(|last_version| last_version == APP_VERSION)
        .unwrap_or(false);

    let should_force_release_channel =
        if APP_VERSION.contains("alpha") && !is_equal_to_current_version {
            true // Always force to alpha if running alpha
        } else if APP_VERSION.contains("beta") && !is_equal_to_current_version {
            // Only force to beta if current channel is stable
            // Don't force down from alpha to beta
            app_config.release_channel == "stable"
        } else {
            false
        };

    // Apply updates if needed
    if should_force_release_channel {
        conn.execute(
            queries::settings::UpdateReleaseChannel::SQL,
            [release_channel],
        )?;
    }

    // Emit status for frontend progress tracking
    println!("_STATUS_:VerifyingTermsAndPrivacy");

    if let Some(latest_sha) = &latest_consent_sha {
        let should_empty_tos_privacy =
            app_config.terms_and_privacy_accepted_checksum.as_ref() != Some(latest_sha);

        info!(
            "Should empty tos_privacy: {}, latest tos_privacy checksum: {}, current tos_privacy checksum: {:?}",
            should_empty_tos_privacy, latest_sha, app_config.terms_and_privacy_accepted_checksum
        );

        if should_empty_tos_privacy {
            conn.execute(
                queries::settings::UpdateTermsAndPrivacyAccepted::SQL,
                [false],
            )?;
            conn.execute(
                queries::settings::UpdateTermsAndPrivacyAcceptedChecksum::SQL,
                [Option::<String>::None],
            )?;
        }
    }

    // Ensure Java profiles exist
    ensure_java_profiles_in_db(&conn)?;

    Ok(())
}

/// Ensures the default Java profiles exist in the database.
fn ensure_java_profiles_in_db(conn: &rusqlite::Connection) -> Result<(), anyhow::Error> {
    use crate::domain::java::SystemJavaProfileName;
    use carbon_repos::queries::java;
    use strum::IntoEnumIterator;

    // Define required basic profiles
    let basic_profiles = [
        ("java8", true),
        ("java16", true),
        ("java17", true),
        ("java21", true),
    ];

    for (name, is_system) in &basic_profiles {
        // Check if profile exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM JavaProfile WHERE name = ?1",
                [name],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !exists {
            trace!("Creating Java profile: {}", name);
            conn.execute(
                java::CreateJavaProfile::SQL,
                rusqlite::params![name, is_system, Option::<String>::None],
            )?;
        }
    }

    // Create system Java profiles from SystemJavaProfileName enum
    for profile in SystemJavaProfileName::iter() {
        let profile_name = profile.to_string();
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM JavaProfile WHERE name = ?1",
                [&profile_name],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !exists {
            trace!("Creating system Java profile: {}", profile_name);
            conn.execute(
                java::CreateJavaProfile::SQL,
                rusqlite::params![&profile_name, true, Option::<String>::None],
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_and_migrate_fresh_db() {
        let dir = tempdir().unwrap();
        let pool = load_and_migrate(dir.path().to_path_buf(), None).unwrap();

        // Verify pool works
        let conn = pool.get().unwrap();

        // Verify app configuration was seeded
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM AppConfiguration", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);

        // Verify Java profiles were created
        let profile_count: i32 = conn
            .query_row("SELECT COUNT(*) FROM JavaProfile", [], |row| row.get(0))
            .unwrap();
        assert!(profile_count >= 4, "Should have at least 4 Java profiles");
    }

    #[test]
    fn test_load_and_migrate_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();

        // First run
        let pool1 = load_and_migrate(path.clone(), None).unwrap();
        drop(pool1);

        // Second run should succeed without errors
        let pool2 = load_and_migrate(path, None).unwrap();

        // Verify still just one app configuration
        let conn = pool2.get().unwrap();
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM AppConfiguration", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }
}
