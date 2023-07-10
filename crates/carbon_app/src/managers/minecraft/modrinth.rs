use std::borrow::BorrowMut;
use std::path::{Path, PathBuf, Component};
use std::sync::Arc;

use carbon_net::{Downloadable, Progress};
use tokio::task::spawn_blocking;

use crate::domain::runtime_path::InstancePath;
use crate::managers::App;

use crate::domain::modplatforms::modrinth::version::{VersionFile, ModpackIndex, ModrinthEnvironmentSupport};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PathTraversalError {
    #[error("Path `{0}` has a root component and joining it will cause a path traversal")]
    PathHasRoot(Path),
    #[error("Path `{0}` climbs above it's root and joining it will cause a path traversal")]
    PathClimbsAboveRoot(Path),
}

/// 1. Reduce multiple slashes to a single slash.
/// 2. Eliminate `.` path name elements (the current directory).
/// 3. Eliminate `..` path name elements (the parent directory) and the non-`.` non-`..`,
/// element that precedes them.
/// 4. Eliminate `..` elements that begin a rooted path, that is, replace `/..` by `/` at the
/// beginning of a path.
/// 5. Leave intact `..` elements that begin a non-rooted path.
///
/// If the result of this process is an empty string, return the relative path `"."`, representing the
/// current directory.
pub fn clean_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut out = Vec::new();
    for comp in path.as_ref().components() {
        match comp {
            Component::CurDir => (),
            Component::ParentDir => match out.last() {
                Some(Component::RootDir) => (),
                Some(Component::Normal(_)) => out.pop(),
                _ => out.push(comp),
            }
            comp => out.push(comp),
        }
    }

    if out.is_empty() {
        PathBuf::from(".")
    } else {
        out.iter().collect()
    }
}

/// lexically checks that the join operation does not climb above the root
/// the returned bath is guaranteed to be under the provided root baring the influence of symbolic
/// links. This should be later checked by calling `canonicalize` after we are sure the parent
/// directories exist.
///
pub fn secure_path_join<P: AsRef<Path>>(root: P, unsafe_path: P) -> Result<PathBuf, PathTraversalError> {
    if unsafe_path.has_root() {
        return Err(PathTraversalError::PathHasRoot(unsafe_path));
    } else if unsafe_path.starts_with("..") {
        return Err(PathTraversalError::PathClimbsAboveRoot(unsafe_path));
    }
    let clean_root = clean_path(root);

    // clean path first to prevent traversing above
    let clean_path = clean_path(unsafe_path);

    // join two clean paths
    let result_path = clean_root.join(clean_path);

    // reclean to resolve remaining indirection
    let clean_result = clean_path(result_path);

    // double check to make sure we haven't climbed out
    if !clean_result.starts_with(clean_root) {
        Err(PathClimbsAboveRoot(unsafe_path))
    } else {
        Ok(clean_result)
    }
}


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
    let pack_download_url = mrpack_file.url.clone();

    // generate uuid
    let uuid = uuid::Uuid::new_v4();
    let file_path = temp_dir.to_path().join(format!("{}.mrpack", uuid));
    let file_downloadable = Downloadable::new(&modpack_download_url, file_path.clone())
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
            progress_percentage_sender.send(ProgressState::DownloadingAddonZip(
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

    let required_files: Vec<_> = index.files.iter().filter(|&file|{
        file.env.map_or(true, |env| match env.client {
            ModrinthEnvironmentSupport::Required => true,
            _ => false,
        })
    }).cloned().collect();

    let optional_files: Vec<_> = index.files.iter().filter(|&file|{
        file.env.map_or(false, |env| match env.client {
            ModrinthEnvironmentSupport::Optional => true,
            _ => false,
        })
    }).cloned().collect();


    let downloadables = {
        let mut handles = Vec::new();

        let semaphore = Arc::new(tokio::sync::Semaphore::new(20));
        let atomic_counter_download_metadata = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let files_len = required_files.len() as u64;

        for file in &required_files {
            let semaphore = semaphore.clone();
            let app = app.clone();
            let instance_path = instance_path.clone();
            let progress_percentage_sender_clone = progress_percentage_sender.clone();
            let atomic_counter = atomic_counter_download_metadata.clone();

            let handle = tokio::spawn(async move {
                let _ = semaphore.acquire().await?;
                let target_path = secure_path_join(instance_path, file.path)?;

                let downloadable = Downloadable::new(
                    file.downloads.first.ok_or(anyhow::anyhow!("Failed to get download url for mod"))?,
                    target_path,
                )
                .with_size(mod_file.file_size as u64);
                progress_percentage_sender_clone.send(ProgressState::AcquiringAddonsMetadata(
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

    // TODO: Unpack overrides

}
