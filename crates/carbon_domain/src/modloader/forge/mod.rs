use async_trait::async_trait;
use std::sync::Weak;
use thiserror::Error;
use tokio::sync::{watch::Sender, RwLock};

use crate::instance::Instance;

use super::{InstallProgress, ModLoaderError, ModLoaderHandler, ModloaderVersion};

#[derive(Error, Debug)]
pub enum ForgeError {}

impl ModLoaderError for ForgeError {}

#[derive(Debug)]
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
impl ModLoaderHandler for ForgeModloader {
    type Error = ForgeError;
    type Stages = InstallStages;

    fn new(mod_loader_version: ModloaderVersion, instance_ref: Weak<RwLock<Instance>>) -> Self {
        ForgeModloader {
            mod_loader_version,
            instance_ref,
        }
    }
    async fn install(
        &self,
        progress_send: Sender<InstallProgress<InstallStages>>,
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
