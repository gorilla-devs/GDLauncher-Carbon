use std::sync::Arc;

use anyhow::Result;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;


use crate::instance::Instance;

use super::{ModLoaderVersion, Modloader, ModloaderVersion};

#[derive(Debug, Serialize, Deserialize)]
pub struct VanillaModLoader {
    mod_loader_version: ModLoaderVersion,
}

impl Modloader for VanillaModLoader {
    fn new(mod_loader_version: ModLoaderVersion, instance: Arc<Instance>) -> Self {
        VanillaModLoader {
            mod_loader_version,
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