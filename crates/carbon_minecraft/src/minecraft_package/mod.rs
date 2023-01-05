use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MinecraftPackage{
    pub version: String
}

impl MinecraftPackage{

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

}