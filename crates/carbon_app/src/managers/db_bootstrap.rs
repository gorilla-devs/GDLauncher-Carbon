use super::{java::JavaManager, settings::terms_and_privacy::TermsAndPrivacy};
use crate::app_version::APP_VERSION;
use carbon_repos::compat::{MigrationSet, OpenVerdict, RefusalKind};
use carbon_repos::db_error::{DbError, DbResult};
use carbon_repos::repos::app_configuration::{self as app_config_repo, AppConfigurationPatch};
use carbon_repos::repos::frontend_preference as frontend_pref_repo;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use sysinfo::System;
use thiserror::Error;
use tracing::{debug, error, instrument, trace};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("error while ensuring java profiles in db")]
    EnsureProfiles(anyhow::Error),
    #[error("error while fetching latest terms and privacy checksum")]
    TermsAndPrivacy(anyhow::Error),
    #[error("database version is newer than app version (backwards migration)")]
    BackwardsMigration,
    #[error("database history diverged from this build at migration {0}")]
    Diverged(i32),
    #[error("database downgrade failed and was rolled back{}", .0.as_ref().map(|p| format!("; a snapshot was preserved at {p}")).unwrap_or_default())]
    DowngradeFailed(Option<String>),
    #[error("database file is corrupt or not a database")]
    Corrupt,
    #[error("database migration failed")]
    MigrationFailed,
}

impl DatabaseError {
    /// True for the fatal DB outcomes whose `_STATUS_:` line the runner already
    /// emitted through the funnel. The caller exits cleanly on these:
    /// the status line is the single signal Electron consumes, so panicking
    /// after a clean emission would double-signal and bury it under a backtrace.
    pub fn is_emitted_db_status(&self) -> bool {
        matches!(
            self,
            DatabaseError::BackwardsMigration
                | DatabaseError::Diverged(_)
                | DatabaseError::DowngradeFailed(_)
                | DatabaseError::Corrupt
                | DatabaseError::MigrationFailed
        )
    }
}

/// The terminal DB status funnel. Every fatal outcome of the
/// migration runner converts to one of these, and [`emit_status`] writes exactly
/// one `_STATUS_:` line — the single place that formats them, so the emittable
/// set is enumerable and test-locked. `Downgraded` is a non-fatal info line;
/// every other variant is fatal and aborts startup.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DbStatus {
    Downgraded,
    BackwardsMigration,
    Diverged(i32),
    /// Carries the pre-downgrade snapshot only when restoring it would change
    /// the database; the recovery screen hides its restore rung without one.
    DowngradeFailed(Option<String>),
    Corrupt,
    MigrationFailed,
}

impl DbStatus {
    /// The exact status line, format-locked to the existing protocol
    /// (`_STATUS_:<EVENT>[|payload]`). `BACKWARDS_MIGRATION` keeps its spelling
    /// so pre-floor Electron builds still parse it.
    fn status_line(&self) -> String {
        match self {
            DbStatus::Downgraded => "_STATUS_:DB_DOWNGRADED".to_string(),
            DbStatus::BackwardsMigration => "_STATUS_:BACKWARDS_MIGRATION".to_string(),
            DbStatus::Diverged(_) => "_STATUS_:DB_DIVERGED".to_string(),
            DbStatus::DowngradeFailed(Some(path)) => {
                format!("_STATUS_:DB_DOWNGRADE_FAILED|{path}")
            }
            DbStatus::DowngradeFailed(None) => "_STATUS_:DB_DOWNGRADE_FAILED".to_string(),
            DbStatus::Corrupt => "_STATUS_:DB_CORRUPT".to_string(),
            DbStatus::MigrationFailed => "_STATUS_:DB_MIGRATION_FAILED".to_string(),
        }
    }

    /// The `DatabaseError` a fatal status returns so the caller can downcast it
    /// (`is_emitted_db_status`) and exit cleanly. `Downgraded` is non-fatal and
    /// never reaches this path.
    fn into_error(self) -> anyhow::Error {
        match self {
            DbStatus::BackwardsMigration => DatabaseError::BackwardsMigration.into(),
            DbStatus::Diverged(v) => DatabaseError::Diverged(v).into(),
            DbStatus::DowngradeFailed(p) => DatabaseError::DowngradeFailed(p).into(),
            DbStatus::Corrupt => DatabaseError::Corrupt.into(),
            DbStatus::MigrationFailed | DbStatus::Downgraded => {
                DatabaseError::MigrationFailed.into()
            }
        }
    }
}

/// The single stdout funnel point: every DB status line passes
/// through here so no fatal path can bypass emission.
fn emit_status(status: &DbStatus) {
    println!("{}", status.status_line());
}

/// Maps the runner outcome to the funnel. `Ok(None)` proceeds silently;
/// `Ok(Some(_))` is a non-fatal info line to emit; `Err(_)` is a fatal status to
/// emit before aborting. Pure over its input, so the mapping is unit-tested per
/// class — the "emission per class" assertion.
fn classify_open(result: DbResult<OpenVerdict>) -> Result<Option<DbStatus>, DbStatus> {
    match result {
        Ok(OpenVerdict::Proceed) => Ok(None),
        Ok(OpenVerdict::Downgraded) => Ok(Some(DbStatus::Downgraded)),
        Ok(OpenVerdict::Refuse(RefusalKind::BackwardsMigration)) => {
            Err(DbStatus::BackwardsMigration)
        }
        Ok(OpenVerdict::Refuse(RefusalKind::Diverged { version })) => {
            Err(DbStatus::Diverged(version))
        }
        Ok(OpenVerdict::Refuse(RefusalKind::DowngradeFailed { snapshot_path })) => Err(
            DbStatus::DowngradeFailed(snapshot_path.map(|p| p.display().to_string())),
        ),
        Err(e) if is_corruption(&e) => Err(DbStatus::Corrupt),
        Err(_) => Err(DbStatus::MigrationFailed),
    }
}

/// True when a rusqlite error is SQLite reporting a corrupt file or a
/// non-database file — the only errors that map to `DB_CORRUPT` rather than the
/// generic `DB_MIGRATION_FAILED`.
fn is_corruption(err: &DbError) -> bool {
    matches!(
        err,
        DbError::Sqlite(rusqlite::Error::SqliteFailure(e, _))
            if matches!(
                e.code,
                rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase
            )
    )
}

/// The pre-downgrade snapshot path beside the database file (matching
/// `carbon_repos::compat`): `<stem>.pre-downgrade.db`.
fn snapshot_path_for(db_path: &Path) -> PathBuf {
    let stem = db_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("gdl_conf");
    db_path.with_file_name(format!("{stem}.pre-downgrade.db"))
}

/// Snapshot retention: keep the single most recent pre-downgrade
/// snapshot, delete it after the next fully-successful launch. Called once this
/// build has opened the database cleanly; a snapshot older than `session_start`
/// is from an earlier session and is now safe to drop, while one created during
/// this session's own down-run is newer and kept so the user can still roll back
/// to it.
fn cleanup_stale_snapshot(db_path: &Path, session_start: SystemTime) {
    let snapshot = snapshot_path_for(db_path);
    let Ok(modified) = std::fs::metadata(&snapshot).and_then(|m| m.modified()) else {
        return;
    };
    if modified < session_start {
        match std::fs::remove_file(&snapshot) {
            Ok(()) => debug!(
                "Removed stale pre-downgrade snapshot {}",
                snapshot.display()
            ),
            Err(e) => tracing::warn!(
                "Failed to remove stale pre-downgrade snapshot {}: {e}",
                snapshot.display()
            ),
        }
    }
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
    // Captured before the runner can take a pre-downgrade snapshot, so
    // `cleanup_stale_snapshot` can tell a snapshot made during THIS session
    // (keep — the user may still roll back to it) from one left by an earlier
    // session (delete now that the database opens cleanly).
    let session_start = SystemTime::now();

    let runtime_path = dunce::simplified(&runtime_path);

    let db_path = runtime_path.join("gdl_conf.db");

    let (migration_set, _migration_count) = carbon_repos::get_migrations();

    debug!("db path: {}", db_path.display());

    debug!("Starting migration procedure");

    // The runner applies pending migrations forward, overlays a newer
    // additive schema, or steps a newer breaking schema back down under a
    // verified snapshot. Every outcome funnels through `classify_open` into a
    // single `_STATUS_:` line: a fatal outcome emits and aborts; a
    // successful down-run emits the non-fatal `DB_DOWNGRADED` info line and
    // continues. `BACKWARDS_MIGRATION` keeps its exact meaning — a database
    // ahead of this build with no downgrade metadata (a pre-floor database).
    match classify_open(migrate_db(&db_path, &migration_set)) {
        Ok(info) => {
            if let Some(status) = info {
                emit_status(&status);
            }
        }
        Err(status) => {
            emit_status(&status);
            error!("Fatal database error: {}", status.status_line());
            return Err(status.into_error());
        }
    }

    // Foreign keys have been OFF for the app's entire life. Turn
    // them ON behind a fail-safe sweep: run it on a dedicated
    // connection with FKs OFF (so repair deletes do not cascade), then open the
    // runtime pools with FKs ON only if the DB is — or was repaired — clean.
    // `GDL_DISABLE_FK_ENFORCEMENT=1` skips the sweep and forces FKs OFF.
    let foreign_keys = decide_foreign_keys(&db_path)?;

    let db = std::sync::Arc::new(
        carbon_repos::db_exec::Db::open(&db_path, 4, foreign_keys)
            .map_err(|e| anyhow::anyhow!("failed to open sqlite executor: {e}"))?,
    );

    seed_init_db(&db, latest_consent_sha).await?;

    // Reached the successful-open point: drop a snapshot
    // left by an earlier session now that this build has opened the database
    // cleanly. A snapshot from this session's own down-run is newer than
    // `session_start` and is kept.
    cleanup_stale_snapshot(&db_path, session_start);

    Ok(LoadedDb { db })
}

/// Runs the migration runner behind the status funnel: opens the migration
/// connection, hardens sidecar permissions, applies the legacy
/// `_prisma_migrations` shim, then hands off to the bidirectional runner (spec
/// §9). Every step returns through `DbResult` so corruption or a runner refusal
/// is classified into a single `_STATUS_:` line by the caller — no fatal step
/// bypasses the funnel.
fn migrate_db(db_path: &Path, migration_set: &MigrationSet) -> DbResult<OpenVerdict> {
    let mut conn = rusqlite::Connection::open(db_path)?;
    // Ride out a transient lock (an AV scan, a backup tool, or a just-exiting
    // previous instance) rather than failing the migration instantly.
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    // On Unix, restrict the DB (and -wal/-shm sidecars) to 0600 since they
    // contain MS access/refresh tokens.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for sidecar in [
            db_path.to_path_buf(),
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

    // Installs created before the switch to the owned runner recorded their
    // applied migrations in a `_prisma_migrations` table and left `user_version`
    // at 0. Read that count to seed `user_version` so the runner resumes where
    // the legacy install left off instead of replaying every migration against a
    // populated database. A missing table (the normal case) errors and yields
    // `None`.
    let already_existing_migration_count: Option<i32> = conn
        .query_row("SELECT COUNT(*) FROM _prisma_migrations", [], |row| {
            row.get(0)
        })
        .ok();

    debug!(
        "Found {:?} applied migrations in the legacy migration table",
        already_existing_migration_count
    );

    conn.pragma_update(None, "journal_mode", &"WAL")?;

    if let Some(count) = already_existing_migration_count {
        conn.pragma_update(None, "user_version", &count)?;
    }

    let _ = conn.execute("DROP TABLE IF EXISTS _prisma_migrations", []);

    debug!("Running bidirectional migration runner");

    let verdict = migration_set.open(&mut conn, db_path)?;

    debug!("Closing migration connection");
    conn.close().map_err(|(_, e)| DbError::Sqlite(e))?;

    Ok(verdict)
}

/// Runs the FK sweep and returns whether the runtime pools should
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
    sweep_conn.busy_timeout(std::time::Duration::from_secs(5))?;
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
    // The heap below is sized from *total* RAM but checked against
    // *available* RAM at launch, so a 7 GB CI runner picks 3072 and then
    // refuses to start. 1024 is the floor: `xms` is created at 1024 and
    // nothing orders the pair, so lower values give the JVM `-Xms` > `-Xmx`.
    #[cfg(feature = "e2e")]
    if let Ok(raw) = std::env::var("GDL_E2E_DEFAULT_XMX_MB") {
        match raw.parse::<i32>() {
            Ok(mb) if mb > 0 => {
                tracing::warn!("E2E MODE: default xmx overridden to {mb} MB");
                return mb;
            }
            _ => tracing::warn!("E2E MODE: ignoring unusable GDL_E2E_DEFAULT_XMX_MB={raw:?}"),
        }
    }

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
    if app_config_repo::count_app_configuration(db).await? == 0 {
        trace!("No app configuration found. Creating default one");

        let installation_id = Uuid::new_v4().to_string();
        let release_channel = release_channel.clone();
        let xmx = find_appropriate_default_xmx().await;

        app_config_repo::insert_app_configuration(db, release_channel, xmx, Some(installation_id))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create default app configuration: {e}"))?;

        trace!("Created default app configuration");
    }

    let app_config = app_config_repo::get_app_configuration(db)
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
    let last_seen_version = frontend_pref_repo::get_preference(db, "last_seen_version")
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
        db.write(move |conn| Ok(query.execute(&conn)?)).await?;
    }

    JavaManager::ensure_profiles_in_db(db)
        .await
        .map_err(DatabaseError::EnsureProfiles)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    // --- Status funnel: every fatal DB class maps to
    // exactly one `_STATUS_:` line. `emit_status` is a single `println!` of
    // `status_line`, so locking the classification and the line text per class
    // is the "emission per class" assertion, verified in-process without
    // capturing stdout.

    fn corrupt_sqlite_error(code: rusqlite::ErrorCode) -> DbError {
        DbError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code,
                extended_code: 0,
            },
            Some("simulated".to_string()),
        ))
    }

    #[test]
    fn status_lines_are_format_locked_per_class() {
        assert_eq!(DbStatus::Downgraded.status_line(), "_STATUS_:DB_DOWNGRADED");
        assert_eq!(
            DbStatus::BackwardsMigration.status_line(),
            "_STATUS_:BACKWARDS_MIGRATION"
        );
        assert_eq!(DbStatus::Diverged(7).status_line(), "_STATUS_:DB_DIVERGED");
        assert_eq!(
            DbStatus::DowngradeFailed(Some("/tmp/gdl_conf.pre-downgrade.db".to_string()))
                .status_line(),
            "_STATUS_:DB_DOWNGRADE_FAILED|/tmp/gdl_conf.pre-downgrade.db"
        );
        assert_eq!(DbStatus::Corrupt.status_line(), "_STATUS_:DB_CORRUPT");
        assert_eq!(
            DbStatus::MigrationFailed.status_line(),
            "_STATUS_:DB_MIGRATION_FAILED"
        );
    }

    #[test]
    fn downgrade_failed_line_omits_the_payload_when_there_is_no_snapshot() {
        // The recovery screen keys its restore rung off the payload's presence,
        // so a rolled-back down-run must emit the bare event: restoring a
        // snapshot identical to the database loops on the recommended action.
        assert_eq!(
            DbStatus::DowngradeFailed(None).status_line(),
            "_STATUS_:DB_DOWNGRADE_FAILED"
        );
    }

    #[test]
    fn downgrade_failed_line_carries_a_windows_path_verbatim() {
        // The snapshot payload can contain a drive-letter colon; Electron parses
        // it by stripping the `_STATUS_:` prefix, so the path travels intact.
        let path = "C:\\Users\\gd\\gdl_conf.pre-downgrade.db";
        assert_eq!(
            DbStatus::DowngradeFailed(Some(path.to_string())).status_line(),
            format!("_STATUS_:DB_DOWNGRADE_FAILED|{path}")
        );
    }

    #[test]
    fn classify_open_maps_each_class() {
        assert_eq!(classify_open(Ok(OpenVerdict::Proceed)), Ok(None));
        assert_eq!(
            classify_open(Ok(OpenVerdict::Downgraded)),
            Ok(Some(DbStatus::Downgraded))
        );
        assert_eq!(
            classify_open(Ok(OpenVerdict::Refuse(RefusalKind::BackwardsMigration))),
            Err(DbStatus::BackwardsMigration)
        );
        assert_eq!(
            classify_open(Ok(OpenVerdict::Refuse(RefusalKind::Diverged {
                version: 3
            }))),
            Err(DbStatus::Diverged(3))
        );
        assert_eq!(
            classify_open(Ok(OpenVerdict::Refuse(RefusalKind::DowngradeFailed {
                snapshot_path: Some(PathBuf::from("/tmp/snap.db")),
            }))),
            Err(DbStatus::DowngradeFailed(Some("/tmp/snap.db".to_string())))
        );
        assert_eq!(
            classify_open(Err(corrupt_sqlite_error(
                rusqlite::ErrorCode::DatabaseCorrupt
            ))),
            Err(DbStatus::Corrupt)
        );
        assert_eq!(
            classify_open(Err(corrupt_sqlite_error(rusqlite::ErrorCode::NotADatabase))),
            Err(DbStatus::Corrupt)
        );
        assert_eq!(
            classify_open(Err(DbError::Conversion("boom".to_string()))),
            Err(DbStatus::MigrationFailed)
        );
        // A non-corruption sqlite failure is a generic migration failure.
        assert_eq!(
            classify_open(Err(corrupt_sqlite_error(
                rusqlite::ErrorCode::ConstraintViolation
            ))),
            Err(DbStatus::MigrationFailed)
        );
    }

    #[test]
    fn is_corruption_only_flags_corrupt_and_notadb() {
        assert!(is_corruption(&corrupt_sqlite_error(
            rusqlite::ErrorCode::DatabaseCorrupt
        )));
        assert!(is_corruption(&corrupt_sqlite_error(
            rusqlite::ErrorCode::NotADatabase
        )));
        assert!(!is_corruption(&corrupt_sqlite_error(
            rusqlite::ErrorCode::ConstraintViolation
        )));
        assert!(!is_corruption(&DbError::Conversion("boom".to_string())));
    }

    #[test]
    fn fatal_statuses_round_trip_to_a_downcastable_database_error() {
        for status in [
            DbStatus::BackwardsMigration,
            DbStatus::Diverged(2),
            DbStatus::DowngradeFailed(Some("/tmp/s.db".to_string())),
            DbStatus::Corrupt,
            DbStatus::MigrationFailed,
        ] {
            let err = status.into_error();
            let db_err = err
                .downcast_ref::<DatabaseError>()
                .expect("fatal status must carry a DatabaseError");
            assert!(
                db_err.is_emitted_db_status(),
                "{db_err:?} must be recognised as an emitted DB status so mod.rs exits cleanly"
            );
        }
    }

    #[tokio::test]
    async fn corrupt_db_file_reports_corrupt_status() {
        // A garbage file is opened lazily; the first header read (journal_mode
        // or the runner's sqlite_master scan) surfaces NOTADB/CORRUPT, funneling
        // to `DB_CORRUPT` rather than the generic failure.
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = dunce::canonicalize(temp_dir.into_path()).unwrap();
        std::fs::write(
            temp_path.join("gdl_conf.db"),
            b"this is definitely not a sqlite database file",
        )
        .unwrap();

        let err = load_and_migrate(temp_path, None)
            .await
            .err()
            .expect("a corrupt db file must abort startup");
        let db_err = err
            .downcast_ref::<DatabaseError>()
            .expect("corrupt db must surface a DatabaseError");
        assert!(
            matches!(db_err, DatabaseError::Corrupt),
            "expected Corrupt, got {db_err:?}"
        );
    }

    #[test]
    fn cleanup_removes_a_previous_sessions_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("gdl_conf.db");
        let snapshot = snapshot_path_for(&db_path);
        std::fs::write(&snapshot, b"old snapshot").unwrap();

        // The session started well after this snapshot was written: it is stale
        // and must be removed on a clean open.
        let session_start = SystemTime::now() + std::time::Duration::from_secs(3600);
        cleanup_stale_snapshot(&db_path, session_start);

        assert!(
            !snapshot.exists(),
            "a snapshot older than the session must be deleted"
        );
    }

    #[test]
    fn cleanup_keeps_a_snapshot_from_this_session() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("gdl_conf.db");
        let snapshot = snapshot_path_for(&db_path);
        std::fs::write(&snapshot, b"fresh snapshot").unwrap();

        // The session started before this snapshot was written (as a down-run
        // during startup would): it is the user's rollback point and must stay.
        let session_start = SystemTime::now() - std::time::Duration::from_secs(3600);
        cleanup_stale_snapshot(&db_path, session_start);

        assert!(
            snapshot.exists(),
            "a snapshot newer than the session must be kept"
        );
    }

    #[test]
    fn cleanup_is_a_noop_without_a_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("gdl_conf.db");
        // No snapshot file present; must not panic or error.
        cleanup_stale_snapshot(&db_path, SystemTime::now());
    }

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
            app_config_repo::get_app_configuration(&db_client.db)
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
                Ok(patch.build().map(|q| q.execute(&conn)).transpose()?)
            })
            .await
            .unwrap();

        let new_checksum = None;

        // Since it's a 500 we should not reset the status
        let db_client = load_and_migrate(temp_path, new_checksum).await.unwrap();

        assert_eq!(
            app_config_repo::get_app_configuration(&db_client.db)
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
