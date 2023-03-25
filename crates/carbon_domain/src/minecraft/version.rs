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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub inherits_from: Option<String>,
    pub arguments: Option<Arguments>,
    pub asset_index: VersionAssetIndex,
    pub assets: Option<String>,
    pub compliance_level: Option<i64>,
    pub downloads: Option<VersionDownloads>,
    pub id: String,
    pub java_version: Option<JavaVersion>,
    #[serde(flatten)]
    pub libraries: Libraries,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Argument {
    Complex(ArgumentEntry),
    String(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Arguments {
    pub game: Vec<Argument>,
    pub jvm: Vec<Argument>,
}

impl Default for Arguments {
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
                            name: Some(OsName::Osx),
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Features {
    pub is_demo_user: Option<bool>,
    pub has_custom_resolution: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArgumentEntry {
    pub rules: Vec<Rule>,
    pub value: Value,
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

        let Some(native) = classifiers.get(OsName::get_os_name()) else {
            return None;
        };

        let checksum = Some(carbon_net::Checksum::Sha1(native.sha1));

        Some(carbon_net::Downloadable {
            url: native.url,
            path: PathBuf::from(base_path).join(native.path),
            checksum,
            size: Some(native.size),
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
    #[serde(rename = "natives-macos")]
    pub natives_macos: Option<MappingsClass>,
    #[serde(rename = "natives-windows")]
    pub natives_windows: Option<MappingsClass>,
    pub sources: Option<MappingsClass>,
    #[serde(rename = "natives-osx")]
    pub natives_osx: Option<MappingsClass>,
}

impl Classifiers {
    pub fn get(self, os: OsName) -> Option<MappingsClass> {
        match os {
            OsName::Linux => self.natives_linux,
            OsName::Windows => self.natives_windows,
            OsName::Osx => self.natives_osx,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule {
    pub action: Action,
    pub os: Option<OsRule>,
    pub features: Option<Features>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

        let is_os_allowed = os.name.clone().unwrap_or_default() == OsName::get_os_name();
        let is_arch_allowed = os.arch.clone().unwrap_or(current_arch.to_string()) == current_arch;
        let is_feature_allowed = self.features.is_none();
        // TODO: Check version

        match self.action {
            Action::Allow => is_os_allowed && is_arch_allowed && is_feature_allowed,
            Action::Disallow => !(is_os_allowed && is_arch_allowed && is_feature_allowed),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    Osx,
}

impl OsName {
    pub fn get_os_name() -> OsName {
        if cfg!(target_os = "linux") {
            OsName::Linux
        } else if cfg!(target_os = "macos") {
            OsName::Osx
        } else if cfg!(target_os = "windows") {
            OsName::Windows
        } else {
            panic!("Unknown OS");
        }
    }
}

impl Default for OsName {
    fn default() -> Self {
        Self::get_os_name()
    }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    StringArray(Vec<String>),
}

#[cfg(test)]
mod test {}
