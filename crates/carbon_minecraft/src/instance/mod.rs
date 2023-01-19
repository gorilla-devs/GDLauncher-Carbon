pub mod scan;
pub mod configuration;
pub mod conversion;
pub mod write;
pub mod delete;

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::instance::InstanceStatus::NotPersisted;
use crate::minecraft_package::MinecraftPackage;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum InstanceStatus{
    Persisted(PathBuf),
    NotPersisted
}

impl Default for InstanceStatus{
    fn default() -> Self {
        NotPersisted
    }
}

#[derive(Debug, Serialize, Deserialize, Hash, Default)]
pub struct Instance{
    pub name: String,
    pub minecraft_package : MinecraftPackage,
    pub persistence_status: InstanceStatus,
}

impl Instance {

    pub fn mutate_persistence_status(self, new_persistence_status : InstanceStatus) -> Instance{
        let mut new_instance = self;
        new_instance.persistence_status = new_persistence_status;
        new_instance
    }

}

pub mod consts{
    pub(crate) const TEMP_CONFIG_FILE_PREFIX: &str = ".temp";
    pub(crate) const CONFIGURATION_FILE_RELATIVE_PATH: &str = ".conf.json";
    pub(crate) const MINECRAFT_PACKAGE_RELATIVE_PATH: &str = "minecraft";
}

