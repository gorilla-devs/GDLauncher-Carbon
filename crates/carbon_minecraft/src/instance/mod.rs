mod instances_scan;
mod error;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use crate::minecraft_mod::MinecraftMod;
use crate::minecraft_package::MinecraftPackage;

use crate::modloaders::{forge::ForgeModloader, vanilla::VanillaModLoader, Modloader};

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct Instance{
    pub name: String,
    pub minecraft_package : MinecraftPackage,
    pub mods : HashSet<MinecraftMod>,
    mod_loader: ModLoader,
}

impl Instance {

    fn get_cli_arguments(&self) -> Vec<String>{ // FIXME: maybe extract a trait ?
        todo!()
    }

    fn get_launch_command_line_pattern(&self) -> String{ // FIXME: maybe make a type for command line ?
        todo!()
    }

}

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct JavaMemoryOverride {
    min_mem_alloc: u16,
    max_mem_alloc: u16,
    perm_gen: u16,
}
