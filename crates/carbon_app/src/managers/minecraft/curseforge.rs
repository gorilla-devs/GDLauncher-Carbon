use crate::managers::App;
use crate::managers::instance::modpack::packinfo::PackInfo;
use crate::managers::vtask::Subtask;
use anyhow::Context;
use carbon_net::{DownloadOptions, Downloadable, Progress};
use carbon_platforms::curseforge::filters::{ModsParameters, ModsParametersBody};
use carbon_platforms::curseforge::{self, CurseForgeResponse, File, HashAlgo};
use carbon_rt_path::InstancePath;
use itertools::Itertools;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::trace;

use super::UpdateValue;
use super::modrinth::{MAX_EXTRACTED_OVERRIDE_BYTES, copy_bounded, is_symlink_mode};

// Download ZIP
// Extract manifest - Parse manifest
// Download mods
// Extract overrides

#[derive(Debug, Copy, Clone)]
pub struct ProgressState {
    pub extract_addon_overrides: UpdateValue<(u64, u64)>,
}

impl ProgressState {
    pub fn new() -> Self {
        Self {
            extract_addon_overrides: UpdateValue::new((0, 0)),
        }
    }
}

#[derive(Debug)]
pub struct ModpackInfo {
    pub manifest: curseforge::manifest::Manifest,
    // (downloadable, existing path from packinfo)
    pub downloadables: Vec<(Downloadable, Option<String>)>,
}

#[tracing::instrument(skip(app, progress_percentage_sender))]
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

    carbon_net::download_multiple(
        &[file_downloadable],
        DownloadOptions::builder()
            .concurrency(1)
            .progress_sender(download_progress_sender)
            .build(),
    )
    .await
    .with_context(|| {
        format!(
            "Failed to download curseforge modpack zip: {:?}",
            cf_addon.download_url
        )
    })?;

    file.try_rename_or_move(target_path).await?;
    Ok(())
}

/// Given the path to an addon zip file, prepares the list of downloadables and the manifest
/// for the modpack.
#[tracing::instrument(skip(app, packinfo, t_addon_metadata, progress_percentage_sender))]
pub async fn prepare_modpack_from_zip(
    app: &App,
    zip_path: &Path,
    instance_path: &InstancePath,
    skip_overrides: bool,
    packinfo: Option<&PackInfo>,
    t_addon_metadata: Subtask,
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

    let mc_manifest = Arc::new(app.minecraft_manager().get_minecraft_manifest().await?);

    let mc_version = manifest.minecraft.version.clone();

    let downloadables = {
        let mut downloadables = Vec::new();

        let cf_manager = &app.modplatforms_manager().curseforge;
        let addons = Arc::new(
            cf_manager
                .get_mods(ModsParameters {
                    body: ModsParametersBody {
                        mod_ids: manifest
                            .files
                            .iter()
                            .map(|file| file.project_id)
                            .collect::<Vec<_>>(),
                    },
                })
                .await?
                .data
                .into_iter()
                .map(|addon| {
                    (
                        addon.id,
                        addon.class_id.unwrap_or(curseforge::ClassId::Mods),
                    )
                })
                .collect::<HashMap<_, _>>(),
        );

        let manifest_files = manifest
            .files
            .iter()
            .map(|file| file.file_id)
            .collect::<Vec<_>>();

        let all_addons = if manifest_files.len() > 0 {
            // curseforge returns 400 bad response if file_ids is empty
            app.modplatforms_manager()
                .curseforge
                .get_files(curseforge::filters::FilesParameters {
                    body: curseforge::filters::FilesParametersBody {
                        file_ids: manifest_files,
                    },
                })
                .await?
                .data
                .into_iter()
                .map(|file| (file.id, file))
                .collect::<HashMap<_, _>>()
        } else {
            HashMap::new()
        };

        for file in &manifest.files {
            let mod_id = file.project_id;
            let file_id = file.file_id;

            let mod_file = all_addons
                .get(&file_id)
                .ok_or(anyhow::anyhow!("Failed to get mod file: {:?}", file_id))?;

            let class_id = addons
                .get(&mod_id)
                .ok_or(anyhow::anyhow!("Failed to get addon: {:?}", mod_id))?;

            let instance_path =
                class_id
                    .clone()
                    .into_path(&instance_path, mc_version.clone(), &mc_manifest);

            let existing_path = packinfo
                .map(|packinfo| 'a: {
                    let packinfo_path = format!("/mods/{}", mod_file.file_name);

                    if let Some(pihashes) = packinfo.files.get(&packinfo_path) {
                        tracing::warn!(?pihashes, ?mod_file.hashes);

                        let md5hash = mod_file
                            .hashes
                            .iter()
                            .filter_map(|hash| match hash.algo {
                                HashAlgo::Md5 => Some(&hash.value),
                                _ => None,
                            })
                            .find_map(|hash| {
                                let mut array = [0u8; 16];
                                hex::decode_to_slice(&hash, &mut array).ok()?;
                                Some(array)
                            });

                        if let Some(md5) = md5hash {
                            if md5 == pihashes.md5 {
                                break 'a Some(packinfo_path);
                            }
                        }
                    }

                    None
                })
                .flatten();

            // CurseForge manifest exposes sha1/md5 per file — verify
            // downloads against them to defend against MITM / CDN poisoning.
            let checksum = mod_file
                .hashes
                .iter()
                .find_map(|h| match h.algo {
                    HashAlgo::Sha1 => Some(carbon_net::Checksum::Sha1(h.value.clone())),
                    _ => None,
                })
                .or_else(|| {
                    mod_file.hashes.iter().find_map(|h| match h.algo {
                        HashAlgo::Md5 => Some(carbon_net::Checksum::Md5(h.value.clone())),
                        _ => None,
                    })
                });

            let downloadable = Downloadable::new(
                mod_file
                    .download_url
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Failed to get download url for mod"))?,
                instance_path.join(&mod_file.file_name),
            )
            .with_size(mod_file.file_length as u64)
            .with_checksum(checksum);

            downloadables.push((downloadable, existing_path));
        }

        downloadables
    };

    t_addon_metadata.complete_opaque();

    if !skip_overrides {
        let override_folder_name = manifest.overrides.clone();
        let override_full_path = instance_path.get_data_path();
        tokio::fs::create_dir_all(&override_full_path).await?;
        spawn_blocking(move || {
            let total_archive_files = archive.len() as u64;
            // Cumulative across the whole pass, not per entry: without this, many
            // entries each just under a per-entry cap could still add up to an
            // unbounded amount written to disk.
            let mut extracted_bytes: u64 = 0;
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;

                if !(*file.name()).starts_with(&override_folder_name) {
                    continue;
                }

                let outpath = match file.enclosed_name() {
                    Some(path) => match path.strip_prefix(&override_folder_name) {
                        Ok(stripped) => Path::new(&override_full_path).join(stripped),
                        // The name begins with the prefix string but is not inside the
                        // overrides directory (e.g. "overrides-extra/..."); skip it instead
                        // of panicking on the non-matching path component.
                        Err(_) => continue,
                    },
                    None => continue,
                };

                if (*file.name()).ends_with('/') {
                    continue;
                } else if is_symlink_mode(file.unix_mode()) {
                    // A symlink entry materialized as a regular file would end up
                    // containing the link's target path text instead of real data.
                    tracing::warn!(
                        "Skipping curseforge override entry `{}`: symlinks are not extracted",
                        file.name()
                    );
                    continue;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(p)?;
                        }
                    }

                    let mut outfile = std::fs::File::create(&outpath)?;
                    let remaining_budget =
                        MAX_EXTRACTED_OVERRIDE_BYTES.saturating_sub(extracted_bytes);
                    extracted_bytes += copy_bounded(&mut file, &mut outfile, remaining_budget)?;
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
