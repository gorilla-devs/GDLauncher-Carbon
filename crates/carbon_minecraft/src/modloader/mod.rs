pub(crate) mod fabric;
pub(crate) mod forge;
pub(crate) mod vanilla;

use std::sync::Weak;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::instance::Instance;

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum ModLoaderOptions {
    Vanilla,
    Forge,
    Fabric,
    LiteLoader,
    Quilt,
}

impl Default for ModLoaderOptions {
    fn default() -> Self {
        ModLoaderOptions::Vanilla
    }
}

pub type ModloaderVersion = String;

pub struct InstallProgress<T> {
    pub count_progress: (u64, u64),
    pub size_progress: (u64, u64),
    pub stage: T,
}

#[async_trait]
pub trait ModLoader
where
    Self: Sized,
{
    // type Stages;
    fn new(mod_loader_version: ModloaderVersion, instance: Weak<RwLock<Instance>>) -> Self
    where
        Self: Sized;
    async fn install(&self) -> Result<()>;
    fn remove(&self) -> Result<()>;
    fn verify(&self) -> Result<()>;
    fn get_version(&self) -> ModloaderVersion;
}
