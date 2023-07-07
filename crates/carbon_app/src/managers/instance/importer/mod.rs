use std::{any::Any, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::managers::AppInner;

pub mod legacy_gdlauncher;

#[derive(Debug, Serialize, Deserialize)]
pub enum Entity {
    LegacyGDLauncher,
}

pub struct ImportableInstance {
    pub name: String,
}

#[async_trait::async_trait]
pub trait InstanceImporter {
    type Config: Sized;

    async fn scan(&mut self, app: Arc<AppInner>) -> anyhow::Result<()>;
    async fn get_available(&self) -> anyhow::Result<Vec<ImportableInstance>>;
    async fn import(&self, app: Arc<AppInner>, index: u32) -> anyhow::Result<()>;
}

#[derive(Debug, Default)]
pub struct Importer {
    pub legacy_gdlauncher: legacy_gdlauncher::LegacyGDLauncherImporter,
}
