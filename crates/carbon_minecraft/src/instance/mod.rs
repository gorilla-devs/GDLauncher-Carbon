pub mod scan;
pub mod error;
pub mod configuration;
pub mod conversion;
pub mod write;
pub mod delete;

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::instance;
use crate::minecraft_package::MinecraftPackage;

#[derive(Debug, Serialize, Deserialize, Hash)]
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
        let mut new_instance = Instance::from(self);
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





