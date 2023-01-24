use super::instance::Instance;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Weak;
use tokio::sync::RwLock;

pub(crate) mod fabric;
pub(crate) mod forge;
pub(crate) mod vanilla;

pub trait ModLoaderError: std::error::Error + Send + Sync + 'static {}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum ModLoader {
    Vanilla,
    Forge,
    Fabric,
    LiteLoader,
    Quilt,
}

impl Default for ModLoader {
    fn default() -> Self {
        ModLoader::Vanilla
    }
}

pub type ModloaderVersion = String;

pub struct InstallProgress<T> {
    pub count_progress: (u64, u64),
    pub size_progress: (u64, u64),
    pub stage: T,
}

#[async_trait]
pub trait ModLoaderT
where
    Self: Sized,
{
    type Error: ModLoaderError;

    fn new(mod_loader_version: ModloaderVersion, instance: Weak<RwLock<Instance>>) -> Self
    where
        Self: Sized;
    async fn install(&self) -> Result<(), Self::Error>;
    fn remove(&self) -> Result<(), Self::Error>;
    fn verify(&self) -> Result<(), Self::Error>;
    fn get_version(&self) -> ModloaderVersion;
}
