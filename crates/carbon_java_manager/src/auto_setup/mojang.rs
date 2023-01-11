use super::{JavaAuto, JavaMeta, JavaProgress};
use crate::{constants::JAVA_RUNTIMES_FOLDER, error::JavaError};
use carbon_net::Downloadable;
use chrono::{DateTime, FixedOffset};
use futures::TryFutureExt;
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    os::unix::prelude::PermissionsExt,
    path::{Path, PathBuf},
};
use tokio::sync::watch::Sender;

pub enum RuntimeEdition {
    Alpha,
    Beta,
    Gamma,
    LegacyJRE,
    MinecraftExe,
}

pub struct MojangRuntime {
    pub version: RuntimeEdition,
    release_date: Option<DateTime<FixedOffset>>,
}

impl MojangRuntime {
    pub fn new(version: RuntimeEdition) -> Self {
        Self {
            version,
            release_date: None,
        }
    }
}

#[async_trait::async_trait]
impl JavaAuto for MojangRuntime {
    async fn setup(
        &mut self,
        base_path: &Path,
        // TODO: implement progress reporting
        progress_report: Sender<JavaProgress>,
    ) -> Result<(), JavaError> {
        let mojang_assets = self
            .get_runtime_assets(&base_path.join(JAVA_RUNTIMES_FOLDER))
            .await?;

        self.release_date = Some(mojang_assets.last_updated);

        let (progress, mut recv) = tokio::sync::watch::channel(carbon_net::Progress {
            current_count: 0,
            current_size: 0,
        });

        let length = &mojang_assets.download.len();

        let task_handle = tokio::spawn(async move {
            carbon_net::download_multiple(mojang_assets.download, progress).await
        });

        while (recv.borrow_mut().changed().await).is_ok() {
            println!("{} / {}", recv.borrow().current_count, length);
        }

        task_handle.await.unwrap().unwrap();

        // Fix permissions
        let java_path = self.locate_binary(base_path);

        let mut perms = std::fs::metadata(&java_path).unwrap().permissions();
        perms.set_mode(0o777);
        std::fs::set_permissions(&java_path, perms).unwrap();
        Ok(())
    }

    async fn get_runtime_assets(&self, runtime_path: &Path) -> Result<JavaMeta, JavaError> {
        let java_os = match std::env::consts::OS {
            "linux" => "linux",
            "windows" => "windows",
            "macos" => "mac",
            _ => unreachable!("Unsupported OS"),
        };

        let java_arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "x86" => "x86",
            "aarch64" => "aarch64",
            _ => unreachable!("Unsupported architecture"),
        };

        let url = "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json".to_string();

        let res = reqwest::get(url)
            .await
            .map_err(JavaError::CannotRetrieveMojangJDKAssets)?;

        let mojang_meta = res
            .json::<MojangMeta>()
            .map_err(JavaError::CannotParseMojangJDKMeta)
            .await?;

        let runtime_meta = match java_os {
            "linux" => {
                if java_arch == "x86" {
                    mojang_meta.linux_i386
                } else {
                    mojang_meta.linux
                }
            }
            "windows" => {
                if java_arch == "x86" {
                    mojang_meta.windows_x86
                } else {
                    mojang_meta.windows_x64
                }
            }
            "mac" => {
                if java_arch == "aarch64" {
                    mojang_meta.mac_os_arm64.or(mojang_meta.mac_os)
                } else {
                    mojang_meta.mac_os
                }
            }
            _ => unreachable!("Unsupported OS"),
        }
        .ok_or_else(|| JavaError::NoJavaMojangJDKAvailableForOSArch)?;

        let runtime_list = match self.version {
            RuntimeEdition::Alpha => runtime_meta.java_runtime_alpha,
            RuntimeEdition::Beta => runtime_meta.java_runtime_beta,
            RuntimeEdition::Gamma => runtime_meta.java_runtime_gamma,
            RuntimeEdition::LegacyJRE => runtime_meta.jre_legacy,
            RuntimeEdition::MinecraftExe => runtime_meta.minecraft_java_exe,
        };

        let runtime = runtime_list
            .first()
            .ok_or_else(|| JavaError::NoJavaMojangJDKAvailableForOSArch)?;

        let res = reqwest::get(&runtime.manifest.url)
            .await
            .map_err(JavaError::CannotRetrieveMojangJDKRuntimeMeta)?;

        let runtime_meta = res
            .json::<MojangRuntimeJDKMeta>()
            .map_err(JavaError::CannotParseMojangJDKRuntimeMeta)
            .await?;

        let mut assets = JavaMeta {
            last_updated: chrono::DateTime::parse_from_rfc3339(&runtime.version.released).map_err(
                |_| JavaError::JavaUpdateDateFromMetaInvalid(runtime.version.released.clone()),
            )?,
            download: vec![],
        };

        for (name, asset) in runtime_meta.files {
            let path = runtime_path.join("mojang").join(&name);
            let downloadable = asset
                .downloads
                .and_then(|d| d.raw)
                .map(|d| carbon_net::Downloadable::new(d.url, path));

            if let Some(downloadable) = downloadable {
                if asset._type == "file" {
                    assets.download.push(downloadable);
                }
            }
        }

        Ok(assets)
    }

    fn locate_binary(&self, base_path: &Path) -> PathBuf {
        match std::env::consts::OS {
            "linux" | "windows" => base_path
                .join(JAVA_RUNTIMES_FOLDER)
                .join("mojang")
                .join("bin")
                .join("java"),
            "macos" => base_path
                .join(JAVA_RUNTIMES_FOLDER)
                .join("mojang")
                .join("Contents")
                .join("Home")
                .join("bin")
                .join("java"),
            _ => unreachable!("Unsupported OS"),
        }
    }

    async fn check_for_updates(&self, runtime_path: &Path) -> Result<bool, JavaError> {
        let mojang_assets = self.get_runtime_assets(&runtime_path).await?;

        let updated_at = mojang_assets.last_updated.timestamp();

        if updated_at
            > self
                .release_date
                .ok_or(JavaError::NoReleaseDateProvidedForJavaComponent)?
                .timestamp()
        {
            return Ok(false);
        }

        Ok(true)
    }

    async fn update(&mut self) -> Result<(), JavaError> {
        todo!()
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct MojangMeta {
    linux: Option<OsRuntime>,
    linux_i386: Option<OsRuntime>,
    mac_os: Option<OsRuntime>,
    mac_os_arm64: Option<OsRuntime>,
    windows_x86: Option<OsRuntime>,
    windows_x64: Option<OsRuntime>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct OsRuntime {
    java_runtime_alpha: Vec<Runtime>,
    java_runtime_beta: Vec<Runtime>,
    java_runtime_gamma: Vec<Runtime>,
    jre_legacy: Vec<Runtime>,
    minecraft_java_exe: Vec<Runtime>,
}

#[derive(Debug, serde::Deserialize)]
struct Runtime {
    manifest: MojangDownloadable,
    version: Version,
}

#[derive(Debug, serde::Deserialize)]
struct MojangDownloadable {
    sha1: String,
    size: u64,
    url: String,
}

#[derive(Debug, serde::Deserialize)]
struct Version {
    name: String,
    released: String,
}

#[derive(Debug, serde::Deserialize)]
struct MojangRuntimeJDKMeta {
    files: HashMap<String, MojangRuntimeJDKMetaAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct MojangRuntimeJDKMetaAsset {
    #[serde(rename = "type")]
    _type: String,
    downloads: Option<MojangRuntimeJDKMetaAssetDownloads>,
}

#[derive(Debug, serde::Deserialize)]
struct MojangRuntimeJDKMetaAssetDownloads {
    lzma: Option<MojangDownloadable>,
    raw: Option<MojangDownloadable>,
}

#[cfg(test)]
mod tests {
    use tokio::sync::watch::channel;

    use super::*;

    #[tokio::test]
    async fn test_setup_mojang_runtime_jre() {
        let current_path = std::env::current_dir().unwrap();

        let mut mojang = MojangRuntime::new(RuntimeEdition::Gamma);

        let (tx, _) = channel(JavaProgress::default());

        mojang.setup(&current_path, tx).await.unwrap();

        let java_path = mojang.locate_binary(&current_path);

        assert!(java_path.exists());

        let java_version = std::process::Command::new(java_path)
            .arg("-version")
            .output()
            .unwrap();

        assert!(java_version.status.success());
    }
}
