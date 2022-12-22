use std::sync::Weak;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};

use crate::instance::Instance;

use super::{ModLoaderVersion, Modloader, ModloaderVersion};

#[derive(Debug)]
pub struct VanillaModLoader {
    mc_version: ModLoaderVersion,
    instance_ref: Weak<Mutex<Instance>>,
}

impl Modloader for VanillaModLoader {
    fn new(mc_version: ModLoaderVersion, instance: Weak<Mutex<Instance>>) -> Self {
        VanillaModLoader {
            mc_version,
            instance_ref: instance,
        }
    }
    fn install(&self, progress_rcv: tokio::sync::watch::Sender<()>) -> Result<()> {
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
