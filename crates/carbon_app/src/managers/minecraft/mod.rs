use carbon_net::Downloadable;
use daedalus::{
    minecraft::{DownloadType, Version, VersionInfo, VersionManifest},
    modded::Manifest,
};
use reqwest::Url;

use crate::domain::{
    java::JavaArch,
    minecraft::minecraft::{
        assets_index_into_vec_downloadable, libraries_into_vec_downloadable,
        version_download_into_downloadable,
    },
};

use self::minecraft::get_lwjgl_meta;

use super::ManagerRef;

pub mod assets;
pub mod curseforge;
pub mod fabric;
pub mod forge;
pub mod minecraft;
pub mod modrinth;
pub mod quilt;

pub(crate) struct MinecraftManager {
    pub meta_base_url: Url,
}

impl MinecraftManager {
    pub fn new() -> Self {
        Self {
            meta_base_url: Url::parse("https://meta.gdl.gg/").unwrap(),
        }
    }
}

impl ManagerRef<'_, MinecraftManager> {
    pub async fn get_minecraft_manifest(&self) -> anyhow::Result<VersionManifest> {
        minecraft::get_manifest(&self.app.reqwest_client, &self.meta_base_url).await
    }

    pub async fn get_minecraft_version(
        self,
        manifest_version_meta: Version,
    ) -> anyhow::Result<VersionInfo> {
        minecraft::get_version(&self.app.reqwest_client, manifest_version_meta).await
    }

    pub async fn get_forge_manifest(&self) -> anyhow::Result<Manifest> {
        forge::get_manifest(&self.app.reqwest_client, &self.meta_base_url).await
    }

    pub async fn get_fabric_manifest(&self) -> anyhow::Result<Manifest> {
        fabric::get_manifest(&self.app.reqwest_client, &self.meta_base_url).await
    }

    pub async fn get_quilt_manifest(&self) -> anyhow::Result<Manifest> {
        quilt::get_manifest(&self.app.reqwest_client, &self.meta_base_url).await
    }

    pub async fn get_all_version_info_files(
        self,
        version_info: VersionInfo,
        java_arch: &JavaArch,
    ) -> anyhow::Result<Vec<Downloadable>> {
        let runtime_path = &self.app.settings_manager().runtime_path;

        let version_id = version_info
            .inherits_from
            .as_ref()
            .unwrap_or(&version_info.id)
            .clone();

        let mut all_files = vec![];

        let lwjgl =
            get_lwjgl_meta(&self.app.reqwest_client, &version_info, &self.meta_base_url).await?;

        let tmp: Vec<_> = version_info
            .libraries
            .into_iter()
            .chain(lwjgl.libraries.into_iter())
            .collect();

        let libraries = libraries_into_vec_downloadable(
            &tmp,
            &runtime_path.get_libraries().to_path(),
            java_arch,
        );

        let client_main_jar = version_download_into_downloadable(
            version_info
                .downloads
                .get(&DownloadType::Client)
                .unwrap()
                .clone(),
            &version_id,
            runtime_path,
        );

        let assets = assets_index_into_vec_downloadable(
            assets::get_meta(
                self.app.reqwest_client.clone(),
                version_info.asset_index,
                runtime_path.get_assets().get_indexes_path(),
            )
            .await?,
            &runtime_path.get_assets(),
        );

        if let Some(logging_xml) = version_info.logging {
            if let Some(client) = logging_xml.get(&daedalus::minecraft::LoggingConfigName::Client) {
                all_files.push(
                    Downloadable::new(
                        client.file.url.clone(),
                        runtime_path
                            .get_logging_configs()
                            .get_client_path(&client.file.id),
                    )
                    .with_size(client.file.size as u64)
                    .with_checksum(Some(carbon_net::Checksum::Sha1(client.file.sha1.clone()))),
                );
            }
        }

        all_files.push(client_main_jar);
        all_files.extend(libraries);
        all_files.extend(assets);

        Ok(all_files)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UpdateValue<T: Copy + Clone + Eq>(pub T);

impl<T: Copy + Clone + Eq> UpdateValue<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn set(&mut self, value: T) {
        self.0 = value;
    }

    pub fn get(&self) -> &T {
        &self.0
    }

    pub fn update_from(&mut self, from: &Self, update_callback: impl FnOnce(T)) {
        if self.0 != from.0 {
            self.0 = from.0;
            update_callback(self.0);
        }
    }
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use carbon_net::Progress;
    use chrono::Utc;

    use crate::managers::{
        account::{FullAccount, FullAccountType},
        java::java_checker::{JavaChecker, RealJavaChecker},
        minecraft::{
            forge::execute_processors,
            minecraft::{extract_natives, get_lwjgl_meta, launch_minecraft},
        },
    };

    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 12)]
    async fn test_download_minecraft() {
        let version = "1.20.1";

        let app = crate::setup_managers_for_test().await;

        let java_component = RealJavaChecker
            .get_bin_info(
                &PathBuf::from("java"),
                crate::domain::java::JavaComponentType::Local,
            )
            .await
            .unwrap();

        let runtime_path = &app.app.settings_manager().runtime_path;
        let instance_path = runtime_path.get_instances().get_instance_path("test");

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
            .find(|v| v.id == version)
            .unwrap()
            .clone();

        let version_info = crate::managers::minecraft::minecraft::get_version(
            &app.reqwest_client,
            manifest_version,
        )
        .await
        .unwrap();

        let lwjgl_group = get_lwjgl_meta(
            &reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build(),
            &version_info,
            &app.minecraft_manager().meta_base_url,
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
        .find(|v| v.id == version)
        .unwrap()
        .loaders[0]
            .clone();

        let forge_version_info =
            crate::managers::minecraft::forge::get_version(&app.reqwest_client, forge_manifest)
                .await
                .unwrap();

        let version_info =
            daedalus::modded::merge_partial_version(forge_version_info, version_info);

        // -----FORGE

        let progress = tokio::sync::watch::channel(Progress::new());

        // tokio::spawn(async move {
        //     while progress.1.changed().await.is_ok() {
        //         let progress_value = progress.1.borrow().clone();
        //         println!("{:?}", progress_value);
        //     }
        // });

        let vanilla_files = app
            .minecraft_manager()
            .get_all_version_info_files(version_info.clone(), &java_component.arch)
            .await
            .unwrap();

        carbon_net::download_multiple(vanilla_files, progress.0, 10)
            .await
            .unwrap();

        extract_natives(
            runtime_path,
            &version_info,
            &lwjgl_group,
            &java_component.arch,
        )
        .await
        .unwrap();

        let libraries_path = runtime_path.get_libraries();
        let game_version = version_info.id.to_string();
        let client_path = runtime_path.get_libraries().get_mc_client(
            version_info
                .inherits_from
                .as_ref()
                .unwrap_or(&version_info.id),
        );

        if let Some(processors) = &version_info.processors {
            execute_processors(
                processors,
                version_info
                    .data
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Data entries missing"))
                    .unwrap(),
                PathBuf::from(java_component.path.clone()),
                instance_path.clone(),
                client_path,
                game_version,
                libraries_path,
                None,
            )
            .await
            .unwrap();
        }

        let full_account = FullAccount {
            username: "test".to_owned(),
            uuid: "test-uuid".to_owned(),
            type_: FullAccountType::Offline,
            last_used: Utc::now().into(),
        };

        let mut child = launch_minecraft(
            java_component,
            full_account,
            2048,
            2048,
            "",
            runtime_path,
            version_info,
            &lwjgl_group,
            instance_path,
        )
        .await
        .unwrap();

        // intercept stdout
        let stdout = child.stdout.take().unwrap();
        let mut reader = tokio::io::BufReader::new(stdout);

        let stderr = child.stderr.take().unwrap();
        let mut reader_err = tokio::io::BufReader::new(stderr);

        tokio::spawn(async move {
            tokio::io::copy(&mut reader, &mut tokio::io::stdout())
                .await
                .unwrap();

            tokio::io::copy(&mut reader_err, &mut tokio::io::stderr())
                .await
                .unwrap();
        });

        let res = child.wait().await.unwrap();

        assert!(res.success());
    }
}
