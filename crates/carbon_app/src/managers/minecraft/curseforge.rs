use std::borrow::BorrowMut;
use std::sync::Arc;

use carbon_net::{Downloadable, Progress};

use crate::domain::modplatforms::curseforge::{self, CurseForgeResponse, File};
use crate::domain::runtime_path::InstancePath;
use crate::managers::App;

// Download ZIP
// Extract manifest - Parse manifest
// Download mods
// Extract overrides

#[derive(Debug)]
pub enum ProgressState {
    DownloadingAddon,
    FetchingModInfo,
}

#[derive(Debug)]
pub struct ProgressEvent {
    pub state: ProgressState,
    pub progress: u8,
}

#[derive(Debug)]
pub struct ModpackInfo {
    pub mc_version: String,
    pub modloader_type: String,    // TODO: actual type
    pub modloader_version: String, // TODO: actual type
    pub downloadables: Vec<Downloadable>,
}

pub async fn get_modpack_info(
    app: &App,
    cf_addon: &File,
    instance_path: InstancePath,
    progress_percentage_sender: Arc<tokio::sync::watch::Sender<ProgressEvent>>,
) -> anyhow::Result<ModpackInfo> {
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
            progress_percentage_sender_clone.send(ProgressEvent {
                state: ProgressState::DownloadingAddon,
                progress: download_progress_recv.borrow().size_progress,
            })?;
        }

        Ok::<(), anyhow::Error>(())
    });

    carbon_net::download_file(&file_downloadable, Some(download_progress_sender)).await?;

    let file_handle = std::fs::File::open(&file_path)?;
    let mut archive = zip::ZipArchive::new(file_handle)?;

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;

        if file.name() == "manifest.json" {
            let manifest: curseforge::manifest::Manifest = serde_json::from_reader(file)?;

            let mut handles = Vec::new();

            let semaphore = Arc::new(tokio::sync::Semaphore::new(20));

            for file in manifest.files {
                let semaphore = semaphore.clone();
                let app = app.clone();
                let instance_path = instance_path.clone();
                let handle = tokio::spawn(async move {
                    let _ = semaphore.acquire().await?;

                    let cf_manager = &app.modplatforms_manager().curseforge;

                    let CurseForgeResponse { data: mod_file, .. } = cf_manager
                        .get_mod_file(curseforge::filters::ModFileParameters {
                            mod_id: file.project_id,
                            file_id: file.file_id,
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

                    Ok::<Downloadable, anyhow::Error>(downloadable)
                });

                handles.push(handle);
            }

            let mut downloadables = Vec::with_capacity(handles.len());
            for handle in handles {
                downloadables.push(handle.await??);
            }

            let mod_loader = manifest
                .minecraft
                .mod_loaders
                .first()
                .ok_or(anyhow::anyhow!("Failed to get mod loader"))?;

            return Ok(ModpackInfo {
                mc_version: manifest.minecraft.version,
                modloader_type: mod_loader.id.clone(), // TODO: find out the type
                modloader_version: mod_loader.id.clone(), // TODO: extract version?
                downloadables,
            });
        }
    }

    Err(anyhow::anyhow!("Failed to find manifest"))
}
