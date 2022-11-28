use std::sync::Arc;

use anyhow::Result;
use serde::{Serialize, Deserialize};

use crate::{
    component::{self, Component, ComponentVersion, ComponentInterface}, instance::Instance,
};

use super::ModLoaderVersion;

#[derive(Debug, Serialize, Deserialize)]
pub struct VanillaModLoader {
    mod_loader_version: ModLoaderVersion,
}

impl ComponentInterface for VanillaModLoader {
    fn new(mod_loader_version: ModLoaderVersion, instance: Arc<Instance>) -> Self {
        VanillaModLoader {
            mod_loader_version,
        }
    }
    fn install(&self) -> Result<()> {
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

impl Into <Component> for VanillaModLoader {
    fn into(self) -> Component {
        Component::VanillaModLoader(self)
    }
}