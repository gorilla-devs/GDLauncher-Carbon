use std::path::PathBuf;

use error::JavaError;
use serde::{Deserialize, Serialize};

mod auto_setup;
mod constants;
mod discovery;
mod error;
mod utils;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct JavaComponent {
    pub path: String,
    pub arch: JavaArch,
    /// Indicates whether the component has manually been added by the user
    pub is_custom: bool,
    pub version: JavaVersion,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum JavaArch {
    Amd64,
    X86,
    Aarch64,
}

impl<'a> From<&JavaArch> for &'a str {
    fn from(arch: &JavaArch) -> &'a str {
        match arch {
            JavaArch::Amd64 => "amd64",
            JavaArch::X86 => "x86",
            JavaArch::Aarch64 => "aarch64",
        }
    }
}

impl<'a> From<&'a str> for JavaArch {
    fn from(s: &'a str) -> Self {
        match s {
            "amd64" => JavaArch::Amd64,
            "x86" => JavaArch::X86,
            "aarch64" => JavaArch::Aarch64,
            _ => panic!("Unknown JavaArch: {s}"),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct JavaVersion {
    pub major: u8,
    pub minor: Option<u8>,
    pub patch: Option<String>,
    pub update_number: Option<String>,
    pub prerelease: Option<String>,
    pub build_metadata: Option<String>,
}

impl JavaVersion {
    fn from_major(major: u8) -> Self {
        Self {
            major,
            minor: None,
            patch: None,
            update_number: None,
            prerelease: None,
            build_metadata: None,
        }
    }
}

pub async fn detect_available_javas() -> Result<Vec<JavaComponent>, JavaError> {
    let mut all_javas = discovery::find_java_paths().await;
    all_javas.push(PathBuf::from("java"));
    let mut available_javas = vec![];

    for java in all_javas {
        match discovery::gather_java_bin_info(&java).await {
            Ok(java_bin_info) => available_javas.push(java_bin_info),
            Err(e) => {
                eprintln!("Failed to gather Java info for {}: {}", java.display(), e);
            }
        };
    }

    Ok(available_javas)
}
