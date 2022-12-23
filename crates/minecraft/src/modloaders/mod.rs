pub mod forge;
pub mod vanilla;

type ModLoaderVersion = String;
pub enum ModLoaderType {
    Vanilla,
    Forge,
}

use std::sync::Weak;

use anyhow::Result;
use tokio::sync::{mpsc, Mutex};

use super::instance::Instance;

pub type ModloaderVersion = String;

pub struct InstallProgress<T> {
    pub progress: u8,
    pub stage: T,
}

pub trait Modloader {
    type Stages;

    fn new(mod_loader_version: ModloaderVersion, instance: Weak<Mutex<Instance>>) -> Self
    where
        Self: Sized;
    fn install(
        &self,
        progress_recv: tokio::sync::watch::Sender<InstallProgress<Self::Stages>>,
    ) -> Result<()>;
    fn remove(&self) -> Result<()>;
    fn verify(&self) -> Result<()>;
    fn get_version(&self) -> ModloaderVersion;
}
