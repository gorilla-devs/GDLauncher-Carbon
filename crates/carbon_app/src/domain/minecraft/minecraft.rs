use std::{collections::HashMap, path::PathBuf};

use carbon_net::{IntoDownloadable, IntoVecDownloadable};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::modded::{Processor, SidedDataEntry};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinecraftManifest {
    pub latest: MinecraftLatest,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinecraftLatest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: VersionType,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: String,
}

// Need custom type to impl external traits for Vec<Library>
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Libraries {
    pub libraries: Vec<Library>,
}
impl Libraries {
    pub fn get_allowed_libraries(&self) -> Vec<Library> {
        let libs = &self.libraries;

        let results = libs.iter().filter(|l| l.is_allowed());

        results.cloned().collect()
    }
}

impl IntoVecDownloadable for Libraries {
    /// Returns a list of Downloadable objects for all libraries (native and non-native)
    fn into_vec_downloadable(self, base_path: &std::path::Path) -> Vec<carbon_net::Downloadable> {
        let mut files = vec![];

        for library in self.libraries {
            if !library.is_allowed() {
                continue;
            }

            if let Some(downloadable) = library.clone().into_lib_downloadable(base_path) {
                files.push(downloadable);
            }

            if let Some(downloadable) = library.into_natives_downloadable(base_path) {
                files.push(downloadable);
            }
        }

        files
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub inherits_from: Option<String>,
    pub arguments: Option<VersionArguments>,
    pub asset_index: VersionAssetIndex,
    pub assets: Option<String>,
    pub downloads: Option<VersionDownloads>,
    pub id: String,
    pub java_version: Option<JavaVersion>,
    #[serde(flatten)]
    pub libraries: Libraries,
    pub logging: Option<Logging>,
    pub main_class: String,
    pub minimum_launcher_version: Option<i64>,
    pub release_time: DateTime<Utc>,
    pub time: DateTime<Utc>,
    pub minecraft_arguments: Option<String>,
    #[serde(rename = "type")]
    pub type_: VersionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// (Forge-only)
    pub data: Option<HashMap<String, SidedDataEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// (Forge-only) The list of processors to run after downloading the files
    pub processors: Option<Vec<Processor>>,
}

impl VersionInfo {
    pub fn is_older_than(&self, other: &VersionInfo) -> bool {
        return &self.release_time < &other.release_time;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VersionType {
    #[serde(rename = "old_alpha")]
    OldAlpha,
    #[serde(rename = "old_beta")]
    OldBeta,
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "snapshot")]
    Snapshot,
}

impl From<VersionType> for String {
    fn from(type_: VersionType) -> Self {
        type_.to_string()
    }
}

impl ToString for VersionType {
    fn to_string(&self) -> String {
        match self {
            VersionType::OldAlpha => "old_alpha".to_string(),
            VersionType::OldBeta => "old_beta".to_string(),
            VersionType::Release => "release".to_string(),
            VersionType::Snapshot => "snapshot".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum Argument {
    Complex(ArgumentEntry),
    String(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionArguments {
    pub game: Vec<Argument>,
    pub jvm: Vec<Argument>,
}

impl VersionArguments {
    pub fn new() -> Self {
        Self {
            game: vec![],
            jvm: vec![],
        }
    }
}

impl Default for VersionArguments {
    fn default() -> Self {
        Self {
            game: vec![
                Argument::String("-Xms${ram}M".to_string()),
                Argument::String("-Xmx${ram}M".to_string()),
                Argument::String("-XX:+UnlockExperimentalVMOptions".to_string()),
                Argument::String("-XX:+UseG1GC".to_string()),
                Argument::String("-XX:G1NewSizePercent=20".to_string()),
                Argument::String("-XX:G1ReservePercent=20".to_string()),
                Argument::String("-XX:MaxGCPauseMillis=50".to_string()),
                Argument::String("-XX:G1HeapRegionSize=32M".to_string()),
                Argument::String("-Dlog4j2.formatMsgNoLookups=true".to_string()),
            ],
            jvm: vec![
                Argument::Complex(ArgumentEntry {
                    rules: vec![Rule {
                        action: Action::Allow,
                        os: Some(OsRule {
                            name: Some(OsName::MacOS),
                            version: None,
                            arch: None,
                        }),
                        features: None,
                    }],
                    value: Value::String("-XstartOnFirstThread".to_string()),
                }),
                Argument::Complex(ArgumentEntry {
                    rules: vec![Rule {
                        action: Action::Allow,
                        os: Some(OsRule {
                            name: Some(OsName::Windows),
                            version: None,
                            arch: None,
                        }),
                        features: None,
                    }],
                    value: Value::String("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump".to_string()),
                }),
                Argument::Complex(ArgumentEntry {
                    rules: vec![Rule {
                        action: Action::Allow,
                        os: Some(OsRule {
                            name: Some(OsName::Windows),
                            version: Some(r#"^10\\."#.to_string()),
                            arch: None,
                        }),
                        features: None,
                    }],
                    value: Value::StringArray(vec![
                        "-Dos.name=Windows 10".to_string(),
                        "-Dos.version=10.0".to_string(),
                    ]),
                }),
                Argument::Complex(ArgumentEntry {
                    rules: vec![Rule {
                        action: Action::Allow,
                        os: Some(OsRule {
                            name: None,
                            version: None,
                            arch: Some("x86".to_string()),
                        }),
                        features: None,
                    }],
                    value: Value::String("-Xss1M".to_string()),
                }),
                Argument::String("-Djava.library.path=${natives_directory}".to_string()),
                Argument::String("-Dminecraft.launcher.brand=${launcher_name}".to_string()),
                Argument::String("-Dminecraft.launcher.version=${launcher_version}".to_string()),

                // Apparently this "hack" is only needed for launcherVersion < 18
                Argument::String(
                    "-Dminecraft.applet.TargetDirectory=${game_directory}".to_string(),
                ),
                Argument::String("-cp".to_string()),
                Argument::String("${classpath}".to_string()),
            ],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Features {
    pub is_demo_user: Option<bool>,
    pub has_custom_resolution: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArgumentEntry {
    pub rules: Vec<Rule>,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionAssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: i64,
    #[serde(rename = "totalSize")]
    pub total_size: Option<i64>,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionDownloads {
    pub client: VersionDownloadsMappingsClass,
    pub client_mappings: Option<VersionDownloadsMappingsClass>,
    pub server: Option<VersionDownloadsMappingsClass>,
    pub server_mappings: Option<VersionDownloadsMappingsClass>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionDownloadsMappingsClass {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

impl IntoDownloadable for VersionDownloadsMappingsClass {
    fn into_downloadable(self, base_path: &std::path::Path) -> carbon_net::Downloadable {
        let jar_path = base_path.join(format!("{}.jar", &self.sha1));

        carbon_net::Downloadable::new(self.url, jar_path)
            .with_checksum(Some(carbon_net::Checksum::Sha1(self.sha1)))
            .with_size(self.size)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<Natives>,
    pub extract: Option<Extract>,
}

impl Library {
    pub fn into_lib_downloadable(
        self,
        base_path: &std::path::Path,
    ) -> Option<carbon_net::Downloadable> {
        let artifact = self.downloads.artifact;

        if let Some(artifact) = artifact {
            let checksum = Some(carbon_net::Checksum::Sha1(artifact.sha1));

            return Some(carbon_net::Downloadable {
                url: artifact.url,
                path: PathBuf::from(base_path).join(artifact.path),
                checksum,
                size: Some(artifact.size),
            });
        }

        None
    }

    pub fn into_natives_downloadable(
        self,
        base_path: &std::path::Path,
    ) -> Option<carbon_net::Downloadable> {
        let Some(classifiers) = self.downloads.classifiers else {
            return None;
        };

        let Some(natives) = self.natives else {
            return None;
        };

        let Some(natives_name) = natives.get_os_specific(OsName::default()) else {
            return None;
        };

        let Some(mapping_class) = classifiers.get(&natives_name) else {
            return None;
        };

        let checksum = Some(carbon_net::Checksum::Sha1(mapping_class.sha1));

        Some(carbon_net::Downloadable {
            url: mapping_class.url,
            path: PathBuf::from(base_path).join(mapping_class.path),
            checksum,
            size: Some(mapping_class.size),
        })
    }
}

impl Library {
    #[tracing::instrument]
    pub fn is_allowed(&self) -> bool {
        let Some(rules) = &self.rules else {
            return true;
        };

        for rule in rules {
            if !rule.is_allowed() {
                return false;
            }
        }

        true
    }

    pub fn is_native_artifact(&self) -> bool {
        self.natives.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MappingsClass {
    pub sha1: String,
    pub size: u64,
    pub url: String,
    pub path: String,
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

    // For some reasons they have both macos and osx...
    #[serde(rename = "natives-macos")]
    pub natives_macos: Option<MappingsClass>,
    #[serde(rename = "natives-osx")]
    pub natives_osx: Option<MappingsClass>,

    #[serde(rename = "natives-windows")]
    pub natives_windows: Option<MappingsClass>,
    pub sources: Option<MappingsClass>,
}

impl Classifiers {
    pub fn get(self, natives_name: &str) -> Option<MappingsClass> {
        match natives_name {
            "natives-linux" => self.natives_linux,
            "natives-macos" => self.natives_macos,
            "natives-osx" => self.natives_osx,
            "natives-windows" => self.natives_windows,
            _ => None,
        }
    }
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

impl Natives {
    fn get_os_specific(self, os: OsName) -> Option<String> {
        match os {
            OsName::Linux => self.linux,
            OsName::Windows => self.windows,
            OsName::MacOS => self.osx,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Rule {
    pub action: Action,
    pub os: Option<OsRule>,
    pub features: Option<Features>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Action {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "disallow")]
    Disallow,
}

impl Rule {
    pub fn is_allowed(&self) -> bool {
        let current_arch = std::env::consts::ARCH;

        let os = self.os.as_ref().unwrap_or(&OsRule {
            name: None,
            version: None,
            arch: None,
        });

        let is_os_allowed = os.name.clone().unwrap_or_default() == OsName::get_current_os();
        let is_arch_allowed = os.arch.clone().unwrap_or(current_arch.to_string()) == current_arch;
        let is_feature_allowed = self.features.is_none();
        // TODO: Check version

        match self.action {
            Action::Allow => is_os_allowed && is_arch_allowed && is_feature_allowed,
            Action::Disallow => !(is_os_allowed && is_arch_allowed && is_feature_allowed),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct OsRule {
    pub name: Option<OsName>,
    pub version: Option<String>,
    pub arch: Option<String>, // TODO: Make enum?
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum OsName {
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "osx")]
    MacOS,
}

impl OsName {
    pub fn get_current_os() -> OsName {
        if cfg!(target_os = "linux") {
            OsName::Linux
        } else if cfg!(target_os = "macos") {
            OsName::MacOS
        } else if cfg!(target_os = "windows") {
            OsName::Windows
        } else {
            panic!("Unknown OS");
        }
    }
}

impl Default for OsName {
    fn default() -> Self {
        Self::get_current_os()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Logging {
    pub client: LoggingClient,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingClient {
    pub argument: String,
    pub file: VersionAssetIndex,
    #[serde(rename = "type")]
    pub client_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum Value {
    String(String),
    StringArray(Vec<String>),
}

#[cfg(test)]
mod test {}
