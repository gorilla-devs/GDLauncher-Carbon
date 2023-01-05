pub(in crate::modloader) mod forge;
pub(in crate::modloader) mod vanilla;

use std::clone;
use std::sync::Weak;

use anyhow::Result;
use tokio::sync::{watch::Sender, RwLock};

use super::instance::Instance;

enum ModLoader{
    Vanilla,
    Forge,
}

pub type ModloaderVersion = String;

pub struct InstallProgress<T> {
    pub count_progress: (u64, u64),
    pub size_progress: (u64, u64),
    pub stage: T,
}

#[async_trait]
pub trait Modloader {
    type Stages;

    fn new(mod_loader_version: ModloaderVersion, instance: Weak<RwLock<Instance>>) -> Self /// instance
        where
            Self: Sized;
    async fn install(&self, progress_recv: Sender<InstallProgress<Self::Stages>>) -> Result<()>;
    fn remove(&self) -> Result<()>;
    fn verify(&self) -> Result<()>;
    fn get_version(&self) -> ModloaderVersion;
}