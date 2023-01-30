use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct MinecraftMod {
    pub id: u128,
    pub name: String,
    pub version: String,
}
