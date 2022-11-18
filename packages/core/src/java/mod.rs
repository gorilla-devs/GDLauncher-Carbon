use std::path::PathBuf;
use once_cell::sync::OnceCell;
use serde::{Serialize, Deserialize};

use self::mc_java::JavaManifest;

mod checker;
mod utils;
mod napi;
mod mc_java;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct JavaComponent {
    pub path: PathBuf,
    pub arch: String,
    /// Indicates whether the component has manually been added by the user
    pub is_custom: bool,
    pub version: JavaVersion
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct JavaVersion {
    pub major: u8,
    pub minor: Option<u8>,
    pub patch: Option<String>,
    pub update_number: Option<String>,
    pub prerelease: Option<String>,
    pub build_metadata: Option<String>,
}

static JAVA_MANIFEST: OnceCell<JavaManifest> = OnceCell::new();