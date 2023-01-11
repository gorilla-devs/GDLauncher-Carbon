use crate::minecraft_package::Library;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MinecraftMod{
    jars: Vec<Library>,
}