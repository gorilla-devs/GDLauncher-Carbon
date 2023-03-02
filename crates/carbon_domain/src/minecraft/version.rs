use std::path::PathBuf;

use carbon_net::{IntoDownloadable, IntoVecDownloadable};
use serde::{Deserialize, Serialize};

// Need custom type to impl external traits for Vec<Library>
#[derive(Debug, Serialize, Deserialize)]
pub struct Libraries {
    libraries: Vec<Library>,
}
impl Libraries {
    pub fn get_libraries(&self) -> &Vec<Library> {
        &self.libraries
    }
}

impl IntoVecDownloadable for Libraries {
    fn into_vec_downloadable(self, base_path: &std::path::Path) -> Vec<carbon_net::Downloadable> {
        let mut files = vec![];

        for library in self.libraries {
            if !library.is_allowed() {
                continue;
            }

            let Some(artifact) = library.downloads.artifact else {
                continue;
            };

            let Some(path) = artifact.path else {
                continue;
            };
            let checksum = Some(carbon_net::Checksum::Sha1(artifact.sha1));

            files.push(carbon_net::Downloadable {
                url: artifact.url,
                path: PathBuf::from(base_path).join(path),
                checksum,
                size: Some(artifact.size),
            })
        }

        files
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub inherits_from: Option<String>,
    pub arguments: Option<Arguments>,
    pub asset_index: VersionAssetIndex,
    pub assets: Option<String>,
    pub compliance_level: Option<i64>,
    pub downloads: Option<VersionInfoDownloads>,
    pub id: String,
    pub java_version: Option<JavaVersion>,
    #[serde(flatten)]
    pub libraries: Option<Libraries>,
    pub logging: Option<Logging>,
    pub main_class: String,
    pub minimum_launcher_version: Option<i64>,
    pub release_time: Option<String>,
    pub time: Option<String>,
    pub minecraft_arguments: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
}

impl Version {
    pub fn is_older_than(&self, other: &Version) -> bool {
        if let Some(release_time) = &self.release_time {
            if let Some(other_release_time) = &other.release_time {
                return release_time < other_release_time;
            }
        }

        if let Some(time) = &self.time {
            if let Some(other_time) = &other.time {
                return time < other_time;
            }
        }

        false
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
pub struct VersionAssetIndex {
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

impl IntoDownloadable for MappingsClass {
    fn into_downloadable(self, base_path: &std::path::Path) -> carbon_net::Downloadable {
        let jar_path = base_path
            .join("clients")
            .join(format!("{}.jar", &self.sha1));

        carbon_net::Downloadable::new(self.url, jar_path)
            .with_checksum(Some(carbon_net::Checksum::Sha1(self.sha1)))
            .with_size(self.size)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Extract {
    pub exclude: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Natives {
    pub osx: Option<String>,
    pub linux: Option<String>,
    pub windows: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryRule {
    pub action: Action,
    pub os: Option<LibOs>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub file: VersionAssetIndex,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "disallow")]
    Disallow,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Name {
    #[serde(rename = "osx")]
    Osx,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "windows")]
    Windows,
}
