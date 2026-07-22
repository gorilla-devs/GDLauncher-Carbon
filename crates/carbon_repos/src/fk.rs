//! Foreign-key enablement sweep (spec §7).
//!
//! Foreign keys have been OFF for the app's entire life: SQLite defaults FKs
//! off per connection, no pragma ever turned them on, and referential integrity
//! was kept manually by manager code. This module turns FKs ON behind a
//! fail-safe sweep.
//!
//! [`sweep_and_decide`] runs after migrations, on a dedicated connection whose
//! FKs are OFF (so repair deletes do not themselves cascade). It runs
//! `PRAGMA foreign_key_check`; a clean check means the runtime pools open with
//! FKs ON. Otherwise it logs every violating row, applies the least-destructive
//! repair for every edge (parent-first, in one transaction), and re-checks. A
//! clean re-check still yields FKs ON; a still-dirty result commits the
//! (independently valid) repairs anyway and falls back to FKs OFF for the
//! session — identical to today's behavior — so startup never fails on
//! integrity grounds.
//!
//! The definitive edge list in [`EDGES`] is derived from the migrated schema's
//! `pragma_foreign_key_list` (spec §7.2, §18.3), each edge tagged with the
//! repair class its parent/child relationship permits. Destructive repair
//! (DELETE) is allowed only on cache-class tables; user-data tables are only
//! null-fixed or reassigned to a default group.

use crate::db_error::DbResult;
use rusqlite::{Connection, OptionalExtension, Transaction};
use tracing::{error, warn};

/// The least-destructive repair an edge's parent/child relationship permits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairClass {
    /// Regenerable cache/derived child row — delete the orphan.
    CacheDelete,
    /// Declared `ON DELETE SET NULL` edge — null the dangling reference, never
    /// delete the row (a `JavaProfile` / the `AppConfiguration` singleton).
    SetNull,
    /// User-data child pointing at a group — reassign to a get-or-created
    /// default group, never delete the `Instance` / `Server`.
    ReassignDefaultGroup,
}

/// One foreign-key edge in the migrated schema, with its repair class.
#[derive(Debug, Clone, Copy)]
pub struct FkEdge {
    /// Child (referencing) table.
    pub child: &'static str,
    /// Child columns holding the reference (>1 for composite FKs).
    pub child_cols: &'static [&'static str],
    /// Parent (referenced) table.
    pub parent: &'static str,
    /// Parent columns referenced, positionally paired with `child_cols`.
    pub parent_cols: &'static [&'static str],
    pub class: RepairClass,
}

/// The complete FK edge list of the migrated schema (user_version 25), derived
/// from `pragma_foreign_key_list`, ordered so that repairs run parent-first:
/// reassigns and null-fixes keep `Instance`/`Server` rows alive for their file
/// caches; first-level cache deletes precede the image caches that hang off
/// them, so a delete that orphans a grandchild is caught in the same pass.
pub const EDGES: &[FkEdge] = &[
    // User-data child → group: reassign (run first so the rows survive for the
    // file-cache edges below).
    FkEdge {
        child: "Instance",
        child_cols: &["groupId"],
        parent: "InstanceGroup",
        parent_cols: &["id"],
        class: RepairClass::ReassignDefaultGroup,
    },
    FkEdge {
        child: "Server",
        child_cols: &["groupId"],
        parent: "ServerGroup",
        parent_cols: &["id"],
        class: RepairClass::ReassignDefaultGroup,
    },
    // Declared SET NULL edges.
    FkEdge {
        child: "JavaProfile",
        child_cols: &["javaId"],
        parent: "Java",
        parent_cols: &["id"],
        class: RepairClass::SetNull,
    },
    FkEdge {
        child: "AppConfiguration",
        child_cols: &["activeAccountUuid"],
        parent: "Account",
        parent_cols: &["uuid"],
        class: RepairClass::SetNull,
    },
    // First-level cache/derived children.
    FkEdge {
        child: "CurseForgeModCache",
        child_cols: &["metadataId"],
        parent: "ModMetadata",
        parent_cols: &["id"],
        class: RepairClass::CacheDelete,
    },
    FkEdge {
        child: "ModrinthModCache",
        child_cols: &["metadataId"],
        parent: "ModMetadata",
        parent_cols: &["id"],
        class: RepairClass::CacheDelete,
    },
    FkEdge {
        child: "LocalModImageCache",
        child_cols: &["metadataId"],
        parent: "ModMetadata",
        parent_cols: &["id"],
        class: RepairClass::CacheDelete,
    },
    FkEdge {
        child: "ModFileCache",
        child_cols: &["metadataId"],
        parent: "ModMetadata",
        parent_cols: &["id"],
        class: RepairClass::CacheDelete,
    },
    FkEdge {
        child: "ModFileCache",
        child_cols: &["instanceId"],
        parent: "Instance",
        parent_cols: &["id"],
        class: RepairClass::CacheDelete,
    },
    FkEdge {
        child: "ServerModFileCache",
        child_cols: &["metadataId"],
        parent: "ModMetadata",
        parent_cols: &["id"],
        class: RepairClass::CacheDelete,
    },
    FkEdge {
        child: "ServerModFileCache",
        child_cols: &["serverId"],
        parent: "Server",
        parent_cols: &["id"],
        class: RepairClass::CacheDelete,
    },
    // Image caches hanging off the mod caches above.
    FkEdge {
        child: "CurseForgeModImageCache",
        child_cols: &["metadataId"],
        parent: "CurseForgeModCache",
        parent_cols: &["metadataId"],
        class: RepairClass::CacheDelete,
    },
    FkEdge {
        child: "ModrinthModImageCache",
        child_cols: &["metadataId"],
        parent: "ModrinthModCache",
        parent_cols: &["metadataId"],
        class: RepairClass::CacheDelete,
    },
    // Modpack image caches (composite FKs).
    FkEdge {
        child: "CurseForgeModpackImageCache",
        child_cols: &["projectId", "fileId"],
        parent: "CurseForgeModpackCache",
        parent_cols: &["projectId", "fileId"],
        class: RepairClass::CacheDelete,
    },
    FkEdge {
        child: "ModrinthModpackImageCache",
        child_cols: &["projectId", "versionId"],
        parent: "ModrinthModpackCache",
        parent_cols: &["projectId", "versionId"],
        class: RepairClass::CacheDelete,
    },
];

/// A single row reported by `PRAGMA foreign_key_check`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Child table containing the offending row.
    pub table: String,
    /// Rowid of the offending row (`None` for WITHOUT ROWID tables).
    pub rowid: Option<i64>,
    /// Parent table the missing reference points at.
    pub parent: String,
    /// Index of the failing FK within the child table (`foreign_key_list.id`).
    pub fkid: i64,
}

/// Outcome of the sweep, deciding whether the runtime pools enable FKs.
#[derive(Debug)]
pub enum SweepOutcome {
    /// `foreign_key_check` is clean (before or after repair) — open with FKs ON.
    Enabled,
    /// Violations survived repair — open with FKs OFF for this session (repairs
    /// were still committed). `violations` are the rows that remained.
    DisabledFallback { violations: Vec<Violation> },
}

/// Runs `PRAGMA foreign_key_check`, collecting every reported violation.
fn foreign_key_check(conn: &Connection) -> rusqlite::Result<Vec<Violation>> {
    let mut st = conn.prepare("PRAGMA foreign_key_check")?;
    let rows = st.query_map([], |r| {
        Ok(Violation {
            table: r.get(0)?,
            rowid: r.get(1)?,
            parent: r.get(2)?,
            fkid: r.get(3)?,
        })
    })?;
    rows.collect()
}

/// `<a> AND <b> AND ...` over the given fragments.
fn join_and(parts: impl IntoIterator<Item = String>) -> String {
    parts.into_iter().collect::<Vec<_>>().join(" AND ")
}

/// `NOT EXISTS (SELECT 1 FROM parent p WHERE p.pc = child.cc AND ...)` for one
/// edge — the orphan predicate (child cols all non-null, no matching parent).
fn orphan_predicate(edge: &FkEdge) -> String {
    let not_null = join_and(
        edge.child_cols
            .iter()
            .map(|c| format!("\"{}\".\"{}\" IS NOT NULL", edge.child, c)),
    );
    let join = join_and(
        edge.child_cols
            .iter()
            .zip(edge.parent_cols.iter())
            .map(|(cc, pc)| format!("p.\"{}\" = \"{}\".\"{}\"", pc, edge.child, cc)),
    );
    format!(
        "{not_null} AND NOT EXISTS (SELECT 1 FROM \"{parent}\" p WHERE {join})",
        not_null = not_null,
        parent = edge.parent,
        join = join,
    )
}

/// Number of orphaned rows for `edge` in the current transaction state.
fn count_orphans(tx: &Transaction, edge: &FkEdge) -> rusqlite::Result<i64> {
    let sql = format!(
        "SELECT COUNT(*) FROM \"{child}\" WHERE {pred}",
        child = edge.child,
        pred = orphan_predicate(edge),
    );
    tx.query_row(&sql, [], |r| r.get(0))
}

/// Returns the id of a usable default group for `config_col`/`group_table`,
/// creating a `"localize➽default"` group (and pointing the singleton config row
/// at it) if none exists. Mirrors the app's get-or-create default-group flow so
/// reassigned rows land where new user-created rows would.
fn get_or_create_default_group(
    tx: &Transaction,
    group_table: &str,
    config_col: &str,
) -> rusqlite::Result<i64> {
    // Existing default from config, if it still points at a real group.
    let existing: Option<i64> = tx
        .query_row(
            &format!(
                "SELECT ac.\"{col}\" FROM AppConfiguration ac
                 WHERE ac.id = 0 AND ac.\"{col}\" IS NOT NULL
                   AND EXISTS (SELECT 1 FROM \"{grp}\" g WHERE g.id = ac.\"{col}\")",
                col = config_col,
                grp = group_table,
            ),
            [],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }

    // Otherwise the first existing group (matches the app's fallback), if any.
    let first: Option<i64> = tx
        .query_row(
            &format!(
                "SELECT id FROM \"{grp}\" ORDER BY groupIndex ASC, id ASC LIMIT 1",
                grp = group_table,
            ),
            [],
            |r| r.get(0),
        )
        .optional()?;
    let id = match first {
        Some(id) => id,
        None => {
            tx.execute(
                &format!(
                    "INSERT INTO \"{grp}\" (name, groupIndex, libraryPosition)
                     VALUES ('localize➽default', 0, NULL)",
                    grp = group_table,
                ),
                [],
            )?;
            tx.last_insert_rowid()
        }
    };

    // Point the singleton config row at the chosen group (no-op if id != 0 row
    // is absent — a DB with orphaned instances always has one).
    tx.execute(
        &format!(
            "UPDATE AppConfiguration SET \"{col}\" = ?1 WHERE id = 0",
            col = config_col
        ),
        [id],
    )?;
    Ok(id)
}

/// Applies the repair for one implicated edge in the current transaction.
fn apply_repair(tx: &Transaction, edge: &FkEdge) -> rusqlite::Result<usize> {
    let pred = orphan_predicate(edge);
    match edge.class {
        RepairClass::CacheDelete => tx.execute(
            &format!(
                "DELETE FROM \"{child}\" WHERE {pred}",
                child = edge.child,
                pred = pred
            ),
            [],
        ),
        RepairClass::SetNull => {
            // All SET NULL edges in this schema are single-column.
            let col = edge.child_cols[0];
            tx.execute(
                &format!(
                    "UPDATE \"{child}\" SET \"{col}\" = NULL WHERE {pred}",
                    child = edge.child,
                    col = col,
                    pred = pred,
                ),
                [],
            )
        }
        RepairClass::ReassignDefaultGroup => {
            // Only create the default group when there is actually work to do,
            // so a clean group edge never spawns a spurious default group.
            if count_orphans(tx, edge)? == 0 {
                return Ok(0);
            }
            let config_col = default_group_config(edge.parent);
            let default_id = get_or_create_default_group(tx, edge.parent, config_col)?;
            let col = edge.child_cols[0];
            tx.execute(
                &format!(
                    "UPDATE \"{child}\" SET \"{col}\" = ?1 WHERE {pred}",
                    child = edge.child,
                    col = col,
                    pred = pred,
                ),
                [default_id],
            )
        }
    }
}

/// Maps a group parent table to its singleton-config default column.
fn default_group_config(group_table: &str) -> &'static str {
    match group_table {
        "InstanceGroup" => "defaultInstanceGroup",
        "ServerGroup" => "defaultServerGroup",
        other => unreachable!("no default-group config column for parent {other}"),
    }
}

/// Runs the FK sweep and decides whether the runtime should enable foreign
/// keys. See the module docs for the full contract. Repairs are always
/// committed (each is independently valid); only the pragma decision changes on
/// a still-dirty re-check.
pub fn sweep_and_decide(conn: &mut Connection) -> DbResult<SweepOutcome> {
    let violations = foreign_key_check(conn)?;
    if violations.is_empty() {
        return Ok(SweepOutcome::Enabled);
    }

    for v in &violations {
        warn!(
            table = %v.table,
            rowid = ?v.rowid,
            parent = %v.parent,
            fkid = v.fkid,
            "foreign key violation before repair sweep",
        );
    }

    // Repair every edge in parent-first order. `CacheDelete`/`SetNull` are
    // idempotent no-ops when an edge is clean, and `ReassignDefaultGroup`
    // self-guards against creating a spurious default group when nothing is
    // orphaned — so running the whole list unconditionally is safe and is
    // required to catch grandchildren orphaned mid-pass (a cache delete can
    // orphan the image cache hanging off it, whose edge was clean at check
    // time and would otherwise be skipped).
    let tx = conn.transaction()?;
    for edge in EDGES {
        apply_repair(&tx, edge)?;
    }
    tx.commit()?;

    let remaining = foreign_key_check(conn)?;
    if remaining.is_empty() {
        Ok(SweepOutcome::Enabled)
    } else {
        for v in &remaining {
            error!(
                table = %v.table,
                rowid = ?v.rowid,
                parent = %v.parent,
                fkid = v.fkid,
                "foreign key violation survived repair sweep; disabling FK enforcement for this session",
            );
        }
        Ok(SweepOutcome::DisabledFallback {
            violations: remaining,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
        let (m, _n) = crate::get_migrations();
        m.to_latest(&mut conn).unwrap();
        // Sweep/repair must see FKs OFF (as the migration connection does) so
        // repair deletes do not cascade under us.
        conn.pragma_update(None, "foreign_keys", &"OFF").unwrap();
        (dir, conn)
    }

    fn fk_violations(conn: &Connection) -> usize {
        foreign_key_check(conn).unwrap().len()
    }

    /// The DB is left FK-consistent: turning enforcement ON reports zero
    /// violations, i.e. the runtime pools could open with FKs ON.
    fn assert_fk_on_consistent(conn: &Connection) {
        conn.pragma_update(None, "foreign_keys", &"ON").unwrap();
        assert_eq!(
            fk_violations(conn),
            0,
            "DB must be FK-consistent after the sweep"
        );
        conn.pragma_update(None, "foreign_keys", &"OFF").unwrap();
    }

    #[test]
    fn clean_db_enables_without_repair() {
        let (_d, mut conn) = migrated();
        assert_eq!(fk_violations(&conn), 0);
        assert!(matches!(
            sweep_and_decide(&mut conn).unwrap(),
            SweepOutcome::Enabled
        ));
    }

    /// Every `(child table, parent table, child column)` foreign key the live
    /// schema declares, read from `pragma_foreign_key_list` over every user
    /// table — the ground truth both directions of `edge_list_matches_schema`
    /// compare `EDGES` against.
    fn schema_fk_triples(conn: &Connection) -> Vec<(String, String, String)> {
        let tables: Vec<String> = {
            let mut st = conn
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type = 'table' \
                     AND name NOT LIKE 'sqlite_%' \
                     AND name NOT IN ('_migrations', '_prisma_migrations')",
                )
                .unwrap();
            let rows = st.query_map([], |r| r.get::<_, String>(0)).unwrap();
            rows.collect::<Result<Vec<_>, _>>().unwrap()
        };

        let mut triples = Vec::new();
        for table in &tables {
            let mut st = conn
                .prepare(&format!("PRAGMA foreign_key_list('{table}')"))
                .unwrap();
            let cols: Vec<String> = st
                .column_names()
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            let ti = |n: &str| cols.iter().position(|c| c == n).unwrap();
            let mut rows = st.query([]).unwrap();
            while let Some(r) = rows.next().unwrap() {
                let parent: String = r.get(ti("table")).unwrap();
                let from: String = r.get(ti("from")).unwrap();
                triples.push((table.clone(), parent, from));
            }
        }
        triples
    }

    #[test]
    fn edge_list_matches_schema() {
        // Bidirectional completeness (spec §7.2, Plan-4 global constraint): the
        // encoded `EDGES` list must be exactly the set of foreign keys the live
        // schema declares. Checking only one direction would let a schema FK
        // silently fall out of the sweep (a new migration adds a reference the
        // sweep never repairs) or an encoded edge point at a reference the
        // schema no longer has.
        let (_d, conn) = migrated();
        let schema = schema_fk_triples(&conn);

        // Forward: every encoded edge exists in the live schema.
        for edge in EDGES {
            let found = schema.iter().any(|(child, parent, from)| {
                child == edge.child
                    && parent == edge.parent
                    && edge.child_cols.contains(&from.as_str())
            });
            assert!(found, "encoded edge {edge:?} not present in live schema");
        }

        // Reverse: every foreign key in the live schema is encoded in EDGES, so
        // no reference can silently fall out of the repair sweep.
        for (child, parent, from) in &schema {
            let encoded = EDGES.iter().any(|edge| {
                edge.child == child
                    && edge.parent == parent
                    && edge.child_cols.contains(&from.as_str())
            });
            assert!(
                encoded,
                "schema foreign key {child}.{from} -> {parent} is not encoded in EDGES; \
                 add it (with its repair class) so the sweep covers it",
            );
        }
    }

    #[test]
    fn dangling_mod_file_cache_instance_is_deleted() {
        let (_d, mut conn) = migrated();
        // ModFileCache row referencing a non-existent instance and metadata.
        conn.execute(
            "INSERT INTO ModFileCache (id, instanceId, filename, filesize, enabled, metadataId)
             VALUES ('mfc1', 999, 'a.jar', 1, 0, 'meta-missing')",
            [],
        )
        .unwrap();
        assert!(fk_violations(&conn) > 0);
        let outcome = sweep_and_decide(&mut conn).unwrap();
        assert!(matches!(outcome, SweepOutcome::Enabled), "got {outcome:?}");
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ModFileCache WHERE id = 'mfc1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0, "orphaned cache row must be deleted");
        assert_fk_on_consistent(&conn);
    }

    #[test]
    fn dangling_java_profile_is_nulled_not_deleted() {
        let (_d, mut conn) = migrated();
        conn.execute(
            "INSERT INTO JavaProfile (name, isSystemProfile, javaId) VALUES ('p', 0, 'ghost-java')",
            [],
        )
        .unwrap();
        assert!(fk_violations(&conn) > 0);
        assert!(matches!(
            sweep_and_decide(&mut conn).unwrap(),
            SweepOutcome::Enabled
        ));
        let java_id: Option<String> = conn
            .query_row("SELECT javaId FROM JavaProfile WHERE name = 'p'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(java_id, None, "javaId must be nulled");
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM JavaProfile WHERE name = 'p'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "the profile itself must survive");
        assert_fk_on_consistent(&conn);
    }

    #[test]
    fn dangling_app_config_active_account_is_nulled() {
        let (_d, mut conn) = migrated();
        crate::repos::app_configuration::insert_app_configuration_conn(
            &crate::db_exec::WriteGuard::new(&mut conn),
            "stable",
            2048,
            None,
        )
        .unwrap();
        conn.execute(
            "UPDATE AppConfiguration SET activeAccountUuid = 'no-such-account' WHERE id = 0",
            [],
        )
        .unwrap();
        assert!(fk_violations(&conn) > 0);
        assert!(matches!(
            sweep_and_decide(&mut conn).unwrap(),
            SweepOutcome::Enabled
        ));
        let active: Option<String> = conn
            .query_row(
                "SELECT activeAccountUuid FROM AppConfiguration WHERE id = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(active, None);
        assert_fk_on_consistent(&conn);
    }

    #[test]
    fn instance_with_dead_group_is_reassigned_to_default() {
        let (_d, mut conn) = migrated();
        crate::repos::app_configuration::insert_app_configuration_conn(
            &crate::db_exec::WriteGuard::new(&mut conn),
            "stable",
            2048,
            None,
        )
        .unwrap();
        // A real default group the config points at.
        let def = crate::repos::instance::insert_group_conn(
            &crate::db_exec::WriteGuard::new(&mut conn),
            "localize➽default",
            0,
            None,
        )
        .unwrap();
        conn.execute(
            "UPDATE AppConfiguration SET defaultInstanceGroup = ?1 WHERE id = 0",
            [def],
        )
        .unwrap();
        // An instance in a group that does not exist.
        conn.execute(
            "INSERT INTO Instance (name, shortpath, \"index\", groupId) VALUES ('i', 'sp', 0, 424242)",
            [],
        )
        .unwrap();
        assert!(fk_violations(&conn) > 0);
        assert!(matches!(
            sweep_and_decide(&mut conn).unwrap(),
            SweepOutcome::Enabled
        ));
        let gid: i64 = conn
            .query_row(
                "SELECT groupId FROM Instance WHERE shortpath = 'sp'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(gid, def, "instance must be reassigned to the default group");
        assert_fk_on_consistent(&conn);
    }

    #[test]
    fn dead_group_reassign_creates_default_when_absent() {
        let (_d, mut conn) = migrated();
        crate::repos::app_configuration::insert_app_configuration_conn(
            &crate::db_exec::WriteGuard::new(&mut conn),
            "stable",
            2048,
            None,
        )
        .unwrap();
        // No group at all; instance points at a missing one.
        conn.execute(
            "INSERT INTO Instance (name, shortpath, \"index\", groupId) VALUES ('i', 'sp', 0, 5)",
            [],
        )
        .unwrap();
        assert!(matches!(
            sweep_and_decide(&mut conn).unwrap(),
            SweepOutcome::Enabled
        ));
        // A default group was created and the instance points at it.
        let gid: i64 = conn
            .query_row(
                "SELECT groupId FROM Instance WHERE shortpath = 'sp'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let name: String = conn
            .query_row("SELECT name FROM InstanceGroup WHERE id = ?1", [gid], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "localize➽default");
        let cfg: Option<i64> = conn
            .query_row(
                "SELECT defaultInstanceGroup FROM AppConfiguration WHERE id = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cfg, Some(gid));
        assert_fk_on_consistent(&conn);
    }

    #[test]
    fn clean_group_edge_does_not_create_spurious_default_group() {
        // A cache violation triggers the sweep, but no instance/server is
        // orphaned — the reassign path must not fabricate a default group.
        let (_d, mut conn) = migrated();
        conn.execute(
            "INSERT INTO CurseForgeModCache (metadataId, murmur2, projectId, fileId, name, version, urlslug, summary, authors, releaseType, updatePaths, cachedAt)
             VALUES ('ghost', 0, 1, 1, 'n', '1', 's', 'x', 'a', 1, '', 0)",
            [],
        )
        .unwrap();
        let groups_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM InstanceGroup", [], |r| r.get(0))
            .unwrap();
        assert!(matches!(
            sweep_and_decide(&mut conn).unwrap(),
            SweepOutcome::Enabled
        ));
        let groups_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM InstanceGroup", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            groups_before, groups_after,
            "no default group should be created"
        );
    }

    #[test]
    fn cascade_chain_metadata_to_image_cache_repaired_in_one_pass() {
        // ModMetadata missing → CurseForgeModCache orphan → its image cache is a
        // grandchild that only becomes orphaned once the mod cache is deleted.
        // The parent-first edge order must catch both in one sweep.
        let (_d, mut conn) = migrated();
        conn.execute(
            "INSERT INTO CurseForgeModCache (metadataId, murmur2, projectId, fileId, name, version, urlslug, summary, authors, releaseType, updatePaths, cachedAt)
             VALUES ('ghost-meta', 0, 7, 1, 'n', '1', 's', 'x', 'a', 1, '', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO CurseForgeModImageCache (metadataId, url) VALUES ('ghost-meta', 'http://x')",
            [],
        )
        .unwrap();
        assert!(matches!(
            sweep_and_decide(&mut conn).unwrap(),
            SweepOutcome::Enabled
        ));
        let modc: i64 = conn
            .query_row("SELECT COUNT(*) FROM CurseForgeModCache", [], |r| r.get(0))
            .unwrap();
        let imgc: i64 = conn
            .query_row("SELECT COUNT(*) FROM CurseForgeModImageCache", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(modc, 0);
        assert_eq!(imgc, 0, "grandchild image cache must be swept too");
        assert_fk_on_consistent(&conn);
    }

    #[test]
    fn unrepairable_violation_falls_back_and_still_commits_repairs() {
        // A synthetic table with an FK to a table the sweep does not know about:
        // its orphan cannot be repaired, forcing the OFF fallback. A concurrent,
        // repairable cache orphan must still be fixed (repairs commit anyway).
        let (_d, mut conn) = migrated();
        conn.execute_batch(
            "CREATE TABLE Widget (id INTEGER PRIMARY KEY);
             CREATE TABLE Gadget (id INTEGER PRIMARY KEY, widgetId INTEGER
                 REFERENCES Widget(id));
             INSERT INTO Gadget (id, widgetId) VALUES (1, 999);",
        )
        .unwrap();
        // A repairable cache orphan alongside the unrepairable one.
        conn.execute(
            "INSERT INTO ModFileCache (id, instanceId, filename, filesize, enabled, metadataId)
             VALUES ('mfc', 123, 'a.jar', 1, 0, 'gone')",
            [],
        )
        .unwrap();

        let outcome = sweep_and_decide(&mut conn).unwrap();
        match outcome {
            SweepOutcome::DisabledFallback { violations } => {
                assert!(
                    violations.iter().any(|v| v.table == "Gadget"),
                    "unrepairable Gadget orphan must remain: {violations:?}"
                );
            }
            other => panic!("expected fallback, got {other:?}"),
        }
        // Repairs to the known edge were still committed.
        let mfc: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ModFileCache WHERE id = 'mfc'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mfc, 0, "repairable orphan must be deleted even on fallback");
    }
}
