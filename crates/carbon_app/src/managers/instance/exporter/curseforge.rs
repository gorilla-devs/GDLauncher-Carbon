use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
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

use super::{ArchiveExporter, InstanceExporter, PRIMARY_LOADER_TYPES};

#[derive(Debug, Default)]
pub struct CurseForgeZipExporter {}

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
            if let Ok((manifest, non_cf_mods)) =
                generate_manifest(&instance_details, &mods, None, "".to_string(), true)
            {
                //TODO: get instance path
                // let archive_exporter = ArchiveExporter::new(output_path, instance_details);
            };
        });

        Ok(task_id)
    }
}

#[derive(Debug, Default)]
struct NonCurseForgeFiles {
    pub files: Vec<PathBuf>,
}


fn generate_manifest(
    instance_details: &InstanceDetails,
    instance_path: InstancePath,
    mods: &Vec<Mod>,
    version: Option<String>,
    author: String,
    use_disabled_as_optional: bool,
) -> anyhow::Result<(Manifest, NonCurseForgeFiles)> {
    let mut primary_count = 0;
    let mut non_curseforge_mods = NonCurseForgeFiles::default();
    let mods_path = instance_path.get_mods_path();
    Ok((
        Manifest {
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
            files: mods
                .iter()
                .filter_map(|mod_| match &mod_.curseforge {
                    Some(cf_metadata) => Some(ManifestFileReference {
                        project_id: cf_metadata.project_id,
                        file_id: cf_metadata.file_id,
                        required: mod_.enabled || !use_disabled_as_optional,
                    }),
                    None => {
                        non_curseforge_mods
                            .files
                            .push(mods_path.join(mod_.filename));
                        None
                    }
                })
                .collect(),
        },
        non_curseforge_mods,
    ))
}
