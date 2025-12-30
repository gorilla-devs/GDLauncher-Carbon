//! Instance and InstanceGroup queries.

use crate::define_query;

// Instance read queries
define_query!(FindInstanceById, "SELECT * FROM Instance WHERE id = ?1");
define_query!(FindInstanceByShortpath, "SELECT * FROM Instance WHERE shortpath = ?1");
define_query!(ListInstances, "SELECT * FROM Instance ORDER BY groupId, `index`");
define_query!(ListInstancesByGroup, "SELECT * FROM Instance WHERE groupId = ?1 ORDER BY `index`");
define_query!(CountInstances, "SELECT COUNT(*) FROM Instance");
define_query!(CountInstancesByGroup, "SELECT COUNT(*) FROM Instance WHERE groupId = ?1");
define_query!(GetMaxInstanceIndex, "SELECT MAX(`index`) FROM Instance WHERE groupId = ?1");

// Instance create queries
define_query!(
    CreateInstance,
    r#"INSERT INTO Instance (name, shortpath, favorite, hasPackUpdate, `index`, groupId)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#
);

// Instance update queries
define_query!(UpdateInstanceName, "UPDATE Instance SET name = ?2 WHERE id = ?1");
define_query!(UpdateInstanceNameAndShortpath, "UPDATE Instance SET name = ?2, shortpath = ?3 WHERE id = ?1");
define_query!(UpdateInstanceFavorite, "UPDATE Instance SET favorite = ?2 WHERE id = ?1");
define_query!(UpdateInstanceHasPackUpdate, "UPDATE Instance SET hasPackUpdate = ?2 WHERE id = ?1");
define_query!(UpdateInstanceIndex, "UPDATE Instance SET `index` = ?2 WHERE id = ?1");
define_query!(UpdateInstanceGroup, "UPDATE Instance SET groupId = ?2 WHERE id = ?1");
define_query!(
    UpdateInstanceGroupAndIndex,
    "UPDATE Instance SET groupId = ?2, `index` = ?3 WHERE id = ?1"
);

// Batch index updates for reordering
define_query!(
    IncrementInstanceIndices,
    "UPDATE Instance SET `index` = `index` + 1 WHERE groupId = ?1 AND `index` >= ?2"
);
define_query!(
    DecrementInstanceIndices,
    "UPDATE Instance SET `index` = `index` - 1 WHERE groupId = ?1 AND `index` > ?2"
);

// Instance delete queries
define_query!(DeleteInstance, "DELETE FROM Instance WHERE id = ?1");
define_query!(DeleteInstanceByShortpath, "DELETE FROM Instance WHERE shortpath = ?1");

// InstanceGroup read queries
define_query!(FindInstanceGroupById, "SELECT * FROM InstanceGroup WHERE id = ?1");
define_query!(FindInstanceGroupByName, "SELECT * FROM InstanceGroup WHERE name = ?1");
define_query!(ListInstanceGroups, "SELECT * FROM InstanceGroup ORDER BY groupIndex");
define_query!(CountInstanceGroups, "SELECT COUNT(*) FROM InstanceGroup");
define_query!(GetMaxGroupIndex, "SELECT MAX(groupIndex) FROM InstanceGroup");

// InstanceGroup with instances join
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
    "INSERT INTO InstanceGroup (name, groupIndex) VALUES (?1, ?2)"
);

// InstanceGroup update queries
define_query!(UpdateInstanceGroupName, "UPDATE InstanceGroup SET name = ?2 WHERE id = ?1");
define_query!(UpdateInstanceGroupIndex, "UPDATE InstanceGroup SET groupIndex = ?2 WHERE id = ?1");

// Batch group index updates for reordering
define_query!(
    IncrementGroupIndices,
    "UPDATE InstanceGroup SET groupIndex = groupIndex + 1 WHERE groupIndex >= ?1"
);
define_query!(
    DecrementGroupIndices,
    "UPDATE InstanceGroup SET groupIndex = groupIndex - 1 WHERE groupIndex > ?1"
);

// InstanceGroup delete queries
define_query!(DeleteInstanceGroup, "DELETE FROM InstanceGroup WHERE id = ?1");

