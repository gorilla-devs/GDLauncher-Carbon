use carbon_domain::instance::{Instance, InstanceStatus};
use carbon_domain::minecraft_package::{MinecraftPackage, MinecraftPackageStatus};
use rspc::Type;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Type, Deserialize)]
pub struct CreateInstanceDto {
    pub name: String,
    pub minecraft_version: String,
    pub path_to_save_at: Option<PathBuf>,
}

impl CreateInstanceDto {
    pub async fn into_instance_with_id(self, uuid: u128) -> Instance {
        Instance {
            name: self.name,
            id: uuid,
            played_time: Duration::default(),
            last_played: None,
            minecraft_package: MinecraftPackage {
                version: self.minecraft_version,
                mods: Default::default(),
                description: "".to_string(),
                mod_loaders: Default::default(),
                status: MinecraftPackageStatus::NotPersisted,
            },
            persistence_status: self
                .path_to_save_at
                .map_or(InstanceStatus::NotPersisted, |path_to_save| {
                    InstanceStatus::Ready(path_to_save)
                }),
            notes: "".to_string(),
        }
    }
}
