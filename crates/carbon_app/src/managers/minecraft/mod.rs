use super::{AppRef, Managers};
use crate::db::{minecraft_manifest::SetParam, PrismaClient};
use crate::managers::ManagersInner;
use carbon_domain::minecraft::manifest::{ManifestVersion, MinecraftManifest};
use carbon_domain::minecraft::version::Library;
use carbon_net::{Downloadable, IntoDownloadable, IntoVecDownloadable};
use rspc::ErrorCode;
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use thiserror::Error;

mod assets;
mod manifest;
mod version;

#[derive(Error, Debug)]
pub enum MinecraftError {
    #[error("Assets error {0}")]
    AssetsError(#[from] assets::AssetsError),
    #[error("Manifest error {0}")]
    ManifestError(#[from] manifest::ManifestError),
    #[error("Version error {0}")]
    VersionError(#[from] version::VersionError),
    #[error("Minecraft version not found")]
    MinecraftVersionNotFound,
    #[error("Libraries not found")]
    LibrariesNotFound,
}

impl From<MinecraftError> for rspc::Error {
    fn from(value: MinecraftError) -> Self {
        rspc::Error::new(
            ErrorCode::InternalServerError,
            format!("Minecraft Error: {}", value),
        )
    }
}

pub(crate) struct MinecraftManager {
    app: AppRef,
}

impl MinecraftManager {
    pub fn new() -> Self {
        Self {
            app: AppRef::uninit(),
        }
    }

    pub fn get_appref(&self) -> &AppRef {
        &self.app
    }

    pub async fn get_minecraft_versions(&self) -> Vec<ManifestVersion> {
        manifest::get_meta(self.app.upgrade().persistence_manager.get_db_client().await)
            .await
            .unwrap()
    }

    pub async fn get_game_download_files_list(
        &self,
        mc_version: String,
    ) -> Result<Vec<String>, MinecraftError> {
        let db_client = self.app.upgrade().persistence_manager.get_db_client().await;

        let manifest = manifest::get_meta(db_client.clone()).await?;

        let manifest_version = manifest
            .iter()
            .find(|v| v.id == mc_version)
            .ok_or(MinecraftError::MinecraftVersionNotFound)?;

        let version = version::get_meta(db_client.clone(), manifest_version.clone()).await?;

        let mut all_files = vec![];

        let libraries = version
            .libraries
            .ok_or(MinecraftError::LibrariesNotFound)?
            .into_vec_downloadable(&std::env::current_dir().unwrap());

        let client_main_jar = version
            .downloads
            .unwrap()
            .client
            .into_downloadable(&std::env::current_dir().unwrap());

        let assets = assets::get_meta(db_client.clone(), version.asset_index.unwrap())
            .await?
            .into_vec_downloadable(&std::env::current_dir().unwrap());

        all_files.push(client_main_jar);
        all_files.extend(libraries);
        all_files.extend(assets);

        Ok(all_files
            .into_iter()
            .map(|file| format!("{} | {}", file.url, file.path.display()))
            .collect())
    }
}
