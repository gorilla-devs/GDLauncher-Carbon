//! Instance and InstanceGroup queries.

use crate::define_query;
use crate::models::{Instance, InstanceGroup};

// Instance read queries
define_query!(FindInstanceById, "SELECT * FROM Instance WHERE id = ?1", (id: i32) -> Instance);
define_query!(
    FindInstanceByShortpath,
    "SELECT * FROM Instance WHERE shortpath = ?1",
    (shortpath: &str) -> Instance
);
define_query!(
    ListInstances,
    "SELECT * FROM Instance ORDER BY groupId, `index`",
    () -> Instance
);
define_query!(
    ListInstancesByGroup,
    "SELECT * FROM Instance WHERE groupId = ?1 ORDER BY `index`",
    (group_id: i32) -> Instance
);
define_query!(CountInstances, "SELECT COUNT(*) FROM Instance", () => i32);
define_query!(
    CountInstancesByGroup,
    "SELECT COUNT(*) FROM Instance WHERE groupId = ?1",
    (group_id: i32) => i32
);
define_query!(
    GetMaxInstanceIndex,
    "SELECT MAX(`index`) FROM Instance WHERE groupId = ?1",
    (group_id: i32) => Option<i32>
);

// Instance create queries
define_query!(
    CreateInstance,
    r#"INSERT INTO Instance (name, shortpath, favorite, hasPackUpdate, `index`, groupId)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
    (name: &str, shortpath: &str, favorite: bool, has_pack_update: bool, index: i32, group_id: i32)
);

// Instance update queries
define_query!(
    UpdateInstanceName,
    "UPDATE Instance SET name = ?2 WHERE id = ?1",
    (id: i32, name: &str)
);
define_query!(
    UpdateInstanceNameAndShortpath,
    "UPDATE Instance SET name = ?2, shortpath = ?3 WHERE id = ?1",
    (id: i32, name: &str, shortpath: &str)
);
define_query!(
    UpdateInstanceFavorite,
    "UPDATE Instance SET favorite = ?2 WHERE id = ?1",
    (id: i32, favorite: bool)
);
define_query!(
    UpdateInstanceHasPackUpdate,
    "UPDATE Instance SET hasPackUpdate = ?2 WHERE id = ?1",
    (id: i32, has_pack_update: bool)
);
define_query!(
    UpdateInstanceIndex,
    "UPDATE Instance SET `index` = ?2 WHERE id = ?1",
    (id: i32, index: i32)
);
define_query!(
    UpdateInstanceGroup,
    "UPDATE Instance SET groupId = ?2 WHERE id = ?1",
    (id: i32, group_id: i32)
);
define_query!(
    UpdateInstanceGroupAndIndex,
    "UPDATE Instance SET groupId = ?2, `index` = ?3 WHERE id = ?1",
    (id: i32, group_id: i32, index: i32)
);

// Batch index updates for reordering
define_query!(
    IncrementInstanceIndices,
    "UPDATE Instance SET `index` = `index` + 1 WHERE groupId = ?1 AND `index` >= ?2",
    (group_id: i32, from_index: i32)
);
define_query!(
    DecrementInstanceIndices,
    "UPDATE Instance SET `index` = `index` - 1 WHERE groupId = ?1 AND `index` > ?2",
    (group_id: i32, from_index: i32)
);

// Instance delete queries
define_query!(DeleteInstance, "DELETE FROM Instance WHERE id = ?1", (id: i32));
define_query!(
    DeleteInstanceByShortpath,
    "DELETE FROM Instance WHERE shortpath = ?1",
    (shortpath: &str)
);

// InstanceGroup read queries
define_query!(
    FindInstanceGroupById,
    "SELECT * FROM InstanceGroup WHERE id = ?1",
    (id: i32) -> InstanceGroup
);
define_query!(
    FindInstanceGroupByName,
    "SELECT * FROM InstanceGroup WHERE name = ?1",
    (name: &str) -> InstanceGroup
);
define_query!(
    ListInstanceGroups,
    "SELECT * FROM InstanceGroup ORDER BY groupIndex",
    () -> InstanceGroup
);
define_query!(CountInstanceGroups, "SELECT COUNT(*) FROM InstanceGroup", () => i32);
define_query!(
    GetMaxGroupIndex,
    "SELECT MAX(groupIndex) FROM InstanceGroup",
    () => Option<i32>
);

// InstanceGroup with instances join - legacy for complex JOINs
define_query!(
    FindInstanceGroupWithInstances,
    r#"SELECT ig.*, i.id as instance_id, i.name as instance_name, i.shortpath, i.favorite, i.hasPackUpdate, i.`index`
    FROM InstanceGroup ig
    LEFT JOIN Instance i ON ig.id = i.groupId
    WHERE ig.id = ?1
    ORDER BY i.`index`"#
);

define_query!(
    ListInstanceGroupsWithInstances,
    r#"SELECT ig.*, i.id as instance_id, i.name as instance_name, i.shortpath, i.favorite, i.hasPackUpdate, i.`index`
    FROM InstanceGroup ig
    LEFT JOIN Instance i ON ig.id = i.groupId
    ORDER BY ig.groupIndex, i.`index`"#
);

// InstanceGroup create queries
define_query!(
    CreateInstanceGroup,
    "INSERT INTO InstanceGroup (name, groupIndex) VALUES (?1, ?2)",
    (name: &str, group_index: i32)
);

// InstanceGroup update queries
define_query!(
    UpdateInstanceGroupName,
    "UPDATE InstanceGroup SET name = ?2 WHERE id = ?1",
    (id: i32, name: &str)
);
define_query!(
    UpdateInstanceGroupIndex,
    "UPDATE InstanceGroup SET groupIndex = ?2 WHERE id = ?1",
    (id: i32, group_index: i32)
);

// Batch group index updates for reordering
define_query!(
    IncrementGroupIndices,
    "UPDATE InstanceGroup SET groupIndex = groupIndex + 1 WHERE groupIndex >= ?1",
    (from_index: i32)
);
define_query!(
    DecrementGroupIndices,
    "UPDATE InstanceGroup SET groupIndex = groupIndex - 1 WHERE groupIndex > ?1",
    (from_index: i32)
);

// InstanceGroup delete queries
define_query!(
    DeleteInstanceGroup,
    "DELETE FROM InstanceGroup WHERE id = ?1",
    (id: i32)
);
