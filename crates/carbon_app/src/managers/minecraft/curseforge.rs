use std::borrow::BorrowMut;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use carbon_net::{Downloadable, Progress};
use tokio::task::spawn_blocking;

use crate::domain::modplatforms::curseforge::{self, CurseForgeResponse, File};
use crate::domain::runtime_path::InstancePath;
use crate::managers::App;

// Download ZIP
// Extract manifest - Parse manifest
// Download mods
// Extract overrides

#[derive(Debug)]
pub enum ProgressState {
    Idle,
    DownloadingAddonZip((u64, u64)),
    ExtractingAddonOverrides((u64, u64)),
    AcquiringAddonsMetadata((u64, u64)),
}

#[derive(Debug)]
pub struct ModpackInfo {
    pub manifest: curseforge::manifest::Manifest,
    pub downloadables: Vec<Downloadable>,
}

pub async fn prepare_modpack(
    app: &App,
    cf_addon: &File,
    instance_path: InstancePath,
    progress_percentage_sender: tokio::sync::watch::Sender<ProgressState>,
) -> anyhow::Result<ModpackInfo> {
    let progress_percentage_sender = Arc::new(progress_percentage_sender);

    let temp_dir = &app.settings_manager().runtime_path.get_temp();
    let modpack_download_url = cf_addon
        .download_url
        .clone()
        .ok_or(anyhow::anyhow!("Failed to get download url"))?;

    // generate uuid
    let uuid = uuid::Uuid::new_v4();
    let file_path = temp_dir.to_path().join(format!("{}.zip", uuid));
    let file_downloadable = Downloadable::new(&modpack_download_url, file_path.clone())
        .with_size(cf_addon.file_length as u64);

    tokio::fs::create_dir_all(
        &file_path
            .parent()
            .ok_or(anyhow::anyhow!("Failed to get parent"))?,
    )
    .await?;

    let (download_progress_sender, mut download_progress_recv) =
        tokio::sync::watch::channel(Progress::new());

    let progress_percentage_sender_clone = progress_percentage_sender.clone();

    tokio::spawn(async move {
        while download_progress_recv.borrow_mut().changed().await.is_ok() {
            progress_percentage_sender_clone.send(ProgressState::DownloadingAddonZip((
                download_progress_recv.borrow().current_size,
                download_progress_recv.borrow().total_size,
            )))?;
        }

        Ok::<(), anyhow::Error>(())
    });

    carbon_net::download_file(&file_downloadable, Some(download_progress_sender)).await?;

    let file_handle = std::fs::File::open(&file_path)?;
    let mut archive = zip::ZipArchive::new(file_handle)?;

    let archive_len = archive.len() as u64;

    let (manifest, downloadables) = {
        let mut i = 0;
        let mut downloadables = Vec::new();
        loop {
            if i > archive_len {
                break Err(anyhow::anyhow!("Failed to find manifest"));
            }

            let file = archive.by_index(i as usize)?;

            if file.name() == "manifest.json" {
                let manifest: curseforge::manifest::Manifest = serde_json::from_reader(file)?;

                let mut handles = Vec::new();

                let semaphore = Arc::new(tokio::sync::Semaphore::new(20));
                let atomic_counter_download_metadata =
                    Arc::new(std::sync::atomic::AtomicU64::new(0));

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
                            .get_mod_file(curseforge::filters::ModFileParameters {
                                mod_id,
                                file_id,
                            })
                            .await?;

                        let instance_path = instance_path.get_mods_path(); // TODO: they could also be other things
                        let downloadable = Downloadable::new(
                            mod_file
                                .download_url
                                .ok_or(anyhow::anyhow!("Failed to get download url for mod"))?,
                            instance_path.join(mod_file.file_name),
                        )
                        .with_size(mod_file.file_length as u64);
                        progress_percentage_sender_clone.send(
                            ProgressState::AcquiringAddonsMetadata((
                                atomic_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
                                files_len,
                            )),
                        )?;

                        Ok::<Downloadable, anyhow::Error>(downloadable)
                    });

                    handles.push(handle);
                }

                for handle in handles {
                    match handle.await? {
                        Ok(downloadable) => {
                            downloadables.push(downloadable);
                        }
                        Err(e) => {
                            println!("Failed to download mod: {:?}", e);
                        }
                    }
                }

                break Ok((manifest, downloadables));
            }

            i += 1;
        }
    }?;

    let override_folder_name = manifest.overrides.clone();
    let override_full_path = instance_path.get_root();
    tokio::fs::create_dir_all(&override_full_path).await?;
    spawn_blocking(move || {
        let total_archive_files = archive.len() as u64;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => Path::new(&override_full_path).join(path),
                None => continue,
            };

            if (*file.name()).starts_with(&override_folder_name) {
                if (*file.name()).ends_with('/') {
                    std::fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(p)?;
                        }
                    }
                    let mut outfile = std::fs::File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }

                // Get and Set permissions
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;

                    if let Some(mode) = file.unix_mode() {
                        std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                    }
                }
            }

            progress_percentage_sender.send(ProgressState::ExtractingAddonOverrides((
                i as u64,
                total_archive_files,
            )))?;
        }

        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(ModpackInfo {
        manifest,
        downloadables,
    })
}

#[cfg(test)]
mod test {
    use crate::domain::runtime_path::InstancePath;
    use crate::managers::minecraft::curseforge::{prepare_modpack, ProgressState};
    use crate::{
        domain::modplatforms::curseforge::filters::ModFileParameters,
        managers::modplatforms::curseforge::CurseForge, setup_managers_for_test,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_prepare_modpack() {
        let app = setup_managers_for_test().await;
        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let curseforge = CurseForge::new(client);

        let temp_path = app.tmpdir.join("test_prepare_modpack");

        let mod_id = 389615;
        let file_id = 3931045;

        let cf_mod = curseforge
            .get_mod_file(ModFileParameters { mod_id, file_id })
            .await
            .unwrap()
            .data;

        let progress = tokio::sync::watch::channel(ProgressState::Idle);

        let result = prepare_modpack(&app, &cf_mod, InstancePath::new(temp_path), progress.0)
            .await
            .unwrap();

        assert!(!result.downloadables.is_empty())
    }
}
