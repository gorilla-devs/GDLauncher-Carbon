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
}

impl Instance {

    fn get_history(&self) -> InstanceHistory{ // FIXME: maybe extract a trait ?
        todo!()
    }

    fn get_cli_arguments(&self) -> Vec<String>{ // FIXME: maybe extract a trait ?
        todo!()
    }

    fn get_launch_command_line_pattern(&self) -> String{ // FIXME: maybe make a type for command line ?
        todo!()
    }

}


struct InstanceHistory{}


//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct JavaMemoryOverride {
    min_mem_alloc: u16,
    max_mem_alloc: u16,
    perm_gen: u16,
}
