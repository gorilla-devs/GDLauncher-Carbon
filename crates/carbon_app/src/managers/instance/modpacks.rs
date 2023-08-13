//
// Modpack prep

use std::{sync::Arc, time::Duration};

use carbon_net::Downloadable;

use crate::{
    domain::{
        instance::info::{CurseforgeModpack, Modpack, ModrinthModpack, StandardVersion},
        modplatforms::{curseforge::filters::ModFileParameters, modrinth::search::VersionID},
        runtime_path::InstancePath,
    },
    managers::{
        minecraft::{curseforge, modrinth},
        vtask::Subtask,
        AppInner,
    },
};

pub enum CurseforgeInstallSource {
    Remote { project_id: i32, file_id: i32 },
    Local { archive_path: String },
}

pub enum ModrinthInstallSource {
    Remote {
        project_id: String,
        version_id: String,
    },
    Local {
        mrpack_path: String,
    },
}

pub struct PrepareModpackSubtasks {
    pub t_request: Subtask,
    pub t_extract_files: Subtask,
    pub t_download_files: Subtask,
    pub t_addon_metadata: Subtask,
}

#[async_trait::async_trait]
pub trait PrepareModpack {
    async fn prepare_modpack(
        &self,
        app: Arc<AppInner>,
        instance_path: InstancePath,
        downloads: &mut Vec<Downloadable>,
        subtasks: PrepareModpackSubtasks,
    ) -> anyhow::Result<StandardVersion>;
}

#[async_trait::async_trait]
impl PrepareModpack for CurseforgeModpack {
    async fn prepare_modpack(
        &self,
        app: Arc<AppInner>,
        instance_path: InstancePath,
        downloads: &mut Vec<Downloadable>,
        subtasks: PrepareModpackSubtasks,
    ) -> anyhow::Result<StandardVersion> {
        tracing::info!("Preparing curseforge modpack `{:?}`", self);
        let t_request = subtasks.t_request;
        let t_extract_files = subtasks.t_extract_files;
        let t_download_files = subtasks.t_download_files;
        let t_addon_metadata = subtasks.t_addon_metadata;
        let install_source = match self {
            CurseforgeModpack::RemoteManaged {
                project_id,
                file_id,
            } => CurseforgeInstallSource::Remote {
                project_id: *project_id,
                file_id: *file_id,
            },
            CurseforgeModpack::LocalManaged {
                project_id: _,
                file_id: _,
                archive_path,
            } => CurseforgeInstallSource::Local {
                archive_path: archive_path.clone(),
            },
            CurseforgeModpack::Unmanaged { archive_path } => CurseforgeInstallSource::Local {
                archive_path: archive_path.clone(),
            },
        };

        let (modpack_progress_tx, mut modpack_progress_rx) =
            tokio::sync::watch::channel(curseforge::ProgressState::new());

        tokio::spawn(async move {
            let mut tracker = curseforge::ProgressState::new();

            while modpack_progress_rx.changed().await.is_ok() {
                {
                    let progress = modpack_progress_rx.borrow();

                    tracker.download_addon_zip.update_from(
                        &progress.download_addon_zip,
                        |(downloaded, total)| {
                            t_download_files.update_download(downloaded as u32, total as u32, true);
                        },
                    );

                    tracker.extract_addon_overrides.update_from(
                        &progress.extract_addon_overrides,
                        |(completed, total)| {
                            t_extract_files.update_items(completed as u32, total as u32);
                        },
                    );

                    tracker.acquire_addon_metadata.update_from(
                        &progress.acquire_addon_metadata,
                        |(completed, total)| {
                            t_addon_metadata.update_items(completed as u32, total as u32);
                        },
                    );
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        let modpack_info = match install_source {
            CurseforgeInstallSource::Remote {
                project_id,
                file_id,
            } => {
                tracing::info!("Curseforge Remote install");
                t_request.start_opaque();
                let file = app
                    .modplatforms_manager()
                    .curseforge
                    .get_mod_file(ModFileParameters {
                        file_id,
                        mod_id: project_id,
                    })
                    .await?
                    .data;
                t_request.complete_opaque();

                curseforge::prepare_modpack_from_addon(
                    &app,
                    &file,
                    instance_path.clone(),
                    modpack_progress_tx,
                )
                .await?
            }
            CurseforgeInstallSource::Local { archive_path } => {
                tracing::info!("Curseforge Local install");
                let file_size = std::fs::metadata(&archive_path)?.len();
                // show pack as already downloaded
                modpack_progress_tx.send_modify(|progress| {
                    progress.download_addon_zip.set((file_size, file_size))
                });

                curseforge::prepare_modpack_from_zip(
                    &app,
                    archive_path.into(),
                    instance_path.clone(),
                    false,
                    modpack_progress_tx,
                )
                .await?
            }
        };

        downloads.extend(modpack_info.downloadables);

        modpack_info.manifest.minecraft.try_into()
    }
}

#[async_trait::async_trait]
impl PrepareModpack for ModrinthModpack {
    async fn prepare_modpack(
        &self,
        app: Arc<AppInner>,
        instance_path: InstancePath,
        downloads: &mut Vec<Downloadable>,
        subtasks: PrepareModpackSubtasks,
    ) -> anyhow::Result<StandardVersion> {
        tracing::info!("Preparing modrinth modpack `{:?}`", self);
        let t_request = subtasks.t_request;
        let t_extract_files = subtasks.t_extract_files;
        let t_download_files = subtasks.t_download_files;
        let t_addon_metadata = subtasks.t_addon_metadata;
        let install_source = match self {
            ModrinthModpack::RemoteManaged {
                project_id,
                version_id,
            } => ModrinthInstallSource::Remote {
                project_id: project_id.clone(),
                version_id: version_id.clone(),
            },
            ModrinthModpack::LocalManaged {
                project_id: _,
                version_id: _,
                mrpack_path,
            } => ModrinthInstallSource::Local {
                mrpack_path: mrpack_path.clone(),
            },
            ModrinthModpack::Unmanaged { mrpack_path } => ModrinthInstallSource::Local {
                mrpack_path: mrpack_path.clone(),
            },
        };

        let (modpack_progress_tx, mut modpack_progress_rx) =
            tokio::sync::watch::channel(modrinth::ProgressState::Idle);

        tokio::spawn(async move {
            while modpack_progress_rx.changed().await.is_ok() {
                {
                    let progress = modpack_progress_rx.borrow();
                    match *progress {
                        modrinth::ProgressState::Idle => {}
                        modrinth::ProgressState::DownloadingMRPack(downloaded, total) => {
                            t_download_files.update_download(downloaded as u32, total as u32, true)
                        }
                        modrinth::ProgressState::ExtractingPackOverrides(count, total) => {
                            t_extract_files.update_items(count as u32, total as u32)
                        }
                        modrinth::ProgressState::ExtractingPackClientOverrides(count, total) => {
                            t_extract_files.update_items(count as u32, total as u32)
                        }
                        modrinth::ProgressState::AcquiringPackMetadata(count, total) => {
                            t_addon_metadata.update_items(count as u32, total as u32)
                        }
                    }
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }

            t_download_files.complete_download();
        });

        let modpack_info = match install_source {
            ModrinthInstallSource::Remote {
                project_id,
                version_id,
            } => {
                tracing::info!("Modrinth Remote Install");
                t_request.start_opaque();
                let file = app
                    .modplatforms_manager()
                    .modrinth
                    .get_version(VersionID(version_id.clone()))
                    .await?
                    .files
                    .into_iter()
                    .reduce(|a, b| if b.primary { b } else { a })
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Modrinth project '{}' version '{}' does not have a file",
                            project_id,
                            version_id
                        )
                    })?;
                t_request.complete_opaque();
                modrinth::prepare_modpack_from_file(
                    &app,
                    &file,
                    instance_path.clone(),
                    modpack_progress_tx,
                )
                .await?
            }
            ModrinthInstallSource::Local { mrpack_path } => {
                tracing::info!("Modrinth Local Install");
                // TODO
                let file_size = std::fs::metadata(&mrpack_path)?.len();
                // show pack as already downloaded
                modpack_progress_tx.send(modrinth::ProgressState::DownloadingMRPack(
                    file_size, file_size,
                ))?;

                modrinth::prepare_modpack_from_mrpack(
                    &app,
                    mrpack_path.into(),
                    instance_path.clone(),
                    false,
                    modpack_progress_tx,
                )
                .await?
            }
        };

        downloads.extend(modpack_info.downloadables);

        modpack_info.index.dependencies.try_into()
    }
}

#[async_trait::async_trait]
impl PrepareModpack for Modpack {
    async fn prepare_modpack(
        &self,
        app: Arc<AppInner>,
        instance_path: InstancePath,
        downloads: &mut Vec<Downloadable>,
        subtasks: PrepareModpackSubtasks,
    ) -> anyhow::Result<StandardVersion> {
        match self {
            Self::Curseforge(modpack) => {
                modpack
                    .prepare_modpack(app, instance_path, downloads, subtasks)
                    .await
            }
            Self::Modrinth(modpack) => {
                modpack
                    .prepare_modpack(app, instance_path, downloads, subtasks)
                    .await
            }
        }
    }
}
