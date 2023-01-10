use std::collections::HashSet;
use std::path::Path;
use crate::minecraft_mod::MinecraftMod;
use crate::modloader::ModLoader;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MinecraftPackage{
    pub version: String,
    pub mods : HashSet<MinecraftMod>,
    mod_loader: Option<ModLoader>,
}

impl MinecraftPackage{

    pub fn new(version: String, mods: HashSet<MinecraftMod>, mod_loader: Option<ModLoader>) -> Self {
        Self { version, mods, mod_loader }
    }

    fn  get_entrypoint_path() -> &Path{
        todo!()
    }

    fn  get_libraries_root_folder_path()-> &Path{
        todo!()
    }

    fn  get_mainclass_classpath()-> String{
        todo!()
    }

    fn  get_libraries_list()->HashSet<Library>{
        todo!()
    }

    fn  get_cli_arguments()->Vec<String>{ //FIXME maybe extract a trait ? see Instance Struct ...
        todo!()
    }

}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct Library{
    name : String
}