use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::modloaders::{vanilla::VanillaModLoader, forge::ForgeModLoader};

use super::instance::Instance;

pub type ComponentVersion = String;

#[derive(Debug, Deserialize, Serialize)]
pub enum Component {
    VanillaModLoader(VanillaModLoader),
    ForgeModLoader(ForgeModLoader),
}

impl Component {
    fn install(&self) -> Result<()> {
        match self {
            Component::VanillaModLoader(v) => v.install(),
            Component::ForgeModLoader(f) => f.install(),
        }
    }
    fn remove(&self) -> Result<()> {
        match self {
            Component::VanillaModLoader(v) => v.remove(),
            Component::ForgeModLoader(f) => f.remove(),
        }
    }
    fn verify(&self) -> Result<()> {
        match self {
            Component::VanillaModLoader(v) => v.verify(),
            Component::ForgeModLoader(f) => f.verify(),
        }
    }
    fn get_version(&self) -> ComponentVersion {
        match self {
            Component::VanillaModLoader(v) => v.get_version(),
            Component::ForgeModLoader(f) => f.get_version(),
        }
    }
}

pub trait ComponentInterface {
    fn new(mod_loader_version: ComponentVersion, instance: Arc<Instance>) -> Self;
    fn install(&self) -> Result<()>;
    fn remove(&self) -> Result<()>;
    fn verify(&self) -> Result<()>;
    fn get_version(&self) -> ComponentVersion;
}