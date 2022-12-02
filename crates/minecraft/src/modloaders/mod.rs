pub mod forge;
pub mod vanilla;

type ModLoaderVersion = String;
pub enum ModLoaderType {
    Vanilla,
    Forge,
}

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;

use super::instance::Instance;

pub type ModloaderVersion = String;

pub trait Modloader {
    fn new(mod_loader_version: ModloaderVersion, instance: Arc<Instance>) -> Self
    where
        Self: Sized;
    fn install(&self, progress_recv: mpsc::Sender<()>) -> Result<()>;
    fn remove(&self) -> Result<()>;
    fn verify(&self) -> Result<()>;
    fn get_version(&self) -> ModloaderVersion;
}
