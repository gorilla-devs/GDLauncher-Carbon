use std::borrow::BorrowMut;
use std::path::Path;
use std::sync::Arc;

use carbon_net::{Downloadable, Progress};
use tokio::task::spawn_blocking;

use crate::domain::modplatforms::curseforge::{self, CurseForgeResponse, File};
use crate::domain::runtime_path::InstancePath;
use crate::managers::App;

use super::UpdateValue;

// Download ZIP
// Extract manifest - Parse manifest
// Download mods
// Extract overrides

#[derive(Debug, Copy, Clone)]
pub struct ProgressState {
    pub extract_addon_overrides: UpdateValue<(u64, u64)>,
    pub acquire_addon_metadata: UpdateValue<(u64, u64)>,
}

impl ProgressState {
    pub fn new() -> Self {
        Self {
            extract_addon_overrides: UpdateValue::new((0, 0)),
            acquire_addon_metadata: UpdateValue::new((0, 0)),
        }
    }
}

#[derive(Debug)]
pub struct ModpackInfo {
    pub manifest: curseforge::manifest::Manifest,
    pub downloadables: Vec<Downloadable>,
}

pub async fn download_modpack_zip(
    app: &App,
    cf_addon: &File,
    target_path: &Path,
    progress_percentage_sender: tokio::sync::watch::Sender<UpdateValue<(u64, u64)>>,
) -> anyhow::Result<()> {
    let modpack_download_url = cf_addon
        .download_url
        .clone()
        .ok_or(anyhow::anyhow!("Failed to get download url"))?;

    // generate uuid
    let file = app
        .settings_manager()
        .runtime_path
        .get_temp()
        .maketmpfile()
        .await?;
    let file_downloadable = Downloadable::new(&modpack_download_url, file.to_path_buf())
        .with_size(cf_addon.file_length as u64);

    tokio::fs::create_dir_all(
        &file
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get parent"))?,
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

    file.rename(target_path).await?;
    Ok(())
}

pub async fn prepare_modpack_from_zip(
    app: &App,
    zip_path: &Path,
    instance_path: &InstancePath,
    skip_overlays: bool,
    progress_percentage_sender: tokio::sync::watch::Sender<ProgressState>,
) -> anyhow::Result<ModpackInfo> {
    let progress_percentage_sender = Arc::new(progress_percentage_sender);

    let file_path_clone = zip_path.to_path_buf();
    let (mut archive, manifest) = spawn_blocking(move || {
        let file = std::fs::File::open(file_path_clone)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let manifest: curseforge::manifest::Manifest = {
            let file = archive.by_name("manifest.json")?;
            serde_json::from_reader(file)?
        };

        Ok::<_, anyhow::Error>((archive, manifest))
    })
    .await??;

    let downloadables = {
        let mut handles = Vec::new();

        let semaphore = Arc::new(tokio::sync::Semaphore::new(20));
        let atomic_counter_download_metadata = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let files_len = manifest.files.len() as u64;

        for file in &manifest.files {
            let semaphore = semaphore.clone();
            let app = app.clone();
            let instance_path = instance_path.clone();
            let progress_percentage_sender_clone = progress_percentage_sender.clone();
            let atomic_counter = atomic_counter_download_metadata.clone();

            let mod_id = file.project_id;
            let file_id = file.file_id;

            let handle = tokio::spawn(async move {
                let _ = semaphore.acquire().await?;

                let cf_manager = &app.modplatforms_manager().curseforge;

                let CurseForgeResponse { data: mod_file, .. } = cf_manager
                    .get_mod_file(curseforge::filters::ModFileParameters { mod_id, file_id })
                    .await?;

                let instance_path = instance_path.get_mods_path(); // TODO: they could also be other things
                let downloadable = Downloadable::new(
                    mod_file
                        .download_url
                        .ok_or(anyhow::anyhow!("Failed to get download url for mod"))?,
                    instance_path.join(mod_file.file_name),
                )
                .with_size(mod_file.file_length as u64);
                progress_percentage_sender_clone.send_modify(|progress| {
                    progress.acquire_addon_metadata.set((
                        atomic_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1,
                        files_len,
                    ));
                });

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

    if !skip_overlays {
        let override_folder_name = manifest.overrides.clone();
        let override_full_path = instance_path.get_data_path();
        tokio::fs::create_dir_all(&override_full_path).await?;
        spawn_blocking(move || {
            let total_archive_files = archive.len() as u64;
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;

                if !(*file.name()).starts_with(&override_folder_name) {
                    continue;
                }

                let outpath = match file.enclosed_name() {
                    Some(path) => Path::new(&override_full_path)
                        .join(path.strip_prefix(&override_folder_name).unwrap()),
                    None => continue,
                };

                if (*file.name()).ends_with('/') {
                    continue;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(p)?;
                        }
                    }

                    let mut outfile = std::fs::File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }

                progress_percentage_sender.send_modify(|progress| {
                    progress
                        .extract_addon_overrides
                        .set((i as u64 + 1, total_archive_files));
                });
            }

            Ok::<(), anyhow::Error>(())
        })
        .await??;
    }

    Ok(ModpackInfo {
        manifest,
        downloadables,
    })
}

// #[cfg(test)]
// mod test {
//     use crate::domain::runtime_path::InstancePath;
//     use crate::managers::minecraft::curseforge::{prepare_modpack, ProgressState};
//     use crate::{
//         domain::modplatforms::curseforge::filters::ModFileParameters,
//         managers::modplatforms::curseforge::CurseForge, setup_managers_for_test,
//     };

//     #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
//     async fn test_prepare_modpack() {
//         let app = setup_managers_for_test().await;
//         let client = reqwest::Client::builder().build().unwrap();
//         let client = reqwest_middleware::ClientBuilder::new(client).build();
//         let curseforge = CurseForge::new(client);

//         let temp_path = app.tmpdir.join("test_prepare_modpack");

//         let mod_id = 389615;
//         let file_id = 3931045;

//         let cf_mod = curseforge
//             .get_mod_file(ModFileParameters { mod_id, file_id })
//             .await
//             .unwrap()
//             .data;

//         let progress = tokio::sync::watch::channel(ProgressState::Idle);

//         let result = prepare_modpack(&app, &cf_mod, InstancePath::new(temp_path), progress.0)
//             .await
//             .unwrap();

//         assert!(!result.downloadables.is_empty())
//     }
// }
