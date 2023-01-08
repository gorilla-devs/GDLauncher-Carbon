use futures::{StreamExt, TryFutureExt};
use sha2::Digest;
use std::path::{Path, PathBuf};
use tokio::{
    fs::OpenOptions,
    sync::watch::{channel, Sender},
};

use crate::{constants::JAVA_RUNTIMES_FOLDER, error::JavaError};

use super::{JavaAuto, JavaProgress};

pub enum JavaMajorSemVer {
    Version8,
    Version17,
}

pub struct AdoptOpenJDK {
    version: JavaMajorSemVer,
    release_date: String,
}

impl AdoptOpenJDK {
    pub fn new(version: JavaMajorSemVer, release_date: String) -> Self {
        Self {
            version,
            release_date,
        }
    }
}

#[async_trait::async_trait]
impl JavaAuto for AdoptOpenJDK {
    async fn setup(
        &self,
        base_path: &Path,
        // TODO: implement progress reporting
        progress_report: Sender<JavaProgress>,
    ) -> Result<(), JavaError> {
        let adoptopenjdk_meta: Vec<Asset> = self.get_runtime_meta().await?;

        let asset = adoptopenjdk_meta
            .first()
            .ok_or(JavaError::NoAdoptOpenJDKMetaValidVersion)?;
        let release_name = asset.release_name.clone();

        let runtime = base_path.join("java_runtime").join("openjdk");
        tokio::fs::create_dir_all(&runtime)
            .await
            .map_err(JavaError::CannotCreateJavaOpenJDKRuntimeDirectory)?;

        let zip_path = runtime.join(format!("{release_name}.tar.gz"));

        let (tx, _) = channel(carbon_net::Progress::default());

        carbon_net::download_file(
            carbon_net::Downloadable::new(&asset.binary.package.link, &zip_path),
            tx,
        )
        .await
        .map_err(|_| JavaError::CannotDownloadJavaOpenJDK)?;
        carbon_compression::decompress(&zip_path, &runtime).await?;

        tokio::fs::remove_file(&zip_path)
            .await
            .map_err(JavaError::CannotDeletePreviouslyDownloadedJavaOpenJDKFile)?;

        Ok(())
    }

    async fn get_runtime_meta<G>(&self) -> Result<G, JavaError>
    where
        G: serde::de::DeserializeOwned + for<'de> serde::Deserialize<'de>,
    {
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

        let version = match self.version {
            JavaMajorSemVer::Version8 => "8",
            JavaMajorSemVer::Version17 => "17",
        };

        let url = format!(
            "https://api.adoptopenjdk.net/v3/assets/latest/{version}/hotspot?architecture={java_arch}&image_type=jre&jvm_impl=hotspot&os={java_os}&page=0&page_size=1&project=jdk&sort_method=DEFAULT&sort_order=DESC"
        );

        let res = reqwest::get(url)
            .await
            .map_err(JavaError::CannotRetrieveOpenJDKAssets)?;

        res.json()
            .map_err(JavaError::CannotParseAdoptOpenJDKMeta)
            .await
    }

    fn locate_binary(&self, base_path: &Path) -> PathBuf {
        match std::env::consts::OS {
            "linux" => {
                todo!()
            }
            "windows" => {
                todo!()
            }
            "macos" => base_path
                .join(JAVA_RUNTIMES_FOLDER)
                .join("openjdk")
                .join("Contents")
                .join("Home")
                .join("bin")
                .join("java"),
            _ => unreachable!("Unsupported OS"),
        }
    }

    async fn check_for_updates(&self) -> Result<(), JavaError> {
        todo!()
    }

    async fn update(&mut self) -> Result<(), JavaError> {
        todo!()
    }
}

#[derive(Debug, serde::Deserialize)]
struct Asset {
    binary: Binary,
    release_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct Binary {
    package: Package,
}

#[derive(Debug, serde::Deserialize)]
struct Package {
    link: String,
    checksum: String,
    size: u64,
}

#[cfg(test)]
mod tests {
    use tokio::sync::watch::channel;

    use super::*;

    #[tokio::test]
    async fn test_setup_openjdk_jre() {
        let current_path = std::env::current_dir().unwrap();

        let adoptopenjdk = AdoptOpenJDK {
            version: JavaMajorSemVer::Version17,
            release_date: "2021-09-14".to_string(),
        };

        let (tx, _) = channel(JavaProgress::default());

        adoptopenjdk.setup(&current_path, tx).await.unwrap();
    }
}
