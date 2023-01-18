use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use crate::minecraft_mod::MinecraftMod;
use crate::minecraft_package::MinecraftPackage;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MinecraftPackageConfigurationFile {
    pub version: String,
    pub description: String,
    pub mods: BTreeSet<MinecraftMod>,
}

impl From<&MinecraftPackage> for MinecraftPackageConfigurationFile {
    fn from(value: &MinecraftPackage) -> Self {
        MinecraftPackageConfigurationFile{
            version: value.version.clone(),
            description: "".to_string(),
            mods: Default::default(),
        }
    }
}