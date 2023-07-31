use super::InstanceArchiveImporter;
use crate::{
    api::{instance::import::FEEntity, keys},
    domain::{
        instance::info::{
            CurseforgeModpack, GameVersion, ModLoader, ModLoaderType, Modpack, StandardVersion,
        },
        modplatforms::curseforge::{
            self,
            filters::{ModsParameters, ModsParametersBody},
        },
        vtask::VisualTaskId,
    },
    managers::{instance::InstanceVersionSource, AppInner},
};
use std::{collections::HashSet, path::PathBuf, sync::Arc};
use tokio::{
    fs::create_dir_all,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
    task::spawn_blocking,
};

#[derive(Debug, Default)]
pub struct CurseforgeInstanceArchiveImporter {}

#[async_trait::async_trait]
impl InstanceArchiveImporter for CurseforgeInstanceArchiveImporter {
    async fn import(&self, app: Arc<AppInner>, path: PathBuf) -> anyhow::Result<VisualTaskId> {
        let file_path_clone = path.clone();

        // make sure this is a valid modpack
        let manifest = spawn_blocking(move || {
            let file = std::fs::File::open(file_path_clone)?;
            let mut archive = zip::ZipArchive::new(file)?;
            let manifest: curseforge::manifest::Manifest = {
                let file = archive.by_name("manifest.json")?;
                serde_json::from_reader(file)?
            };

            Ok::<_, anyhow::Error>(manifest)
        })
        .await??;

        let mut content = tokio::fs::read(path).await?;
        let murmur2 = tokio::task::spawn_blocking(move || {
            murmurhash32::murmurhash2({
                // curseforge's weird api
                content.retain(|&x| x != 9 && x != 10 && x != 13 && x != 32);
                &content
            })
        })
        .await?;
        let fp_response = app
            .modplatforms_manager()
            .curseforge
            .get_fingerprints(&vec![murmur2])
            .await?
            .data;
        // let mods_response = app
        //     .modplatforms_manager()
        //     .curseforge
        //     .get_mods(ModsParameters {
        //         body: ModsParametersBody {
        //             mod_ids: fp_response
        //                 .exact_matches
        //                 .iter()
        //                 .map(|m| m.file.mod_id)
        //                 .collect::<Vec<_>>(),
        //         },
        //     })
        //     .await?
        //     .data;
        let mut matches = fp_response
            .exact_fingerprints
            .into_iter()
            .zip(fp_response.exact_matches.into_iter())
            // .zip(mods_response.into_iter())
            // .map(|((fingerprint, fp_match), proj)| (fingerprint, fp_match, proj))
            .collect::<Vec<_>>();
        let modpack = matches
            .first()
            .map(|(_, fp_match)| {
                Modpack::CurseforgeLocal(
                    CurseforgeModpack {
                        project_id: fp_match.file.mod_id,
                        file_id: fp_match.file.id,
                    },
                    path.clone(),
                )
            })
            .unwrap_or_else(|| Modpack::CurseforgeUnmanaged(path.clone()));

        let install_source = InstanceVersionSource::Modpack(modpack);

        // TODO: set instance name and icon properly
        let icon : Option<()> = None;
        let instance_name = "".to_string();
        let created_instance_id = app
            .instance_manager()
            .create_instance(
                app.instance_manager().get_default_group().await?,
                instance_name,
                icon.is_some(),
                install_source,
                "".to_string(),
            )
            .await?;

        let (_, task_id) = app
            .instance_manager()
            .prepare_game(created_instance_id, None)
            .await?;

        Ok(task_id)

    }
}
