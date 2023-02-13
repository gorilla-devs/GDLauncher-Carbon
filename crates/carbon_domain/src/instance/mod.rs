use crate::minecraft_package::MinecraftPackage;
use crate::{
    instance::InstanceStatus::NotPersisted,
    minecraft_package::configuration::MinecraftPackageConfigurationFile,
};
use rspc::Type;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Ord, PartialOrd, Type)]
pub enum InstanceStatus {
    NotPersisted,
    Installing(PathBuf),
    Ready(PathBuf),
}

impl Default for InstanceStatus {
    fn default() -> Self {
        NotPersisted
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd, Type)]
pub struct Instance {
    pub name: String,
    pub uuid: String,
    pub played_time: Duration,
    pub last_played: Option<std::time::SystemTime>,
    pub notes: String,
    pub minecraft_package: MinecraftPackage,
    pub status: InstanceStatus,
}

impl Instance {
    pub fn mutate_persistence_status(self, new_persistence_status: InstanceStatus) -> Instance {
        let mut new_instance = self;
        new_instance.status = new_persistence_status;
        new_instance
    }
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct InstanceConfigurationFile {
    pub instance_name: String,
    pub notes: String,
    pub played_time: Duration,
    pub last_played: Option<std::time::SystemTime>,
    #[serde(rename = "minecraft_package")]
    pub minecraft_package_configuration: MinecraftPackageConfigurationFile,
}

impl From<&Instance> for InstanceConfigurationFile {
    fn from(value: &Instance) -> Self {
        InstanceConfigurationFile {
            instance_name: value.name.clone(),
            notes: value.notes.clone(),
            played_time: value.played_time,
            last_played: value.last_played,
            minecraft_package_configuration: MinecraftPackageConfigurationFile {
                version: value.minecraft_package.version.clone(),
                description: value.minecraft_package.description.clone(),
                modloader: value.minecraft_package.mod_loaders.clone(),
            },
        }
    }
}
