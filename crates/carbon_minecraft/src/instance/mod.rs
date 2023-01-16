pub mod scan;
pub mod error;
pub mod configuration;
pub mod conversion;
pub mod write;
pub mod delete;

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::minecraft_package::MinecraftPackage;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum InstanceStatus{
    Persisted(PathBuf),
    NotPersisted
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct Instance{
    pub name: String,
    pub minecraft_package : MinecraftPackage,
    pub persistence_status: InstanceStatus
}

impl Instance {

    pub fn mutate_persistence_status(self, new_persistence_status : InstanceStatus) -> Instance{
        let mut new_instance = self;
        new_instance.persistence_status = new_persistence_status;
        new_instance
    }

}

impl Default for Instance {
    // todo : provisional implementation, but is good to have a Default impl for the root of our domain
    fn default() -> Self {
        Instance{
            name: "".to_string(),
            minecraft_package: MinecraftPackage {
                version: "".to_string(),
                mods: Default::default(),
                core_jars: vec![],
                path: Default::default(),
            },
            persistence_status: InstanceStatus::NotPersisted,
        }
    }
}



pub mod consts{
    #[cfg(not(target_os="windows"))]
    pub(crate) const SUBFOLDERS_TREE: &[&str] = &[
        "minecraft",
        "minecraft/mods",
        "minecraft/core",
        "minecraft/save_files",
    ];

    #[cfg(target_os="windows")]
    pub(crate) const SUBFOLDERS_TREE: &'static [&'static str] = &[
        PathBuf::from("minecraft"),
        PathBuf::from(r"minecraft\mods"),
        PathBuf::from(r"minecraft\core"),
        PathBuf::from(r"minecraft\save_files"),
    ];

    pub(crate) const CONFIGURATION_FILE_RELATIVE_PATH: &str = ".conf.json";

    pub(crate) const MINECRAFT_PACKAGE_RELATIVE_PATH: &str = "minecraft";

    pub(crate) const FILES_TREE: &[&str] = &[CONFIGURATION_FILE_RELATIVE_PATH];

}

