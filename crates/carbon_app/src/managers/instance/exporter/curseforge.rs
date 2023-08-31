use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use tokio::sync::Mutex;
use walkdir::WalkDir;

use crate::{
    api::translation::Translation,
    domain::{
        instance::{InstanceDetails, InstanceId, Mod},
        modplatforms::curseforge::manifest::{
            CFModLoader, Manifest, ManifestFileReference, Minecraft,
        },
        runtime_path::InstancePath,
        vtask::VisualTaskId,
    },
    managers::{vtask::VisualTask, AppInner},
};

use super::{
    ArchiveExporter, ExportInstanceFileEntry, ExportInstanceFileInfo, ExportInstanceFileMode,
    InstanceExporter, PRIMARY_LOADER_TYPES,
};

#[derive(Debug, Default)]
pub struct CurseForgeZipExporter {
    pub export_files: Mutex<Vec<ExportInstanceFileEntry<CurseForgePlatformData>>>
}

#[async_trait::async_trait]
impl InstanceExporter for CurseForgeZipExporter {
    async fn export<F: Fn(&Path) -> bool + Send>(
        &self,
        app: Arc<AppInner>,
        instance_id: InstanceId,
        output_path: PathBuf,
        filter: F,
    ) -> anyhow::Result<VisualTaskId> {
        let instance_manager = app.instance_manager();
        let instance_details = instance_manager.instance_details(instance_id).await?;

        let task = VisualTask::new(Translation::InstanceTaskExport {
            instance_name: instance_details.name.clone(),
        });

        let task_id = app.task_manager().spawn_task(&task).await;

        let instance_path = instance_manager.get_path(instance_id).await?;
        let mods = instance_manager.list_mods(instance_id).await?;

        let _export_task = tokio::spawn(async move {
            if let Ok(manifest) = generate_manifest(
                &instance_details,
                &instance_path,
                &mods,
                None,
                "".to_string(),
                true,
            ) {
                //TODO: get instance path
                // let archive_exporter = ArchiveExporter::new(output_path, instance_details);
            };
        });

        Ok(task_id)
    }
}

#[derive(Debug, Default)]
struct CurseForgePlatformData {
    mod_id: u32,
    file_id: u32,
}

static CF_FILE_EXTENSIONS: [&str; 2] = ["jar", "zip"];

fn filter_export_file(file: &ExportInstanceFileInfo<CurseForgePlatformData>) -> bool {
    if file
        .relative_path
        .extension()
        .map(|ext| ext == "disabled")
        .unwrap_or(false)
    {
        if file
            .relative_path
            .file_stem()
            .and_then(|stem| {
                Path::new(stem)
                    .extension()
                    .map(|ext| CF_FILE_EXTENSIONS.iter().any(|&cf_ext| ext == cf_ext))
            })
            .unwrap_or(false)
        {
            true
        } else {
            false
        }
    } else {
        if file
            .relative_path
            .extension()
            .map(|ext| CF_FILE_EXTENSIONS.iter().any(|&cf_ext| ext == cf_ext))
            .unwrap_or(false)
        {
            true
        } else {
            false
        }
    }
}

async fn identify_files(
    app: Arc<AppInner>,
    entries: Vec<ExportInstanceFileEntry<CurseForgePlatformData>>,
) -> anyhow::Result<Vec<ExportInstanceFileEntry<CurseForgePlatformData>>> {
    let mut entries = entries;
    let files = entries
        .iter_mut()
        .map(|entry| entry.flatten_mut())
        .flatten()
        .filter(|file| filter_export_file(file))
        .collect::<Vec<_>>();

    let mut file_murmur_pairs =
        futures::future::join_all(files.into_iter().map(|file| async move {
            let mut murmur_content = tokio::fs::read(&file.full_path).await?;
            // drop "whitespace" (curseforge api)
            murmur_content.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
            let murmur2 = murmurhash32::murmurhash2(&murmur_content);
            Ok::<_, anyhow::Error>((file, murmur2))
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    while !file_murmur_pairs.is_empty() {
        let (mut file_fingerprint_pairs, fingerprints) = file_murmur_pairs
            .drain(0..usize::min(1000, file_murmur_pairs.len()))
            .map(|(file, fingerprint)| ((file, fingerprint), fingerprint))
            .unzip::<_, _, Vec<_>, Vec<_>>();

        let fp_response = app
            .modplatforms_manager()
            .curseforge
            .get_fingerprints(&fingerprints[..])
            .await?
            .data;

        for (fingerprint, fp_match) in fp_response
            .exact_fingerprints
            .into_iter()
            .zip(fp_response.exact_matches.into_iter())
        {
            let (file, _) = file_fingerprint_pairs
                .iter_mut()
                .find(|(_file, fp)| fp == &fingerprint)
                .ok_or_else(|| {
                    anyhow::anyhow!("Invalid/Unknown fingerprint returned by curseforge API")
                })?;

            // make sure the file is still available remotely
            if fp_match.file.is_available {
                file.platform_data = Some(CurseForgePlatformData {
                    mod_id: fp_match.file.mod_id,
                    file_id: fp_match.file.id,
                })
            }
        }
    }

    Ok(entries)
}

async fn generate_manifest(
    instance_details: &InstanceDetails,
    instance_path: &InstancePath,
    files: &Vec<ExportInstanceFileInfo<CurseForgePlatformData>>,
    version: Option<String>,
    author: String,
) -> anyhow::Result<Manifest> {
    let mut primary_count = 0;
    Ok(Manifest {
        minecraft: Minecraft {
            version: instance_details.version.clone().ok_or_else(|| {
                anyhow::anyhow!("Instance has no Minecraft version and can not be exported")
            })?,
            mod_loaders: instance_details
                .modloaders
                .iter()
                .map(|modloader| {
                    let primary =
                        PRIMARY_LOADER_TYPES.contains(&modloader.type_) && primary_count == 0;
                    if primary {
                        primary_count += 1;
                    }
                    CFModLoader {
                        id: format!("{}-{}", modloader.type_.to_string(), modloader.version),
                        primary,
                    }
                })
                .collect(),
        },
        manifest_type: "minecraftModpack".to_string(),
        name: instance_details.name.clone(),
        version,
        author,
        overrides: "overrides".to_string(),
        files: files
            .iter()
            .filter_map(|file| {
                let required = match file.export_mode {
                    ExportInstanceFileMode::Ignore => return None,
                    ExportInstanceFileMode::Required => true,
                    ExportInstanceFileMode::Optional => false,
                };
                match &file.platform_data {
                    Some(cf_metadata) => Some(ManifestFileReference {
                        project_id: cf_metadata.mod_id,
                        file_id: cf_metadata.file_id,
                        required,
                    }),
                    None => {
                        None
                    }
                }
            })
            .collect(),
    })
}
