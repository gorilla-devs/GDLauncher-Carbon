use super::{InstallProgress, ModLoaderError, ModLoaderHandler, ModloaderVersion};
use crate::{instance::Instance, minecraft::meta::McMeta};
use async_trait::async_trait;
use std::sync::Weak;
use thiserror::Error;
use tokio::sync::{watch::Sender, RwLock};
use tracing::trace;

mod tests;

#[derive(Error, Debug)]
pub enum VanillaError {
    #[error("Failed to download manifest meta")]
    DownloadManifestMetaFailed,
    #[error("Minecraft version not found in meta")]
    VersionNotFound(ModloaderVersion),
    #[error("Failed to download version meta")]
    DownloadVersionMetaFailed,
    #[error("Failed to download asset index meta")]
    DownloadAssetIndexMetaFailed,
    #[error("Failed to download asset index files")]
    DownloadAssetIndexFilesFailed,
    #[error("Failed to gather allowed libraries")]
    GatherAllowedLibrariesFailed,
    #[error("Failed to get jar client")]
    GetJarClientFailed,
    #[error("Failed to download files")]
    DownloadFilesFailed,
    #[error("Failed to extract natives")]
    ExtractNativesFailed,
}

impl ModLoaderError for VanillaError {}

#[derive(Debug)]
pub enum InstallStages {
    DownloadingAssets,
    DownloadingLibraries,
    ExtractingNatives,
}

#[derive(Debug)]
pub struct VanillaModLoader {
    mc_version: super::ModloaderVersion,
    instance_ref: Weak<RwLock<Instance>>,
}

#[async_trait]
impl ModLoaderHandler for VanillaModLoader {
    type Error = VanillaError;
    type Stages = InstallStages;

    fn new(mc_version: ModloaderVersion, instance_ref: Weak<RwLock<Instance>>) -> Self {
        VanillaModLoader {
            mc_version,
            instance_ref,
        }
    }
    async fn install(
        &self,
        progress_send: Sender<InstallProgress<InstallStages>>,
    ) -> Result<(), VanillaError> {
        let mc_version = &self.mc_version;
        // TODO: GET BASE_DIR FROM SOMEWHERE
        let base_dir = std::env::current_dir().unwrap().join("MC_TEST");

        let meta = McMeta::download_manifest_meta()
            .await
            .map_err(|_| VanillaError::DownloadManifestMetaFailed)?;

        let version_meta = meta
            .versions
            .iter()
            .find(|version| &version.id == mc_version)
            .ok_or_else(|| VanillaError::VersionNotFound(mc_version.clone()))?
            .get_version_meta(&base_dir)
            .await
            .map_err(|_| VanillaError::DownloadVersionMetaFailed)?;

        let mut downloads = vec![];

        let asset_index = version_meta
            .get_asset_index_meta(&base_dir)
            .await
            .map_err(|_| VanillaError::DownloadAssetIndexMetaFailed)?;

        let assets = asset_index
            .get_asset_files_downloadable(&base_dir)
            .await
            .map_err(|_| VanillaError::DownloadAssetIndexFilesFailed)?;
        downloads.extend(assets);

        let libs = version_meta
            .get_allowed_libraries_downloadable(&base_dir)
            .await
            .map_err(|_| VanillaError::GatherAllowedLibrariesFailed)?;
        downloads.extend(libs);

        let client = version_meta
            .get_jar_client_downloadable(&base_dir)
            .await
            .map_err(|_| VanillaError::GetJarClientFailed)?;
        downloads.push(client);

        let total_size = downloads
            .iter()
            .map(|download| download.size.unwrap_or(0))
            .sum::<u64>()
            / (1024 * 1024);

        // TODO: Map to InstallStages progress (?)
        let (progress, mut progress_handle) = tokio::sync::watch::channel(carbon_net::Progress {
            current_count: 0,
            current_size: 0,
        });
        let length = &downloads.len();

        let handle = tokio::spawn(async move {
            carbon_net::download_multiple(downloads, progress)
                .await
                .map_err(|_| VanillaError::DownloadFilesFailed)?;
            Ok::<_, VanillaError>(())
        });

        // let instance_ref = self.instance_ref.upgrade().unwrap();
        // let instance = instance_ref.read().await;

        while progress_handle.changed().await.is_ok() {
            trace!(
                "Progress: {} / {} - {} / {} MB",
                progress_handle.borrow().current_count,
                length - 1,
                progress_handle.borrow().current_size,
                total_size
            );
        }

        handle.await.expect("Join failed???")?;

        version_meta
            .extract_natives(&base_dir, mc_version)
            .await
            .map_err(|_| VanillaError::ExtractNativesFailed)?;

        Ok(())
    }
    fn remove(&self) -> Result<(), VanillaError> {
        Ok(())
    }
    fn verify(&self) -> Result<(), VanillaError> {
        Ok(())
    }
    fn get_version(&self) -> ModloaderVersion {
        self.mc_version.clone()
    }
}
