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
pub struct VanillaModLoader {
    mc_version: ModLoaderVersion,
    instance_ref: Weak<RwLock<Instance>>,
}

impl Modloader for VanillaModLoader {
    type Stages = InstallStages;

    fn new(mc_version: ModLoaderVersion, instance_ref: Weak<RwLock<Instance>>) -> Self {
        VanillaModLoader {
            mc_version,
            instance_ref,
        }
    }
    fn install(&self, progress_send: Sender<InstallProgress<InstallStages>>) -> Result<()> {
        progress_send.send(InstallProgress {
            progress: 0,
            stage: InstallStages::Downloading,
        });

        Ok(())
    }
    fn remove(&self) -> Result<()> {
        Ok(())
    }
    fn verify(&self) -> Result<()> {
        Ok(())
    }
    fn get_version(&self) -> ModloaderVersion {
        self.mc_version.clone()
    }
}
