use std::collections::HashSet;

use crate::{minecraft_package::MinecraftPackage, modloader::ModLoader};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MinecraftPackageConfigurationFile {
    pub version: String,
    pub description: String,
    pub modloader: HashSet<ModLoader>,
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
