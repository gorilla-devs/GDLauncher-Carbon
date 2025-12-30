//! Instance and InstanceGroup models.

use carbon_macro::FromRow;
use serde::{Deserialize, Serialize};

/// Game instance (Minecraft installation).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    /// Auto-incremented ID.
    pub id: i32,
    /// Display name.
    pub name: String,
    /// Short path identifier (unique).
    pub shortpath: String,
    /// Whether this instance is favorited.
    pub favorite: bool,
    /// Whether there's a modpack update available.
    pub has_pack_update: bool,
    /// Display order index within the group.
    pub index: i32,
    /// ID of the group this instance belongs to.
    pub group_id: i32,
}

/// Group for organizing instances.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct InstanceGroup {
    /// Auto-incremented ID.
    pub id: i32,
    /// Display name.
    pub name: String,
    /// Display order index.
    pub group_index: i32,
}

/// Instance group with its instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceGroupWithInstances {
    pub group: InstanceGroup,
    pub instances: Vec<Instance>,
}

/// Builder for creating new instances.
#[derive(Debug, Clone)]
pub struct NewInstance {
    pub name: String,
    pub shortpath: String,
    pub group_id: i32,
    pub index: i32,
    pub favorite: bool,
}

impl NewInstance {
    pub fn new(name: String, shortpath: String, group_id: i32, index: i32) -> Self {
        Self {
            name,
            shortpath,
            group_id,
            index,
            favorite: false,
        }
    }
}
