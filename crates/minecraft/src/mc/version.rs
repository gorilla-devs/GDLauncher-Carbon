use std::{borrow::Borrow, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

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

impl Version {
    #[tracing::instrument]
    pub fn merge(self, lower: Self) -> Self {
        let mut merged = Self {
            inherits_from: None,
            arguments: None,
            asset_index: None,
            assets: None,
            compliance_level: None,
            downloads: None,
            id: None,
            java_version: None,
            libraries: None,
            logging: None,
            main_class: None,
            minimum_launcher_version: None,
            release_time: None,
            time: None,
            version_info_type: None,
        };

        if let Some(arguments) = lower.arguments {
            let current_arguments = self.arguments.unwrap_or(Arguments {
                game: Some(vec![]),
                jvm: Some(vec![]),
            });

            let jvm = current_arguments
                .jvm
                .unwrap_or_default()
                .into_iter()
                .chain(arguments.jvm.unwrap_or_default().into_iter())
                .collect();

            let game = current_arguments
                .game
                .unwrap_or_default()
                .into_iter()
                .chain(arguments.game.unwrap_or_default())
                .collect();

            merged.arguments = Some(Arguments {
                game: Some(game),
                jvm: Some(jvm),
            })
        }

        merged.inherits_from = lower.inherits_from.or(self.inherits_from);
        merged.asset_index = self.asset_index.or(lower.asset_index);
        merged.assets = self.assets.or(lower.assets);
        merged.compliance_level = self.compliance_level.or(lower.compliance_level);
        merged.downloads = self.downloads.or(lower.downloads);
        merged.id = self.id.or(lower.id);
        merged.java_version = self.java_version.or(lower.java_version);
        merged.libraries = Some(
            self.libraries
                .unwrap_or_default()
                .into_iter()
                .chain(lower.libraries.unwrap_or_default().into_iter())
                .collect(),
        );
        merged.main_class = self.main_class.or(lower.main_class);
        merged.minimum_launcher_version = self
            .minimum_launcher_version
            .or(lower.minimum_launcher_version);
        merged.release_time = self.release_time.or(lower.release_time);
        merged.time = self.time.or(lower.time);
        merged.version_info_type = self.version_info_type.or(lower.version_info_type);

        merged
    }

    pub async fn retrieve_asset_index_meta(&self) -> Result<super::assets::AssetIndex> {
        let url = self.asset_index.as_ref().unwrap().url.clone();
        let resp = reqwest::get(&url).await?.json().await?;

        Ok(resp)
    }

    pub fn filter_allowed_libraries(&self) -> Vec<&Library> {
        let Some(libraries) = self.libraries.as_ref() else {
            return vec![];
        };
        libraries
            .iter()
            .filter(|library| library.check_allowed_rules())
            .collect()
    }

    #[tracing::instrument]
    pub async fn download_allowed_libraries(&self) -> Result<()> {
        let libraries = self.filter_allowed_libraries();

        let downloads: Vec<crate::net::Download> = libraries
            .iter()
            .filter_map(|library| (*library).try_into().ok())
            .collect::<Vec<_>>();

        trace!("Downloading libraries {downloads:#?}");

        let (progress, mut progress_handle) = tokio::sync::watch::channel(0);

        let length = &downloads.len();
        let handle = tokio::spawn(async move {
            crate::net::download_multiple(downloads, progress).await?;
            Ok::<(), anyhow::Error>(())
        });

        while progress_handle.changed().await.is_ok() {
            trace!("Progress: {} - {}", *progress_handle.borrow(), length);
        }

        handle.await??;

        Ok(())
    }
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
    pub fn check_allowed_rules(&self) -> bool {
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

impl TryFrom<&Library> for crate::net::Download {
    type Error = anyhow::Error;

    fn try_from(value: &Library) -> Result<Self, Self::Error> {
        let artifact =
            value.downloads.artifact.clone().ok_or_else(|| {
                anyhow::anyhow!("Could not find artifact for library {}", value.name)
            })?;
        let path = std::env::current_dir()?.join("libraries").join(
            artifact
                .path
                .ok_or_else(|| anyhow::anyhow!("No path in lib"))?,
        );
        let checksum = Some(crate::net::Checksum::Sha1(artifact.sha1));

        Ok(crate::net::Download {
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
