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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MinecraftPackage {
    pub version: String,
    pub mods: BTreeSet<MinecraftMod>,
    pub description: String,
    pub modloader: HashSet<ModLoader>,
}

impl MinecraftPackage {
    pub fn new(mc_version: impl Into<String>) -> Self {
        let default_modloaders = {
            let mut default_modloaders = HashSet::new();
            default_modloaders.insert(ModLoader::Vanilla);
            default_modloaders
        };

        MinecraftPackage {
            version: mc_version.into(),
            mods: Default::default(),
            description: "".to_string(),
            modloader: default_modloaders,
        }
    }
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
