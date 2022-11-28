use std::sync::Arc;

use anyhow::Result;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;

use crate::{
    component::{Component, ComponentVersion}, instance::Instance,
};

use super::ModLoaderVersion;

#[derive(Debug, Serialize, Deserialize)]
pub struct ForgeModloader {
    mod_loader_version: ModLoaderVersion,
}

impl Component for ForgeModloader {
    fn new(mod_loader_version: ModLoaderVersion, instance: Arc<Instance>) -> Self {
        ForgeModloader {
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
    fn get_version(&self) -> ComponentVersion {
        self.mod_loader_version.clone()
    }
}