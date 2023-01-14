use serde::{Deserialize, Serialize};
use crate::minecraft_package::Library;

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct MinecraftMod{
    name: String,
    version: String,
    jars: Vec<Library>,
}