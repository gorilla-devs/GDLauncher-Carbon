use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::modloaders::{forge::ForgeModloader, vanilla::VanillaModLoader, Modloader};

#[derive(Debug, Serialize, Deserialize)]
struct JavaMemoryOverride {
    min_mem_alloc: u16,
    max_mem_alloc: u16,
    perm_gen: u16,
}

#[derive(Debug)]
pub enum Modloaders {
    Vanilla(VanillaModLoader),
    Forge(ForgeModloader),
}

#[derive(Debug)]
pub struct Instance {
    pub name: String,
    pub modloaders: HashSet<Modloaders>,
}

impl Instance {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            modloaders: HashSet::new(),
        }
    }
}
