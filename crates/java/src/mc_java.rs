use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct JavaManifest {
    pub gamecore: Gamecore,
    pub linux: OsRuntime,
    pub linux_i386: OsRuntime,
    pub mac_os: OsRuntime,
    pub mac_os_arm64: OsRuntime,
    pub windows_x64: OsRuntime,
    pub windows_x86: OsRuntime,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Gamecore {
    pub java_runtime_alpha: Vec<Value>,
    pub java_runtime_beta: Vec<Value>,
    pub java_runtime_gamma: Vec<Value>,
    pub jre_legacy: Vec<Value>,
    pub minecraft_java_exe: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OsRuntime {
    pub java_runtime_alpha: Vec<JavaRuntime>,
    pub java_runtime_beta: Vec<JavaRuntime>,
    pub java_runtime_gamma: Vec<JavaRuntime>,
    pub jre_legacy: Vec<JavaRuntime>,
    pub minecraft_java_exe: Vec<JavaRuntime>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Availability {
    pub group: i64,
    pub progress: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    pub sha1: String,
    pub size: i64,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Version {
    pub name: String,
    pub released: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct JavaRuntime {
    pub availability: Availability,
    pub manifest: Manifest,
    pub version: Version,
}

pub const JAVA_MANIFEST_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

pub async fn fetch_java_manifest() -> Result<JavaManifest> {
    let resp: JavaManifest = reqwest::get(JAVA_MANIFEST_URL)
        .await?
        .json()
        .await
        .context("Couldn't fetch/parse java manifest")?;
    Ok(resp)
}
