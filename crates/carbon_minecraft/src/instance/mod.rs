pub mod configuration;
pub mod conversion;
pub mod delete;
pub mod scan;
pub mod write;

use crate::minecraft_package::MinecraftPackage;
use crate::{
    instance::InstanceStatus::NotPersisted,
    minecraft_package::configuration::MinecraftPackageConfigurationFile,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum InstanceStatus {
    NotPersisted,
    Installing,
    Ready(PathBuf),
}

impl Default for InstanceStatus {
    fn default() -> Self {
        NotPersisted
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Instance {
    pub name: String,
    pub id: u128,
    pub played_time: Duration,
    pub last_played: std::time::SystemTime,
    pub notes: String,
    pub minecraft_package: MinecraftPackage,
    pub persistence_status: InstanceStatus,
}

impl Instance {
    fn new(mc_version: impl Into<String>) -> Self {
        Instance {
            name: String::from("Unnamed Instance"),
            id: Uuid::new_v4().to_string(),
            played_time: Default::default(),
            last_played: (),
            notes: "".to_string(),
            minecraft_package: MinecraftPackage::new(mc_version),
            persistence_status: InstanceStatus::default(),
        }
    }
}

impl Instance {
    pub fn mutate_persistence_status(self, new_persistence_status: InstanceStatus) -> Instance {
        let mut new_instance = self;
        new_instance.persistence_status = new_persistence_status;
        new_instance
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceConfigurationFile {
    pub instance_name: String,
    #[serde(rename = "minecraft_package")]
    pub minecraft_package_configuration: MinecraftPackageConfigurationFile,
}

impl From<&Instance> for InstanceConfigurationFile {
    fn from(value: &Instance) -> Self {
        InstanceConfigurationFile {
            instance_name: value.name.clone(),
            minecraft_package_configuration: MinecraftPackageConfigurationFile {
                version: value.minecraft_package.version.clone(),
                description: value.minecraft_package.description.clone(),
                modloader: value.minecraft_package.modloaders.clone(),
            },
        }
    }
}

pub mod consts {
    pub(crate) const TEMP_CONFIG_FILE_PREFIX: &str = ".temp";
    pub(crate) const CONFIGURATION_FILE_RELATIVE_PATH: &str = ".conf.json";
    pub(crate) const MINECRAFT_PACKAGE_RELATIVE_PATH: &str = "minecraft";
}

pub struct Instances {
    pub instances_path: PathBuf,
    pub instances: HashSet<Instance>,
}

impl Instances {
    pub fn new(instances_path: PathBuf) -> Self {
        Instances {
            instances_path,
            instances: HashSet::new(),
        }
    }
}
