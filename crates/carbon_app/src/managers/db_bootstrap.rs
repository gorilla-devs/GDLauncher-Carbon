use super::{java::JavaManager, settings::terms_and_privacy::TermsAndPrivacy};
use crate::app_version::APP_VERSION;
use carbon_repos::repos::app_configuration::{self as app_config_repo, AppConfigurationPatch};
use carbon_repos::repos::frontend_preference as frontend_pref_repo;
use std::path::PathBuf;
use sysinfo::System;
use thiserror::Error;
use tracing::{debug, error, instrument, trace};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("error while trying to migrate the database")]
    MigrationConn(#[from] rusqlite::Error),
    #[error("error while trying to migrate the database")]
    Migration(#[from] carbon_repos::db_error::DbError),
    #[error("error while ensuring java profiles in db")]
    EnsureProfiles(anyhow::Error),
    #[error("error while fetching latest terms and privacy checksum")]
    TermsAndPrivacy(anyhow::Error),
    #[error("database version is newer than app version (backwards migration)")]
    BackwardsMigration,
    #[error("database history diverged from this build at migration {0}")]
    Diverged(i32),
    #[error("database downgrade failed; a snapshot was preserved at {0}")]
    DowngradeFailed(String),
}

/// The rusqlite-backed executor, opened against the migrated on-disk database.
pub(super) struct LoadedDb {
    pub db: std::sync::Arc<carbon_repos::db_exec::Db>,
}

#[instrument]
pub(super) async fn load_and_migrate(
    runtime_path: PathBuf,
    latest_consent_sha: Option<String>,
) -> Result<LoadedDb, anyhow::Error> {
    let runtime_path = dunce::simplified(&runtime_path);

    let db_path = runtime_path.join("gdl_conf.db");

    let (migration_set, migration_count) = carbon_repos::get_migrations();

    debug!("db path: {}", db_path.display());

    debug!("Starting migration procedure");

    let mut conn = rusqlite::Connection::open(&db_path)?;

    // On Unix, restrict the DB (and -wal/-shm sidecars) to 0600 since they
    // contain MS access/refresh tokens.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for sidecar in [
            db_path.clone(),
            db_path.with_extension("db-wal"),
            db_path.with_extension("db-shm"),
        ] {
            if sidecar.exists() {
                let _ = std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o600));
            }
        }
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
    }

    // Installs created before the switch to rusqlite_migration recorded their
    // applied migrations in a `_prisma_migrations` table and left
    // `user_version` at 0. Read that count so it can seed `user_version`,
    // letting the migration runner resume from where the legacy install left
    // off instead of replaying every migration against a populated database.
    let results: Result<i32, _> =
        conn.query_row("SELECT COUNT(*) FROM _prisma_migrations", [], |row| {
            row.get(0)
        });

    let already_existing_migration_count = match results {
        Ok(value) => Some(value),
        Err(err) => None,
    };

    debug!(
        "Found {:?} applied migrations in the legacy migration table. Converting them",
        already_existing_migration_count
    );

    conn.pragma_update(None, "journal_mode", &"WAL")
        .map_err(|e| anyhow::anyhow!("Failed to set journal_mode=WAL: {e}"))?;

    if let Some(already_existing_migration_count) = already_existing_migration_count {
        conn.pragma_update(None, "user_version", &already_existing_migration_count)?;
    }

    let _ = conn.execute("DROP TABLE IF EXISTS _prisma_migrations", []);

    debug!("Running bidirectional migration runner");

    // The runner (spec §9) either applies pending migrations forward, overlays a
    // newer additive schema, or steps a newer breaking schema back down under a
    // verified snapshot. Every fatal outcome funnels through a single
    // `_STATUS_:` line before returning (spec §13). `BACKWARDS_MIGRATION` keeps
    // its exact meaning: a database ahead of this build with no downgrade
    // metadata (a pre-floor database).
    match migration_set.open(&mut conn, &db_path) {
        Ok(carbon_repos::compat::OpenVerdict::Proceed) => {}
        Ok(carbon_repos::compat::OpenVerdict::Downgraded) => {
            // A newer schema was stepped back to this build's version and
            // verified against ground truth. Non-fatal: startup continues.
            println!("_STATUS_:DB_DOWNGRADED");
        }
        Ok(carbon_repos::compat::OpenVerdict::Refuse(
            carbon_repos::compat::RefusalKind::BackwardsMigration,
        )) => {
            debug!(
                "Backwards migration detected: database is ahead of this build's {} migrations with no downgrade metadata",
                migration_count
            );
            println!("_STATUS_:BACKWARDS_MIGRATION");
            return Err(DatabaseError::BackwardsMigration.into());
        }
        Ok(carbon_repos::compat::OpenVerdict::Refuse(
            carbon_repos::compat::RefusalKind::Diverged { version },
        )) => {
            error!("Database history diverged from this build at migration {version}");
            println!("_STATUS_:DB_DIVERGED");
            return Err(DatabaseError::Diverged(version).into());
        }
        Ok(carbon_repos::compat::OpenVerdict::Refuse(
            carbon_repos::compat::RefusalKind::DowngradeFailed { snapshot_path },
        )) => {
            let snapshot = snapshot_path.display().to_string();
            error!("Database downgrade failed; snapshot preserved at {snapshot}");
            println!("_STATUS_:DB_DOWNGRADE_FAILED|{snapshot}");
            return Err(DatabaseError::DowngradeFailed(snapshot).into());
        }
        Err(e) => {
            error!("Database migration failed: {e}");
            println!("_STATUS_:DB_MIGRATION_FAILED");
            return Err(DatabaseError::from(e).into());
        }
    }

    debug!("Closing migration connection");

    conn.close()
        .map_err(|(_, e)| anyhow::anyhow!("Failed to close migration DB connection: {e}"))?;

    // Foreign keys have been OFF for the app's entire life (spec §2.3). Turn
    // them ON behind a fail-safe sweep (spec §7): run it on a dedicated
    // connection with FKs OFF (so repair deletes do not cascade), then open the
    // runtime pools with FKs ON only if the DB is — or was repaired — clean.
    // `GDL_DISABLE_FK_ENFORCEMENT=1` skips the sweep and forces FKs OFF.
    let foreign_keys = decide_foreign_keys(&db_path)?;

    let db = std::sync::Arc::new(
        carbon_repos::db_exec::Db::open(&db_path, 4, foreign_keys)
            .map_err(|e| anyhow::anyhow!("failed to open sqlite executor: {e}"))?,
    );

    seed_init_db(&db, latest_consent_sha).await?;

    Ok(LoadedDb { db })
}

/// Runs the FK sweep (spec §7) and returns whether the runtime pools should
/// enable foreign-key enforcement. Never fails startup on integrity grounds: an
/// unrepairable violation or a sweep error falls back to FKs OFF (today's
/// behavior) and reports to Sentry, and the app continues.
fn decide_foreign_keys(db_path: &std::path::Path) -> Result<bool, anyhow::Error> {
    if std::env::var("GDL_DISABLE_FK_ENFORCEMENT").as_deref() == Ok("1") {
        tracing::warn!(
            "GDL_DISABLE_FK_ENFORCEMENT=1 set; skipping FK sweep and leaving foreign keys OFF"
        );
        return Ok(false);
    }

    let mut sweep_conn = rusqlite::Connection::open(db_path)?;
    // Match the migration connection: FKs OFF so repair deletes do not cascade
    // under the sweep (`foreign_key_check` works regardless of this pragma).
    sweep_conn.pragma_update(None, "foreign_keys", &"OFF")?;

    let enforce = match carbon_repos::fk::sweep_and_decide(&mut sweep_conn) {
        Ok(carbon_repos::fk::SweepOutcome::Enabled) => true,
        Ok(carbon_repos::fk::SweepOutcome::DisabledFallback { violations }) => {
            let msg = format!(
                "FK sweep left {} unrepairable violation(s); running with foreign keys OFF for this session",
                violations.len()
            );
            tracing::warn!("{msg}");
            sentry::capture_message(&msg, sentry::Level::Warning);
            false
        }
        Err(e) => {
            let msg =
                format!("FK sweep errored ({e}); running with foreign keys OFF for this session");
            tracing::warn!("{msg}");
            sentry::capture_message(&msg, sentry::Level::Warning);
            false
        }
    };

    sweep_conn
        .close()
        .map_err(|(_, e)| anyhow::anyhow!("Failed to close FK sweep connection: {e}"))?;
    Ok(enforce)
}

async fn find_appropriate_default_xmx() -> i32 {
    let mut memory = System::new();
    memory.refresh_memory();

    match memory.total_memory() / 1024 / 1024 {
        0..=4096 => 1024,
        4097..=6144 => 2048,
        6145..=8192 => 3072,
        _ => 4096,
    }
}

/// Checks if an installation ID falls within the beta prompt cohort.
/// Uses the same approach as electron-updater's staged rollouts:
/// converts the first 8 hex chars of the UUID to a percentage (0-1).
pub fn is_in_beta_prompt_cohort(installation_id: &str, percentage: f64) -> bool {
    // Parse the first 8 hex characters of the UUID as a u32
    let hex_prefix = installation_id
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(8)
        .collect::<String>();

    if hex_prefix.len() < 8 {
        return false;
    }

    let value = u32::from_str_radix(&hex_prefix, 16).unwrap_or(u32::MAX);
    let normalized = value as f64 / u32::MAX as f64;

    normalized < percentage
}

async fn seed_init_db(
    db: &carbon_repos::db_exec::Db,
    latest_consent_sha: Option<String>,
) -> Result<(), anyhow::Error> {
    let release_channel = match APP_VERSION {
        v if v.contains("alpha") => "alpha",
        v if v.contains("beta") => "beta",
        _ => "stable",
    }
    .to_string();

    // Create base app config
    if db.read(|conn| Ok(app_config_repo::count_app_configuration(conn)?)).await? == 0 {
        trace!("No app configuration found. Creating default one");

        let installation_id = Uuid::new_v4().to_string();
        let release_channel = release_channel.clone();
        let xmx = find_appropriate_default_xmx().await;

        db.write(move |conn| {
            Ok(app_config_repo::insert_app_configuration(
                conn,
                &release_channel,
                xmx,
                Some(&installation_id),
            )?)
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create default app configuration: {e}"))?;

        trace!("Created default app configuration");
    }

    let app_config = db
        .read(|conn| Ok(app_config_repo::get_app_configuration(conn)?))
        .await?
        .expect("It's unreasonable to expect that the app configuration doesn't exist");

    let mut patch = AppConfigurationPatch::default();

    // Ensure installation ID exists and is a valid UUID (migration path)
    let needs_new_installation_id = match &app_config.installation_id {
        None => true,
        Some(id) => Uuid::parse_str(id).is_err(), // Regenerate if not a valid UUID
    };

    if needs_new_installation_id {
        let installation_id = Uuid::new_v4().to_string();
        patch.installation_id = Some(Some(installation_id));
        trace!("Generated installation ID for existing configuration");
    }

    // Check last seen version from FrontendPreference
    let last_seen_version = db
        .read(|conn| Ok(frontend_pref_repo::get_preference(conn, "last_seen_version")?))
        .await?
        .map(|pref| pref.value);

    let is_equal_to_current_version = last_seen_version
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

    if should_force_release_channel {
        patch.release_channel = Some(String::from(release_channel));
    }

    // Emit status for frontend progress tracking
    println!("_STATUS_:VerifyingTermsAndPrivacy");

    if latest_consent_sha.is_some() {
        let mut should_empty_tos_privacy = false;

        if app_config.terms_and_privacy_accepted_checksum != latest_consent_sha {
            should_empty_tos_privacy = true;
        }

        tracing::info!(
            "Should empty tos_privacy: {}, latest tos_privacy checksum: {}, current tos_privacy checksum: {:?}",
            should_empty_tos_privacy,
            latest_consent_sha.expect("We just checked .is_some()"),
            app_config.terms_and_privacy_accepted_checksum
        );

        if should_empty_tos_privacy {
            patch.terms_and_privacy_accepted = Some(false);
            patch.terms_and_privacy_accepted_checksum = Some(None);
        }
    }

    if let Some(query) = patch.build() {
        db.write(move |conn| Ok(query.execute(conn)?)).await?;
    }

    JavaManager::ensure_profiles_in_db(db)
        .await
        .map_err(DatabaseError::EnsureProfiles)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_migrate_tos_privacy_should_reset_status_200() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = dunce::canonicalize(temp_dir.into_path()).unwrap();

        let initial_checksum = Some(String::from("initial"));

        let db_client = load_and_migrate(temp_path.clone(), initial_checksum)
            .await
            .unwrap();

        let new_checksum = Some(String::from("new"));

        let db_client = load_and_migrate(temp_path, new_checksum).await.unwrap();

        assert_eq!(
            db_client
                .db
                .read(|conn| Ok(app_config_repo::get_app_configuration(conn)?))
                .await
                .unwrap()
                .unwrap()
                .terms_and_privacy_accepted_checksum,
            None
        );
    }

    #[tokio::test]
    async fn test_migrate_tos_privacy_should_not_reset_status_500() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = dunce::canonicalize(temp_dir.into_path()).unwrap();
        let initial_checksum = Some(String::from("initial"));

        let db_client = load_and_migrate(temp_path.clone(), initial_checksum.clone())
            .await
            .unwrap();

        let checksum_to_set = initial_checksum.clone();
        db_client
            .db
            .write(move |conn| {
                let patch = AppConfigurationPatch {
                    terms_and_privacy_accepted_checksum: Some(checksum_to_set),
                    ..Default::default()
                };
                Ok(patch.build().map(|q| q.execute(conn)).transpose()?)
            })
            .await
            .unwrap();

        let new_checksum = None;

        // Since it's a 500 we should not reset the status
        let db_client = load_and_migrate(temp_path, new_checksum).await.unwrap();

        assert_eq!(
            db_client
                .db
                .read(|conn| Ok(app_config_repo::get_app_configuration(conn)?))
                .await
                .unwrap()
                .unwrap()
                .terms_and_privacy_accepted_checksum,
            initial_checksum
        );
    }

    mod beta_prompt_cohort {
        use super::super::is_in_beta_prompt_cohort;

        #[test]
        fn at_0_percent_never_includes() {
            assert!(!is_in_beta_prompt_cohort(
                "00000000-0000-0000-0000-000000000000",
                0.0
            ));
            assert!(!is_in_beta_prompt_cohort(
                "ffffffff-ffff-ffff-ffff-ffffffffffff",
                0.0
            ));
        }

        #[test]
        fn at_100_percent_includes_almost_all() {
            // At 100%, everyone except the theoretical max ID is included
            assert!(is_in_beta_prompt_cohort(
                "00000000-0000-0000-0000-000000000000",
                1.0
            ));
            // fffffffe is just below max, should be included
            assert!(is_in_beta_prompt_cohort(
                "fffffffe-ffff-ffff-ffff-ffffffffffff",
                1.0
            ));
            // ffffffff gives normalized = 1.0, and check is strictly <, so excluded
            // This is an edge case (1 in 4.3 billion) and acceptable behavior
            assert!(!is_in_beta_prompt_cohort(
                "ffffffff-ffff-ffff-ffff-ffffffffffff",
                1.0
            ));
        }

        #[test]
        fn is_deterministic() {
            let id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
            let result1 = is_in_beta_prompt_cohort(id, 0.5);
            let result2 = is_in_beta_prompt_cohort(id, 0.5);
            assert_eq!(result1, result2);
        }

        #[test]
        fn low_id_in_small_cohort() {
            // ID starting with "00000000" → normalized ~0.0, should be in small cohorts
            assert!(is_in_beta_prompt_cohort(
                "00000000-1234-5678-9abc-def012345678",
                0.01
            ));
        }

        #[test]
        fn high_id_not_in_small_cohort() {
            // ID starting with "ffffffff" → normalized ~1.0, should NOT be in small cohorts
            assert!(!is_in_beta_prompt_cohort(
                "ffffffff-1234-5678-9abc-def012345678",
                0.99
            ));
        }

        #[test]
        fn invalid_id_too_short() {
            // IDs with < 8 hex chars should return false
            assert!(!is_in_beta_prompt_cohort("abc", 1.0));
            assert!(!is_in_beta_prompt_cohort("", 1.0));
        }

        #[test]
        fn handles_non_hex_characters() {
            // Should filter out non-hex chars and use remaining
            // "ab-cd-ef-12" has only 8 hex chars: abcdef12
            assert!(is_in_beta_prompt_cohort("ab-cd-ef-12", 1.0));
            // But "ab-cd" only has 4 hex chars, not enough
            assert!(!is_in_beta_prompt_cohort("ab-cd", 1.0));
        }

        #[test]
        fn boundary_at_3_percent() {
            // 3% of u32::MAX = 0.03 * 4294967295 ≈ 128849018 = 0x07AE147A
            // IDs below this threshold should be included
            assert!(is_in_beta_prompt_cohort(
                "07ae147a-0000-0000-0000-000000000000",
                0.03
            ));
            // IDs at or above should not be included (normalized >= 0.03)
            assert!(!is_in_beta_prompt_cohort(
                "07ae147b-0000-0000-0000-000000000000",
                0.03
            ));
        }

        #[test]
        fn mid_range_values() {
            // 50% threshold = 0x7FFFFFFF
            // ID starting with "40000000" → normalized ~0.25, should be in 50% cohort
            assert!(is_in_beta_prompt_cohort(
                "40000000-0000-0000-0000-000000000000",
                0.5
            ));
            // ID starting with "80000000" → normalized ~0.5, should NOT be in 50% cohort
            assert!(!is_in_beta_prompt_cohort(
                "80000000-0000-0000-0000-000000000000",
                0.5
            ));
        }
    }
}
