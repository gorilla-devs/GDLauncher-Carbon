use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
/// An asset of the game
pub struct Asset {
    /// The SHA1 hash of the asset file
    pub hash: String,
    /// The size of the asset file
    pub size: u32,
}

#[derive(Serialize, Deserialize, Debug)]
/// An index containing all assets the game needs
pub struct AssetsIndex {
    /// A hashmap containing the filename (key) and asset (value)
    pub objects: HashMap<String, Asset>,
}