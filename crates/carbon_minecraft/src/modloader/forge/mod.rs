use async_trait::async_trait;
use std::sync::Weak;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::instance::Instance;

use super::{ModLoader, ModLoaderError, ModloaderVersion};

#[derive(Error, Debug)]
pub enum ForgeError {}

impl ModLoaderError for ForgeError {}

pub enum InstallStages {
    Downloading,
    ExtractingNatives,
}

#[derive(Debug)]
pub struct ForgeModloader {
    mod_loader_version: ModloaderVersion,
    instance_ref: Weak<RwLock<Instance>>,
}

#[async_trait]
impl ModLoader for ForgeModloader {
    type Error = ForgeError;

    fn new(mod_loader_version: ModloaderVersion, instance_ref: Weak<RwLock<Instance>>) -> Self {
        ForgeModloader {
            mod_loader_version,
            instance_ref,
        }
    }
    async fn install(
        &self, /*progress_send: Sender<InstallProgress<InstallStages>>*/
    ) -> Result<(), ForgeError> {
        Ok(())
    }
    fn remove(&self) -> Result<(), ForgeError> {
        Ok(())
    }
    fn verify(&self) -> Result<(), ForgeError> {
        Ok(())
    }
    fn get_version(&self) -> ModloaderVersion {
        self.mod_loader_version.clone()
    }
}
