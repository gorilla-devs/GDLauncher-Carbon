use super::{java::JavaManager, settings::terms_and_privacy::TermsAndPrivacy};
use crate::app_version::APP_VERSION;
use carbon_repos::db::PrismaClient;
use carbon_repos::db::{self, app_configuration, frontend_preference};
use carbon_repos::db::{
    http_cache::{SetParam, WhereParam},
    read_filters::StringFilter,
};
use carbon_repos::pcr::raw;
use ring::rand::SecureRandom;
use serde::Deserialize;
use std::path::PathBuf;
use sysinfo::System;
use thiserror::Error;
use tracing::{debug, error, instrument, trace};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("error raised while trying to build the client for DB: {0}")]
    Client(#[from] carbon_repos::pcr::NewClientError),
    #[error("error while trying to migrate the database")]
    MigrationConn(#[from] rusqlite::Error),
    #[error("error while trying to migrate the database")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("error while trying to query db")]
    Query(#[from] carbon_repos::pcr::QueryError),
    #[error("error while ensuring java profiles in db")]
    EnsureProfiles(anyhow::Error),
    #[error("error while fetching latest terms and privacy checksum")]
    TermsAndPrivacy(anyhow::Error),
    #[error("database version is newer than app version (backwards migration)")]
    BackwardsMigration,
}

#[instrument]
pub(super) async fn load_and_migrate(
    runtime_path: PathBuf,
    latest_consent_sha: Option<String>,
) -> Result<PrismaClient, anyhow::Error> {
    let runtime_path = dunce::simplified(&runtime_path);

    let db_uri = format!(
        "file:{}?connection_limit=1",
        runtime_path.join("gdl_conf.db").to_str().unwrap()
    );

    let (migrations, migration_count) = carbon_repos::get_migrations();

    debug!("db uri: {}", db_uri);

    debug!("Starting migration procedure");

    let mut conn = rusqlite::Connection::open(&db_uri)?;

    let results: Result<i32, _> =
        conn.query_row("SELECT COUNT(*) FROM _prisma_migrations", [], |row| {
            row.get(0)
        });

    let already_existing_migration_count = match results {
        Ok(value) => Some(value),
        Err(err) => None,
    };

    debug!(
        "Found {:?} migrations from prisma. Converting them",
        already_existing_migration_count
    );

    conn.pragma_update(None, "journal_mode", &"WAL").unwrap();

    if let Some(already_existing_migration_count) = already_existing_migration_count {
        conn.pragma_update(None, "user_version", &already_existing_migration_count)?;
    }

    let _ = conn.execute("DROP TABLE IF EXISTS _prisma_migrations", []);

    // Check for backwards migration before attempting to migrate
    let user_version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap_or(0);

    if user_version > migration_count {
        debug!(
            "Backwards migration detected: database version {} > app migrations {}",
            user_version, migration_count
        );
        println!("_STATUS_:BACKWARDS_MIGRATION");
        return Err(DatabaseError::BackwardsMigration.into());
    }

    debug!("Migrating database");

    migrations.to_latest(&mut conn)?;

    debug!("Closing migration connection");

    conn.close().unwrap();

    debug!("Starting prisma connection");

    let db_client = db::new_client_with_url(&db_uri)
        .await
        .map_err(DatabaseError::Client)?;

    #[derive(Deserialize)]
    struct Whatever {}

    let _: Vec<Whatever> = db_client
        ._query_raw(raw!("PRAGMA journal_mode=WAL;"))
        .exec()
        .await
        .unwrap();
    let _: Vec<Whatever> = db_client
        ._query_raw(raw!("PRAGMA synchronous=normal;"))
        .exec()
        .await
        .unwrap();
    let _: Vec<Whatever> = db_client
        ._query_raw(raw!("PRAGMA temp_store=MEMORY;"))
        .exec()
        .await
        .unwrap();
    let _: Vec<Whatever> = db_client
        ._query_raw(raw!("PRAGMA mmap_size = 30000000000;"))
        .exec()
        .await
        .unwrap();

    // Maybe sweep expired HTTPCache rows + ensure expiresAt index. Both
    // operations hold the SQLite writer lock for their duration, so we gate
    // them on a cheap file-size estimate (PRAGMA page_count * page_size, no
    // table scan) to avoid hanging startup for users with huge DBs.
    maybe_sweep_http_cache_and_index(&db_client).await;

    seed_init_db(&db_client, latest_consent_sha).await?;

    Ok(db_client)
}

/// Skip auto-sweep entirely above this size — the in-app DB-bloat banner
/// (`pages/withAds.tsx`) takes over and the user cleans manually via the
/// CacheCleanup modal. Must stay in sync with `DB_BLOAT_THRESHOLD_BYTES` on
/// the frontend.
const DB_SKIP_SWEEP_BYTES: i64 = 2 * 1024 * 1024 * 1024;

/// Below this, the DB is essentially empty (fresh install, post-VACUUM, etc.)
/// and there's nothing to sweep — we still try to ensure the index since
/// it's free on an empty table.
const DB_SKIP_SWEEP_MIN_BYTES: i64 = 1 * 1024 * 1024;

/// Live-data threshold for ensuring the `HTTPCache(expiresAt)` index. The
/// CREATE INDEX itself holds the writer lock for an O(table) build, so we
/// only run it when the table is small enough that the build is brief
/// (sub-second on SSD, a few seconds on HDD).
const INDEX_BUILD_LIVE_BYTES: i64 = 200 * 1024 * 1024;

async fn read_pragma_i64(db_client: &db::PrismaClient, sql: &str) -> Option<i64> {
    #[derive(Deserialize)]
    struct PragmaRow {
        // SQLite returns the value as the column name of the pragma; deserializing
        // an arbitrary single-column row is annoying, so we use a flat map.
        #[serde(flatten)]
        fields: std::collections::HashMap<String, serde_json::Value>,
    }

    match db_client
        ._query_raw::<PragmaRow>(carbon_repos::pcr::raw::Raw::new(sql, vec![]))
        .exec()
        .await
    {
        Ok(rows) => rows
            .into_iter()
            .next()
            .and_then(|row| row.fields.into_iter().next())
            .and_then(|(_, v)| v.as_i64()),
        Err(e) => {
            tracing::warn!("[startup-timing] failed to read pragma `{sql}`: {e}");
            None
        }
    }
}

async fn read_db_sizes(db_client: &db::PrismaClient) -> Option<(i64, i64)> {
    let page_count = read_pragma_i64(db_client, "PRAGMA page_count;").await?;
    let page_size = read_pragma_i64(db_client, "PRAGMA page_size;").await?;
    let freelist = read_pragma_i64(db_client, "PRAGMA freelist_count;")
        .await
        .unwrap_or(0);
    let file_bytes = page_count.saturating_mul(page_size);
    let live_bytes = (page_count.saturating_sub(freelist)).saturating_mul(page_size);
    Some((file_bytes, live_bytes))
}

/// Try to ensure the `HTTPCache(expiresAt)` index, gated on live data being
/// small enough that the build is brief. Returns true if the index now
/// exists (whether we just created it or it already did).
async fn try_ensure_expires_at_index(db_client: &db::PrismaClient, live_bytes: i64) -> bool {
    if live_bytes >= INDEX_BUILD_LIVE_BYTES {
        tracing::info!(
            "[startup-timing] live data is {} MB (>={} MB), skipping index build",
            live_bytes / (1024 * 1024),
            INDEX_BUILD_LIVE_BYTES / (1024 * 1024)
        );
        return false;
    }
    let t = std::time::Instant::now();
    match db_client
        ._execute_raw(raw!(
            "CREATE INDEX IF NOT EXISTS HTTPCache_expiresAt_idx ON HTTPCache(expiresAt)"
        ))
        .exec()
        .await
    {
        Ok(_) => {
            tracing::info!(
                "[startup-timing] HTTPCache(expiresAt) index ensured in {:.2}s (live={} MB)",
                t.elapsed().as_secs_f64(),
                live_bytes / (1024 * 1024)
            );
            true
        }
        Err(e) => {
            tracing::warn!(
                "[startup-timing] failed to ensure HTTPCache(expiresAt) index in {:.2}s: {e}",
                t.elapsed().as_secs_f64()
            );
            false
        }
    }
}

async fn maybe_sweep_http_cache_and_index(db_client: &db::PrismaClient) {
    let Some((file_bytes, live_bytes)) = read_db_sizes(db_client).await else {
        // If we can't size the DB cheaply, do nothing — this is a best-effort
        // maintenance pass and shouldn't fail startup.
        return;
    };

    tracing::info!(
        "[startup-timing] db size estimate: file={} MB live={} MB",
        file_bytes / (1024 * 1024),
        live_bytes / (1024 * 1024)
    );

    if file_bytes > DB_SKIP_SWEEP_BYTES {
        tracing::info!(
            "[startup-timing] db file is {} MB (>{} MB), skipping HTTPCache sweep + index — banner will prompt user to clean manually",
            file_bytes / (1024 * 1024),
            DB_SKIP_SWEEP_BYTES / (1024 * 1024)
        );
        return;
    }

    // Build the index BEFORE the sweep when live data is small enough. This
    // turns the sweep's `WHERE expiresAt < ...` lookup from a full table scan
    // into an index seek — important for HDD users on the first launch after
    // this code ships, when no index exists yet.
    let pre_sweep_indexed = try_ensure_expires_at_index(db_client, live_bytes).await;

    if file_bytes >= DB_SKIP_SWEEP_MIN_BYTES {
        let t = std::time::Instant::now();
        match db_client
            ._execute_raw(raw!(
                "DELETE FROM HTTPCache WHERE expiresAt IS NOT NULL AND expiresAt < datetime('now')"
            ))
            .exec()
            .await
        {
            Ok(deleted) => tracing::info!(
                "[startup-timing] HTTPCache sweep deleted {deleted} row(s) in {:.2}s",
                t.elapsed().as_secs_f64()
            ),
            Err(e) => tracing::warn!(
                "[startup-timing] HTTPCache sweep failed in {:.2}s: {e}",
                t.elapsed().as_secs_f64()
            ),
        }
    } else {
        tracing::info!(
            "[startup-timing] db file is only {} MB, skipping sweep (nothing to clean)",
            file_bytes / (1024 * 1024)
        );
    }

    // If the index wasn't built pre-sweep (live was over the threshold), the
    // sweep may have moved enough rows to the freelist that live data has now
    // dropped below it — retry once so the next startup's sweep is fast.
    if !pre_sweep_indexed {
        let Some((_, live_after)) = read_db_sizes(db_client).await else {
            return;
        };
        try_ensure_expires_at_index(db_client, live_after).await;
    }
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
    db_client: &PrismaClient,
    latest_consent_sha: Option<String>,
) -> Result<(), anyhow::Error> {
    let release_channel = match APP_VERSION {
        v if v.contains("alpha") => "alpha",
        v if v.contains("beta") => "beta",
        _ => "stable",
    }
    .to_string();

    // Create base app config
    if db_client.app_configuration().count(vec![]).exec().await? == 0 {
        trace!("No app configuration found. Creating default one");

        let installation_id = Uuid::new_v4().to_string();

        db_client
            .app_configuration()
            .create(
                release_channel.clone(),
                find_appropriate_default_xmx().await,
                vec![app_configuration::installation_id::set(Some(
                    installation_id,
                ))],
            )
            .exec()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create default app configuration: {e}"))?;

        trace!("Created default app configuration");
    }

    let app_config = db_client
        .app_configuration()
        .find_unique(db::app_configuration::id::equals(0))
        .exec()
        .await?
        .expect("It's unreasonable to expect that the app configuration doesn't exist");

    let mut updates = vec![];

    // Ensure installation ID exists and is a valid UUID (migration path)
    let needs_new_installation_id = match &app_config.installation_id {
        None => true,
        Some(id) => Uuid::parse_str(id).is_err(), // Regenerate if not a valid UUID
    };

    if needs_new_installation_id {
        let installation_id = Uuid::new_v4().to_string();
        updates.push(app_configuration::installation_id::set(Some(
            installation_id,
        )));
        trace!("Generated installation ID for existing configuration");
    }

    // Check last seen version from FrontendPreference
    let last_seen_version = db_client
        .frontend_preference()
        .find_unique(frontend_preference::key::equals(
            "last_seen_version".to_string(),
        ))
        .exec()
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
        updates.push(app_configuration::release_channel::set(String::from(
            release_channel,
        )));
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
            updates.push(app_configuration::terms_and_privacy_accepted::set(false));
            updates.push(app_configuration::terms_and_privacy_accepted_checksum::set(
                None,
            ));
        }
    }

    db_client
        .app_configuration()
        .update(db::app_configuration::id::equals(0), updates)
        .exec()
        .await?;

    JavaManager::ensure_profiles_in_db(db_client)
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
                .app_configuration()
                .find_unique(db::app_configuration::id::equals(0))
                .exec()
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

        db_client
            .app_configuration()
            .update(
                db::app_configuration::id::equals(0),
                vec![
                    db::app_configuration::terms_and_privacy_accepted_checksum::set(
                        initial_checksum.clone(),
                    ),
                ],
            )
            .exec()
            .await
            .unwrap();

        let new_checksum = None;

        // Since it's a 500 we should not reset the status
        let db_client = load_and_migrate(temp_path, new_checksum).await.unwrap();

        assert_eq!(
            db_client
                .app_configuration()
                .find_unique(db::app_configuration::id::equals(0))
                .exec()
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
