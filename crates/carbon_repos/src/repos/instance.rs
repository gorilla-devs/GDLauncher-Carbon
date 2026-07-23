//! Repository queries for the `Instance` and `InstanceGroup` tables.
//!
//! The library ordering the launcher exposes rests on two parallel systems
//! kept in sync: the legacy non-null `groupIndex`/`index` and the nullable
//! `libraryPosition`. `libraryPosition IS NOT NULL` is used as a deliberate
//! proxy for "belongs to the default group / library root" so a query never
//! has to resolve the default group id (which would re-enter `get_default_group`
//! and deadlock on `index_lock`).

use crate::db_error::DbResult;
use crate::db_exec::{Db, ReadAccess, WriteAccess};
use crate::queries;
use crate::registry::QueryCheck;
use rusqlite::named_params;

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct InstanceRow {
    pub id: i32,
    pub name: String,
    pub shortpath: String,
    pub favorite: bool,
    pub has_pack_update: bool,
    pub index: i32,
    pub library_position: Option<i32>,
    pub group_id: i32,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct InstanceGroupRow {
    pub id: i32,
    pub name: String,
    pub group_index: i32,
    pub library_position: Option<i32>,
}

queries! {
    // ---- Instance reads ----
    fn get_all_instances() -> Vec<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance";
    fn get_all_instances_ordered_by_index() -> Vec<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance ORDER BY \"index\" ASC";
    fn get_instances_by_group(group_id: i32) -> Vec<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance WHERE groupId = :group_id";
    fn get_instances_by_group_ordered_by_index(group_id: i32) -> Vec<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance WHERE groupId = :group_id ORDER BY \"index\" ASC";
    fn get_instance(id: i32) -> Option<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance WHERE id = :id";
    fn get_instance_by_shortpath(shortpath: &str) -> Option<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance WHERE shortpath = :shortpath";
    fn max_library_position_instance() -> Option<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance WHERE libraryPosition IS NOT NULL ORDER BY libraryPosition DESC LIMIT 1";
    fn min_library_position_instance_in_group(group_id: i32) -> Option<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance WHERE groupId = :group_id AND libraryPosition IS NOT NULL ORDER BY libraryPosition ASC LIMIT 1";
    fn max_library_position_instance_in_group(group_id: i32) -> Option<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance WHERE groupId = :group_id AND libraryPosition IS NOT NULL ORDER BY libraryPosition DESC LIMIT 1";
    fn min_index_instance_in_group(group_id: i32) -> Option<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance WHERE groupId = :group_id ORDER BY \"index\" ASC LIMIT 1";
    fn first_instance_in_group(group_id: i32) -> Option<InstanceRow> =
        "SELECT id, name, shortpath, favorite, hasPackUpdate, \"index\", libraryPosition, groupId FROM Instance WHERE groupId = :group_id LIMIT 1";
    fn count_instances_in_group(group_id: i32) -> i64 =
        "SELECT COUNT(*) FROM Instance WHERE groupId = :group_id";

    // ---- Instance single-row updates ----
    fn set_instance_name_and_shortpath(id: i32, name: &str, shortpath: &str) -> usize =
        "UPDATE Instance SET name = :name, shortpath = :shortpath WHERE id = :id";
    fn set_instance_favorite(id: i32, favorite: bool) -> usize =
        "UPDATE Instance SET favorite = :favorite WHERE id = :id";
    fn set_instance_index(id: i32, index: i32) -> usize =
        "UPDATE Instance SET \"index\" = :index WHERE id = :id";
    fn set_instance_index_and_library_position(id: i32, index: i32, library_position: Option<i32>) -> usize =
        "UPDATE Instance SET \"index\" = :index, libraryPosition = :library_position WHERE id = :id";
    fn set_instance_group_index_library_position(id: i32, group_id: i32, index: i32, library_position: Option<i32>) -> usize =
        "UPDATE Instance SET groupId = :group_id, \"index\" = :index, libraryPosition = :library_position WHERE id = :id";

    // ---- Instance shift (update_many) queries ----
    fn shift_instance_library_positions_down(gt: i32, lte: i32) -> usize =
        "UPDATE Instance SET libraryPosition = libraryPosition - 1 WHERE libraryPosition > :gt AND libraryPosition <= :lte";
    fn shift_instance_library_positions_up(gte: i32, lt: i32) -> usize =
        "UPDATE Instance SET libraryPosition = libraryPosition + 1 WHERE libraryPosition >= :gte AND libraryPosition < :lt";
    fn shift_instance_indexes_down_exclusive(group_id: i32, gt: i32, lt: i32) -> usize =
        "UPDATE Instance SET \"index\" = \"index\" - 1 WHERE groupId = :group_id AND \"index\" > :gt AND \"index\" < :lt";
    fn shift_instance_indexes_up_range(group_id: i32, gte: i32, lt: i32) -> usize =
        "UPDATE Instance SET \"index\" = \"index\" + 1 WHERE groupId = :group_id AND \"index\" >= :gte AND \"index\" < :lt";
    fn shift_instance_indexes_down_after(group_id: i32, gt: i32) -> usize =
        "UPDATE Instance SET \"index\" = \"index\" - 1 WHERE groupId = :group_id AND \"index\" > :gt";
    fn shift_instance_indexes_up_from(group_id: i32, gte: i32) -> usize =
        "UPDATE Instance SET \"index\" = \"index\" + 1 WHERE groupId = :group_id AND \"index\" >= :gte";
    fn shift_instance_library_positions_up_in_group_except(group_id: i32, gte: i32, excluded_id: i32) -> usize =
        "UPDATE Instance SET libraryPosition = libraryPosition + 1 WHERE groupId = :group_id AND libraryPosition >= :gte AND id <> :excluded_id";
    fn shift_instance_library_positions_down_in_group(group_id: i32, gt: i32) -> usize =
        "UPDATE Instance SET libraryPosition = libraryPosition - 1 WHERE groupId = :group_id AND libraryPosition > :gt";
    fn shift_all_instance_library_positions_up(gte: i32) -> usize =
        "UPDATE Instance SET libraryPosition = libraryPosition + 1 WHERE libraryPosition >= :gte";
    fn move_all_instances_to_group(from_group: i32, to_group: i32, base_index: i32) -> usize =
        "UPDATE Instance SET groupId = :to_group, \"index\" = \"index\" + :base_index WHERE groupId = :from_group";

    // ---- Instance deletes ----
    fn delete_instance(id: i32) -> usize =
        "DELETE FROM Instance WHERE id = :id";
    fn delete_instances_by_shortpath(shortpath: &str) -> usize =
        "DELETE FROM Instance WHERE shortpath = :shortpath";

    // ---- InstanceGroup reads ----
    fn get_all_groups() -> Vec<InstanceGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM InstanceGroup";
    fn get_all_groups_ordered_by_group_index() -> Vec<InstanceGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM InstanceGroup ORDER BY groupIndex ASC";
    fn get_groups_with_library_position_ordered() -> Vec<InstanceGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM InstanceGroup WHERE libraryPosition IS NOT NULL ORDER BY libraryPosition ASC";
    fn get_group(id: i32) -> Option<InstanceGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM InstanceGroup WHERE id = :id";
    fn find_group_by_name(name: &str) -> Option<InstanceGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM InstanceGroup WHERE name = :name LIMIT 1";
    fn max_library_position_group() -> Option<InstanceGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM InstanceGroup WHERE libraryPosition IS NOT NULL ORDER BY libraryPosition DESC LIMIT 1";
    fn min_library_position_group() -> Option<InstanceGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM InstanceGroup WHERE libraryPosition IS NOT NULL ORDER BY libraryPosition ASC LIMIT 1";
    fn count_groups() -> i64 =
        "SELECT COUNT(*) FROM InstanceGroup";

    // ---- InstanceGroup single-row updates ----
    fn set_group_library_position(id: i32, library_position: Option<i32>) -> usize =
        "UPDATE InstanceGroup SET libraryPosition = :library_position WHERE id = :id";
    fn set_group_index(id: i32, group_index: i32) -> usize =
        "UPDATE InstanceGroup SET groupIndex = :group_index WHERE id = :id";
    fn set_group_name(id: i32, name: &str) -> usize =
        "UPDATE InstanceGroup SET name = :name WHERE id = :id";
    fn set_group_index_and_library_position(id: i32, group_index: i32, library_position: Option<i32>) -> usize =
        "UPDATE InstanceGroup SET groupIndex = :group_index, libraryPosition = :library_position WHERE id = :id";

    // ---- InstanceGroup shift (update_many) queries ----
    fn shift_group_library_positions_down(gt: i32, lte: i32) -> usize =
        "UPDATE InstanceGroup SET libraryPosition = libraryPosition - 1 WHERE libraryPosition > :gt AND libraryPosition <= :lte";
    fn shift_group_library_positions_up(gte: i32, lt: i32) -> usize =
        "UPDATE InstanceGroup SET libraryPosition = libraryPosition + 1 WHERE libraryPosition >= :gte AND libraryPosition < :lt";
    fn shift_all_group_library_positions_up_from(gte: i32) -> usize =
        "UPDATE InstanceGroup SET libraryPosition = libraryPosition + 1 WHERE libraryPosition >= :gte";
    fn shift_all_group_library_positions_down_after(gt: i32) -> usize =
        "UPDATE InstanceGroup SET libraryPosition = libraryPosition - 1 WHERE libraryPosition > :gt";

    // ---- InstanceGroup delete ----
    fn delete_group(id: i32) -> usize =
        "DELETE FROM InstanceGroup WHERE id = :id";
}

/// `INSERT` shared by `add_instance_tx` and its `QueryCheck`. `favorite` and
/// `hasPackUpdate` are omitted so they take their DDL `DEFAULT false`.
const INSERT_INSTANCE_SQL: &str =
    "INSERT INTO Instance (name, shortpath, \"index\", groupId, libraryPosition)
         VALUES (:name, :shortpath, :index, :group_id, :library_position)";

/// `INSERT` shared by `insert_group`/`create_default_group_tx` and its `QueryCheck`.
const INSERT_GROUP_SQL: &str = "INSERT INTO InstanceGroup (name, groupIndex, libraryPosition)
         VALUES (:name, :group_index, :library_position)";

/// Point-writes the default instance group into the singleton config row, run
/// inside `create_default_group_tx`.
const UPDATE_APP_CONFIG_DEFAULT_INSTANCE_GROUP_SQL: &str =
    "UPDATE AppConfiguration SET defaultInstanceGroup = :group_id WHERE id = 0";

/// One index shift to run inside `move_instance_tx`, mapping to a registered
/// `shift_instance_indexes_*` query. Encodes the conditional index shift
/// applied alongside the final instance update.
#[derive(Debug, Clone, Copy)]
pub enum IndexShift {
    DownExclusive { group_id: i32, gt: i32, lt: i32 },
    UpRange { group_id: i32, gte: i32, lt: i32 },
    DownAfter { group_id: i32, gt: i32 },
    UpFrom { group_id: i32, gte: i32 },
}

/// One group row to restamp during `arrange_library_tx`.
#[derive(Debug, Clone, Copy)]
pub struct GroupArrange {
    pub id: i32,
    pub group_index: i32,
    pub library_position: Option<i32>,
    /// The default group updates `groupIndex` only (its `libraryPosition`
    /// stays null); folders update both.
    pub set_library_position: bool,
}

/// One instance row to restamp during `arrange_library_tx`.
#[derive(Debug, Clone, Copy)]
pub struct InstanceArrange {
    pub id: i32,
    pub index: i32,
    pub library_position: Option<i32>,
}

/// Inserts a group and returns its new id. Hand-written to return
/// `last_insert_rowid()`.
pub fn insert_group_conn(
    conn: &impl crate::db_exec::WriteAccess,
    name: &str,
    group_index: i32,
    library_position: Option<i32>,
) -> Result<i64, rusqlite::Error> {
    let mut st = conn.prepare_cached(INSERT_GROUP_SQL)?;
    st.execute(named_params! {
        ":name": name, ":group_index": group_index, ":library_position": library_position,
    })?;
    Ok(conn.last_insert_rowid())
}

/// Pool-routing wrapper for [`insert_group_conn`].
pub async fn insert_group(
    db: &Db,
    name: String,
    group_index: i32,
    library_position: Option<i32>,
) -> DbResult<i64> {
    db.write(move |conn| {
        Ok(insert_group_conn(
            &conn,
            &name,
            group_index,
            library_position,
        )?)
    })
    .await
}

/// Runs the conditional index shifts and the moved instance's final placement
/// in one write-pool transaction.
pub async fn move_instance_tx(
    db: &Db,
    shifts: Vec<IndexShift>,
    instance_id: i32,
    group_id: i32,
    index: i32,
    library_position: Option<i32>,
) -> DbResult<()> {
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        for shift in &shifts {
            match *shift {
                IndexShift::DownExclusive { group_id, gt, lt } => {
                    shift_instance_indexes_down_exclusive_conn(&tx, group_id, gt, lt)?;
                }
                IndexShift::UpRange { group_id, gte, lt } => {
                    shift_instance_indexes_up_range_conn(&tx, group_id, gte, lt)?;
                }
                IndexShift::DownAfter { group_id, gt } => {
                    shift_instance_indexes_down_after_conn(&tx, group_id, gt)?;
                }
                IndexShift::UpFrom { group_id, gte } => {
                    shift_instance_indexes_up_from_conn(&tx, group_id, gte)?;
                }
            }
        }
        set_instance_group_index_library_position_conn(
            &tx,
            instance_id,
            group_id,
            index,
            library_position,
        )?;
        tx.commit()?;
        Ok(())
    })
    .await
}

/// Restamps folder/default-group `groupIndex`/`libraryPosition` and ungrouped
/// instance `index`/`libraryPosition` in one write-pool transaction.
pub async fn arrange_library_tx(
    db: &Db,
    groups: Vec<GroupArrange>,
    instances: Vec<InstanceArrange>,
) -> DbResult<()> {
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        for g in &groups {
            if g.set_library_position {
                set_group_index_and_library_position_conn(
                    &tx,
                    g.id,
                    g.group_index,
                    g.library_position,
                )?;
            } else {
                set_group_index_conn(&tx, g.id, g.group_index)?;
            }
        }
        for i in &instances {
            set_instance_index_and_library_position_conn(&tx, i.id, i.index, i.library_position)?;
        }
        tx.commit()?;
        Ok(())
    })
    .await
}

/// Restamps every instance's `index` within a folder in one write-pool
/// transaction.
pub async fn set_instance_indexes_tx(db: &Db, updates: Vec<(i32, i32)>) -> DbResult<()> {
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        for (id, index) in &updates {
            set_instance_index_conn(&tx, *id, *index)?;
        }
        tx.commit()?;
        Ok(())
    })
    .await
}

/// Deletes any row at `shortpath` then inserts the new instance, returning its
/// id, in one write-pool transaction.
pub async fn add_instance_tx(
    db: &Db,
    name: String,
    shortpath: String,
    index: i32,
    group_id: i32,
    library_position: Option<i32>,
) -> DbResult<i64> {
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        delete_instances_by_shortpath_conn(&tx, &shortpath)?;
        {
            let mut st = tx.prepare_cached(INSERT_INSTANCE_SQL)?;
            st.execute(named_params! {
                ":name": name, ":shortpath": shortpath, ":index": index,
                ":group_id": group_id, ":library_position": library_position,
            })?;
        }
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    })
    .await
}

/// Moves every instance of `group_id` into `default_group_id` (offsetting their
/// index by `base_index`) then deletes the group, in one write-pool transaction.
///
/// `base_index` is computed by the caller (the instance-side oddity counts the
/// group being deleted); this fn does not recompute it.
pub async fn delete_group_tx(
    db: &Db,
    group_id: i32,
    default_group_id: i32,
    base_index: i32,
) -> DbResult<()> {
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        move_all_instances_to_group_conn(&tx, group_id, default_group_id, base_index)?;
        delete_group_conn(&tx, group_id)?;
        tx.commit()?;
        Ok(())
    })
    .await
}

/// Creates the `"localize➽default"` group and points the singleton config row
/// at it, in one write-pool transaction. Returns the new group id.
pub async fn create_default_group_tx(db: &Db, group_index: i32) -> DbResult<i32> {
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        {
            let mut st = tx.prepare_cached(INSERT_GROUP_SQL)?;
            st.execute(named_params! {
                ":name": "localize➽default", ":group_index": group_index,
                ":library_position": None::<i32>,
            })?;
        }
        let id = tx.last_insert_rowid() as i32;
        {
            let mut st = tx.prepare_cached(UPDATE_APP_CONFIG_DEFAULT_INSTANCE_GROUP_SQL)?;
            st.execute(named_params! { ":group_id": id })?;
        }
        tx.commit()?;
        Ok(id)
    })
    .await
}

const INSERT_INSTANCE_CHECK: QueryCheck = QueryCheck {
    name: "add_instance_tx::insert_instance",
    sql: INSERT_INSTANCE_SQL,
    params: &[
        ":name",
        ":shortpath",
        ":index",
        ":group_id",
        ":library_position",
    ],
    columns: None,
    class: crate::registry::class_of(INSERT_INSTANCE_SQL),
    routes_write: true,
};
const INSERT_GROUP_CHECK: QueryCheck = QueryCheck {
    name: "insert_group",
    sql: INSERT_GROUP_SQL,
    params: &[":name", ":group_index", ":library_position"],
    columns: None,
    class: crate::registry::class_of(INSERT_GROUP_SQL),
    routes_write: true,
};
const UPDATE_APP_CONFIG_DEFAULT_INSTANCE_GROUP_CHECK: QueryCheck = QueryCheck {
    name: "create_default_group_tx::set_default_instance_group",
    sql: UPDATE_APP_CONFIG_DEFAULT_INSTANCE_GROUP_SQL,
    params: &[":group_id"],
    columns: None,
    class: crate::registry::class_of(UPDATE_APP_CONFIG_DEFAULT_INSTANCE_GROUP_SQL),
    routes_write: true,
};

/// Every checkable query in this module: the macro-generated `QUERIES` plus the
/// three hand-written statements.
pub fn all_queries() -> Vec<QueryCheck> {
    let mut all: Vec<QueryCheck> = QUERIES.to_vec();
    all.push(INSERT_INSTANCE_CHECK);
    all.push(INSERT_GROUP_CHECK);
    all.push(UPDATE_APP_CONFIG_DEFAULT_INSTANCE_GROUP_CHECK);
    all
}

/// Deletes an instance together with the mod-file cache rows that belong to
/// it, in one write-pool transaction.
///
/// The `ON DELETE CASCADE` edge does this itself whenever foreign keys are
/// enforced, but `fk::sweep_and_decide` falls back to leaving them off for the
/// whole session when it meets a violation it cannot repair, and
/// `GDL_DISABLE_FK_ENFORCEMENT=1` selects the same state. Issuing the delete
/// here rather than leaving it to the caller keeps the two halves together —
/// without it, an FK-off session leaks `ModFileCache` rows for every deleted
/// instance, which in turn keeps their referenced `ModMetadata` rows (and
/// transitively the CurseForge/Modrinth caches) alive, since
/// `gc_orphan_metadata` only reclaims metadata nothing still references.
pub async fn delete_instance_tx(db: &Db, id: i32) -> DbResult<usize> {
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        crate::repos::mod_file_cache::delete_mod_file_cache_by_instance_conn(&tx, id)?;
        let removed = delete_instance_conn(&tx, id)?;
        tx.commit()?;
        Ok(removed)
    })
    .await
}
