use super::modpack::TSubtasks;
use crate::api::translation::Translation;
use crate::domain::instance::info::{
    self, Instance, JavaOverride, Modpack, ModpackInfo, StandardVersion,
};
use crate::domain::instance::{self as domain, GameLogId, InstanceId};
use crate::domain::java::{JavaComponent, JavaComponentType, SystemJavaProfileName};
use crate::domain::metrics::GDLMetricsEvent;
use crate::domain::vtask::VisualTaskId;
use crate::managers::instance::log::GameLog;
use crate::managers::instance::modpack::{packinfo, PackVersionFile};
use crate::managers::instance::schema::make_instance_config;
use crate::managers::java::java_checker::{JavaChecker, RealJavaChecker};
use crate::managers::java::managed::Step;
use crate::managers::minecraft::assets::{get_assets_dir, AssetsDir};
use crate::managers::minecraft::modrinth;
use crate::managers::modplatforms::curseforge::convert_cf_version_to_standard_version;
use crate::managers::modplatforms::modrinth::convert_mr_version_to_standard_version;
use crate::managers::vtask::Subtask;
use crate::managers::AppInner;
use crate::{
    domain::instance::info::{GameVersion, ModLoader, ModLoaderType},
    managers::{
        self,
        account::FullAccount,
        vtask::{NonFailedDismissError, TaskState, VisualTask},
        ManagerRef,
    },
};
use anyhow::{anyhow, bail, Context};
use carbon_net::{DownloadOptions, Downloadable};
use carbon_parsing::log::{LogParser, ParsedItem};
use chrono::{DateTime, Local, Utc};
use daedalus::minecraft::{LibraryGroup, VersionInfo};
use futures::Future;
use md5::{Digest, Md5};
use std::collections::HashSet;
use std::fmt::Debug;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{watch, Mutex, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::{io::AsyncReadExt, sync::mpsc};
use tracing::{debug, info, trace};

pub async fn process_minecraft(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    deep_check: bool,
    instance_shortpath: String,
    t_subtasks: &TSubtasks,
    mut version_info: VersionInfo,
    version: &StandardVersion,
    java: &JavaComponent,
    log: &watch::Sender<GameLog>,
    file: Option<&mut File>,
    downloads: &mut Vec<Downloadable>,
) -> anyhow::Result<(LibraryGroup, AssetsDir, VersionInfo)> {
    let runtime_path = app.settings_manager().runtime_path.clone();
    let instance_path = runtime_path
        .get_instances()
        .get_instance_path(&instance_shortpath);

    let instance_root = instance_path.get_root();
    let setup_path = instance_root.join(".setup");
    let is_setup = setup_path.is_dir();
    let is_modpack_complete = setup_path.join("modpack-complete").exists();

    t_subtasks.t_request_modloader_info.start_opaque();

    let dummy_string = daedalus::BRANDING
        .get_or_init(daedalus::Branding::default)
        .dummy_replace_string
        .clone();

    for modloader in version.modloaders.iter() {
        match modloader {
            ModLoader {
                type_: ModLoaderType::Forge,
                version: forge_version,
            } => {
                if forge_version.is_empty() {
                    anyhow::bail!("Forge version is empty");
                }

                let forge_version = crate::managers::minecraft::forge::get_version(
                    app.prisma_client.clone(),
                    &app.reqwest_client,
                    &*forge_version,
                    &app.minecraft_manager().meta_base_url,
                )
                .await?;

                version_info = daedalus::modded::merge_partial_version(forge_version, version_info);
            }
            ModLoader {
                type_: ModLoaderType::Neoforge,
                version: neoforge_version,
            } => {
                if neoforge_version.is_empty() {
                    anyhow::bail!("Neoforge version is empty");
                }

                let neoforge_version = crate::managers::minecraft::neoforge::get_version(
                    app.prisma_client.clone(),
                    &app.reqwest_client,
                    &*neoforge_version,
                    &app.minecraft_manager().meta_base_url,
                )
                .await?;

                version_info =
                    daedalus::modded::merge_partial_version(neoforge_version, version_info);
            }
            ModLoader {
                type_: ModLoaderType::Fabric,
                version: fabric_version,
            } => {
                if fabric_version.is_empty() {
                    anyhow::bail!("Fabric version is empty");
                }

                let fabric_version = crate::managers::minecraft::fabric::replace_template(
                    &crate::managers::minecraft::fabric::get_version(
                        app.prisma_client.clone(),
                        &app.reqwest_client,
                        &fabric_version,
                        &app.minecraft_manager().meta_base_url,
                    )
                    .await?,
                    &version.release,
                    &dummy_string,
                );

                version_info =
                    daedalus::modded::merge_partial_version(fabric_version, version_info);
            }
            ModLoader {
                type_: ModLoaderType::Quilt,
                version: quilt_version,
            } => {
                if quilt_version.is_empty() {
                    anyhow::bail!("Quilt version is empty");
                }

                let quilt_version = crate::managers::minecraft::quilt::replace_template(
                    &crate::managers::minecraft::quilt::get_version(
                        app.prisma_client.clone(),
                        &app.reqwest_client,
                        &quilt_version,
                        &app.minecraft_manager().meta_base_url,
                    )
                    .await?,
                    &version.release,
                    &dummy_string,
                );

                version_info = daedalus::modded::merge_partial_version(quilt_version, version_info);
            }
        }
    }

    t_subtasks.t_request_modloader_info.complete_opaque();

    t_subtasks.t_request_minecraft_files.start_opaque();

    let (lwjgl_group, version_files) = app
        .minecraft_manager()
        .get_all_version_info_files(version_info.clone(), &java.arch, &log, file)
        .await?;

    downloads.extend(version_files);

    t_subtasks.t_request_minecraft_files.complete_opaque();

    let concurrency = app
        .settings_manager()
        .get_settings()
        .await?
        .concurrent_downloads;

    let (progress_watch_tx, mut progress_watch_rx) =
        tokio::sync::watch::channel(carbon_net::Progress::new());

    let t_subtasks_clone = Arc::clone(t_subtasks);

    t_subtasks.t_scan_files.start_opaque();
    let completion = tokio::spawn(async move {
        while progress_watch_rx.changed().await.is_ok() {
            {
                let progress = progress_watch_rx.borrow();
                t_subtasks_clone.t_scan_files.update_download(
                    progress.current_size as u32,
                    progress.total_size as u32,
                    false,
                );
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        t_subtasks_clone.t_scan_files.complete_opaque();
    });

    let download_required = carbon_net::download_multiple(
        &downloads[..],
        DownloadOptions::builder()
            .concurrency(concurrency as usize)
            .progress_sender(progress_watch_tx)
            .deep_check(deep_check)
            .only_validate(true)
            .build(),
    )
    .await
    .with_context(|| format!("Failed to verify instance files for instance {instance_id}"))?;

    completion.await?;

    if download_required {
        let instance_manager = app.instance_manager();
        let _lock = instance_manager
            .persistence_manager
            .instance_download_lock
            .acquire()
            .await
            .unwrap();

        let (progress_watch_tx, mut progress_watch_rx) =
            tokio::sync::watch::channel(carbon_net::Progress::new());

        let t_subtasks_clone = Arc::clone(t_subtasks);

        t_subtasks.t_download_files.start_opaque();
        let completion = tokio::spawn(async move {
            while progress_watch_rx.changed().await.is_ok() {
                {
                    let progress = progress_watch_rx.borrow();
                    t_subtasks_clone.t_download_files.update_download(
                        progress.current_size as u32,
                        progress.total_size as u32,
                        false,
                    );
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }

            t_subtasks_clone.t_download_files.complete_opaque();
        });

        carbon_net::download_multiple(
            &downloads[..],
            DownloadOptions::builder()
                .concurrency(concurrency as usize)
                .deep_check(deep_check)
                .progress_sender(progress_watch_tx)
                .build(),
        )
        .await
        .with_context(|| format!("Failed to download instance files for instance {instance_id}"))?;

        completion.await?;
    }

    if !download_required {
        t_subtasks.t_extract_natives.set_weight(10.0);
        t_subtasks.t_reconstruct_assets.set_weight(10.0);
    }

    t_subtasks.t_extract_natives.start_opaque();
    managers::minecraft::minecraft::extract_natives(
        &runtime_path,
        &version_info,
        &lwjgl_group,
        &java.arch,
    )
    .await?;
    t_subtasks.t_extract_natives.complete_opaque();

    t_subtasks.t_reconstruct_assets.start_opaque();
    managers::minecraft::assets::reconstruct_assets(
        Arc::clone(&app.prisma_client),
        app.reqwest_client.clone(),
        &version_info.asset_index,
        runtime_path.get_assets(),
        instance_path.get_resources_path(),
    )
    .await?;
    t_subtasks.t_reconstruct_assets.complete_opaque();

    let libraries_path = runtime_path.get_libraries();
    let game_version = version_info.id.to_string();
    let client_path = runtime_path.get_libraries().get_mc_client(
        version_info
            .inherits_from
            .as_ref()
            .unwrap_or(&version_info.id),
    );
    let assets_dir = get_assets_dir(
        app.prisma_client.clone(),
        app.reqwest_client.clone(),
        &version_info.asset_index,
        runtime_path.get_assets(),
        instance_path.get_resources_path(),
    )
    .await
    .unwrap();

    for modloader in version.modloaders.iter() {
        let instance_path = instance_path.clone();
        let client_path = client_path.clone();
        let game_version = game_version.clone();
        let libraries_path = libraries_path.clone();

        match modloader {
            ModLoader {
                type_: ModLoaderType::Forge,
                ..
            } => {
                if let Some(t_forge_processors) = &t_subtasks.t_forge_processors {
                    t_forge_processors.start_opaque();

                    let instance_manager = app.instance_manager();
                    let _lock = instance_manager
                        .persistence_manager
                        .loader_install_lock
                        .acquire()
                        .await
                        .unwrap();

                    if let Some(processors) = &version_info.processors {
                        managers::minecraft::forge::execute_processors(
                            processors,
                            version_info
                                .data
                                .as_ref()
                                .ok_or_else(|| anyhow::anyhow!("Data entries missing"))?,
                            PathBuf::from(&java.path),
                            instance_path,
                            client_path,
                            game_version,
                            libraries_path,
                            Some(Box::new(|current, total| {
                                t_forge_processors.update_items(current, total);
                            })),
                        )
                        .await?;
                    }

                    t_forge_processors.complete_opaque();
                }
            }
            ModLoader {
                type_: ModLoaderType::Neoforge,
                ..
            } => {
                if let Some(t_neoforge_processors) = &t_subtasks.t_neoforge_processors {
                    t_neoforge_processors.start_opaque();

                    let instance_manager = app.instance_manager();
                    let _lock = instance_manager
                        .persistence_manager
                        .loader_install_lock
                        .acquire()
                        .await
                        .unwrap();

                    if let Some(processors) = &version_info.processors {
                        managers::minecraft::neoforge::execute_processors(
                            processors,
                            version_info
                                .data
                                .as_ref()
                                .ok_or_else(|| anyhow::anyhow!("Data entries missing"))?,
                            PathBuf::from(&java.path),
                            instance_path.clone(),
                            client_path,
                            game_version,
                            libraries_path,
                            Some(Box::new(|current, total| {
                                t_neoforge_processors.update_items(current, total);
                            })),
                        )
                        .await?;
                    }

                    t_neoforge_processors.complete_opaque();
                }
            }
            _ => {}
        }
    }

    Ok((lwjgl_group, assets_dir, version_info))
}
