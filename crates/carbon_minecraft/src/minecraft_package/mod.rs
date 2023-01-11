pub(crate) mod package_scan;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use crate::minecraft_mod::MinecraftMod;
use crate::modloader::ModLoader;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct Library {
    name: String,
    file_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MinecraftPackage {
    pub version: String,
    pub mods: HashSet<MinecraftMod>,
    mod_loader: Option<ModLoader>,
    jars: Vec<Library>,
}

impl MinecraftPackage {

    fn get_entrypoint_path() -> &Path {
        todo!()
    }

    fn get_libraries_root_folder_path() -> &Path {
        todo!()
    }

    fn get_mainclass_classpath() -> String {
        todo!()
    }

    fn get_libraries_list() -> HashSet<Library> {
        todo!()
    }

    fn get_cli_arguments() -> Vec<String> { //FIXME maybe extract a trait ? see Instance Struct ...
        todo!()
    }

    pub fn mod_loader(&self) -> &Option<ModLoader> {
        &self.mod_loader
    }

    pub fn new(
        version: String,
        mods: HashSet<MinecraftMod>,
        mod_loader: Option<ModLoader>,
        jars: Vec<Library>,
    ) -> Self {
        Self { version, mods, mod_loader, jars }
    }

}