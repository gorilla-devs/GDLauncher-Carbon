pub(in crate::modloader) mod forge;
pub(in crate::modloader) mod vanilla;
mod fabric;

use std::sync::Weak;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::instance::Instance;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub enum ModLoader {
    Forge,
    Fabric,
    LiteLoader,
    Quilt,
}

pub type ModloaderVersion = String;

pub struct InstallProgress<T> {
    pub count_progress: (u64, u64),
    pub size_progress: (u64, u64),
    pub stage: T,
}

#[async_trait]
trait Modloader {
    type Stages;
    fn new(mod_loader_version: ModloaderVersion, instance: Weak<RwLock<Instance>>) -> Self where Self: Sized;
    async fn install(&self) -> Result<()>;
    fn remove(&self) -> Result<()>;
    fn verify(&self) -> Result<()>;
    fn get_version(&self) -> ModloaderVersion;
}