use std::{
    borrow::Borrow,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, trace};

use crate::net::Download;

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

    pub async fn get_asset_index_meta(
        &self,
        base_path: &Path,
    ) -> Result<super::assets::AssetIndex> {
        let try_download = || async move {
            let url = self
                .asset_index
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("No asset index"))?
                .url
                .clone();
            let resp = reqwest::get(&url)
                .await?
                .json::<super::assets::AssetIndex>()
                .await?;

            Ok::<_, anyhow::Error>(resp)
        };

        let meta_dir = base_path.join("assets").join("indexes");

        let resp = match try_download().await {
            Ok(resp) => {
                if !meta_dir.exists() {
                    tokio::fs::create_dir_all(&meta_dir).await?;
                }

                let meta_path = meta_dir.join(format!(
                    "{}.json",
                    self.id
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("No id for asset"))?
                ));

                let mut file = tokio::fs::File::create(&meta_path).await?;
                file.write_all(serde_json::to_string(&resp)?.as_bytes())
                    .await?;
                resp
            }
            Err(e) => {
                trace!("Failed to download asset index meta: {e}. Fallback to trying reading cached file");
                let meta_path = meta_dir.join(format!(
                    "{}.json",
                    self.id
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("No id for asset"))?
                ));

                let mut file = tokio::fs::File::open(&meta_path).await?;
                let mut file_str = String::new();
                file.read_to_string(&mut file_str).await?;
                serde_json::from_str(&file_str)?
            }
        };

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
    pub async fn get_allowed_libraries(
        &self,
        base_path: &Path,
    ) -> Result<Vec<crate::net::Download>> {
        let libraries = self.filter_allowed_libraries();

        let mut downloads: Vec<crate::net::Download> = vec![];

        for library in libraries {
            let lib: Result<Download, anyhow::Error> = library.try_into();
            if let Ok(mut lib) = lib {
                lib.path = base_path.join("libraries").join(lib.path); // how do we do this from inside the TryFrom impl?
                downloads.push(lib);
            }

            if let Some(natives) = &library.downloads.classifiers {
                let native: Result<Download, anyhow::Error> = natives.try_into();
                if let Ok(mut native) = native {
                    native.path = base_path.join("libraries").join(native.path); // how do we do this from inside the TryFrom impl?
                    downloads.push(native);
                }
            }
        }

        Ok(downloads)
    }

    pub async fn get_jar_client(&self, base_path: &Path) -> Result<crate::net::Download> {
        let jar = &self
            .downloads
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No downloads"))?
            .client;

        let version_id = self
            .id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No id for client jar"))?;

        let jar_path = base_path.join("clients").join(format!("{version_id}.jar"));

        Ok(crate::net::Download::new(jar.url.clone(), jar_path)
            .with_checksum(crate::net::Checksum::Sha1(jar.sha1.clone()))
            .with_size(jar.size))
    }

    pub async fn extract_natives(&self, base_path: &Path) -> Result<()> {
        let libraries = self.filter_allowed_libraries();

        for library in libraries {
            println!("Extracting natives for {library:#?}");
            if let Some(natives) = &library.downloads.classifiers {
                let native: Result<Download, anyhow::Error> = natives.try_into();
                if let Ok(native) = native {
                    let native_lib_path = base_path.join("libraries").join(native.path);
                    let extract_dir = base_path.join("natives");
                    if !extract_dir.exists() {
                        tokio::fs::create_dir_all(&extract_dir).await?;
                    }
                    let mut exclude = vec![];
                    if let Some(extract) = &library.extract {
                        exclude.extend(extract.exclude.clone());
                    }

                    let jh = tokio::task::spawn_blocking(move || async move {
                        trace!(
                            "Extracting natives PATH for {native_lib_path:#?} to {extract_dir:#?}"
                        );
                        let file = std::fs::File::open(&native_lib_path)?;

                        let mut archive = zip::ZipArchive::new(file)?;

                        'outer_zip: for i in 0..archive.len() {
                            let mut file = archive.by_index(i)?;
                            let outpath = file.name();

                            for pattern in &exclude {
                                if outpath.starts_with(pattern.as_str()) {
                                    continue 'outer_zip;
                                }
                            }

                            if (file.name()).ends_with('/') {
                                std::fs::create_dir_all(&extract_dir.join(outpath))?;
                            } else {
                                let mut outfile =
                                    std::fs::File::create(&extract_dir.join(outpath))?;
                                std::io::copy(&mut file, &mut outfile)?;
                            }
                        }

                        Ok::<_, anyhow::Error>(())
                    });

                    jh.await?.await?;
                }
            }
        }

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
        let path = PathBuf::new().join(
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

impl TryFrom<&Classifiers> for crate::net::Download {
    type Error = anyhow::Error;

    fn try_from(value: &Classifiers) -> Result<Self, Self::Error> {
        let classifier = value.clone();
        let download = match std::env::consts::OS {
            "windows" => {
                if let Some(windows) = classifier.natives_windows {
                    let path = PathBuf::from(
                        windows
                            .path
                            .ok_or_else(|| anyhow::anyhow!("No path in lib"))?,
                    );
                    let checksum = Some(crate::net::Checksum::Sha1(windows.sha1));

                    crate::net::Download {
                        url: windows.url,
                        path,
                        checksum,
                        size: Some(windows.size),
                    }
                } else {
                    bail!("No windows natives");
                }
            }
            "macos" => {
                if let Some(macos) = classifier.natives_macos {
                    let path = macos
                        .path
                        .ok_or_else(|| anyhow::anyhow!("No path in lib"))?;

                    let checksum = Some(crate::net::Checksum::Sha1(macos.sha1));

                    crate::net::Download {
                        url: macos.url,
                        path: PathBuf::from(path),
                        checksum,
                        size: Some(macos.size),
                    }
                } else if let Some(osx) = classifier.natives_osx {
                    let path =
                        PathBuf::from(osx.path.ok_or_else(|| anyhow::anyhow!("No path in lib"))?);
                    let checksum = Some(crate::net::Checksum::Sha1(osx.sha1));

                    crate::net::Download {
                        url: osx.url,
                        path,
                        checksum,
                        size: Some(osx.size),
                    }
                } else {
                    bail!("No macos natives");
                }
            }
            "linux" => {
                if let Some(linux) = classifier.natives_linux {
                    let path = PathBuf::from(
                        linux
                            .path
                            .ok_or_else(|| anyhow::anyhow!("No path in lib"))?,
                    );
                    let checksum = Some(crate::net::Checksum::Sha1(linux.sha1));

                    crate::net::Download {
                        url: linux.url,
                        path,
                        checksum,
                        size: Some(linux.size),
                    }
                } else {
                    bail!("No linux natives");
                }
            }
            _ => bail!("Unknown OS"),
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
