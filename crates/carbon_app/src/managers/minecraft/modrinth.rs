use std::borrow::BorrowMut;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use carbon_net::{Downloadable, Progress};
use tokio::task::spawn_blocking;

use crate::domain::runtime_path::InstancePath;
use crate::managers::App;

use crate::domain::modplatforms::modrinth::version::{
    ModpackIndex, ModrinthEnvironmentSupport, VersionFile,
};

use super::secure_path_join;



#[derive(Debug, Copy, Clone)]
pub enum ProgressState {
    Idle,
    DownloadingMRPack(u64, u64),
    ExtractingPackOverrides(u64, u64),
    AcquiringPackMetadata(u64, u64),
}

#[derive(Debug)]
pub struct ModpackInfo {
    pub index: ModpackIndex,
    pub downloadables: Vec<Downloadable>,
}

pub async fn prepare_modpack_from_file(
    app: &App,
    mrpack_file: &VersionFile,
    instance_path: InstancePath,
    progress_percentage_sender: tokio::sync::watch::Sender<ProgressState>,
) -> anyhow::Result<ModpackInfo> {
    let temp_dir = &app.settings_manager().runtime_path.get_temp();
    let _pack_download_url = mrpack_file.url.clone();

    // generate uuid
    let uuid = uuid::Uuid::new_v4();
    let file_path = temp_dir.to_path().join(format!("{}.mrpack", uuid));
    let file_downloadable = Downloadable::new(mrpack_file.url.to_string(), file_path.clone())
        .with_size(mrpack_file.size as u64);

    tokio::fs::create_dir_all(
        &file_path
            .parent()
            .ok_or(anyhow::anyhow!("Failed to get parent"))?,
    )
    .await?;

    let (download_progress_sender, mut download_progress_recv) =
        tokio::sync::watch::channel(Progress::new());

    let progress_percentage_sender = tokio::spawn(async move {
        while download_progress_recv.borrow_mut().changed().await.is_ok() {
            progress_percentage_sender.send(ProgressState::DownloadingMRPack(
                download_progress_recv.borrow().current_size,
                download_progress_recv.borrow().total_size,
            ))?;
        }

        Ok::<_, anyhow::Error>(progress_percentage_sender)
    });

    carbon_net::download_file(&file_downloadable, Some(download_progress_sender)).await?;

    let progress_percentage_sender = progress_percentage_sender.await??;

    prepare_modpack_from_mrpack(app, file_path, instance_path, progress_percentage_sender).await
}

pub async fn prepare_modpack_from_mrpack(
    app: &App,
    mrpack_path: PathBuf,
    instance_path: InstancePath,
    progress_percentage_sender: tokio::sync::watch::Sender<ProgressState>,
) -> anyhow::Result<ModpackInfo> {
    let progress_percentage_sender = Arc::new(progress_percentage_sender);

    let file_path_clone = mrpack_path.clone();
    let (mut archive, index) = spawn_blocking(move || {
        let file = std::fs::File::open(file_path_clone)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let index: ModpackIndex = {
            let file = archive.by_name("modrinth.index.json")?;
            serde_json::from_reader(file)?
        };

        Ok::<_, anyhow::Error>((archive, index))
    })
    .await??;

    let required_files: Vec<_> = index
        .files
        .iter()
        .filter(|&file| {
            file.env.as_ref().map_or(true, |env| {
                matches!(env.client, ModrinthEnvironmentSupport::Required)
            })
        })
        .cloned()
        .collect();

    let _optional_files: Vec<_> = index
        .files
        .iter()
        .filter(|&file| {
            file.env.as_ref().map_or(false, |env| {
                matches!(env.client, ModrinthEnvironmentSupport::Optional)
            })
        })
        .cloned()
        .collect();

    let downloadables = {
        let mut handles = Vec::new();

        let semaphore = Arc::new(tokio::sync::Semaphore::new(20));
        let atomic_counter_download_metadata = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let files_len = required_files.len() as u64;

        let data_path = instance_path.get_data_path();
        tokio::fs::create_dir_all(&data_path).await?;

        for file in required_files {
            let semaphore = semaphore.clone();
            let _app = app.clone();
            let instance_path = instance_path.clone();
            let progress_percentage_sender_clone = progress_percentage_sender.clone();
            let atomic_counter = atomic_counter_download_metadata.clone();

            let data_path = instance_path.get_data_path();
            let handle = tokio::spawn(async move {
                let _ = semaphore.acquire().await?;
                let target_path = secure_path_join(&data_path, &file.path)?;

                let downloadable = Downloadable::new(
                    file.downloads
                        .first()
                        .ok_or(anyhow::anyhow!("Failed to get download url for mod"))?
                        .to_string(),
                    target_path,
                )
                .with_size(file.file_size as u64);
                progress_percentage_sender_clone.send(ProgressState::AcquiringPackMetadata(
                    atomic_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
                    files_len,
                ))?;

                Ok::<Downloadable, anyhow::Error>(downloadable)
            });

            handles.push(handle);
        }

        futures::future::join_all(handles)
            .await
            .into_iter()
            .flatten()
            .collect::<Result<Vec<_>, _>>()?
    };

    let data_path = instance_path.get_data_path();
    let overrides_folder_name = "overrides";
    spawn_blocking(move || {
        let total_archive_files = archive.len() as u64;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if !(file.name().starts_with(overrides_folder_name)) {
                continue;
            }

            let out_path = match file.enclosed_name() {
                Some(path) => secure_path_join(
                    Path::new(&data_path),
                    path.strip_prefix(overrides_folder_name).expect(
                        "valid path as we skipped paths that did not start with this prefix",
                    ),
                )?,
                None => continue,
            };

            if file.name().ends_with('/') {
                continue;
            } else {
                if let Some(parent) = out_path.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                let mut out_file = std::fs::File::create(&out_path)?;

                std::io::copy(&mut file, &mut out_file)?;
            }

            progress_percentage_sender.send(ProgressState::ExtractingPackOverrides(
                i as u64,
                total_archive_files,
            ))?;
        }

        Ok::<(), anyhow::Error>(())
    })
    .await??;

    tokio::fs::remove_file(&mrpack_path).await?;

    Ok(ModpackInfo {
        index,
        downloadables,
    })
}
