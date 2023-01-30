pub(crate) mod configuration;

use crate::minecraft_mod::MinecraftMod;
use crate::minecraft_package::configuration::MinecraftPackageConfigurationFile;
use crate::modloader::ModLoader;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub enum MinecraftPackageInstallationStage {
    DownloadingAssets,
    DownloadingLibraries,
    ExtractingNatives,
    InstallingModLoader,
    InstallingMod,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub enum MinecraftPackageStatus {
    NotPersisted,
    Queued,
    Installing(MinecraftPackageInstallationStage),
    Ready(PathBuf),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct MinecraftPackage {
    pub version: String,
    pub mods: BTreeSet<MinecraftMod>,
    pub description: String,
    pub modloaders: BTreeSet<ModLoader>,
    pub status: MinecraftPackageStatus
}

impl MinecraftPackage {
    pub fn new(mc_version: impl Into<String>) -> Self {
        let default_modloader = {
            let mut default_modloader = BTreeSet::new();
            default_modloader.insert(ModLoader::Vanilla);
            default_modloader
        };

        MinecraftPackage {
            version: mc_version.into(),
            mods: Default::default(),
            description: "".to_string(),
            modloaders: default_modloader,
            status: MinecraftPackageStatus::NotPersisted,
        }
    }
}

impl From<MinecraftPackageConfigurationFile> for MinecraftPackage {
    fn from(value: MinecraftPackageConfigurationFile) -> Self {
        MinecraftPackage {
            version: value.version,
            mods: Default::default(),
            description: "".to_string(),
            modloaders: value.modloader,
            status: MinecraftPackageStatus::NotPersisted,
        }
    }
}

impl From<&MinecraftPackageConfigurationFile> for MinecraftPackage {
    fn from(value: &MinecraftPackageConfigurationFile) -> Self {
        MinecraftPackage {
            version: value.version.clone(),
            mods: Default::default(),
            description: "".to_string(),
            modloaders: value.modloader.clone(),
            status: MinecraftPackageStatus::NotPersisted,
        }
    }
}
