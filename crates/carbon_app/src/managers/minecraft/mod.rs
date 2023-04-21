use anyhow::anyhow;
use carbon_net::{Downloadable, IntoDownloadable, IntoVecDownloadable};
use reqwest::Url;

use crate::domain::minecraft::{
    minecraft::{ManifestVersion, MinecraftManifest, VersionInfo},
    modded::ModdedManifest,
};

use super::ManagerRef;

mod assets;
mod forge;
mod minecraft;

pub(crate) struct MinecraftManager {
    meta_base_url: Url,
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
        minecraft::get_manifest(self.app.reqwest_client.clone(), &self.meta_base_url).await
    }

    pub async fn get_minecraft_version(
        &self,
        manifest_version_meta: ManifestVersion,
    ) -> anyhow::Result<VersionInfo> {
        minecraft::get_version(self.app.reqwest_client.clone(), manifest_version_meta).await
    }

    pub async fn get_forge_manifest(&self) -> anyhow::Result<ModdedManifest> {
        forge::get_manifest(self.app.reqwest_client.clone(), &self.meta_base_url).await
    }

    pub async fn get_game_download_files_list(
        self,
        mc_version: String,
    ) -> anyhow::Result<Vec<Downloadable>> {
        let runtime_path = &self.app.settings_manager().runtime_path;

        let manifest = minecraft::get_manifest(
            self.app.reqwest_client.clone(),
            &Url::parse("https://meta.gdlauncher.com/").unwrap(),
        )
        .await?;

        let manifest_version = manifest
            .versions
            .iter()
            .find(|v| v.id == mc_version)
            .ok_or(anyhow!("Minecraft version not found"))?;

        let version =
            minecraft::get_version(self.app.reqwest_client.clone(), manifest_version.clone())
                .await?;

        let mut all_files = vec![];

        let libraries = version
            .libraries
            .into_vec_downloadable(&runtime_path.get_libraries().to_path());

        let client_main_jar = version
            .downloads
            .unwrap()
            .client
            .into_downloadable(&runtime_path.get_versions().get_clients_path());

        let assets = assets::get_meta(
            self.app.reqwest_client.clone(),
            version.asset_index,
            runtime_path.get_assets().get_indexes_path(),
        )
        .await?
        .into_vec_downloadable(&runtime_path.get_assets().to_path());

        all_files.push(client_main_jar);
        all_files.extend(libraries);
        all_files.extend(assets);

        Ok(all_files)
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use chrono::Utc;

    use crate::managers::{
        account::{FullAccount, FullAccountType},
        minecraft::minecraft::generate_startup_command,
    };

    // #[tokio::test(flavor = "multi_thread", worker_threads = 12)]
    // async fn test_download_minecraft() {
    //     use crate::managers::minecraft::version::extract_natives;
    //     use carbon_domain::minecraft::manifest::MinecraftManifest;
    //     use carbon_net::Progress;

    //     let app = crate::setup_managers_for_test().await;

    //     let runtime_path = &app.app.settings_manager().runtime_path;

    //     let manifest = MinecraftManifest::fetch().await.unwrap();

    //     let version = manifest
    //         .versions
    //         .into_iter()
    //         .find(|v| v.id == "1.16.5")
    //         .unwrap()
    //         .fetch()
    //         .await
    //         .unwrap();

    //     // Move all metas to json instead of db
    //     // Download assets json to assets/

    //     let files = app
    //         .minecraft_manager()
    //         .get_game_download_files_list("1.16.5".to_string())
    //         .await
    //         .unwrap();

    //     let progress = tokio::sync::watch::channel(Progress::new());

    //     tokio::spawn(async move {
    //         let mut rec = progress.1.clone();
    //         while let Ok(p) = rec.changed().await {
    //             println!("Progress: {:#?}", p);
    //         }
    //     });

    //     carbon_net::download_multiple(files, progress.0)
    //         .await
    //         .unwrap();

    //     extract_natives(runtime_path, &version).await;

    //     let full_account = FullAccount {
    //         username: "test".to_owned(),
    //         uuid: "test-uuid".to_owned(),
    //         type_: FullAccountType::Offline,
    //         last_used: Utc::now().into(),
    //     };

    //     let instance_id = "something";

    //     let instance_path = runtime_path
    //         .get_instances()
    //         .get_instance_path(instance_id.to_owned())
    //         .get_root();

    //     tokio::fs::create_dir_all(&instance_path).await.unwrap();

    //     let command =
    //         generate_startup_command(full_account, 2048, 2048, runtime_path, version, instance_id)
    //             .await;

    //     for c in command.iter() {
    //         println!("Command: {}", c);
    //     }

    //     let mut command_exec = tokio::process::Command::new("java");

    //     let child = command_exec.args(command);

    //     println!("Command: {:#?}", child);

    //     let mut child = child.spawn().unwrap();

    //     let _ = child.wait().await;

    //     // assert!(status.success());
    // }
}
