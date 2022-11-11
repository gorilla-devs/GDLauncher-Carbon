use std::path::PathBuf;

use anyhow::Result;
use once_cell::sync::OnceCell;

use self::mc_java::JavaManifest;

mod checker;
mod utils;
mod napi;
mod mc_java;

pub struct JavaComponent {
    pub path: PathBuf,
    pub full_version: String,
    pub arch: String,
    /// Indicates whether the component has manually been added by the user
    pub is_custom: bool,
}

static JAVA_MANIFEST: OnceCell<JavaManifest> = OnceCell::new();