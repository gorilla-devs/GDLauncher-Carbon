use anyhow::anyhow;
use carbon_net::{Downloadable, IntoDownloadable, IntoVecDownloadable};

use crate::domain::minecraft::manifest::ManifestVersion;

use super::ManagerRef;

mod assets;
mod manifest;
mod version;

pub const MC_MANIFEST_META_URL: &str = "https://meta.modrinth.com/minecraft/v0/manifest.json";
pub const FORGE_MANIFEST_META_URL: &str = "https://meta.modrinth.com/forge/v0/manifest.json";
pub const FABRIC_MANIFEST_META_URL: &str = "https://meta.modrinth.com/fabric/v0/manifest.json";

pub(crate) struct MinecraftManager {}

impl MinecraftManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl ManagerRef<'_, MinecraftManager> {
    pub async fn get_minecraft_versions(&self) -> Vec<ManifestVersion> {
        manifest::get_manifest_meta(self.app.reqwest_client.clone())
            .await
            .unwrap()
    }

    pub async fn get_game_download_files_list(
        self,
        mc_version: String,
    ) -> anyhow::Result<Vec<Downloadable>> {
        let runtime_path = &self.app.settings_manager().runtime_path;

        let manifest = manifest::get_manifest_meta(self.app.reqwest_client.clone()).await?;

        let manifest_version = manifest
            .iter()
            .find(|v| v.id == mc_version)
            .ok_or(anyhow!("Minecraft version not found"))?;

        let version = version::get_version_meta(
            self.app.reqwest_client.clone(),
            manifest_version.clone(),
            runtime_path.get_versions().get_clients_path(),
        )
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
        minecraft::version::generate_startup_command,
    };

    // #[tokio::test(flavor = "multi_thread", worker_threads = 12)]
    // async fn test_download_minecraft() {
    //     use crate::managers::minecraft::version::extract_natives;
    //     use crate::domain::minecraft::manifest::MinecraftManifest;
    //     use carbon_net::Progress;

    //     let app = crate::setup_managers_for_test().await;

    //     let runtime_path = &app.configuration_manager().runtime_path;

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

    //     let mut command_exec = tokio::process::Command::new("java");

    //     println!("Command: {:#?}", command);

    //     let mut child = command_exec
    //         .args(command.split_ascii_whitespace())
    //         .spawn()
    //         .unwrap();

    //     let status = child.wait().await.unwrap();

    //     assert!(status.success());
    // }
}
