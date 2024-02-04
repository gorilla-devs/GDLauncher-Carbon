use std::collections::HashMap;

use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        instance::{
            info::{self, CurseforgeModpack, Modpack, ModpackInfo, ModrinthModpack},
            InstanceId,
        },
        modplatforms::{
            curseforge::{
                self,
                filters::{
                    ModFilesParameters, ModFilesParametersQuery, ModParameters, ModsParameters,
                    ModsParametersBody,
                },
            },
            modrinth::{project::ProjectVersionsFilters, search::ProjectID},
        },
        vtask::VisualTaskId,
    },
    managers::{instance::InvalidInstanceIdError, ManagerRef},
};

use super::{InstanceData, InstanceManager, InstanceType};

pub mod packinfo;

impl ManagerRef<'_, InstanceManager> {
    pub async fn check_curseforge_modpack_updates(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data()?;

        let Some(ModpackInfo {
            modpack: Modpack::Curseforge(modpack),
            ..
        }) = data.config.modpack.clone()
        else {
            bail!("Instance is not a curseforge modpack");
        };

        drop(instances);

        let response = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mod_files(ModFilesParameters {
                mod_id: modpack.project_id as i32,
                query: ModFilesParametersQuery {
                    game_version: None,
                    mod_loader_type: None,
                    game_version_type_id: None,
                    index: None,
                    page_size: None,
                },
            })
            .await?;

        let has_update = !response
            .data
            .first()
            .map(|file| file.id as u32 == modpack.file_id)
            .unwrap_or(false);

        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data_mut()?;
        data.modpack_update_curseforge = Some(has_update);

        Ok(())
    }

    pub async fn check_modrinth_modpack_updates(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data()?;

        let Some(ModpackInfo {
            modpack: Modpack::Modrinth(modpack),
            ..
        }) = data.config.modpack.clone()
        else {
            bail!("Instance is not a modrinth modpack");
        };

        drop(instances);

        let response = self
            .app
            .modplatforms_manager()
            .modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(modpack.project_id),
                game_versions: Some(Vec::new()),
                loaders: Some(Vec::new()),
                offset: None,
                limit: None,
            })
            .await?;

        let has_update = response
            .0
            .first()
            .map(|v| v.id != modpack.version_id)
            .unwrap_or(false);

        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data_mut()?;
        data.modpack_update_modrinth = Some(has_update);

        Ok(())
    }

    pub async fn change_modpack(
        self,
        instance_id: InstanceId,
        modpack: Modpack,
    ) -> anyhow::Result<VisualTaskId> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data()?;
        if data.config.modpack.is_none() {
            anyhow::bail!("Instance does not have an associated modpack");
        }

        let runtime_path = self.app.settings_manager().runtime_path.clone();
        let instance_path = runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath);

        drop(instances);

        let pack_version_text = serde_json::to_string(&PackVersionFile::from(modpack))?;

        let setup_path = instance_path.get_root().join(".setup");

        if setup_path.exists() {
            anyhow::bail!("Instance has not completed the setup phase, attempting to change the modpack may irreparably damage it.");
        }

        tokio::fs::create_dir_all(&setup_path).await?;

        let update_file_path = setup_path.join("change-pack-version.json");

        runtime_path
            .get_temp()
            .write_file_atomic(update_file_path, pack_version_text)
            .await?;

        self.app
            .instance_manager()
            .prepare_game(instance_id, None, None, true)
            .await
            .map(|r| r.1)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "platform")]
pub enum PackVersionFile {
    Curseforge {
        project_id: u32,
        file_id: u32,
    },
    Modrinth {
        project_id: String,
        version_id: String,
    },
}

impl From<Modpack> for PackVersionFile {
    fn from(value: Modpack) -> Self {
        match value {
            Modpack::Curseforge(CurseforgeModpack {
                project_id,
                file_id,
            }) => Self::Curseforge {
                project_id,
                file_id,
            },
            Modpack::Modrinth(ModrinthModpack {
                project_id,
                version_id,
            }) => Self::Modrinth {
                project_id,
                version_id,
            },
        }
    }
}

impl From<PackVersionFile> for Modpack {
    fn from(value: PackVersionFile) -> Self {
        match value {
            PackVersionFile::Curseforge {
                project_id,
                file_id,
            } => Self::Curseforge(CurseforgeModpack {
                project_id,
                file_id,
            }),
            PackVersionFile::Modrinth {
                project_id,
                version_id,
            } => Self::Modrinth(ModrinthModpack {
                project_id,
                version_id,
            }),
        }
    }
}
