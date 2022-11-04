pub mod error;
pub mod components;

use std::sync::Arc;

use error::ComponentError;
use serde::{Deserialize, Serialize};

use super::instance::Instance;

use self::components::modloaders::{vanilla::VanillaModLoader, forge::ForgeModLoader};

pub type ComponentVersion = String;

#[derive(Debug, Deserialize, Serialize)]
pub enum Component {
    VanillaModLoader(VanillaModLoader),
    ForgeModLoader(ForgeModLoader),
}

impl Component {
    fn install(&self) -> Result<(), ComponentError> {
        match self {
            Component::VanillaModLoader(v) => v.install(),
            Component::ForgeModLoader(f) => f.install(),
        }
    }
    fn remove(&self) -> Result<(), ComponentError> {
        match self {
            Component::VanillaModLoader(v) => v.remove(),
            Component::ForgeModLoader(f) => f.remove(),
        }
    }
    fn verify(&self) -> Result<(), ComponentError> {
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
    fn install(&self) -> Result<(), ComponentError>;
    fn remove(&self) -> Result<(), ComponentError>;
    fn verify(&self) -> Result<(), ComponentError>;
    fn get_version(&self) -> ComponentVersion;
}