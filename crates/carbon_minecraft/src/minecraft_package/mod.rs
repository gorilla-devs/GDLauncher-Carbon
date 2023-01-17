pub(crate) mod configuration;

use std::collections::BTreeSet;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::minecraft_mod::MinecraftMod;
use crate::minecraft_package::configuration::MinecraftPackageConfigurationFile;

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Library {
    name: String,
    file_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Hash, Default)]
pub struct MinecraftPackage {
    pub version: String,
    pub mods: BTreeSet<MinecraftMod>,
    pub description: String
}


impl From<MinecraftPackageConfigurationFile> for MinecraftPackage{
    fn from(value: MinecraftPackageConfigurationFile) -> Self {
        MinecraftPackage {
            version: value.version,
            mods: value.mods,
            description: "".to_string()
        }
    }
}

impl From<&MinecraftPackageConfigurationFile> for MinecraftPackage{
    fn from(value: &MinecraftPackageConfigurationFile) -> Self {
        MinecraftPackage {
            version: value.version.clone(),
            mods: value.mods.clone(),
            description: "".to_string()
        }
    }
}