//! Repository queries for the `Server` and `ServerGroup` tables.
//!
//! Mirrors the instance repository: the same two parallel ordering systems
//! (`groupIndex`/`index` and the nullable `libraryPosition`) are kept in sync,
//! and `libraryPosition IS NOT NULL` is used as a deliberate proxy for
//! "belongs to the default group / library root" so a query never has to
//! resolve the default group id. None of these queries add or remove an
//! `ORDER BY` relative to the prisma-client-rust originals.
//!
//! Two behaviors differ from the instance side and are preserved verbatim:
//! the library-position shifts during a group move are scoped to the default
//! group on the `Server` table (the instance side shifts across all rows), and
//! `delete_server_group` computes `base_index` from the DEFAULT group (the
//! instance side counts the group being deleted).

use crate::dbtypes::DbDateTime;
use crate::queries;
use crate::registry::{DynamicQuery, QueryCheck};
use chrono::{DateTime, FixedOffset};
use rusqlite::{Connection, named_params};

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ServerRow {
    pub id: i32,
    pub name: String,
    pub shortpath: String,
    pub favorite: bool,
    pub index: i32,
    pub library_position: Option<i32>,
    pub group_id: i32,
    pub server_type: String,
    pub game_version: String,
    pub port: i32,
    pub motd: String,
    pub max_players: i32,
    pub online_mode: bool,
    pub xmx: i32,
    pub xms: i32,
    pub extra_java_args: String,
    pub auto_restart: bool,
    pub date_created: DateTime<FixedOffset>,
    pub last_started: Option<DateTime<FixedOffset>>,
    pub provider_type: String,
    pub hosted_server_id: Option<String>,
    pub icon_revision: Option<i32>,
    pub modloader_type: Option<String>,
    pub modloader_version: Option<String>,
    pub modpack_platform: Option<String>,
    pub modpack_project_id: Option<String>,
    pub modpack_file_id: Option<String>,
}

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct ServerGroupRow {
    pub id: i32,
    pub name: String,
    pub group_index: i32,
    pub library_position: Option<i32>,
}

queries! {
    // ---- Server reads ----
    fn get_all_servers() -> Vec<ServerRow> =
        "SELECT id, name, shortpath, favorite, \"index\", libraryPosition, groupId, serverType, gameVersion, port, motd, maxPlayers, onlineMode, xmx, xms, extraJavaArgs, autoRestart, dateCreated, lastStarted, providerType, hostedServerId, iconRevision, modloaderType, modloaderVersion, modpackPlatform, modpackProjectId, modpackFileId FROM Server";
    fn get_all_servers_ordered_by_index() -> Vec<ServerRow> =
        "SELECT id, name, shortpath, favorite, \"index\", libraryPosition, groupId, serverType, gameVersion, port, motd, maxPlayers, onlineMode, xmx, xms, extraJavaArgs, autoRestart, dateCreated, lastStarted, providerType, hostedServerId, iconRevision, modloaderType, modloaderVersion, modpackPlatform, modpackProjectId, modpackFileId FROM Server ORDER BY \"index\" ASC";
    fn get_servers_by_group(group_id: i32) -> Vec<ServerRow> =
        "SELECT id, name, shortpath, favorite, \"index\", libraryPosition, groupId, serverType, gameVersion, port, motd, maxPlayers, onlineMode, xmx, xms, extraJavaArgs, autoRestart, dateCreated, lastStarted, providerType, hostedServerId, iconRevision, modloaderType, modloaderVersion, modpackPlatform, modpackProjectId, modpackFileId FROM Server WHERE groupId = :group_id";
    fn get_server(id: i32) -> Option<ServerRow> =
        "SELECT id, name, shortpath, favorite, \"index\", libraryPosition, groupId, serverType, gameVersion, port, motd, maxPlayers, onlineMode, xmx, xms, extraJavaArgs, autoRestart, dateCreated, lastStarted, providerType, hostedServerId, iconRevision, modloaderType, modloaderVersion, modpackPlatform, modpackProjectId, modpackFileId FROM Server WHERE id = :id";
    fn min_index_server_in_group(group_id: i32) -> Option<ServerRow> =
        "SELECT id, name, shortpath, favorite, \"index\", libraryPosition, groupId, serverType, gameVersion, port, motd, maxPlayers, onlineMode, xmx, xms, extraJavaArgs, autoRestart, dateCreated, lastStarted, providerType, hostedServerId, iconRevision, modloaderType, modloaderVersion, modpackPlatform, modpackProjectId, modpackFileId FROM Server WHERE groupId = :group_id ORDER BY \"index\" ASC LIMIT 1";
    fn min_library_position_server_in_group(group_id: i32) -> Option<ServerRow> =
        "SELECT id, name, shortpath, favorite, \"index\", libraryPosition, groupId, serverType, gameVersion, port, motd, maxPlayers, onlineMode, xmx, xms, extraJavaArgs, autoRestart, dateCreated, lastStarted, providerType, hostedServerId, iconRevision, modloaderType, modloaderVersion, modpackPlatform, modpackProjectId, modpackFileId FROM Server WHERE groupId = :group_id AND libraryPosition IS NOT NULL ORDER BY libraryPosition ASC LIMIT 1";
    fn max_library_position_server_in_group(group_id: i32) -> Option<ServerRow> =
        "SELECT id, name, shortpath, favorite, \"index\", libraryPosition, groupId, serverType, gameVersion, port, motd, maxPlayers, onlineMode, xmx, xms, extraJavaArgs, autoRestart, dateCreated, lastStarted, providerType, hostedServerId, iconRevision, modloaderType, modloaderVersion, modpackPlatform, modpackProjectId, modpackFileId FROM Server WHERE groupId = :group_id AND libraryPosition IS NOT NULL ORDER BY libraryPosition DESC LIMIT 1";
    fn first_server_in_group(group_id: i32) -> Option<ServerRow> =
        "SELECT id, name, shortpath, favorite, \"index\", libraryPosition, groupId, serverType, gameVersion, port, motd, maxPlayers, onlineMode, xmx, xms, extraJavaArgs, autoRestart, dateCreated, lastStarted, providerType, hostedServerId, iconRevision, modloaderType, modloaderVersion, modpackPlatform, modpackProjectId, modpackFileId FROM Server WHERE groupId = :group_id LIMIT 1";
    fn count_servers_in_group(group_id: i32) -> i64 =
        "SELECT COUNT(*) FROM Server WHERE groupId = :group_id";

    // ---- Server single-row updates ----
    fn set_server_icon_revision(id: i32, icon_revision: Option<i32>) -> usize =
        "UPDATE Server SET iconRevision = :icon_revision WHERE id = :id";
    fn set_server_game_version_and_modloader(id: i32, game_version: &str, modloader_type: Option<&str>, modloader_version: Option<&str>) -> usize =
        "UPDATE Server SET gameVersion = :game_version, modloaderType = :modloader_type, modloaderVersion = :modloader_version WHERE id = :id";
    fn set_server_last_started(id: i32, last_started: Option<DbDateTime>) -> usize =
        "UPDATE Server SET lastStarted = :last_started WHERE id = :id";
    fn set_server_favorite(id: i32, favorite: bool) -> usize =
        "UPDATE Server SET favorite = :favorite WHERE id = :id";
    fn set_server_group_index_library_position(id: i32, group_id: i32, index: i32, library_position: Option<i32>) -> usize =
        "UPDATE Server SET groupId = :group_id, \"index\" = :index, libraryPosition = :library_position WHERE id = :id";
    fn set_server_index_and_library_position(id: i32, index: i32, library_position: Option<i32>) -> usize =
        "UPDATE Server SET \"index\" = :index, libraryPosition = :library_position WHERE id = :id";

    // ---- Server shift (update_many) queries ----
    fn shift_server_indexes_down_exclusive(group_id: i32, gt: i32, lt: i32) -> usize =
        "UPDATE Server SET \"index\" = \"index\" - 1 WHERE groupId = :group_id AND \"index\" > :gt AND \"index\" < :lt";
    fn shift_server_indexes_up_range(group_id: i32, gte: i32, lt: i32) -> usize =
        "UPDATE Server SET \"index\" = \"index\" + 1 WHERE groupId = :group_id AND \"index\" >= :gte AND \"index\" < :lt";
    fn shift_server_indexes_down_after(group_id: i32, gt: i32) -> usize =
        "UPDATE Server SET \"index\" = \"index\" - 1 WHERE groupId = :group_id AND \"index\" > :gt";
    fn shift_server_indexes_up_from(group_id: i32, gte: i32) -> usize =
        "UPDATE Server SET \"index\" = \"index\" + 1 WHERE groupId = :group_id AND \"index\" >= :gte";
    fn shift_server_library_positions_up_in_group_except(group_id: i32, gte: i32, excluded_id: i32) -> usize =
        "UPDATE Server SET libraryPosition = libraryPosition + 1 WHERE groupId = :group_id AND libraryPosition >= :gte AND id <> :excluded_id";
    fn shift_server_library_positions_down_in_group(group_id: i32, gt: i32) -> usize =
        "UPDATE Server SET libraryPosition = libraryPosition - 1 WHERE groupId = :group_id AND libraryPosition > :gt";
    fn shift_server_library_positions_up_in_group(group_id: i32, gte: i32) -> usize =
        "UPDATE Server SET libraryPosition = libraryPosition + 1 WHERE groupId = :group_id AND libraryPosition >= :gte";
    fn shift_server_library_positions_down_scoped(group_id: i32, gt: i32, lte: i32) -> usize =
        "UPDATE Server SET libraryPosition = libraryPosition - 1 WHERE groupId = :group_id AND libraryPosition > :gt AND libraryPosition <= :lte";
    fn shift_server_library_positions_up_scoped(group_id: i32, gte: i32, lt: i32) -> usize =
        "UPDATE Server SET libraryPosition = libraryPosition + 1 WHERE groupId = :group_id AND libraryPosition >= :gte AND libraryPosition < :lt";
    fn move_all_servers_to_group(from_group: i32, to_group: i32, base_index: i32) -> usize =
        "UPDATE Server SET groupId = :to_group, \"index\" = \"index\" + :base_index WHERE groupId = :from_group";

    // ---- Server delete ----
    fn delete_server(id: i32) -> usize =
        "DELETE FROM Server WHERE id = :id";

    // ---- ServerGroup reads ----
    fn get_all_server_groups() -> Vec<ServerGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM ServerGroup";
    fn get_all_server_groups_ordered_by_group_index() -> Vec<ServerGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM ServerGroup ORDER BY groupIndex ASC";
    fn first_server_group_ordered_by_group_index() -> Option<ServerGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM ServerGroup ORDER BY groupIndex ASC LIMIT 1";
    fn get_server_groups_with_library_position_ordered() -> Vec<ServerGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM ServerGroup WHERE libraryPosition IS NOT NULL ORDER BY libraryPosition ASC";
    fn get_server_group(id: i32) -> Option<ServerGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM ServerGroup WHERE id = :id";
    fn find_server_group_by_name(name: &str) -> Option<ServerGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM ServerGroup WHERE name = :name LIMIT 1";
    fn max_library_position_server_group() -> Option<ServerGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM ServerGroup WHERE libraryPosition IS NOT NULL ORDER BY libraryPosition DESC LIMIT 1";
    fn min_library_position_server_group() -> Option<ServerGroupRow> =
        "SELECT id, name, groupIndex, libraryPosition FROM ServerGroup WHERE libraryPosition IS NOT NULL ORDER BY libraryPosition ASC LIMIT 1";
    fn count_server_groups() -> i64 =
        "SELECT COUNT(*) FROM ServerGroup";

    // ---- ServerGroup single-row updates ----
    fn set_server_group_library_position(id: i32, library_position: Option<i32>) -> usize =
        "UPDATE ServerGroup SET libraryPosition = :library_position WHERE id = :id";
    fn set_server_group_index(id: i32, group_index: i32) -> usize =
        "UPDATE ServerGroup SET groupIndex = :group_index WHERE id = :id";
    fn set_server_group_index_and_library_position(id: i32, group_index: i32, library_position: Option<i32>) -> usize =
        "UPDATE ServerGroup SET groupIndex = :group_index, libraryPosition = :library_position WHERE id = :id";
    fn set_server_group_name(id: i32, name: &str) -> usize =
        "UPDATE ServerGroup SET name = :name WHERE id = :id";

    // ---- ServerGroup shift (update_many) queries ----
    fn shift_server_group_library_positions_down(gt: i32, lte: i32) -> usize =
        "UPDATE ServerGroup SET libraryPosition = libraryPosition - 1 WHERE libraryPosition > :gt AND libraryPosition <= :lte";
    fn shift_server_group_library_positions_up(gte: i32, lt: i32) -> usize =
        "UPDATE ServerGroup SET libraryPosition = libraryPosition + 1 WHERE libraryPosition >= :gte AND libraryPosition < :lt";
    fn shift_all_server_group_library_positions_up_from(gte: i32) -> usize =
        "UPDATE ServerGroup SET libraryPosition = libraryPosition + 1 WHERE libraryPosition >= :gte";
    fn shift_all_server_group_library_positions_down_after(gt: i32) -> usize =
        "UPDATE ServerGroup SET libraryPosition = libraryPosition - 1 WHERE libraryPosition > :gt";

    // ---- ServerGroup delete ----
    fn delete_server_group(id: i32) -> usize =
        "DELETE FROM ServerGroup WHERE id = :id";
}

/// `INSERT` shared by `insert_server` and its `QueryCheck`. The columns not
/// listed (motd, maxPlayers, onlineMode, xmx, xms, extraJavaArgs, autoRestart,
/// providerType, hostedServerId, iconRevision, lastStarted) take their DDL
/// defaults, matching PCR's partial `create`. `dateCreated` is written
/// explicitly as epoch-millis (PCR's `@default(now())` was client-generated as
/// millis, not the SQL `CURRENT_TIMESTAMP` text default).
const INSERT_SERVER_SQL: &str =
    "INSERT INTO Server (name, shortpath, \"index\", groupId, gameVersion, port, serverType, modloaderType, modloaderVersion, modpackPlatform, modpackProjectId, modpackFileId, libraryPosition, dateCreated)
         VALUES (:name, :shortpath, :index, :group_id, :game_version, :port, :server_type, :modloader_type, :modloader_version, :modpack_platform, :modpack_project_id, :modpack_file_id, :library_position, :date_created)";

/// `INSERT` shared by `insert_server_group` and its `QueryCheck`.
const INSERT_SERVER_GROUP_SQL: &str =
    "INSERT INTO ServerGroup (name, groupIndex, libraryPosition)
         VALUES (:name, :group_index, :library_position)";

/// One index shift to run inside `move_server_tx`, mapping to a registered
/// `shift_server_indexes_*` query. Mirrors the conditional `index_shifts`
/// vector PCR batched with the final server update.
#[derive(Debug, Clone, Copy)]
pub enum IndexShift {
    DownExclusive { group_id: i32, gt: i32, lt: i32 },
    UpRange { group_id: i32, gte: i32, lt: i32 },
    DownAfter { group_id: i32, gt: i32 },
    UpFrom { group_id: i32, gte: i32 },
}

/// One group row to restamp during `arrange_server_library_tx`.
#[derive(Debug, Clone, Copy)]
pub struct ServerGroupArrange {
    pub id: i32,
    pub group_index: i32,
    pub library_position: Option<i32>,
    /// The default group updates `groupIndex` only (its `libraryPosition`
    /// stays null); folders update both.
    pub set_library_position: bool,
}

/// One server row to restamp during `arrange_server_library_tx`.
#[derive(Debug, Clone, Copy)]
pub struct ServerArrange {
    pub id: i32,
    pub index: i32,
    pub library_position: Option<i32>,
}

/// Inserts a server and returns its new id. Hand-written to return
/// `last_insert_rowid()` and to write `dateCreated` explicitly as millis.
#[allow(clippy::too_many_arguments)]
pub fn insert_server(
    conn: &Connection,
    name: &str,
    shortpath: &str,
    index: i32,
    group_id: i32,
    game_version: &str,
    port: i32,
    server_type: &str,
    modloader_type: Option<&str>,
    modloader_version: Option<&str>,
    modpack_platform: Option<&str>,
    modpack_project_id: Option<&str>,
    modpack_file_id: Option<&str>,
    library_position: Option<i32>,
    date_created: DbDateTime,
) -> Result<i64, rusqlite::Error> {
    let mut st = conn.prepare_cached(INSERT_SERVER_SQL)?;
    st.execute(named_params! {
        ":name": name,
        ":shortpath": shortpath,
        ":index": index,
        ":group_id": group_id,
        ":game_version": game_version,
        ":port": port,
        ":server_type": server_type,
        ":modloader_type": modloader_type,
        ":modloader_version": modloader_version,
        ":modpack_platform": modpack_platform,
        ":modpack_project_id": modpack_project_id,
        ":modpack_file_id": modpack_file_id,
        ":library_position": library_position,
        ":date_created": date_created,
    })?;
    Ok(conn.last_insert_rowid())
}

/// Inserts a server group and returns its new id. Hand-written to return
/// `last_insert_rowid()`.
pub fn insert_server_group(
    conn: &Connection,
    name: &str,
    group_index: i32,
    library_position: Option<i32>,
) -> Result<i64, rusqlite::Error> {
    let mut st = conn.prepare_cached(INSERT_SERVER_GROUP_SQL)?;
    st.execute(named_params! {
        ":name": name, ":group_index": group_index, ":library_position": library_position,
    })?;
    Ok(conn.last_insert_rowid())
}

/// Runs the conditional index shifts and the moved server's final placement in
/// one transaction (PCR `_batch((index_shifts, server.update(...)))`).
pub fn move_server_tx(
    conn: &mut Connection,
    shifts: &[IndexShift],
    server_id: i32,
    group_id: i32,
    index: i32,
    library_position: Option<i32>,
) -> Result<(), rusqlite::Error> {
    let tx = conn.transaction()?;
    for shift in shifts {
        match *shift {
            IndexShift::DownExclusive { group_id, gt, lt } => {
                shift_server_indexes_down_exclusive(&tx, group_id, gt, lt)?;
            }
            IndexShift::UpRange { group_id, gte, lt } => {
                shift_server_indexes_up_range(&tx, group_id, gte, lt)?;
            }
            IndexShift::DownAfter { group_id, gt } => {
                shift_server_indexes_down_after(&tx, group_id, gt)?;
            }
            IndexShift::UpFrom { group_id, gte } => {
                shift_server_indexes_up_from(&tx, group_id, gte)?;
            }
        }
    }
    set_server_group_index_library_position(&tx, server_id, group_id, index, library_position)?;
    tx.commit()
}

/// Restamps folder/default-group `groupIndex`/`libraryPosition` and ungrouped
/// server `index`/`libraryPosition` in one transaction (PCR ran the group and
/// server `_batch`es separately).
pub fn arrange_server_library_tx(
    conn: &mut Connection,
    groups: &[ServerGroupArrange],
    servers: &[ServerArrange],
) -> Result<(), rusqlite::Error> {
    let tx = conn.transaction()?;
    for g in groups {
        if g.set_library_position {
            set_server_group_index_and_library_position(&tx, g.id, g.group_index, g.library_position)?;
        } else {
            set_server_group_index(&tx, g.id, g.group_index)?;
        }
    }
    for s in servers {
        set_server_index_and_library_position(&tx, s.id, s.index, s.library_position)?;
    }
    tx.commit()
}

/// Moves every server of `group_id` into `default_group_id` (offsetting their
/// index by `base_index`) then deletes the group, in one transaction
/// (PCR `_batch((update_many[SetGroupId, IncrementIndex], group.delete))`).
///
/// `base_index` is computed by the caller exactly as PCR did — the server-side
/// oddity counts the DEFAULT group (not the group being deleted); this fn does
/// not recompute it.
pub fn delete_server_group_tx(
    conn: &mut Connection,
    group_id: i32,
    default_group_id: i32,
    base_index: i32,
) -> Result<(), rusqlite::Error> {
    let tx = conn.transaction()?;
    move_all_servers_to_group(&tx, group_id, default_group_id, base_index)?;
    delete_server_group(&tx, group_id)?;
    tx.commit()
}

/// A partial update to a single `Server` row. Each present field becomes one
/// `SET col = :param` clause; absent fields are left untouched. All targeted
/// columns are `NOT NULL`, so a plain `Option<T>` suffices. Covers both PCR
/// partial-update sites (`update_server` and `update_server_properties`).
#[derive(Debug, Default, Clone)]
pub struct ServerPatch {
    pub name: Option<String>,
    pub xmx: Option<i32>,
    pub xms: Option<i32>,
    pub extra_java_args: Option<String>,
    pub auto_restart: Option<bool>,
    pub port: Option<i32>,
    pub motd: Option<String>,
    pub max_players: Option<i32>,
    pub online_mode: Option<bool>,
}

impl ServerPatch {
    /// Assembles `UPDATE Server SET ... WHERE id = :id` from the present fields.
    /// Returns `None` when no field is set (nothing to write).
    pub fn build(self, id: i32) -> Option<DynamicQuery> {
        let mut sets: Vec<&'static str> = Vec::new();
        let mut params: Vec<(&'static str, Box<dyn rusqlite::types::ToSql + Send>)> = Vec::new();

        macro_rules! push {
            ($field:expr, $set:literal, $param:literal) => {
                if let Some(v) = $field {
                    sets.push($set);
                    params.push(($param, Box::new(v)));
                }
            };
        }

        push!(self.name, "name = :name", ":name");
        push!(self.xmx, "xmx = :xmx", ":xmx");
        push!(self.xms, "xms = :xms", ":xms");
        push!(self.extra_java_args, "extraJavaArgs = :extraJavaArgs", ":extraJavaArgs");
        push!(self.auto_restart, "autoRestart = :autoRestart", ":autoRestart");
        push!(self.port, "port = :port", ":port");
        push!(self.motd, "motd = :motd", ":motd");
        push!(self.max_players, "maxPlayers = :maxPlayers", ":maxPlayers");
        push!(self.online_mode, "onlineMode = :onlineMode", ":onlineMode");

        if sets.is_empty() {
            return None;
        }

        params.push((":id", Box::new(id)));
        let sql = format!("UPDATE Server SET {} WHERE id = :id", sets.join(", "));
        Some(DynamicQuery { sql, params })
    }
}

const INSERT_SERVER_CHECK: QueryCheck = QueryCheck {
    name: "insert_server",
    sql: INSERT_SERVER_SQL,
    params: &[
        ":name", ":shortpath", ":index", ":group_id", ":game_version", ":port", ":server_type",
        ":modloader_type", ":modloader_version", ":modpack_platform", ":modpack_project_id",
        ":modpack_file_id", ":library_position", ":date_created",
    ],
    columns: None,
};
const INSERT_SERVER_GROUP_CHECK: QueryCheck = QueryCheck {
    name: "insert_server_group",
    sql: INSERT_SERVER_GROUP_SQL,
    params: &[":name", ":group_index", ":library_position"],
    columns: None,
};

/// Every checkable query in this module: the macro-generated `QUERIES` plus the
/// two hand-written statements.
pub fn all_queries() -> Vec<QueryCheck> {
    let mut all: Vec<QueryCheck> = QUERIES.to_vec();
    all.push(INSERT_SERVER_CHECK);
    all.push(INSERT_SERVER_GROUP_CHECK);
    all
}
