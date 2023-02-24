use std::collections::BTreeSet;

use crate::{minecraft_package::MinecraftPackage, modloader::ModLoader};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct MinecraftPackageConfigurationFile {
    pub version: String,
    pub description: String,
    pub modloader: BTreeSet<ModLoader>,
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
