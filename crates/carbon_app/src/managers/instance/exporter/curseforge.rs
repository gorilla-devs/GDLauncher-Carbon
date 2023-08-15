use std::{path::PathBuf, sync::Arc};

use crate::{
    api::translation::Translation,
    domain::{
        instance::{
            info::{GameVersion, ModLoaderType, StandardVersion},
            InstanceDetails, InstanceId, Mod,
        },
        modplatforms::curseforge::manifest::{
            CFModLoader, Manifest, ManifestFileReference, Minecraft,
        },
        vtask::VisualTaskId,
    },
    managers::{vtask::VisualTask, AppInner},
};

use super::{InstanceExporter, ArchiveExporter};

#[derive(Debug, Default)]
pub struct CurseForgeZipExporter {}

#[async_trait::async_trait]
impl InstanceExporter for CurseForgeZipExporter {
    async fn export(
        &self,
        app: Arc<AppInner>,
        instance_id: InstanceId,
        output_path: PathBuf,
    ) -> anyhow::Result<VisualTaskId> {
        let instance_manager = app.instance_manager();
        let instance_details = instance_manager.instance_details(instance_id).await?;

        let task = VisualTask::new(Translation::InstanceTaskExport {
            instance_name: instance_details.name.clone(),
        });

        let task_id = app.task_manager().spawn_task(&task).await;

        let mods = instance_manager.list_mods(instance_id).await?;

        let _export_task = tokio::spawn(async move {
            if let Ok((manifest, non_cf_mods)) =
                generate_manfest(&instance_details, &mods, None, "".to_string(), true)
            {
                //TODO: get instance path
                // let archive_exporter = ArchiveExporter::new(output_path, instance_details);
            };
        });

        Ok(task_id)
    }
}

static PRMARY_LOADER_TYPES: [ModLoaderType; 3] = [
    ModLoaderType::Forge,
    ModLoaderType::Fabric,
    ModLoaderType::Quilt,
];

#[derive(Debug, Default)]
struct NonCurseForgeMods {
    pub mods: Vec<Mod>,
}

fn generate_manfest(
    instance_details: &InstanceDetails,
    mods: &Vec<Mod>,
    version: Option<String>,
    author: String,
    use_disabled_as_optional: bool,
) -> anyhow::Result<(Manifest, NonCurseForgeMods)> {
    let mut primary_count = 0;
    let mut non_curseforge_mods = NonCurseForgeMods::default();
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
                            PRMARY_LOADER_TYPES.contains(&modloader.type_) && primary_count == 0;
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
                        project_id: cf_metadata.project_id as i32,
                        file_id: cf_metadata.file_id as i32,
                        required: mod_.enabled || !use_disabled_as_optional,
                    }),
                    None => {
                        non_curseforge_mods.mods.push(mod_.clone());
                        None
                    }
                })
                .collect(),
        },
        non_curseforge_mods,
    ))
}
