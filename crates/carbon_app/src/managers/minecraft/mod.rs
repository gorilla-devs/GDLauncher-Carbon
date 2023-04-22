use std::path::PathBuf;

use anyhow::anyhow;
use carbon_net::{Downloadable, IntoDownloadable, IntoVecDownloadable, Progress};
use reqwest::Url;

use crate::domain::minecraft::{
    minecraft::{ManifestVersion, MinecraftManifest, VersionInfo},
    modded::ModdedManifest,
};

use self::forge::execute_processors;

use super::ManagerRef;

mod assets;
mod forge;
mod minecraft;

pub(crate) struct MinecraftManager {
    pub meta_base_url: Url,
}

impl MinecraftManager {
    pub fn new() -> Self {
        Self {
            meta_base_url: Url::parse("https://meta.gdlauncher.com/").unwrap(),
        }
    }
}

impl ManagerRef<'_, MinecraftManager> {
    pub async fn get_minecraft_manifest(&self) -> anyhow::Result<MinecraftManifest> {
        minecraft::get_manifest(&self.app.reqwest_client, &self.meta_base_url).await
    }

    pub async fn get_minecraft_version(
        &self,
        manifest_version_meta: ManifestVersion,
    ) -> anyhow::Result<VersionInfo> {
        minecraft::get_version(&self.app.reqwest_client, manifest_version_meta).await
    }

    pub async fn get_forge_manifest(&self) -> anyhow::Result<ModdedManifest> {
        forge::get_manifest(&self.app.reqwest_client, &self.meta_base_url).await
    }

    pub async fn download_minecraft(
        self,
        version_info: VersionInfo,
        progress: tokio::sync::watch::Sender<Progress>,
    ) -> anyhow::Result<()> {
        let runtime_path = &self.app.settings_manager().runtime_path;

        let mut all_files = vec![];

        let libraries = version_info
            .libraries
            .into_vec_downloadable(&runtime_path.get_libraries().to_path());

        let client_main_jar = version_info
            .downloads
            .unwrap()
            .client
            .into_downloadable(&runtime_path.get_versions().get_clients_path());

        let assets = assets::get_meta(
            self.app.reqwest_client.clone(),
            version_info.asset_index,
            runtime_path.get_assets().get_indexes_path(),
        )
        .await?
        .into_vec_downloadable(&runtime_path.get_assets().to_path());

        all_files.push(client_main_jar);
        all_files.extend(libraries);
        all_files.extend(assets);

        carbon_net::download_multiple(all_files, progress).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use carbon_net::Progress;
    use chrono::Utc;

    use crate::{
        domain::minecraft::modded::merge_partial_version,
        managers::{
            account::{FullAccount, FullAccountType},
            minecraft::{
                forge::execute_processors,
                minecraft::{extract_natives, launch_minecraft},
            },
        },
    };

    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 12)]
    async fn test_download_minecraft() {
        let app = crate::setup_managers_for_test().await;

        let runtime_path = &app.app.settings_manager().runtime_path;
        let instance_path = runtime_path
            .get_instances()
            .get_instance_path("test".to_owned());

        std::fs::create_dir_all(instance_path.get_root()).unwrap();

        let manifest = crate::managers::minecraft::minecraft::get_manifest(
            &app.reqwest_client,
            &app.minecraft_manager.meta_base_url,
        )
        .await
        .unwrap();

        let manifest_version = manifest
            .versions
            .iter()
            .find(|v| v.id == "1.16.5")
            .unwrap()
            .clone();

        let version_info = crate::managers::minecraft::minecraft::get_version(
            &app.reqwest_client,
            manifest_version,
        )
        .await
        .unwrap();

        // Uncomment for FORGE
        // -----FORGE

        let forge_manifest = crate::managers::minecraft::forge::get_manifest(
            &app.reqwest_client.clone(),
            &app.minecraft_manager.meta_base_url,
        )
        .await
        .unwrap()
        .game_versions
        .into_iter()
        .find(|v| v.id == "1.16.5")
        .unwrap()
        .loaders[0]
            .clone();

        let forge_version_info =
            crate::managers::minecraft::forge::get_version(&app.reqwest_client, forge_manifest)
                .await
                .unwrap();

        let version_info = merge_partial_version(forge_version_info, version_info);

        // -----FORGE

        let progress = tokio::sync::watch::channel(Progress::new());

        app.minecraft_manager()
            .download_minecraft(version_info.clone(), progress.0)
            .await
            .unwrap();

        extract_natives(runtime_path, &version_info).await;

        let libraries_path = runtime_path.get_libraries();
        let game_version = version_info
            .inherits_from
            .as_ref()
            .unwrap_or(&version_info.id)
            .to_string();
        let client_path = runtime_path
            .get_versions()
            .get_clients_path()
            .join(format!("{}.jar", game_version));

        if let Some(processors) = &version_info.processors {
            let _ = execute_processors(
                processors,
                version_info
                    .data
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Data entries missing"))
                    .unwrap(),
                PathBuf::from("java"),
                instance_path.clone(),
                client_path,
                game_version,
                libraries_path,
            )
            .await;
        }

        let full_account = FullAccount {
            username: "test".to_owned(),
            uuid: "test-uuid".to_owned(),
            type_: FullAccountType::Offline,
            last_used: Utc::now().into(),
        };

        let mut child = launch_minecraft(
            PathBuf::from("java"),
            full_account,
            2048_u16,
            2048_u16,
            runtime_path,
            version_info,
            instance_path,
        )
        .await
        .unwrap();

        let _ = child.wait().await;

        // assert!(status.success());
    }
}
