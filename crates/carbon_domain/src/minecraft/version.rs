use carbon_net::Downloadable;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::trace;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub inherits_from: Option<String>,
    pub arguments: Option<Arguments>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndex>,
    pub assets: Option<String>,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: Option<i64>,
    pub downloads: Option<VersionInfoDownloads>,
    pub id: Option<String>,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersion>,
    pub libraries: Option<Vec<Library>>,
    pub logging: Option<Logging>,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: Option<i64>,
    #[serde(rename = "releaseTime")]
    pub release_time: Option<String>,
    pub time: Option<String>,
    #[serde(rename = "type")]
    pub version_info_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Arguments {
    pub game: Option<Vec<GameElement>>,
    pub jvm: Option<Vec<JvmElement>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameClass {
    pub rules: Vec<GameRule>,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameRule {
    pub action: Action,
    pub features: Features,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Features {
    pub is_demo_user: Option<bool>,
    pub has_custom_resolution: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JvmClass {
    pub rules: Vec<JvmRule>,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JvmRule {
    pub action: Action,
    pub os: JvmOs,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JvmOs {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: i64,
    #[serde(rename = "totalSize")]
    pub total_size: Option<i64>,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfoDownloads {
    pub client: MappingsClass,
    pub client_mappings: Option<MappingsClass>,
    pub server: Option<MappingsClass>,
    pub server_mappings: Option<MappingsClass>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MappingsClass {
    pub sha1: String,
    pub size: u64,
    pub url: String,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    pub downloads: LibraryDownloads,
    /// Url only appears in some forge libraries apparently
    pub url: Option<String>,
    pub name: String,
    pub rules: Option<Vec<LibraryRule>>,
    pub natives: Option<Natives>,
    pub extract: Option<Extract>,
}

impl Library {
    #[tracing::instrument]
    pub fn is_allowed(&self) -> bool {
        let Some(rules) = &self.rules else {
            return true;
        };

        for rule in rules {
            match rule.action {
                Action::Allow => {
                    if let Some(os) = &rule.os {
                        if match os.name {
                            Name::Linux => cfg!(target_os = "linux"),
                            Name::Osx => cfg!(target_os = "macos"),
                            Name::Windows => cfg!(target_os = "windows"),
                        } {
                            continue;
                        }
                        return false;
                    }
                    continue;
                }
                Action::Disallow => {
                    if let Some(os) = &rule.os {
                        if match os.name {
                            Name::Linux => cfg!(target_os = "linux"),
                            Name::Osx => cfg!(target_os = "macos"),
                            Name::Windows => cfg!(target_os = "windows"),
                        } {
                            return false;
                        }
                        continue;
                    }
                    return false;
                }
            }
        }

        true
    }
}

impl TryFrom<&Library> for carbon_net::Downloadable {
    type Error = impl Error;

    fn try_from(value: &Library) -> Result<Self, Self::Error> {
        let artifact = value
            .downloads
            .artifact
            .clone()
            .ok_or_else(|| VersionError::NoArtifactFoundInLib(value.name.clone()))?;
        let path = PathBuf::new().join(
            artifact
                .path
                .ok_or_else(|| VersionError::NoPathFoundInLib(value.name.clone()))?,
        );
        let checksum = Some(carbon_net::Checksum::Sha1(artifact.sha1));

        Ok(carbon_net::Downloadable {
            url: artifact.url,
            path,
            checksum,
            size: Some(artifact.size),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryDownloads {
    pub artifact: Option<MappingsClass>,
    pub classifiers: Option<Classifiers>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Classifiers {
    pub javadoc: Option<MappingsClass>,
    #[serde(rename = "natives-linux")]
    pub natives_linux: Option<MappingsClass>,
    #[serde(rename = "natives-macos")]
    pub natives_macos: Option<MappingsClass>,
    #[serde(rename = "natives-windows")]
    pub natives_windows: Option<MappingsClass>,
    pub sources: Option<MappingsClass>,
    #[serde(rename = "natives-osx")]
    pub natives_osx: Option<MappingsClass>,
}

impl TryFrom<&Classifiers> for carbon_net::Downloadable {
    type Error = VersionError;

    fn try_from(value: &Classifiers) -> Result<Self, Self::Error> {
        let classifier = value.clone();
        let download = match std::env::consts::OS {
            "windows" => {
                if let Some(windows) = classifier.natives_windows {
                    let path =
                        PathBuf::from(windows.path.ok_or(VersionError::NoPathFoundInClassifier)?);
                    let checksum = Some(carbon_net::Checksum::Sha1(windows.sha1));

                    carbon_net::Downloadable {
                        url: windows.url,
                        path,
                        checksum,
                        size: Some(windows.size),
                    }
                } else {
                    return Err(VersionError::NoWindowNativesFoundInClassifier);
                }
            }
            "macos" => {
                if let Some(macos) = classifier.natives_macos {
                    let path = macos.path.ok_or(VersionError::NoPathFoundInClassifier)?;

                    let checksum = Some(carbon_net::Checksum::Sha1(macos.sha1));

                    carbon_net::Downloadable {
                        url: macos.url,
                        path: PathBuf::from(path),
                        checksum,
                        size: Some(macos.size),
                    }
                } else if let Some(osx) = classifier.natives_osx {
                    let path =
                        PathBuf::from(osx.path.ok_or(VersionError::NoPathFoundInClassifier)?);
                    let checksum = Some(carbon_net::Checksum::Sha1(osx.sha1));

                    carbon_net::Downloadable {
                        url: osx.url,
                        path,
                        checksum,
                        size: Some(osx.size),
                    }
                } else {
                    return Err(VersionError::NoMacOSNativesFoundInClassifier);
                }
            }
            "linux" => {
                if let Some(linux) = classifier.natives_linux {
                    let path =
                        PathBuf::from(linux.path.ok_or(VersionError::NoPathFoundInClassifier)?);
                    let checksum = Some(carbon_net::Checksum::Sha1(linux.sha1));

                    carbon_net::Downloadable {
                        url: linux.url,
                        path,
                        checksum,
                        size: Some(linux.size),
                    }
                } else {
                    return Err(VersionError::NoLinuxNativesFoundInClassifier);
                }
            }
            _ => panic!("Unsupported OS"),
        };

        Ok(download)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Extract {
    pub exclude: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Natives {
    pub osx: Option<String>,
    pub linux: Option<String>,
    pub windows: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryRule {
    pub action: Action,
    pub os: Option<LibOs>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibOs {
    pub name: Name,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Logging {
    pub client: LoggingClient,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggingClient {
    pub argument: String,
    pub file: AssetIndex,
    #[serde(rename = "type")]
    pub client_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GameElement {
    GameClass(GameClass),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    String(String),
    StringArray(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JvmElement {
    JvmClass(JvmClass),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Action {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "disallow")]
    Disallow,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Name {
    #[serde(rename = "osx")]
    Osx,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "windows")]
    Windows,
}
