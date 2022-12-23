use std::sync::{Arc, Weak};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};

use crate::instance::Instance;

use super::{InstallProgress, ModLoaderVersion, Modloader, ModloaderVersion};

pub enum InstallStages {
    Downloading,
    ExtractingNatives,
}

#[derive(Debug)]
pub struct ForgeModloader {
    mod_loader_version: ModLoaderVersion,
    instance_ref: Weak<Mutex<Instance>>,
}

impl Modloader for ForgeModloader {
    type Stages = InstallStages;

    fn new(mod_loader_version: ModLoaderVersion, instance: Weak<Mutex<Instance>>) -> Self {
        ForgeModloader {
            mod_loader_version,
            instance_ref: instance,
        }
    }
    fn install(
        &self,
        progress_send: tokio::sync::watch::Sender<InstallProgress<InstallStages>>,
    ) -> Result<()> {
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
