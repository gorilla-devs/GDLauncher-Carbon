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
use crate::managers::instance::modpack::packinfo::PackInfo;

use thiserror::Error;

use super::UpdateValue;

#[derive(Error, Debug)]
pub enum PathTraversalError {
    #[error("Path `{0}` has a root component and joining it will cause a path traversal")]
    PathHasRoot(PathBuf),
    #[error("Path `{0}` climbs above it's root and joining it will cause a path traversal")]
    PathClimbsAboveRoot(PathBuf),
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
pub fn path_clean<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut out = Vec::new();
    for comp in path.as_ref().components() {
        match comp {
            Component::CurDir => (),
            Component::ParentDir => match out.last() {
                Some(Component::RootDir) => (),
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                _ => out.push(comp),
            },
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
pub fn secure_path_join<P1: AsRef<Path>, P2: AsRef<Path>>(
    root: P1,
    unsafe_path: P2,
) -> Result<PathBuf, PathTraversalError> {
    let unsafe_path = unsafe_path.as_ref();
    if unsafe_path.has_root() {
        return Err(PathTraversalError::PathHasRoot(unsafe_path.to_path_buf()));
    } else if unsafe_path.starts_with("..") {
        return Err(PathTraversalError::PathClimbsAboveRoot(
            unsafe_path.to_path_buf(),
        ));
    }
    let clean_root = path_clean(root);

    // clean path first to prevent traversing above
    let clean_path = path_clean(unsafe_path);

    // join two clean paths
    let result_path = clean_root.join(clean_path);

    // reclean to resolve remaining indirection
    let clean_result = path_clean(result_path);

    // double check to make sure we haven't climbed out
    if !clean_result.starts_with(clean_root) {
        Err(PathTraversalError::PathClimbsAboveRoot(
            unsafe_path.to_path_buf(),
        ))
    } else {
        Ok(clean_result)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ProgressState {
    Idle,
    ExtractingPackOverrides(u64, u64),
    AcquiringPackMetadata(u64, u64),
}

#[derive(Debug)]
pub struct ModpackInfo {
    pub index: ModpackIndex,
    // (downloadable, existing path)
    pub downloadables: Vec<(Downloadable, Option<String>)>,
}

pub async fn download_mrpack(
    app: &App,
    mrpack_file: &VersionFile,
    target_path: &Path,
    progress_percentage_sender: tokio::sync::watch::Sender<UpdateValue<(u64, u64)>>,
) -> anyhow::Result<()> {
    let _pack_download_url = mrpack_file.url.clone();

    // generate uuid
    let file = app
        .settings_manager()
        .runtime_path
        .get_temp()
        .maketmpfile()
        .await?;
    let file_downloadable = Downloadable::new(&mrpack_file.url.to_string(), file.to_path_buf())
        .with_size(mrpack_file.size as u64);

    tokio::fs::create_dir_all(
        &file
            .parent()
            .ok_or(anyhow::anyhow!("Failed to get parent"))?,
    )
    .await?;

    let (download_progress_sender, mut download_progress_recv) =
        tokio::sync::watch::channel(Progress::new());

    tokio::spawn(async move {
        while download_progress_recv.borrow_mut().changed().await.is_ok() {
            let p = download_progress_recv.borrow();
            progress_percentage_sender
                .send_modify(|progress| progress.set((p.current_size, p.total_size)));
        }

        Ok::<_, anyhow::Error>(progress_percentage_sender)
    });

    carbon_net::download_file(&file_downloadable, Some(download_progress_sender)).await?;

    file.try_rename_or_move(target_path).await?;
    Ok(())
}

pub async fn prepare_modpack_from_mrpack(
    app: &App,
    mrpack_path: &Path,
    instance_path: &InstancePath,
    skip_overlays: bool,
    packinfo: Option<&PackInfo>,
    progress_percentage_sender: tokio::sync::watch::Sender<ProgressState>,
) -> anyhow::Result<ModpackInfo> {
    let file_path_clone = mrpack_path.to_path_buf();
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
            file.env.as_ref().map_or(true, |env| match env.client {
                ModrinthEnvironmentSupport::Required => true,
                _ => false,
            })
        })
        .cloned()
        .collect();

    let _optional_files: Vec<_> = index
        .files
        .iter()
        .filter(|&file| {
            file.env.as_ref().map_or(false, |env| match env.client {
                ModrinthEnvironmentSupport::Optional => true,
                _ => false,
            })
        })
        .cloned()
        .collect();

    let downloadables = {
        let files_len = required_files.len() as u64;

        let data_path = instance_path.get_data_path();
        tokio::fs::create_dir_all(&data_path).await?;

        let instance_path = instance_path.clone();

        required_files
            .into_iter()
            .enumerate()
            .map(|(i, file)| {
                let _app = app.clone();

                let data_path = instance_path.get_data_path();

                let existing_path = packinfo
                    .map(|packinfo| {
                        let mut sha512 = [0u8; 64];
                        hex::decode_to_slice(file.hashes.sha512, &mut sha512).ok()?;

                        let packinfo_path = format!("/{}", file.path);

                        match packinfo.files.get(&packinfo_path) {
                            Some(hashes) if sha512 == hashes.sha512 => Some(packinfo_path),
                            _ => None,
                        }
                    })
                    .flatten();

                let target_path = secure_path_join(&data_path, &file.path)?;

                let downloadable = Downloadable::new(
                    file.downloads
                        .first()
                        .ok_or(anyhow::anyhow!("Failed to get download url for mod"))?
                        .to_string(),
                    target_path,
                )
                .with_size(file.file_size as u64);

                progress_percentage_sender
                    .send(ProgressState::AcquiringPackMetadata(i as u64, files_len))?;

                Ok::<_, anyhow::Error>((downloadable, existing_path))
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    if !skip_overlays {
        let data_path = instance_path.get_data_path();
        let overrides_folder_name = "overrides";
        spawn_blocking(move || {
            let total_archive_files = archive.len() as u64;
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                if !(file.name().starts_with(&overrides_folder_name)) {
                    continue;
                }

                let out_path = match file.enclosed_name() {
                    Some(path) => secure_path_join(
                        Path::new(&data_path),
                        path.strip_prefix(&overrides_folder_name).expect(
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
    }

    Ok(ModpackInfo {
        index,
        downloadables,
    })
}
