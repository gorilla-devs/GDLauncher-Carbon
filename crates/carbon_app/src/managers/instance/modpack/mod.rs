use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        instance::{
            info::{self, CurseforgeModpack, Modpack, ModrinthModpack},
            InstanceId,
        },
        modplatforms::{
            curseforge::{
                self,
                filters::{ModParameters, ModsParameters, ModsParametersBody},
            },
            modrinth::{project::ProjectVersionsFilters, search::ProjectID},
        },
    },
    managers::{instance::InvalidInstanceIdError, ManagerRef},
};

use super::{InstanceData, InstanceManager, InstanceType};

pub mod packinfo;
//mod curseforge;

impl ManagerRef<'_, InstanceManager> {
    pub async fn check_curseforge_modpack_updates(self) -> anyhow::Result<()> {
        let instances = self.instances.read().await;

        let project_ids = instances
            .iter()
            .filter_map(|instance| match &instance.1.type_ {
                InstanceType::Valid(InstanceData {
                    config:
                        info::Instance {
                            modpack: Some(Modpack::Curseforge(modpack)),
                            ..
                        },
                    ..
                }) => Some(modpack.project_id as i32),
                _ => None,
            })
            .collect();

        drop(instances);

        let mut response = self
            .app
            .modplatforms_manager()
            .curseforge
            .get_mods(ModsParameters {
                body: ModsParametersBody {
                    mod_ids: project_ids,
                },
            })
            .await?
            .data
            .into_iter()
            .map(|m| (m.id as u32, m))
            .collect::<HashMap<_, _>>();

        let mut instances = self.instances.write().await;

        for (_, instance) in &mut *instances {
            let InstanceType::Valid(InstanceData {
                config:
                    info::Instance {
                        modpack: Some(Modpack::Curseforge(modpack)),
                        ..
                    },
                modpack_update_curseforge,
                ..
            }) = &mut instance.type_
            else {
                continue;
            };

            let Some(m) = response.get(&modpack.project_id) else {
                continue;
            };

            // Figuring out which version follows the current version is actually somewhat difficult,
            // so we check if this version is the latest by seeing if it's in the latest file list.
            *modpack_update_curseforge = Some(
                !m.latest_files
                    .iter()
                    .any(|file| file.id as u32 == modpack.file_id),
            );
        }

        Ok(())
    }

    pub async fn check_modrinth_modpack_updates(self) -> anyhow::Result<()> {
        let instances = self.instances.read().await;

        let project_ids = instances
            .iter()
            .filter_map(|instance| match &instance.1.type_ {
                InstanceType::Valid(InstanceData {
                    config:
                        info::Instance {
                            modpack: Some(Modpack::Modrinth(modpack)),
                            ..
                        },
                    ..
                }) => Some(modpack.project_id.clone()),
                _ => None,
            });

        let mut responses_future = futures::future::join_all(
            project_ids
                .map(|project_id| async move {
                    (
                        project_id.clone(),
                        self.app
                            .modplatforms_manager()
                            .modrinth
                            .get_project_versions(ProjectVersionsFilters {
                                project_id: ProjectID(project_id),
                                loaders: Vec::new(),
                                game_versions: Vec::new(),
                            })
                            .await,
                    )
                })
                .collect::<Vec<_>>(),
        );

        drop(instances);

        let responses = responses_future
            .await
            .into_iter()
            .map(|(pid, response)| response.map(|r| (pid, r)))
            .collect::<Result<HashMap<_, _>, _>>()?;

        let mut instances = self.instances.write().await;

        for (_, instance) in &mut *instances {
            let InstanceType::Valid(InstanceData {
                config:
                    info::Instance {
                        modpack: Some(Modpack::Modrinth(modpack)),
                        ..
                    },
                modpack_update_modrinth,
                ..
            }) = &mut instance.type_
            else {
                continue;
            };

            let Some(m) = responses.get(&modpack.project_id) else {
                continue;
            };

            *modpack_update_modrinth = Some(
                m.0.first()
                    .map(|v| v.id != modpack.version_id)
                    .unwrap_or(false),
            );
        }

        Ok(())
    }

    pub async fn change_modpack_version(
        self,
        instance_id: InstanceId,
        modpack: Modpack,
    ) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let data = instance.data()?;
        let Some(modpack) = data.config.modpack.clone() else {
            anyhow::bail!("Instance does not have an associated modpack");
        };

        let runtime_path = self.app.settings_manager().runtime_path.clone();
        let instance_path = runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath);

        drop(instances);

        let pack_version_text = serde_json::to_string(&PackVersionFile::from(modpack))?;

        let setup_path = instance_path.get_root().join(".setup");
        let update_file_path = setup_path.join("change-pack-version.json");

        let update_file = runtime_path.get_temp().maketmpfile().await?;

        tokio::fs::create_dir_all(setup_path).await?;
        tokio::fs::write(&*update_file, pack_version_text).await?;
        drop(update_file);

        todo!("call run.rs")
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
