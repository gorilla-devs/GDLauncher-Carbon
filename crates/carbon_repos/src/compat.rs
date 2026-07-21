//! Bidirectional migration mechanism (spec §9): the `_migrations` metadata
//! table, an owned per-migration applier, and the open logic that lets an older
//! binary open a database written by a newer one.
//!
//! The knowledge an old binary needs to step back down travels *inside* the
//! database. Every applied migration records — in the same transaction as its
//! `up` — its checksum, kind, and stored `down_sql`, so no reachable state ever
//! has a schema change without its antidote. When a binary opens a database
//! whose `user_version` is ahead of its own migration count, it reads those
//! rows and either overlays (all the extra migrations are additive, so the
//! newer schema is safe to run against untouched) or, if any is breaking, runs
//! the stored downs in reverse under a pre-taken file snapshot and verifies the
//! result against its own ground-truth schema before committing.
//!
//! The forward path ([`MigrationSet::to_latest`]) applies each pending
//! migration in its own transaction with foreign keys OFF on the migration
//! connection, counting `user_version` up one per applied migration, and
//! records each migration's metadata row in that same transaction.

use crate::db_error::{DbError, DbResult};
use crate::schema_dump::dump_schema;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Whether a migration can be safely overlaid by an older binary (`Additive`)
/// or requires a down-run to step back past it (`Breaking`).
///
/// `Additive` means the migration only adds objects an older binary ignores and
/// introduces no constraint capable of rejecting that binary's writes. Kind is
/// derived, not trusted, by the generator (spec §10.2, Task 3); this enum is the
/// runtime carrier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationKind {
    Additive,
    Breaking,
}

impl MigrationKind {
    /// The `kind` column's stored text form.
    pub fn as_str(self) -> &'static str {
        match self {
            MigrationKind::Additive => "additive",
            MigrationKind::Breaking => "breaking",
        }
    }

    /// Parses the stored `kind` text back into the enum.
    pub fn parse(s: &str) -> Option<MigrationKind> {
        match s {
            "additive" => Some(MigrationKind::Additive),
            "breaking" => Some(MigrationKind::Breaking),
            _ => None,
        }
    }
}

/// One migration's full definition, carried by the binary. Replaces the bare
/// `up` SQL string the runner used to hold: the down script, kind, and
/// lossiness declaration travel alongside so the runner can both apply forward
/// and, for older binaries, step back down.
#[derive(Clone, Copy, Debug)]
pub struct MigrationDef {
    /// The migration directory name (e.g. `20240120134904_init`), stored in the
    /// `name` column.
    pub name: &'static str,
    /// The `up` SQL, applied forward and hashed into the checksum.
    pub up_sql: &'static str,
    /// The stored `down` SQL, executed in reverse when an older binary steps
    /// past this migration. `None` for the historical migrations that predate
    /// the floor and are never down-run.
    pub down_sql: Option<&'static str>,
    /// Whether an older binary may overlay this migration or must down-run it.
    pub kind: MigrationKind,
    /// Lossiness declaration stored in `data_down`: `full`, `partial:<fields>`,
    /// or `none`.
    pub data_down: &'static str,
}

/// The binary's ordered migration list. `version` is the 1-based index into
/// `migrations`; the on-disk `user_version` counts the same sequence.
pub struct MigrationSet {
    pub migrations: Vec<MigrationDef>,
}

/// What the runner decided after inspecting the database, consumed by the
/// bootstrap which turns it into `_STATUS_:` lines. Every fatal outcome is a
/// [`RefusalKind`]; `Proceed` and `Downgraded` continue startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenVerdict {
    /// Startup continues against the existing (possibly overlaid) schema.
    Proceed,
    /// A breaking down-run succeeded and the schema is back at the binary's own
    /// version; startup continues. Bootstrap emits `_STATUS_:DB_DOWNGRADED`.
    Downgraded,
    /// Startup must abort; see [`RefusalKind`].
    Refuse(RefusalKind),
}

/// Why the runner refused to proceed. Each maps to a distinct fatal status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalKind {
    /// `user_version` is ahead but the metadata to step back is missing — a
    /// pre-floor database an old binary cannot understand. Today's behavior.
    BackwardsMigration,
    /// A recorded migration's checksum disagrees with this binary's for the same
    /// version: divergent history (e.g. a stable hotfix vs a different beta at
    /// the same number). Never assume; refuse.
    Diverged { version: i32 },
    /// A breaking down-run could not restore the binary's own schema. The
    /// database is left untouched (rolled back) and the pre-downgrade snapshot
    /// is preserved for recovery.
    DowngradeFailed { snapshot_path: PathBuf },
}

const CREATE_MIGRATIONS_TABLE: &str = "CREATE TABLE IF NOT EXISTS _migrations (\
    version INTEGER PRIMARY KEY, \
    name TEXT NOT NULL, \
    checksum TEXT NOT NULL, \
    kind TEXT NOT NULL, \
    down_sql TEXT, \
    data_down TEXT NOT NULL DEFAULT 'full', \
    applied_at INTEGER NOT NULL)";

const MIGRATION_ROW_COLUMNS: &str =
    "_migrations (version, name, checksum, kind, down_sql, data_down, applied_at) \
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

impl MigrationSet {
    /// The number of migrations this binary carries — the `user_version` a
    /// fully-upgraded database reaches.
    pub fn count(&self) -> i32 {
        self.migrations.len() as i32
    }

    /// sha256 (hex) of a migration's `up` SQL. Detects a divergent history at
    /// the same version number.
    pub fn checksum(&self, version: i32) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.migrations[(version - 1) as usize].up_sql.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Forward-only application, used by tests and tooling that always start at
    /// or below the binary's version. Ensures the metadata table exists,
    /// backfills rows for migrations already applied to the schema, then applies
    /// every pending migration — each `up` plus its `_migrations` row plus the
    /// `user_version` bump in one transaction.
    pub fn to_latest(&self, conn: &mut Connection) -> DbResult<()> {
        ensure_migrations_table(conn)?;
        // Table redefinitions in the historical migrations require FKs OFF on
        // the migration connection (spec §7.1), exactly as before.
        conn.pragma_update(None, "foreign_keys", "OFF")?;
        let count = self.count();
        let user_version = read_user_version(conn)?;
        self.backfill_applied(conn, user_version.min(count))?;
        self.apply_pending(conn, user_version, count)?;
        Ok(())
    }

    /// The production open path (spec §9.2). Returns the verdict the bootstrap
    /// turns into a status line.
    ///
    /// - `user_version <= count`: backfill + apply pending forward, `Proceed`.
    /// - `user_version > count`, all extra migrations additive: overlay,
    ///   touching nothing, `Proceed`.
    /// - `user_version > count`, any extra breaking: snapshot, run the stored
    ///   downs in reverse in one verified transaction, `Downgraded` on success
    ///   or `Refuse(DowngradeFailed)` with the snapshot preserved.
    /// - metadata missing above `count`: `Refuse(BackwardsMigration)`.
    /// - a recorded checksum disagrees at or below `count`: `Refuse(Diverged)`.
    pub fn open(&self, conn: &mut Connection, db_path: &Path) -> DbResult<OpenVerdict> {
        ensure_migrations_table(conn)?;
        conn.pragma_update(None, "foreign_keys", "OFF")?;

        let count = self.count();
        let user_version = read_user_version(conn)?;

        // Divergence is checked before anything else: a forked history at a
        // known version taints the whole database, up or down.
        if let Some(version) = self.first_divergent(conn, user_version.min(count))? {
            return Ok(OpenVerdict::Refuse(RefusalKind::Diverged { version }));
        }

        if user_version > count {
            return self.handle_ahead(conn, db_path, count, user_version);
        }

        self.backfill_applied(conn, user_version)?;
        self.apply_pending(conn, user_version, count)?;
        Ok(OpenVerdict::Proceed)
    }

    /// Records `_migrations` rows for migrations `1..=upto` that are already
    /// applied to the schema but lack a metadata row — the first-run backfill
    /// for a database upgraded by a pre-floor binary. Idempotent via
    /// `INSERT OR IGNORE`.
    fn backfill_applied(&self, conn: &Connection, upto: i32) -> DbResult<()> {
        for version in 1..=upto {
            let def = &self.migrations[(version - 1) as usize];
            conn.execute(
                &format!("INSERT OR IGNORE INTO {MIGRATION_ROW_COLUMNS}"),
                params![
                    version,
                    def.name,
                    self.checksum(version),
                    def.kind.as_str(),
                    def.down_sql,
                    def.data_down,
                    now_millis(),
                ],
            )?;
        }
        Ok(())
    }

    /// Applies migrations `from+1..=to`, each in its own transaction: `up`, the
    /// `_migrations` row, and the `user_version` bump commit together so no
    /// reachable state carries a schema change without its metadata.
    fn apply_pending(&self, conn: &mut Connection, from: i32, to: i32) -> DbResult<()> {
        for version in (from + 1)..=to {
            let def = &self.migrations[(version - 1) as usize];
            let tx = conn.transaction()?;
            tx.execute_batch(def.up_sql)?;
            tx.execute(
                &format!("INSERT INTO {MIGRATION_ROW_COLUMNS}"),
                params![
                    version,
                    def.name,
                    self.checksum(version),
                    def.kind.as_str(),
                    def.down_sql,
                    def.data_down,
                    now_millis(),
                ],
            )?;
            tx.pragma_update(None, "user_version", version)?;
            tx.commit()?;
        }
        Ok(())
    }

    /// Returns the lowest version `1..=upto` whose recorded checksum disagrees
    /// with this binary's, or `None` if every recorded row matches.
    fn first_divergent(&self, conn: &Connection, upto: i32) -> DbResult<Option<i32>> {
        if upto < 1 {
            return Ok(None);
        }
        let mut stmt = conn.prepare(
            "SELECT version, checksum FROM _migrations WHERE version BETWEEN 1 AND ?1 ORDER BY version",
        )?;
        let rows = stmt.query_map([upto], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (version, stored) = row?;
            if (version as usize) <= self.migrations.len() && self.checksum(version) != stored {
                return Ok(Some(version));
            }
        }
        Ok(None)
    }

    /// `user_version > count`: decide overlay vs down-run vs backwards refusal.
    fn handle_ahead(
        &self,
        conn: &mut Connection,
        db_path: &Path,
        count: i32,
        user_version: i32,
    ) -> DbResult<OpenVerdict> {
        // Rows for every version ahead of our own, newest first (down-run order).
        let ahead = {
            let mut stmt = conn.prepare(
                "SELECT version, kind, down_sql FROM _migrations WHERE version > ?1 ORDER BY version DESC",
            )?;
            let rows = stmt.query_map([count], |row| {
                Ok((
                    row.get::<_, i32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        // Any version in (count, user_version] without a metadata row is a
        // pre-floor database we cannot step back through.
        let present: HashSet<i32> = ahead.iter().map(|(v, _, _)| *v).collect();
        if ((count + 1)..=user_version).any(|v| !present.contains(&v)) {
            return Ok(OpenVerdict::Refuse(RefusalKind::BackwardsMigration));
        }

        if ahead
            .iter()
            .all(|(_, kind, _)| MigrationKind::parse(kind) == Some(MigrationKind::Additive))
        {
            tracing::info!(
                "overlay: {} additive migration(s) ahead of own count {}; running against the newer schema untouched",
                ahead.len(),
                count
            );
            return Ok(OpenVerdict::Proceed);
        }

        self.down_run(conn, db_path, count, &ahead)
    }

    /// Runs the stored downs for `ahead` (newest first) under a pre-taken file
    /// snapshot, in one transaction, and verifies the result equals this
    /// binary's own schema at `count` before committing. Any failure rolls back
    /// and preserves the snapshot.
    fn down_run(
        &self,
        conn: &mut Connection,
        db_path: &Path,
        count: i32,
        ahead: &[(i32, String, Option<String>)],
    ) -> DbResult<OpenVerdict> {
        // Snapshot the file BEFORE any destructive step. Checkpoint first so the
        // -wal contents are folded into the main file the copy captures.
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        let snapshot_path = snapshot_path_for(db_path);
        std::fs::copy(db_path, &snapshot_path).map_err(|e| {
            DbError::Conversion(format!("pre-downgrade snapshot copy failed: {e}"))
        })?;

        let reference = self.reference_schema(count)?;

        let tx = conn.transaction()?;
        for (version, _kind, down_sql) in ahead {
            let Some(sql) = down_sql else {
                // A breaking migration ahead with no stored down cannot be
                // reversed; refuse and keep the snapshot.
                tracing::error!(
                    "down-run refused: migration {version} is breaking but has no stored down"
                );
                drop(tx);
                return Ok(OpenVerdict::Refuse(RefusalKind::DowngradeFailed { snapshot_path }));
            };
            if let Err(e) = tx.execute_batch(sql) {
                tracing::error!("down-run of migration {version} failed: {e}");
                drop(tx); // rollback: the whole down-run is atomic
                return Ok(OpenVerdict::Refuse(RefusalKind::DowngradeFailed { snapshot_path }));
            }
            tx.execute("DELETE FROM _migrations WHERE version = ?1", params![version])?;
        }
        tx.pragma_update(None, "user_version", count)?;

        // Verify against our own ground truth before committing: the future's
        // downs are trusted, then checked.
        let actual = dump_schema(&tx)?;
        if actual == reference {
            tx.commit()?;
            Ok(OpenVerdict::Downgraded)
        } else {
            tracing::error!(
                "down-run verification failed: resulting schema does not match this build's schema at version {count}"
            );
            drop(tx); // rollback: never leave a half-downgraded schema
            Ok(OpenVerdict::Refuse(RefusalKind::DowngradeFailed { snapshot_path }))
        }
    }

    /// Builds this binary's pristine schema at `version` in memory by applying
    /// its own ups `1..=version`, then dumps it through the shared normalizer —
    /// the ground truth a down-run result must match.
    fn reference_schema(&self, version: i32) -> DbResult<String> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "OFF")?;
        for v in 1..=version {
            conn.execute_batch(self.migrations[(v - 1) as usize].up_sql)?;
        }
        dump_schema(&conn)
    }
}

/// Creates the `_migrations` bookkeeping table if it does not already exist.
fn ensure_migrations_table(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(CREATE_MIGRATIONS_TABLE)?;
    Ok(())
}

/// Reads `PRAGMA user_version`, defaulting to 0.
fn read_user_version(conn: &Connection) -> DbResult<i32> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

/// The pre-downgrade snapshot path beside the database file: `<stem>.pre-downgrade.db`
/// (so the runtime `gdl_conf.db` yields `gdl_conf.pre-downgrade.db`).
fn snapshot_path_for(db_path: &Path) -> PathBuf {
    let stem = db_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("gdl_conf");
    db_path.with_file_name(format!("{stem}.pre-downgrade.db"))
}

/// Current unix time in milliseconds — the storage unit for `applied_at`, matching
/// every other datetime column (spec §2.1).
fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
