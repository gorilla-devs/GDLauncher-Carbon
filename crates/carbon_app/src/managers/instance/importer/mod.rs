use std::{sync::Arc, path::PathBuf};

use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use crate::{domain::vtask::VisualTaskId, managers::AppInner};

pub mod archive_importer;
pub mod legacy_gdlauncher;

#[derive(Debug, Serialize, Deserialize, EnumIter)]
pub enum Entity {
    LegacyGDLauncher,
    MRPack,
    Modrinth,
    CurseForgeZip,
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

    async fn scan(&mut self, app: Arc<AppInner>) -> anyhow::Result<()>;
    async fn get_available(&self) -> anyhow::Result<Vec<ImportableInstance>>;
    async fn import(&self, app: Arc<AppInner>, index: u32) -> anyhow::Result<VisualTaskId>;
}

#[derive(Debug, Default)]
pub struct Importer {
    pub legacy_gdlauncher: legacy_gdlauncher::LegacyGDLauncherImporter,
}


#[async_trait::async_trait]
pub trait InstanceArchiveImporter {
    async fn import(&self, app: Arc<AppInner>, path: PathBuf) -> anyhow::Result<VisualTaskId>;
}

