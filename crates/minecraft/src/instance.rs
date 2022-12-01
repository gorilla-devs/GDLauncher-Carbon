use serde::{Deserialize, Serialize};

use crate::modloaders::{forge::ForgeModloader, vanilla::VanillaModLoader, Modloader};

#[derive(Debug, Serialize, Deserialize)]
struct JavaMemoryOverride {
    min_mem_alloc: u16,
    max_mem_alloc: u16,
    perm_gen: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct Modloaders {
    forge: Option<ForgeModloader>,
    vanilla: Option<VanillaModLoader>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Instance {
    modloaders: Modloaders,
    custom_lwjgl: Option<String>,
}

impl Instance {}
