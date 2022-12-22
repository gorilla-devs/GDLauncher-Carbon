use serde::{Deserialize, Serialize};

use crate::modloaders::{forge::ForgeModloader, vanilla::VanillaModLoader, Modloader};

#[derive(Debug, Serialize, Deserialize)]
struct JavaMemoryOverride {
    min_mem_alloc: u16,
    max_mem_alloc: u16,
    perm_gen: u16,
}

#[derive(Debug)]
pub struct Modloaders {
    pub vanilla: Option<VanillaModLoader>,
    pub forge: Option<ForgeModloader>,
}

impl Modloaders {
    pub fn new(vanilla: Option<VanillaModLoader>, forge: Option<ForgeModloader>) -> Self {
        Self { vanilla, forge }
    }
}

#[derive(Debug)]
pub struct Instance {
    name: String,
    modloaders: Modloaders,
}

impl Instance {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            modloaders: Modloaders::new(None, None),
        }
    }
    pub fn with_modloaders(&mut self, modloaders: Modloaders) {
        self.modloaders = modloaders;
    }
}
