use std::sync::{Arc, Weak};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};

use crate::instance::Instance;

use super::{ModLoaderVersion, Modloader, ModloaderVersion};

#[derive(Debug)]
pub struct ForgeModloader {
    mod_loader_version: ModLoaderVersion,
    instance_ref: Weak<Mutex<Instance>>,
}

impl Modloader for ForgeModloader {
    fn new(mod_loader_version: ModLoaderVersion, instance: Weak<Mutex<Instance>>) -> Self {
        ForgeModloader {
            mod_loader_version,
            instance_ref: instance,
        }
    }
    fn install(&self, progress_rcv: mpsc::Sender<()>) -> Result<()> {
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
