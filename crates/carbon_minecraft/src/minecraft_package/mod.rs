pub(crate) mod scan;
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

#[derive( Debug, Serialize, Deserialize, Hash )]
pub struct MinecraftPackage {
    pub version: String,
    pub mods: BTreeSet<MinecraftMod>,
    // pub mod_loader: Option<ModLoader>,
    pub core_jars: Vec<Library>,
    pub path: PathBuf
}



impl From<MinecraftPackageConfigurationFile> for MinecraftPackage{
    fn from(value: MinecraftPackageConfigurationFile) -> Self {
        let core_jars = Vec::new();
        MinecraftPackage {
            version: value.version,
            mods: value.mods,
            core_jars,
            path: PathBuf::new() // fixme: provvisory implemetation
        }
    }
}

impl From<&MinecraftPackageConfigurationFile> for MinecraftPackage{
    fn from(value: &MinecraftPackageConfigurationFile) -> Self {
        let core_jars = Vec::new();
        MinecraftPackage {
            version: value.version.clone(),
            mods: value.mods.clone(),
            core_jars,
            path: PathBuf::new() // fixme: provvisory implemetation
        }
    }
}

