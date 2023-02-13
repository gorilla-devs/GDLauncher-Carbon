use carbon_domain::instance::InstanceStatus::NotPersisted;
use carbon_domain::instance::{Instance, InstanceStatus};
use carbon_domain::minecraft_package::{MinecraftPackage, MinecraftPackageStatus};
use rspc::Type;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Type, Deserialize, Serialize, Clone)]
pub struct CreateInstanceDto {
    pub name: String,
    pub minecraft_version: String,
    pub path_to_save_at: Option<PathBuf>,
}

impl CreateInstanceDto {
    pub async fn into_instance_with_id(self, uuid: u128) -> Instance {
        Instance {
            name: self.name,
            uuid: uuid.to_string(),
            played_time: Duration::default(),
            last_played: None,
            minecraft_package: MinecraftPackage {
                version: self.minecraft_version,
                mods: Default::default(),
                description: "".to_string(),
                mod_loaders: Default::default(),
                status: MinecraftPackageStatus::NotPersisted,
            },
            status: NotPersisted,
            notes: "".to_string(),
        }
    }
}
