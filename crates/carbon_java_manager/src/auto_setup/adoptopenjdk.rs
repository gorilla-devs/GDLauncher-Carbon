use carbon_net::Downloadable;
use chrono::{DateTime, FixedOffset};
use futures::TryFutureExt;
use std::path::{Path, PathBuf};
use tokio::sync::watch::{channel, Sender};

use crate::{constants::JAVA_RUNTIMES_FOLDER, error::JavaError, JavaVersion};

use super::{JavaAuto, JavaMeta, JavaProgress};

pub struct AdoptOpenJDK {
    version: JavaVersion,
    extract_folder_name: Option<String>,
    release_date: Option<DateTime<FixedOffset>>,
}

impl AdoptOpenJDK {
    pub fn new(version: JavaVersion) -> Self {
        Self {
            version,
            extract_folder_name: None,
            release_date: None,
        }
    }
}

#[async_trait::async_trait]
impl JavaAuto for AdoptOpenJDK {
    async fn setup(
        &mut self,
        base_path: &Path,
        // TODO: implement progress reporting
        progress_report: Sender<JavaProgress>,
    ) -> Result<(), JavaError> {
        let runtime = base_path.join(JAVA_RUNTIMES_FOLDER).join("openjdk");
        let meta = self.get_runtime_assets(&runtime).await?;

        self.release_date = Some(meta.last_updated);
        self.extract_folder_name = Some(meta.extract_folder_name);

        tokio::fs::create_dir_all(&runtime)
            .await
            .map_err(JavaError::CannotCreateJavaOpenJDKRuntimeDirectory)?;

        let (tx, _) = channel(carbon_net::Progress::default());

        let download = &meta.download[0];
        let download_path = &download.path.clone();

        carbon_net::download_file(download, tx)
            .await
            .map_err(|_| JavaError::CannotDownloadJavaOpenJDK)?;
        carbon_compression::decompress(&download_path, &runtime).await?;

        tokio::fs::remove_file(&download_path)
            .await
            .map_err(JavaError::CannotDeletePreviouslyDownloadedJavaOpenJDKFile)?;

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

        let version = match self.version.major {
            8 => "8",
            17 => "17",
            _ => return Err(JavaError::JavaAutoSetupVersionNotSupported),
        };

        let url = format!(
            "https://api.adoptopenjdk.net/v3/assets/latest/{version}/hotspot?architecture={java_arch}&image_type=jre&jvm_impl=hotspot&os={java_os}&page=0&page_size=1&project=jdk&sort_method=DEFAULT&sort_order=DESC"
        );

        let json_res = reqwest::get(url)
            .await
            .map_err(JavaError::CannotRetrieveOpenJDKAssets)?
            .json::<Vec<Asset>>()
            .map_err(JavaError::CannotParseAdoptOpenJDKMeta)
            .await?;

        let asset = json_res
            .first()
            .ok_or(JavaError::NoAdoptOpenJDKMetaValidVersion)?;

        let meta = JavaMeta {
            last_updated: chrono::DateTime::parse_from_rfc3339(&asset.binary.updated_at).map_err(
                |_| JavaError::JavaUpdateDateFromMetaInvalid(asset.binary.updated_at.clone()),
            )?,
            extract_folder_name: format!(
                "{}-{}-{}",
                asset.binary.project, asset.version.openjdk_version, asset.binary.image_type
            ),
            download: vec![Downloadable::new(
                &asset.binary.package.link,
                runtime_path.join(&asset.binary.package.name),
            )],
        };

        Ok(meta)
    }

    fn locate_binary(&self, base_path: &Path) -> Result<PathBuf, JavaError> {
        let path = match std::env::consts::OS {
            "linux" | "windows" => base_path
                .join(JAVA_RUNTIMES_FOLDER)
                .join("openjdk")
                .join(self.extract_folder_name.clone().unwrap())
                .join("bin")
                .join("java"),
            "macos" => base_path
                .join(JAVA_RUNTIMES_FOLDER)
                .join("openjdk")
                .join(self.extract_folder_name.clone().unwrap())
                .join("Contents")
                .join("Home")
                .join("bin")
                .join("java"),
            _ => unreachable!("Unsupported OS"),
        };

        Ok(path)
    }

    async fn check_for_updates(&self, base_path: &Path) -> Result<bool, JavaError> {
        let meta: JavaMeta = self.get_runtime_assets(base_path).await?;

        if meta.last_updated.timestamp()
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
struct Asset {
    binary: Binary,
    release_name: String,
    version: Version,
}

#[derive(Debug, serde::Deserialize)]
struct Version {
    openjdk_version: String,
}

#[derive(Debug, serde::Deserialize)]
struct Binary {
    package: Package,
    updated_at: String,
    image_type: String,
    project: String,
}

#[derive(Debug, serde::Deserialize)]
struct Package {
    link: String,
    checksum: String,
    size: u64,
    name: String,
}

#[cfg(test)]
mod tests {
    use tokio::sync::watch::channel;

    use super::*;

    #[tokio::test]
    async fn test_setup_openjdk_jre() {
        let current_path = std::env::current_dir().unwrap();

        let mut adoptopenjdk = AdoptOpenJDK::new(JavaVersion::from_major(17));

        let (tx, _) = channel(JavaProgress::default());

        adoptopenjdk.setup(&current_path, tx).await.unwrap();

        let java_path = adoptopenjdk.locate_binary(&current_path).unwrap();
        println!("{:?}", java_path);

        assert!(java_path.exists());

        let java_version = std::process::Command::new(java_path)
            .arg("-version")
            .output()
            .unwrap();

        assert!(java_version.status.success());
    }
}
