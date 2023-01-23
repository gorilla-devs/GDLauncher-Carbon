use crate::{minecraft_package::MinecraftPackage, modloader::ModLoaderOptions};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MinecraftPackageConfigurationFile {
    pub version: String,
    pub description: String,
    pub modloader: ModLoaderOptions,
}

impl From<&MinecraftPackage> for MinecraftPackageConfigurationFile {
    fn from(value: &MinecraftPackage) -> Self {
        MinecraftPackageConfigurationFile {
            version: value.version.clone(),
            description: "".to_string(),
            modloader: value.modloader.clone(),
        }
    }
}
