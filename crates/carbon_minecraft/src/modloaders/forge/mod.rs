use std::sync::Weak;

use anyhow::Result;
use tokio::sync::{watch::Sender, RwLock};

use crate::instance::Instance;

use super::{InstallProgress, ModLoaderVersion, Modloader, ModloaderVersion};

pub enum InstallStages {
    Downloading,
    ExtractingNatives,
}

#[derive(Debug)]
pub struct ForgeModloader {
    mod_loader_version: ModLoaderVersion,
    instance_ref: Weak<RwLock<Instance>>,
}

impl Modloader for ForgeModloader {
    type Stages = InstallStages;

    fn new(mod_loader_version: ModLoaderVersion, instance_ref: Weak<RwLock<Instance>>) -> Self {
        ForgeModloader {
            mod_loader_version,
            instance_ref,
        }
    }
    fn install(&self, progress_send: Sender<InstallProgress<InstallStages>>) -> Result<()> {
        Ok(())
    }
    fn remove(&self) -> Result<()> {
        Ok(())
    }
    fn verify(&self) -> Result<()> {
        Ok(())
    }
    fn get_version(&self) -> ModloaderVersion {
        self.mod_loader_version.clone()
    }
}
