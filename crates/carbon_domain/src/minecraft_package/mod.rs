pub(crate) mod configuration;

use crate::minecraft_mod::MinecraftMod;
use crate::minecraft_package::configuration::MinecraftConfiguration;
use crate::modloader::ModLoader;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct MinecraftPackage {
    pub version: String,
    pub mods: BTreeSet<MinecraftMod>,
    pub description: String,
    pub modloader: BTreeSet<ModLoader>,
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
            modloader: default_modloader,
        }
    }
}

impl From<MinecraftConfiguration> for MinecraftPackage {
    fn from(value: MinecraftConfiguration) -> Self {
        MinecraftPackage {
            version: value.version,
            mods: Default::default(),
            description: "".to_string(),
            modloader: value.modloader,
        }
    }
}

impl From<&MinecraftConfiguration> for MinecraftPackage {
    fn from(value: &MinecraftConfiguration) -> Self {
        MinecraftPackage {
            version: value.version.clone(),
            mods: Default::default(),
            description: "".to_string(),
            modloader: value.modloader.clone(),
        }
    }
}
