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
    OpenFailed(DbRecoveryCause),
}

/// Recovery guidance uses SQLite primary codes and typed refusals, never
/// error text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DbRecoveryCause {
    Locked,
    Access,
    Storage,
    /// A 2024 (Prisma-era) migration history this build refuses to convert:
    /// restarting or updating cannot change it.
    History,
}
impl DbRecoveryCause {
    fn wire(&self) -> &'static str {
        match self {
            Self::Locked => "locked",
            Self::Access => "access",
            Self::Storage => "storage",
            Self::History => "history",
        }
    }
}
fn recovery_cause(err: &DbError) -> Option<DbRecoveryCause> {
    let error = match err {
        DbError::LegacyHistory(_) => return Some(DbRecoveryCause::History),
        DbError::Sqlite(rusqlite::Error::SqliteFailure(error, _)) => error,
        _ => return None,
    };
    match error.code {
        rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked => {
            Some(DbRecoveryCause::Locked)
        }
        rusqlite::ErrorCode::CannotOpen
        | rusqlite::ErrorCode::ReadOnly
        | rusqlite::ErrorCode::PermissionDenied => Some(DbRecoveryCause::Access),
        rusqlite::ErrorCode::DiskFull | rusqlite::ErrorCode::SystemIoFailure => {
            Some(DbRecoveryCause::Storage)
        }
        _ => None,
    }
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
            DbStatus::OpenFailed(cause) => format!("_STATUS_:DB_MIGRATION_FAILED|{}", cause.wire()),
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
            DbStatus::MigrationFailed | DbStatus::OpenFailed(_) | DbStatus::Downgraded => {
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
        Err(e) => Err(recovery_cause(&e)
            .map(DbStatus::OpenFailed)
            .unwrap_or(DbStatus::MigrationFailed)),
    }
}

/// True when a rusqlite error is SQLite reporting a corrupt file or a
/// non-database file. Other primary codes can supply non-corruption recovery
/// guidance on `DB_MIGRATION_FAILED`; unknown errors keep its bare legacy form.
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

    // Journal-mode changes cannot run inside the atomic history conversion.
    conn.pragma_update(None, "journal_mode", &"WAL")?;
    migration_set.convert_legacy_history(&mut conn)?;

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
    fn task24_db_recovery_noncorruption_causes_have_structured_status() {
        for (code, category) in [
            (rusqlite::ErrorCode::DatabaseBusy, "locked"),
            (rusqlite::ErrorCode::DatabaseLocked, "locked"),
            (rusqlite::ErrorCode::CannotOpen, "access"),
            (rusqlite::ErrorCode::ReadOnly, "access"),
            (rusqlite::ErrorCode::PermissionDenied, "access"),
            (rusqlite::ErrorCode::DiskFull, "storage"),
            (rusqlite::ErrorCode::SystemIoFailure, "storage"),
        ] {
            let status = classify_open(Err(corrupt_sqlite_error(code))).unwrap_err();
            assert_eq!(
                status.status_line(),
                format!("_STATUS_:DB_MIGRATION_FAILED|{category}")
            );
            assert!(
                status
                    .into_error()
                    .downcast_ref::<DatabaseError>()
                    .unwrap()
                    .is_emitted_db_status()
            );
        }
    }

    #[test]
    fn task24_db_recovery_real_cannot_open_preserves_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gdl_conf.db");
        std::fs::create_dir(&path).unwrap();
        let snapshot = snapshot_path_for(&path);
        std::fs::write(&snapshot, b"synthetic previous snapshot").unwrap();
        let status =
            classify_open(migrate_db(&path, &carbon_repos::get_migrations().0)).unwrap_err();
        assert_eq!(status.status_line(), "_STATUS_:DB_MIGRATION_FAILED|access");
        assert_eq!(
            std::fs::read(snapshot).unwrap(),
            b"synthetic previous snapshot"
        );
        assert!(path.is_dir());
    }

    #[test]
    fn task24_db_recovery_real_persistent_lock_preserves_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gdl_conf.db");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE kept(value TEXT); INSERT INTO kept VALUES('synthetic'); BEGIN IMMEDIATE").unwrap();
        let snapshot = snapshot_path_for(&path);
        std::fs::write(&snapshot, b"prior snapshot").unwrap();
        let status =
            classify_open(migrate_db(&path, &carbon_repos::get_migrations().0)).unwrap_err();
        assert_eq!(status.status_line(), "_STATUS_:DB_MIGRATION_FAILED|locked");
        assert_eq!(
            conn.query_row("SELECT value FROM kept", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "synthetic"
        );
        assert_eq!(std::fs::read(snapshot).unwrap(), b"prior snapshot");
        conn.execute_batch("ROLLBACK").unwrap();
    }

    #[test]
    fn task24_db_recovery_transient_lock_uses_existing_busy_tolerance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gdl_conf.db");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; BEGIN IMMEDIATE")
            .unwrap();
        let unlock = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            conn.execute_batch("ROLLBACK").unwrap();
        });
        assert_eq!(
            classify_open(migrate_db(&path, &carbon_repos::get_migrations().0)),
            Ok(None)
        );
        unlock.join().unwrap();
        let current = rusqlite::Connection::open(&path).unwrap();
        assert_eq!(
            current
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            carbon_repos::get_migrations().0.count()
        );
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
        // A refused Prisma-era history has its own cause, never the unknown one.
        let history = classify_open(Err(DbError::LegacyHistory("refused".to_string())));
        assert_eq!(history, Err(DbStatus::OpenFailed(DbRecoveryCause::History)));
        assert_eq!(
            history.unwrap_err().status_line(),
            "_STATUS_:DB_MIGRATION_FAILED|history"
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

#[cfg(test)]
mod task15_bootstrap_tests {
    use super::*;
    #[test]
    fn task15_populated_prisma_shim_and_post_shim_upgrade_keep_identities() {
        for prisma in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("fixture.db");
            let (current, _) = carbon_repos::get_migrations();
            let mut old = carbon_repos::get_migrations().0;
            old.migrations.truncate(25);
            {
                let mut c = rusqlite::Connection::open(&path).unwrap();
                old.to_latest(&mut c).unwrap();
                c.execute_batch("INSERT INTO InstanceGroup(id,name,groupIndex) VALUES(1,'root',0);
             INSERT INTO Instance(id,name,shortpath,\"index\",groupId) VALUES(1,'fixture','fixture',0,1);
             INSERT INTO Java(id,path,major,fullVersion,type,os,arch,vendor,isValid) VALUES('java','fixture-java',21,'21','local','linux','x64','unknown',1);
             INSERT INTO JavaProfile(name,isSystemProfile,javaId) VALUES('profile',0,'java');
             INSERT INTO ModMetadata(id,murmur2,sha512,sha1,modloaders) VALUES('physical',1,x'01',x'02','forge');
             INSERT INTO ModFileCache(id,instanceId,filename,filesize,enabled,addonType,metadataId) VALUES('file',1,'mod.jar',4,0,'mods','physical');
             DROP TABLE _migrations;").unwrap();
                if prisma {
                    c.execute_batch("CREATE TABLE _prisma_migrations(id TEXT PRIMARY KEY,migration_name TEXT,finished_at TEXT,rolled_back_at TEXT);PRAGMA user_version=0").unwrap();
                    for (i, migration) in old.migrations.iter().enumerate() {
                        c.execute(
                            "INSERT INTO _prisma_migrations VALUES(?1,?2,'fixture-success',NULL)",
                            rusqlite::params![i.to_string(), migration.name],
                        )
                        .unwrap();
                    }
                }
            }
            assert_eq!(migrate_db(&path, &current).unwrap(), OpenVerdict::Proceed);
            assert_eq!(migrate_db(&path, &current).unwrap(), OpenVerdict::Proceed);
            let c = rusqlite::Connection::open(&path).unwrap();
            assert_eq!(
                c.query_row(
                    "SELECT id||':'||metadataId||':'||enabled FROM ModFileCache",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "file:physical:0"
            );
            assert_eq!(
                c.query_row(
                    "SELECT javaId FROM JavaProfile WHERE name='profile'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "java"
            );
            assert_eq!(
                c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                    .unwrap(),
                29
            );
            assert_eq!(
                c.query_row("SELECT count(*) FROM _migrations", [], |r| r
                    .get::<_, i32>(0))
                    .unwrap(),
                29
            );
            assert!(
                c.prepare("PRAGMA foreign_key_check")
                    .unwrap()
                    .query([])
                    .unwrap()
                    .next()
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[cfg(test)]
mod task21_legacy_tests {
    use super::*;
    fn fixture(prefix: usize) -> (tempfile::TempDir, PathBuf, MigrationSet) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.db");
        let (current, _) = carbon_repos::get_migrations();
        let mut old = carbon_repos::get_migrations().0;
        old.migrations.truncate(prefix);
        let mut conn = rusqlite::Connection::open(&path).unwrap();
        old.to_latest(&mut conn).unwrap();
        conn.execute_batch("DROP TABLE _migrations; PRAGMA user_version=0;
            CREATE TABLE _prisma_migrations(id TEXT PRIMARY KEY,migration_name TEXT,finished_at TEXT,rolled_back_at TEXT);
            INSERT INTO InstanceGroup(id,name,groupIndex) VALUES(77,'kept',0);").unwrap();
        for (i, def) in old.migrations.iter().enumerate() {
            conn.execute(
                "INSERT INTO _prisma_migrations VALUES(?1,?2,'completed',NULL)",
                rusqlite::params![i.to_string(), def.name],
            )
            .unwrap();
        }
        (dir, path, current)
    }
    fn version(path: &Path) -> i32 {
        rusqlite::Connection::open(path)
            .unwrap()
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    }
    fn history(path: &Path) -> i32 {
        rusqlite::Connection::open(path)
            .unwrap()
            .query_row("SELECT count(*) FROM _prisma_migrations", [], |r| r.get(0))
            .unwrap()
    }
    #[test]
    fn task21_legacy_rolled_back_and_failed_attempts_add_no_slots() {
        for prefix in [24, 25] {
            let (_dir, path, current) = fixture(prefix);
            let c = rusqlite::Connection::open(&path).unwrap();
            c.execute(
                "INSERT INTO _prisma_migrations VALUES('retry',?1,NULL,'rolled-back')",
                [current.migrations[24].name],
            )
            .unwrap();
            c.execute(
                "INSERT INTO _prisma_migrations VALUES('unfinished',?1,NULL,NULL)",
                [current.migrations[24].name],
            )
            .unwrap();
            c.execute("INSERT INTO _prisma_migrations VALUES('finished-rolled-back',?1,'completed','rolled-back')",[current.migrations[24].name]).unwrap();
            drop(c);
            assert_eq!(migrate_db(&path, &current).unwrap(), OpenVerdict::Proceed);
            let c = rusqlite::Connection::open(&path).unwrap();
            assert!(
                c.prepare("SELECT modpackPlatform FROM Server").is_ok(),
                "pending migration25 must actually run"
            );
            assert_eq!(version(&path), current.count());
            assert_eq!(
                c.query_row("SELECT name FROM InstanceGroup WHERE id=77", [], |r| r
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
                "kept"
            );
            assert_eq!(migrate_db(&path, &current).unwrap(), OpenVerdict::Proceed);
        }
    }
    #[test]
    fn task21_legacy_malformed_history_is_retained() {
        let (_dir, path, current) = fixture(25);
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch("DROP TABLE _prisma_migrations;CREATE TABLE _prisma_migrations(id TEXT,finished_at TEXT);INSERT INTO _prisma_migrations VALUES('bad','done')").unwrap();
        let before = carbon_repos::schema_dump::dump_schema(&c).unwrap();
        let history_schema: String = c
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='_prisma_migrations'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let error = migrate_db(&path, &current).unwrap_err().to_string();
        assert!(
            error.contains("required attempt columns")
                && error.contains("Original history retained")
                && error.contains("contact support"),
            "{error}"
        );
        assert_eq!(history(&path), 1);
        assert_eq!(version(&path), 0);
        assert_eq!(
            c.query_row(
                "SELECT sql FROM sqlite_master WHERE name='_prisma_migrations'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            history_schema
        );
        assert_eq!(
            c.query_row(
                "SELECT id||':'||finished_at FROM _prisma_migrations",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "bad:done"
        );
        assert_eq!(
            c.query_row("SELECT name FROM InstanceGroup WHERE id=77", [], |r| r
                .get::<_, String>(
                0
            ))
            .unwrap(),
            "kept"
        );
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='_migrations'",
                [],
                |r| r.get::<_, i32>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(carbon_repos::schema_dump::dump_schema(&c).unwrap(), before);
    }
    #[test]
    fn task21_legacy_gap_unknown_duplicate_and_schema_mismatch_refuse_without_mutation() {
        for case in ["gap", "unknown", "duplicate", "schema"] {
            let (_dir, path, current) = fixture(25);
            let c = rusqlite::Connection::open(&path).unwrap();
            match case {
                "gap" => {
                    c.execute("DELETE FROM _prisma_migrations WHERE id='3'", [])
                        .unwrap();
                }
                "unknown" => {
                    c.execute_batch("INSERT INTO _prisma_migrations VALUES('unknown','unknown-name','done',NULL)").unwrap();
                }
                "duplicate" => {
                    c.execute(
                        "INSERT INTO _prisma_migrations VALUES('duplicate',?1,'done',NULL)",
                        [current.migrations[24].name],
                    )
                    .unwrap();
                }
                _ => {
                    c.execute_batch("ALTER TABLE Server DROP COLUMN modpackPlatform")
                        .unwrap();
                }
            }
            let before = carbon_repos::schema_dump::dump_schema(&c).unwrap();
            let rows = history(&path);
            assert!(migrate_db(&path, &current).is_err(), "{case}");
            assert_eq!(history(&path), rows);
            assert_eq!(version(&path), 0);
            assert_eq!(carbon_repos::schema_dump::dump_schema(&c).unwrap(), before);
        }
    }
    #[test]
    fn task21_legacy_metadata_write_failure_rolls_conversion_back() {
        let (_dir, path, current) = fixture(25);
        let mut c = rusqlite::Connection::open(&path).unwrap();
        let mut prefix = carbon_repos::get_migrations().0;
        prefix.migrations.truncate(25);
        // Create the actual metadata shape without changing legacy schema/history.
        c.execute_batch("PRAGMA user_version=25").unwrap();
        prefix.to_latest(&mut c).unwrap();
        c.execute_batch("DELETE FROM _migrations;PRAGMA user_version=0;
            CREATE TEMP TRIGGER reject_second BEFORE INSERT ON _migrations WHEN NEW.version=2 BEGIN SELECT RAISE(ABORT,'synthetic late metadata failure');END;").unwrap();
        assert!(current.convert_legacy_history(&mut c).is_err());
        assert_eq!(
            c.query_row("SELECT count(*) FROM _migrations", [], |r| r
                .get::<_, i32>(0))
                .unwrap(),
            0
        );
        assert_eq!(history(&path), 25);
        assert_eq!(version(&path), 0);
        c.execute_batch("DROP TRIGGER reject_second").unwrap();
        current.convert_legacy_history(&mut c).unwrap();
        assert_eq!(version(&path), 25);
        assert_eq!(
            c.query_row(
                "SELECT checksum FROM _migrations WHERE version=25",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            current.checksum(25)
        );
        assert_eq!(migrate_db(&path, &current).unwrap(), OpenVerdict::Proceed);
    }
    #[test]
    fn task21_legacy_valid_checksum_and_contradictory_owned_metadata() {
        for bad in [false, true] {
            let (_dir, path, current) = fixture(25);
            let c = rusqlite::Connection::open(&path).unwrap();
            c.execute_batch("ALTER TABLE _prisma_migrations ADD COLUMN checksum TEXT")
                .unwrap();
            for i in 0..25 {
                c.execute(
                    "UPDATE _prisma_migrations SET checksum=?1 WHERE id=?2",
                    rusqlite::params![current.checksum(i + 1), i.to_string()],
                )
                .unwrap();
            }
            if bad {
                c.execute_batch("UPDATE _prisma_migrations SET checksum='changed' WHERE id='24'")
                    .unwrap();
            }
            let result = migrate_db(&path, &current);
            if bad {
                assert!(result.is_err());
                assert_eq!(history(&path), 25);
                assert_eq!(version(&path), 0);
            } else {
                assert_eq!(result.unwrap(), OpenVerdict::Proceed);
            }
        }
        let (_dir, path, current) = fixture(25);
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch("PRAGMA user_version=24").unwrap();
        assert!(migrate_db(&path, &current).is_err());
        assert_eq!(history(&path), 25);
        assert_eq!(version(&path), 24);
    }
    #[test]
    fn task21_legacy_conversion_does_not_promise_pending_upgrade_atomicity() {
        let (_dir, path, mut current) = fixture(25);
        current.migrations.push(carbon_repos::compat::MigrationDef {
            name: "synthetic_pending_failure",
            up_sql: "SELECT nonexistent_function();",
            down_sql: None,
            kind: carbon_repos::compat::MigrationKind::Additive,
            data_down: "full",
        });
        assert!(migrate_db(&path, &current).is_err());
        assert_eq!(version(&path), 29);
        let c = rusqlite::Connection::open(&path).unwrap();
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='_prisma_migrations'",
                [],
                |r| r.get::<_, i32>(0)
            )
            .unwrap(),
            0
        );
        current.migrations.pop();
        assert_eq!(migrate_db(&path, &current).unwrap(), OpenVerdict::Proceed);
    }
    #[test]
    fn task21_legacy_absent_history_fresh_and_clean_prefix_controls() {
        let dir = tempfile::tempdir().unwrap();
        let fresh = dir.path().join("fresh.db");
        let current = carbon_repos::get_migrations().0;
        assert_eq!(migrate_db(&fresh, &current).unwrap(), OpenVerdict::Proceed);
        assert_eq!(version(&fresh), 29);
        for prefix in [24, 25] {
            let (_dir, path, current) = fixture(prefix);
            assert_eq!(migrate_db(&path, &current).unwrap(), OpenVerdict::Proceed);
            let c = rusqlite::Connection::open(&path).unwrap();
            assert!(
                c.prepare("PRAGMA foreign_key_check")
                    .unwrap()
                    .query([])
                    .unwrap()
                    .next()
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                c.query_row("SELECT count(*) FROM _migrations", [], |r| r
                    .get::<_, i32>(0))
                    .unwrap(),
                29
            );
        }
    }
}

/// Legacy conversion against the history table Prisma's engine really wrote
/// (2.0.0-2.0.9, before 2.0.11-beta dropped it): Prisma's exact SQLite DDL,
/// DATETIME columns holding INTEGER `timestamp_millis()`, sha256 checksums of
/// the migration scripts. The task15/task21 fixtures above use a reduced table
/// with TEXT timestamps and never reach the decode path real users take.
#[cfg(test)]
mod prisma_era_history_tests {
    use super::*;
    use carbon_repos::compat::sha256_hex;
    use rusqlite::Connection;

    /// prisma-engines (pcr-0.6.5) SQLite flavour `_prisma_migrations` DDL.
    const PRISMA_MIGRATIONS_DDL: &str = r#"CREATE TABLE "_prisma_migrations" ("id" TEXT PRIMARY KEY NOT NULL, "checksum" TEXT NOT NULL, "finished_at" DATETIME, "migration_name" TEXT NOT NULL, "logs" TEXT, "rolled_back_at" DATETIME, "started_at" DATETIME NOT NULL DEFAULT current_timestamp, "applied_steps_count" INTEGER UNSIGNED NOT NULL DEFAULT 0)"#;

    /// 2024-03-31: when the last Prisma-era builds were writing these rows.
    const BASE_MILLIS: i64 = 1_711_900_000_000;

    /// The number of migrations the last Prisma-era build (2.0.9) shipped.
    const PRISMA_ERA_COUNT: usize = 11;

    /// One `_prisma_migrations` row, bound the way quaint bound it: every
    /// timestamp as an i64 of milliseconds.
    struct Attempt<'a> {
        id: String,
        name: &'a str,
        checksum: String,
        started: i64,
        finished: Option<i64>,
        rolled_back: Option<i64>,
        logs: Option<&'a str>,
        steps: i64,
    }

    fn insert(conn: &Connection, a: Attempt) {
        conn.execute(
            "INSERT INTO _prisma_migrations \
             (id, checksum, finished_at, migration_name, logs, rolled_back_at, started_at, applied_steps_count) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                a.id,
                a.checksum,
                a.finished,
                a.name,
                a.logs,
                a.rolled_back,
                a.started,
                a.steps
            ],
        )
        .unwrap();
    }

    fn completed<'a>(id: &str, def: &'a carbon_repos::compat::MigrationDef, i: i64) -> Attempt<'a> {
        let started = BASE_MILLIS + i * 60_000;
        Attempt {
            id: id.to_string(),
            name: def.name,
            checksum: sha256_hex(def.up_sql.as_bytes()),
            started,
            finished: Some(started + 250),
            rolled_back: None,
            logs: None,
            steps: 1,
        }
    }

    /// A gdl_conf.db as a Prisma-era build left it: Prisma created its history
    /// table, then applied each of the first `prefix` real migration scripts
    /// and recorded one completed attempt per script. Prisma never set
    /// `user_version` and never created `_migrations`. Carries a signed-in
    /// account plus the user data an upgrade must keep: settings and an
    /// instance in a group.
    fn prisma_era_db(prefix: usize) -> (tempfile::TempDir, PathBuf, MigrationSet) {
        prisma_era_db_from(prefix, None)
    }

    /// [`prisma_era_db`], with `init` as the script Prisma applied (and
    /// checksummed) for the first migration instead of today's.
    fn prisma_era_db_from(
        prefix: usize,
        init: Option<&'static str>,
    ) -> (tempfile::TempDir, PathBuf, MigrationSet) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gdl_conf.db");
        let (current, _) = carbon_repos::get_migrations();
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        conn.execute_batch(PRISMA_MIGRATIONS_DDL).unwrap();
        for (i, def) in current.migrations.iter().take(prefix).enumerate() {
            let applied = match init {
                Some(script) if i == 0 => carbon_repos::compat::MigrationDef {
                    up_sql: script,
                    ..*def
                },
                _ => *def,
            };
            conn.execute_batch(applied.up_sql).unwrap();
            insert(
                &conn,
                completed(&format!("completed-{i}"), &applied, i as i64),
            );
        }
        conn.execute(
            "INSERT INTO Account (uuid, username, tokenExpires, lastUsed) VALUES ('prisma-era-uuid', 'PrismaEraUser', ?1, ?1)",
            [BASE_MILLIS],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO AppConfiguration (id, releaseChannel, xmx, activeAccountUuid, randomUserUuid, secret, metricsEnabledLastUpdate) \
             VALUES (0, 'stable', 4096, 'prisma-era-uuid', 'prisma-era-random', x'01', ?1)",
            [BASE_MILLIS],
        )
        .unwrap();
        conn.execute_batch(
            "INSERT INTO InstanceGroup (id, name, groupIndex) VALUES (1, 'Prisma era group', 0);
             INSERT INTO Instance (id, name, shortpath, \"index\", groupId) VALUES (1, 'Prisma era instance', 'prisma-era', 0, 1);",
        )
        .unwrap();
        assert_eq!(version(&path), 0);
        (dir, path, current)
    }

    /// Fails the fixture if it ever drifts back to TEXT timestamps: every
    /// timestamp Prisma wrote is stored as an INTEGER.
    fn assert_integer_millis(path: &Path) {
        let conn = Connection::open(path).unwrap();
        let (finished, not_integer): (i64, i64) = conn
            .query_row(
                "SELECT \
                   (SELECT COUNT(*) FROM _prisma_migrations WHERE typeof(finished_at) = 'integer'), \
                   (SELECT COUNT(*) FROM _prisma_migrations WHERE typeof(started_at) <> 'integer' \
                      OR typeof(finished_at) NOT IN ('integer', 'null') \
                      OR typeof(rolled_back_at) NOT IN ('integer', 'null'))",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(finished > 0, "fixture has no completed attempt");
        assert_eq!(not_integer, 0, "fixture stores a timestamp as non-INTEGER");
    }

    fn version(path: &Path) -> i32 {
        Connection::open(path)
            .unwrap()
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    }

    fn has_history(path: &Path) -> bool {
        Connection::open(path)
            .unwrap()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = '_prisma_migrations')",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }

    /// Every `_prisma_migrations` row, in a stable order, with storage classes.
    fn history_rows(path: &Path) -> Vec<String> {
        Connection::open(path)
            .unwrap()
            .prepare(
                "SELECT id || '|' || migration_name || '|' || checksum || '|' || \
                 ifnull(finished_at, '') || ':' || typeof(finished_at) || '|' || \
                 ifnull(rolled_back_at, '') || ':' || typeof(rolled_back_at) \
                 FROM _prisma_migrations ORDER BY id",
            )
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// `(version, name, checksum)` of every `_migrations` row, in order.
    fn recorded(path: &Path) -> Vec<(i32, String, String)> {
        Connection::open(path)
            .unwrap()
            .prepare("SELECT version, name, checksum FROM _migrations ORDER BY version")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn expected_rows(current: &MigrationSet, upto: i32) -> Vec<(i32, String, String)> {
        (1..=upto)
            .map(|v| {
                (
                    v,
                    current.migrations[(v - 1) as usize].name.to_string(),
                    current.checksum(v),
                )
            })
            .collect()
    }

    /// Checks one Prisma-era history twice on fresh copies: the conversion on
    /// its own must record exactly `expected` migrations, and the whole startup
    /// path (`migrate_db`) must then carry the database to this build's latest
    /// version with the user's data intact.
    fn assert_converts_to(
        prefix: usize,
        expected: i32,
        extra: impl Fn(&Connection, &MigrationSet),
    ) {
        assert_converts_from(None, prefix, expected, extra)
    }

    /// [`assert_converts_to`] for a history whose first script was `init`.
    fn assert_converts_from(
        init: Option<&'static str>,
        prefix: usize,
        expected: i32,
        extra: impl Fn(&Connection, &MigrationSet),
    ) {
        let (_dir, path, current) = prisma_era_db_from(prefix, init);
        extra(&Connection::open(&path).unwrap(), &current);
        assert_integer_millis(&path);
        let mut conn = Connection::open(&path).unwrap();
        current.convert_legacy_history(&mut conn).unwrap();
        drop(conn);
        assert_eq!(version(&path), expected);
        assert!(!has_history(&path));
        assert_eq!(recorded(&path), expected_rows(&current, expected));

        let (_dir, path, current) = prisma_era_db_from(prefix, init);
        extra(&Connection::open(&path).unwrap(), &current);
        assert_integer_millis(&path);
        assert_eq!(migrate_db(&path, &current).unwrap(), OpenVerdict::Proceed);
        assert_eq!(version(&path), current.count());
        assert!(!has_history(&path));
        assert_eq!(recorded(&path), expected_rows(&current, current.count()));
        let conn = Connection::open(&path).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT releaseChannel || ':' || xmx FROM AppConfiguration WHERE id = 0",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "stable:4096"
        );
        assert_eq!(
            conn.query_row(
                "SELECT g.name || ':' || i.name FROM Instance i JOIN InstanceGroup g ON g.id = i.groupId WHERE i.id = 1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "Prisma era group:Prisma era instance"
        );
        // 20241124163738_gdl_accounts signs every account out on purpose (the
        // Microsoft app id changed), exactly as it did for this cohort on
        // 2.0.11-2.0.40: proof that the forward chain really ran past it.
        assert_eq!(
            conn.query_row(
                "SELECT (SELECT COUNT(*) FROM Account) || ':' || \
                 ifnull((SELECT activeAccountUuid FROM AppConfiguration WHERE id = 0), 'none')",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "0:none"
        );
        assert!(
            conn.prepare("PRAGMA foreign_key_check")
                .unwrap()
                .query([])
                .unwrap()
                .next()
                .unwrap()
                .is_none()
        );
        drop(conn);
        assert_eq!(migrate_db(&path, &current).unwrap(), OpenVerdict::Proceed);
        assert_eq!(version(&path), current.count());
    }

    #[test]
    fn prisma_era_history_with_integer_millis_converts_and_migrates_to_latest() {
        assert_converts_to(PRISMA_ERA_COUNT, PRISMA_ERA_COUNT as i32, |_, _| {});
    }

    #[test]
    fn prisma_era_failed_and_rolled_back_attempts_before_a_retry_are_not_counted() {
        // Migration 10 failed (Prisma logged it, no `finished_at`), was marked
        // rolled back, then a retry succeeded: the fixture's completed row.
        // Migration 11 also carries an attempt that finished and was later
        // marked rolled back, so both INTEGER `rolled_back_at` and INTEGER
        // `finished_at` on a skipped row go through the decode. A count of
        // rows would say 13; only the 11 completed names apply.
        assert_converts_to(
            PRISMA_ERA_COUNT,
            PRISMA_ERA_COUNT as i32,
            |conn, current| {
                let tenth = &current.migrations[9];
                let started = BASE_MILLIS + 9 * 60_000 - 30_000;
                insert(
                    conn,
                    Attempt {
                        id: "failed-9".to_string(),
                        name: tenth.name,
                        checksum: sha256_hex(tenth.up_sql.as_bytes()),
                        started,
                        finished: None,
                        rolled_back: Some(started + 10_000),
                        logs: Some("Database error: database is locked"),
                        steps: 0,
                    },
                );
                let mut rolled_back = completed("rolled-back-10", &current.migrations[10], 9);
                rolled_back.rolled_back = Some(BASE_MILLIS + 9 * 60_000 + 5_000);
                insert(conn, rolled_back);
                let rows: i64 = conn
                    .query_row("SELECT COUNT(*) FROM _prisma_migrations", [], |r| r.get(0))
                    .unwrap();
                assert_eq!(rows, PRISMA_ERA_COUNT as i64 + 2);
            },
        );
    }

    /// `20240120134904_init` as 2.0.0-beta builds before e1e55175a
    /// (2024-01-29) applied it: `concurrentDownloads DEFAULT 8`, and the
    /// checksum Prisma recorded for those databases.
    const PRE_EDIT_INIT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/prisma_beta_init/20240120134904_init.sql"
    ));

    #[test]
    fn prisma_era_early_beta_init_checksum_converts_and_migrates_to_latest() {
        assert_eq!(
            sha256_hex(PRE_EDIT_INIT.as_bytes()),
            "51757b54d124de3e2cbcd9859ac0428a08a319b82626ec6887c2e34f30eee8ee"
        );
        assert_converts_from(
            Some(PRE_EDIT_INIT),
            PRISMA_ERA_COUNT,
            PRISMA_ERA_COUNT as i32,
            |conn, current| {
                let recorded: String = conn
                    .query_row(
                        "SELECT checksum FROM _prisma_migrations WHERE id = 'completed-0'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                assert_eq!(recorded, sha256_hex(PRE_EDIT_INIT.as_bytes()));
                assert_ne!(recorded, current.checksum(1));
            },
        );
    }

    #[test]
    fn prisma_era_early_beta_init_still_goes_through_the_schema_gate() {
        let (_dir, path, current) = prisma_era_db_from(PRISMA_ERA_COUNT, Some(PRE_EDIT_INIT));
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(r#"ALTER TABLE "AppConfiguration" DROP COLUMN "lastAppVersion""#)
            .unwrap();
        let rows = history_rows(&path);
        let error = migrate_db(&path, &current).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("schema does not match completed prefix 11"),
            "{error}"
        );
        assert_eq!(history_rows(&path), rows);
        assert_eq!(version(&path), 0);
        // The alias is for that one script: the same checksum on another
        // migration is still a mismatch.
        let (_dir, path, current) = prisma_era_db(PRISMA_ERA_COUNT);
        Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE _prisma_migrations SET checksum = ?1 WHERE id = 'completed-1'",
                [sha256_hex(PRE_EDIT_INIT.as_bytes())],
            )
            .unwrap();
        let error = migrate_db(&path, &current).unwrap_err().to_string();
        assert!(error.contains("checksum mismatch for"), "{error}");
        assert_eq!(version(&path), 0);
    }

    #[test]
    fn prisma_era_two_point_zero_point_one_prefix_converts_and_migrates_forward() {
        assert_converts_to(9, 9, |_, _| {});
    }

    #[test]
    fn prisma_era_inconsistent_history_is_refused_at_its_gate_without_mutation() {
        for (case, reason) in [
            ("gap", "do not form a contiguous prefix"),
            ("unknown", "unknown completed migration 'unknown-name'"),
            ("duplicate", "duplicate completed migration"),
            ("checksum", "checksum mismatch for"),
            ("schema", "schema does not match completed prefix 11"),
        ] {
            let (_dir, path, current) = prisma_era_db(PRISMA_ERA_COUNT);
            let conn = Connection::open(&path).unwrap();
            let last = &current.migrations[PRISMA_ERA_COUNT - 1];
            match case {
                "gap" => {
                    conn.execute(
                        "DELETE FROM _prisma_migrations WHERE id = 'completed-2'",
                        [],
                    )
                    .unwrap();
                }
                "unknown" => {
                    let mut attempt = completed("unknown", last, 20);
                    attempt.name = "unknown-name";
                    insert(&conn, attempt);
                }
                "duplicate" => insert(&conn, completed("duplicate", last, 20)),
                "checksum" => {
                    conn.execute(
                        "UPDATE _prisma_migrations SET checksum = 'changed' WHERE id = ?1",
                        [format!("completed-{}", PRISMA_ERA_COUNT - 1)],
                    )
                    .unwrap();
                }
                // The history claims migration 11 completed, but the column it
                // adds is missing from the schema.
                _ => {
                    conn.execute_batch(
                        r#"ALTER TABLE "AppConfiguration" DROP COLUMN "lastAppVersion""#,
                    )
                    .unwrap();
                }
            }
            assert_integer_millis(&path);
            let schema = carbon_repos::schema_dump::dump_schema(&conn).unwrap();
            let rows = history_rows(&path);

            let error = migrate_db(&path, &current).unwrap_err();
            let message = error.to_string();
            assert!(
                message.contains(reason) && message.contains("Original history retained"),
                "{case}: {message}"
            );
            assert_eq!(
                classify_open(Err(error)).unwrap_err().status_line(),
                "_STATUS_:DB_MIGRATION_FAILED|history",
                "{case}"
            );
            assert_eq!(history_rows(&path), rows, "{case}");
            assert_eq!(version(&path), 0, "{case}");
            assert_eq!(
                carbon_repos::schema_dump::dump_schema(&conn).unwrap(),
                schema,
                "{case}"
            );
        }
    }
}
