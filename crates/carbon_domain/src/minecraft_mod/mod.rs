use rspc::Type;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Ord, PartialOrd, Clone, Type)]
pub struct MinecraftMod {
    pub id: u128,
    pub name: String,
    pub version: String,
}
