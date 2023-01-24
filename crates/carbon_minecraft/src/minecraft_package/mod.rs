pub(crate) mod configuration;

use crate::minecraft_mod::MinecraftMod;
use crate::minecraft_package::configuration::MinecraftPackageConfigurationFile;
use crate::modloader::ModLoader;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Library {
    name: String,
    file_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct MinecraftPackage {
    pub version: String,
    pub mods: BTreeSet<MinecraftMod>,
    pub description: String,
    pub modloader: HashSet<ModLoader>,
}

impl From<MinecraftPackageConfigurationFile> for MinecraftPackage {
    fn from(value: MinecraftPackageConfigurationFile) -> Self {
        MinecraftPackage {
            version: value.version,
            mods: Default::default(),
            description: "".to_string(),
            modloader: value.modloader,
        }
    }
}

impl From<&MinecraftPackageConfigurationFile> for MinecraftPackage {
    fn from(value: &MinecraftPackageConfigurationFile) -> Self {
        MinecraftPackage {
            version: value.version.clone(),
            mods: Default::default(),
            description: "".to_string(),
            modloader: value.modloader.clone(),
        }
    }
}
