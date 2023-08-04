use std::{sync::Arc, path::PathBuf};

use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;


use crate::{domain::vtask::VisualTaskId, managers::AppInner};

pub mod archive_importer;
pub mod legacy_gdlauncher;

#[derive(Debug, Serialize, Deserialize, EnumIter)]
pub enum Entity {
    LegacyGDLauncher,
    MRPack(String),
    Modrinth,
    CurseForgeZip(String),
    CurseForge,
    ATLauncher,
    Technic,
    FTB,
    MultiMC,
    PrismLauncher,
}

impl Entity {
    pub fn get_available() -> Vec<Self> {
        use strum::IntoEnumIterator;
        Self::iter().collect()
    }
}

pub struct ImportableInstance {
    pub name: String,
}

#[async_trait::async_trait]
pub trait InstanceImporter {
    type Config: Sized;

    async fn scan(&mut self, app: Arc<AppInner>, path: Option<PathBuf>) -> anyhow::Result<()>;
    async fn get_available(&self) -> anyhow::Result<Vec<ImportableInstance>>;
    async fn import(&self, app: Arc<AppInner>, index: u32, name: &str) -> anyhow::Result<VisualTaskId>;
}

#[derive(Debug, Default)]
pub struct Importer {
    pub legacy_gdlauncher: legacy_gdlauncher::LegacyGDLauncherImporter,
    pub curseforge_zip: archive_importer::CurseForgeZipImporter,
}

#[derive(Debug)]
pub enum ImportIcon {
    Local(String),
    Remote(String),
}



