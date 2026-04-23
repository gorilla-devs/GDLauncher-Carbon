use self::export::InstanceExportManager;
use self::importer::InstanceImportManager;
use self::log::GameLog;
use self::run::{LaunchState, PersistenceManager};
use super::ManagerRef;
use super::metadata::cache;
use super::modplatforms::curseforge::CurseForge;
use super::vtask::{TaskState, VisualTask};
use crate::api::keys::instance::*;
use crate::api::translation::Translation;
use crate::domain::instance::info::{GameVersion, InstanceIcon, Modpack};
use crate::domain::instance::{
    self as domain, GameLogId, GroupId, InstanceFolder, InstanceId, InstanceModpackInfo,
};
use crate::domain::java::{SYSTEM_JAVA_PROFILE_NAME_PREFIX, SystemJavaProfileName};
use crate::domain::vtask::VisualTaskId;
use crate::livenesstracker::LivenessTracker;
use crate::managers::instance::modpack::PackVersionFile;
use anyhow::bail;
use anyhow::{Context, anyhow};
use carbon_platforms::ModPlatform;
use carbon_platforms::curseforge::filters::{ModFileParameters, ModParameters};
use carbon_platforms::modrinth::search::{ProjectID, VersionID};
use carbon_repos::db::read_filters::StringFilter;
use carbon_repos::db::{self, read_filters::IntFilter};
use carbon_repos::pcr::Direction;
use chrono::{DateTime, Utc};
use daedalus::minecraft::MinecraftJavaProfile;
use dashmap::DashMap;
use db::instance::Data as CachedInstance;
use domain::info;
use fs_extra::dir::CopyOptions;
use futures::future::BoxFuture;
use futures::{Future, join};
use serde::Serialize;
use serde_json::error::Category as JsonErrorType;
use specta::Type;
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, io, ops::Deref, path::PathBuf};
use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard, RwLock, watch};
use tracing::{info, trace};
use unicode_segmentation::UnicodeSegmentation;

pub mod explore;
pub mod export;
pub mod importer;
pub mod installer;
pub mod log;
pub mod modpack;
mod mods;
pub mod run;
mod schema;

#[derive(Debug)]
pub struct InstanceManager {
    pub(crate) instances: RwLock<HashMap<InstanceId, Instance>>,
    index_lock: Mutex<()>,
    // seperate lock to prevent a deadlock with the index lock
    path_lock: Mutex<()>,
    // Per-instance operation locks to allow parallel updates to different instances
    // while preventing concurrent updates to the same instance
    instance_op_locks: Arc<DashMap<InstanceId, Arc<Mutex<()>>>>,
    loaded_icon: Mutex<Option<(String, Vec<u8>)>>,
    persistence_manager: PersistenceManager,
    import_manager: InstanceImportManager,
    export_manager: InstanceExportManager,
    game_logs: RwLock<
        HashMap<
            GameLogId,
            (
                InstanceId,
                chrono::DateTime<chrono::Local>,
                watch::Receiver<GameLog>,
            ),
        >,
    >,
    modpack_info_semaphore: Mutex<()>,
    pub any_instance_running: Arc<watch::Sender<bool>>,
    instance_running_tracker: Arc<LivenessTracker>,
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceManager {
    pub fn new() -> Self {
        let any_instance_running = Arc::new(watch::channel(false).0);

        Self {
            instances: RwLock::new(HashMap::new()),
            index_lock: Mutex::new(()),
            path_lock: Mutex::new(()),
            instance_op_locks: Arc::new(DashMap::new()),
            loaded_icon: Mutex::new(None),
            persistence_manager: PersistenceManager::new(),
            import_manager: InstanceImportManager::new(),
            export_manager: InstanceExportManager::new(),
            game_logs: RwLock::new(HashMap::new()),
            modpack_info_semaphore: Mutex::new(()),
            any_instance_running: any_instance_running.clone(),
            instance_running_tracker: LivenessTracker::new(move |count| {
                drop(any_instance_running.send_replace(count != 0))
            }),
        }
    }
}

const MAX_PATH: usize = if cfg!(windows) { 260 } else { 4096 };
const ILLEGAL_CHARS: &[char] = &['/', ':', '\\', '<', '>', '*', '|', '"', '?', '^'];
const ILLEGAL_NAMES: &[&str] = &[
    "con", "prn", "aux", "clock$", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
    "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

fn sanitize_name(name: &str) -> String {
    let mut sanitized = name.trim().to_string();

    if ILLEGAL_NAMES.contains(&(&name.to_lowercase() as &str)) {
        sanitized = format!("_{}", sanitized);
    }

    if sanitized.starts_with('.') || sanitized.starts_with('~') {
        sanitized.replace_range(0..1, "_");
    }

    if sanitized.ends_with('.') || sanitized.ends_with('~') {
        sanitized.replace_range(sanitized.len() - 1.., "_");
    }

    sanitized
        .chars()
        .map(|c| if ILLEGAL_CHARS.contains(&c) { '_' } else { c })
        .collect()
}

fn truncate_name(name: &str, instance_path: &Path) -> String {
    let available_length = MAX_PATH - 3 /* for discriminators */ - 1 /* for _ the separator */ - 1 /* for final null character (on windows) */ - path_length(instance_path);
    name.graphemes(true)
        .take_while(|g| {
            let new_len = path_length(&instance_path.join(g));
            new_len <= available_length
        })
        .collect()
}

#[cfg(windows)]
fn path_length(path: &Path) -> usize {
    path.as_os_str().encode_wide().count()
}

#[cfg(not(windows))]
fn path_length(path: &Path) -> usize {
    path.as_os_str().len()
}

impl<'s> ManagerRef<'s, InstanceManager> {
    pub async fn launch_background_tasks(self) {
        let _ = self.scan_instances().await;
        self.import_manager().launch_background_tasks();
    }

    pub async fn scan_instances(self) -> anyhow::Result<()> {
        let instance_cache = self
            .app
            .prisma_client
            .instance()
            .find_many(vec![])
            .exec()
            .await?;

        let instance_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path();

        let mut stream = tokio::fs::read_dir(instance_path).await?;

        let updates_semaphore = Arc::new(tokio::sync::Semaphore::new(20));

        while let Some(dir) = stream.next_entry().await? {
            let path = dir.path();

            let shortpath = path
                .file_name()
                .expect("path given to scan_instance should never have a null filename")
                .to_str()
                .expect("current GDL versions only support UTF8 paths")
                .to_string();

            let cached = instance_cache
                .iter()
                .find(|instance| instance.shortpath == shortpath);

            let Some(mut instance) = self.scan_instance(shortpath, path, cached).await? else {
                continue;
            };
            let InstanceType::Valid(data) = &instance.type_ else {
                continue;
            };

            let instance_id = match cached {
                Some(cached) => InstanceId(cached.id),
                None => {
                    self.add_instance(
                        data.config.name.clone(),
                        instance.shortpath.clone(),
                        self.get_default_group().await?,
                    )
                    .await?
                }
            };

            let mut instances = self.instances.write().await;

            if let (
                Instance {
                    type_: InstanceType::Valid(data),
                    ..
                },
                Some(Instance {
                    type_: InstanceType::Valid(old_data),
                    ..
                }),
            ) = (&mut instance, instances.remove(&instance_id))
            {
                data.state = old_data.state;
            }

            instances.insert(instance_id, instance);
            drop(instances);

            self.app
                .meta_cache_manager()
                .queue_caching(
                    crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
                    false,
                )
                .await;

            let app = self.app.clone();
            let updates_semaphore = Arc::clone(&updates_semaphore);
            tokio::task::spawn(async move {
                let _permit = updates_semaphore.acquire().await.unwrap();

                trace!("Instance modpack update for {instance_id}",);

                // ignore errors
                let (_, _) = join!(
                    app.instance_manager()
                        .check_curseforge_modpack_updates(instance_id),
                    app.instance_manager()
                        .check_modrinth_modpack_updates(instance_id),
                );

                tokio::time::sleep(Duration::from_millis(10)).await;
            });
        }

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(())
    }

    /// Scan the given path as an instance folder.
    ///
    /// If cached is Some an Instance will always be returned, though it may be missing files.
    /// If cached is None an Instance will only be returned if a config file is present.
    async fn scan_instance(
        self,
        shortpath: String,
        path: PathBuf,
        cached: Option<&CachedInstance>,
    ) -> anyhow::Result<Option<Instance>> {
        let config_path = path.join("instance.json");

        let config_text = match tokio::fs::read_to_string(config_path.clone()).await {
            Ok(x) => x,
            Err(e) => {
                // if we aren't already tracking this instance just ignore it.
                if cached.is_some() {
                    let invalid_type = match e.kind() {
                        io::ErrorKind::NotFound => InvalidConfiguration::NoFile,
                        _ => InvalidConfiguration::IoError(e.to_string()),
                    };

                    return Ok(Some(Instance {
                        shortpath: shortpath.clone(),
                        type_: InstanceType::Invalid(invalid_type),
                    }));
                } else {
                    return Ok(None);
                }
            }
        };

        match schema::parse_and_update_instance_config(self.app.clone(), &config_text, config_path)
            .await
        {
            Ok(config) => {
                let icon_revision = match &config.icon {
                    InstanceIcon::Default => None,
                    InstanceIcon::RelativePath(_) => Some(1),
                };

                let instance = InstanceData {
                    favorite: cached.map(|cached| cached.favorite).unwrap_or(false),
                    config,
                    state: run::LaunchState::Inactive { failed_task: None },
                    modpack_update_curseforge: None,
                    modpack_update_modrinth: None,
                    icon_revision,
                };

                Ok(Some(Instance {
                    shortpath: shortpath.clone(),
                    type_: InstanceType::Valid(instance),
                }))
            }
            Err(e) => {
                let try_downcast = e.downcast_ref::<serde_json::Error>();
                let error = InvalidConfiguration::Invalid(ConfigurationParseError {
                    type_: try_downcast
                        .map(|e| match e.classify() {
                            JsonErrorType::Data => ConfigurationParseErrorType::Data,
                            JsonErrorType::Syntax => ConfigurationParseErrorType::Syntax,
                            JsonErrorType::Eof => ConfigurationParseErrorType::Eof,
                            JsonErrorType::Io => unreachable!(),
                        })
                        .unwrap_or(ConfigurationParseErrorType::Unknown),
                    line: try_downcast.map(|e| e.line()).unwrap_or_default() as u32, // will panic with more lines but that dosen't really seem like a problem
                    message: try_downcast
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| e.to_string()),
                    config_text,
                });

                Ok(Some(Instance {
                    shortpath,
                    type_: InstanceType::Invalid(error),
                }))
            }
        }
    }

    pub async fn list_groups(self) -> anyhow::Result<Vec<ListGroup>> {
        use db::{instance, instance_group};

        let groups = self
            .app
            .prisma_client
            .instance_group()
            .find_many(vec![])
            .order_by(instance_group::OrderByParam::GroupIndex(Direction::Asc))
            .with(
                db::instance_group::instances::fetch(vec![])
                    .order_by(instance::OrderByParam::Index(Direction::Asc)),
            )
            .exec()
            .await?;

        let active_instances = self.instances.read().await;
        Ok(groups
            .into_iter()
            .map(|group| ListGroup {
                id: GroupId(group.id),
                name: group.name,
                library_position: group.library_position,
                instances: group
                    .instances
                    .expect("instance groups were requested with group list yet are not present")
                    .into_iter()
                    .filter_map(
                        |instance| match active_instances.get(&InstanceId(instance.id)) {
                            Some(data) => Some((instance, &data.type_)),
                            None => None,
                        },
                    )
                    .map(|(instance, status)| ListInstance {
                        id: InstanceId(instance.id),
                        group_id: GroupId(instance.group_id),
                        index: instance.index,
                        library_position: instance.library_position,
                        name: instance.name,
                        favorite: instance.favorite,
                        icon_revision: match &status {
                            InstanceType::Valid(data) => data.icon_revision,
                            InstanceType::Invalid(_) => None,
                        },
                        status: match status {
                            InstanceType::Valid(status) => {
                                ListInstanceStatus::Valid(ValidListInstance {
                                    mc_version: match &status.config.game_configuration.version {
                                        Some(GameVersion::Standard(version)) => {
                                            Some(version.release.clone())
                                        }
                                        Some(GameVersion::Custom(name)) => Some(name.clone()),
                                        None => None,
                                    },
                                    modloader: match &status.config.game_configuration.version {
                                        Some(GameVersion::Standard(version)) => {
                                            match version.modloaders.iter().next() {
                                                Some(modloader) => Some(modloader.type_),
                                                None => None,
                                            }
                                        }
                                        Some(GameVersion::Custom(_)) => None,
                                        None => None,
                                    },
                                    modloader_version: match &status
                                        .config
                                        .game_configuration
                                        .version
                                    {
                                        Some(GameVersion::Standard(version)) => version
                                            .modloaders
                                            .iter()
                                            .next()
                                            .map(|m| m.version.clone()),
                                        Some(GameVersion::Custom(_)) => None,
                                        None => None,
                                    },
                                    modpack: status
                                        .config
                                        .modpack
                                        .as_ref()
                                        .map(|modpack| modpack.modpack.clone()),
                                    state: (&status.state).into(),
                                })
                            }
                            InstanceType::Invalid(status) => {
                                ListInstanceStatus::Invalid(match status {
                                    InvalidConfiguration::NoFile => {
                                        InvalidListInstance::JsonMissing
                                    }
                                    InvalidConfiguration::Invalid(error) => {
                                        InvalidListInstance::JsonError(error.clone())
                                    }
                                    InvalidConfiguration::IoError(error) => {
                                        InvalidListInstance::Other(error.clone())
                                    }
                                })
                            }
                        },
                        locked: match status {
                            InstanceType::Valid(status) => status
                                .config
                                .modpack
                                .as_ref()
                                .map(|modpack| modpack.locked)
                                .unwrap_or(false),
                            InstanceType::Invalid(status) => false,
                        },
                        last_played: match status {
                            InstanceType::Valid(status) => status.config.last_played,
                            InstanceType::Invalid(status) => None,
                        },
                        date_created: match status {
                            InstanceType::Valid(status) => status.config.date_created,
                            InstanceType::Invalid(status) => DateTime::default(),
                        },
                        date_updated: match status {
                            InstanceType::Valid(status) => status.config.date_updated,
                            InstanceType::Invalid(status) => DateTime::default(),
                        },
                        seconds_played: match status {
                            InstanceType::Valid(status) => status.config.seconds_played,
                            InstanceType::Invalid(status) => 0,
                        },
                    })
                    .collect::<Vec<_>>(),
            })
            .collect::<Vec<_>>())
    }

    /// Move the given group to a position in the library based on the target.
    /// Groups can now be interleaved with ungrouped instances using libraryPosition.
    pub async fn move_group(self, group: GroupId, target: GroupMoveTarget) -> anyhow::Result<()> {
        use db::instance::{
            SetParam as InstanceSetParam, UniqueWhereParam as InstanceUniqueWhereParam,
            WhereParam as InstanceWhereParam,
        };
        use db::instance_group::{SetParam, UniqueWhereParam, WhereParam};

        // lock indexes while we're changing them
        let _index_lock = self.index_lock.lock().await;

        // Get the group we're moving
        let moving_group = self
            .app
            .prisma_client
            .instance_group()
            .find_unique(UniqueWhereParam::IdEquals(*group))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("GroupId is not in database, this should never happen"))?;

        let start_pos = moving_group.library_position;

        // Determine the target libraryPosition based on the target type
        let target_pos = match target {
            GroupMoveTarget::BeforeGroup(target_group_id) => {
                let target_group = self
                    .app
                    .prisma_client
                    .instance_group()
                    .find_unique(UniqueWhereParam::IdEquals(*target_group_id))
                    .exec()
                    .await?
                    .ok_or_else(|| anyhow!("Target GroupId is not in database"))?;

                target_group.library_position.ok_or_else(|| {
                    anyhow!("Target group has no libraryPosition (is it the default group?)")
                })?
            }
            GroupMoveTarget::BeforeInstance(instance_id) => {
                // Instance must be in the default group (ungrouped) — detected via
                // library_position being set (only default-group instances have one).
                let instance = self
                    .app
                    .prisma_client
                    .instance()
                    .find_unique(InstanceUniqueWhereParam::IdEquals(*instance_id))
                    .exec()
                    .await?
                    .ok_or_else(|| anyhow!("InstanceId is not in database"))?;

                instance.library_position.ok_or_else(|| {
                    anyhow!(
                        "Can only position a group before ungrouped instances (instances in default group)"
                    )
                })?
            }
            GroupMoveTarget::EndOfLibrary => {
                // Find the maximum libraryPosition across ungrouped instances and groups.
                // library_position is only set on default-group instances, so the
                // filter alone is sufficient (no need to join against the default group).
                let max_instance_pos: Option<i32> = self
                    .app
                    .prisma_client
                    .instance()
                    .find_first(vec![InstanceWhereParam::LibraryPosition(
                        db::read_filters::IntNullableFilter::Not(None),
                    )])
                    .order_by(db::instance::OrderByParam::LibraryPosition(
                        carbon_repos::pcr::Direction::Desc,
                    ))
                    .exec()
                    .await?
                    .and_then(|i| i.library_position);

                let max_group_pos: Option<i32> = self
                    .app
                    .prisma_client
                    .instance_group()
                    .find_first(vec![WhereParam::LibraryPosition(
                        db::read_filters::IntNullableFilter::Not(None),
                    )])
                    .order_by(db::instance_group::OrderByParam::LibraryPosition(
                        carbon_repos::pcr::Direction::Desc,
                    ))
                    .exec()
                    .await?
                    .and_then(|g| g.library_position);

                let max_pos = max_instance_pos
                    .unwrap_or(0)
                    .max(max_group_pos.unwrap_or(0));
                max_pos + 1
            }
        };

        let Some(start_pos) = start_pos else {
            // Group has no libraryPosition (shouldn't happen for non-default groups)
            bail!("Group has no libraryPosition - cannot move");
        };

        if start_pos == target_pos {
            return Ok(());
        }

        // Shift libraryPositions of items between start and target
        // For groups:
        if start_pos < target_pos {
            // Moving forward: shift items in (start, target] down by 1
            self.app
                .prisma_client
                .instance_group()
                .update_many(
                    vec![
                        WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Gt(
                            start_pos,
                        )),
                        WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Lte(
                            target_pos - 1,
                        )),
                    ],
                    vec![SetParam::DecrementLibraryPosition(1)],
                )
                .exec()
                .await?;

            // For ungrouped instances (library_position is only set on them):
            self.app
                .prisma_client
                .instance()
                .update_many(
                    vec![
                        InstanceWhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Gt(start_pos),
                        ),
                        InstanceWhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Lte(target_pos - 1),
                        ),
                    ],
                    vec![InstanceSetParam::DecrementLibraryPosition(1)],
                )
                .exec()
                .await?;

            // Update the group to target - 1 (since items shifted down)
            self.app
                .prisma_client
                .instance_group()
                .update(
                    UniqueWhereParam::IdEquals(*group),
                    vec![SetParam::SetLibraryPosition(Some(target_pos - 1))],
                )
                .exec()
                .await?;
        } else {
            // Moving backward: shift items in [target, start) up by 1
            self.app
                .prisma_client
                .instance_group()
                .update_many(
                    vec![
                        WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Gte(
                            target_pos,
                        )),
                        WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Lt(
                            start_pos,
                        )),
                    ],
                    vec![SetParam::IncrementLibraryPosition(1)],
                )
                .exec()
                .await?;

            // For ungrouped instances (library_position is only set on them):
            self.app
                .prisma_client
                .instance()
                .update_many(
                    vec![
                        InstanceWhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Gte(target_pos),
                        ),
                        InstanceWhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Lt(start_pos),
                        ),
                    ],
                    vec![InstanceSetParam::IncrementLibraryPosition(1)],
                )
                .exec()
                .await?;

            // Update the group to target
            self.app
                .prisma_client
                .instance_group()
                .update(
                    UniqueWhereParam::IdEquals(*group),
                    vec![SetParam::SetLibraryPosition(Some(target_pos))],
                )
                .exec()
                .await?;
        }

        // Also keep groupIndex in sync for backwards compatibility
        // (This maintains the old ordering system while we transition)
        let all_groups = self
            .app
            .prisma_client
            .instance_group()
            .find_many(vec![WhereParam::LibraryPosition(
                db::read_filters::IntNullableFilter::Not(None),
            )])
            .order_by(db::instance_group::OrderByParam::LibraryPosition(
                carbon_repos::pcr::Direction::Asc,
            ))
            .exec()
            .await?;

        for (idx, g) in all_groups.iter().enumerate() {
            self.app
                .prisma_client
                .instance_group()
                .update(
                    UniqueWhereParam::IdEquals(g.id),
                    vec![SetParam::SetGroupIndex(idx as i32)],
                )
                .exec()
                .await?;
        }

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        Ok(())
    }

    /// Move the given instance to the index directly before `target` in the target instance group.
    /// If `target` is None, move to the end of the instance group.
    /// Also handles libraryPosition for instances in the default group (library root).
    pub async fn move_instance(
        self,
        instance: InstanceId,
        target: InstanceMoveTarget,
    ) -> anyhow::Result<()> {
        use db::instance::{SetParam, UniqueWhereParam, WhereParam};

        // Materialize the default group before taking index_lock (see create_group).
        let default_group_id = self.get_default_group().await?;

        // lock indexes while we're changing them
        let _index_lock = self.index_lock.lock().await;

        let (start_group, start_idx, start_library_pos) = {
            let instance = self
                .app
                .prisma_client
                .instance()
                .find_unique(UniqueWhereParam::IdEquals(*instance))
                .exec()
                .await?
                .ok_or_else(|| {
                    anyhow!("InstanceId is not in database, this should never happen")
                })?;

            (
                GroupId(instance.group_id),
                instance.index,
                instance.library_position,
            )
        };

        let (target_group, target_idx, target_library_pos) = match target {
            InstanceMoveTarget::Before(target) => {
                let inst = self
                    .app
                    .prisma_client
                    .instance()
                    .find_unique(UniqueWhereParam::IdEquals(*target))
                    .exec()
                    .await?
                    .ok_or_else(|| {
                        anyhow!("InstanceId is not in database, this should never happen")
                    })?;

                (GroupId(inst.group_id), inst.index, inst.library_position)
            }
            InstanceMoveTarget::BeginningOfGroup(group) => {
                // If target is default group, find the minimum libraryPosition
                let lib_pos = if group == default_group_id {
                    let min_pos = self
                        .app
                        .prisma_client
                        .instance()
                        .find_first(vec![
                            WhereParam::GroupId(IntFilter::Equals(*group)),
                            WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Not(
                                None,
                            )),
                        ])
                        .order_by(db::instance::OrderByParam::LibraryPosition(
                            carbon_repos::pcr::Direction::Asc,
                        ))
                        .exec()
                        .await?
                        .and_then(|i| i.library_position);

                    // If no instances with libraryPosition, start at 0
                    Some(min_pos.unwrap_or(0))
                } else {
                    None
                };

                // Indices are prepend-allocated (see `next_instance_index`), so
                // the "beginning" is strictly less than the current minimum, not
                // a fixed 0.
                let min_idx: Option<i32> = self
                    .app
                    .prisma_client
                    .instance()
                    .find_first(vec![WhereParam::GroupId(IntFilter::Equals(*group))])
                    .order_by(db::instance::OrderByParam::Index(
                        carbon_repos::pcr::Direction::Asc,
                    ))
                    .exec()
                    .await?
                    .map(|i| i.index);

                let target_idx = min_idx.map(|n| n - 1).unwrap_or(0);
                (group, target_idx, lib_pos)
            }
            InstanceMoveTarget::EndOfGroup(group) => {
                let target_idx = self
                    .app
                    .prisma_client
                    .instance()
                    .count(vec![WhereParam::GroupId(IntFilter::Equals(*group))])
                    .exec()
                    .await? as i32;

                // If target is default group, find the maximum libraryPosition + 1
                let lib_pos = if group == default_group_id {
                    let max_instance_pos = self
                        .app
                        .prisma_client
                        .instance()
                        .find_first(vec![
                            WhereParam::GroupId(IntFilter::Equals(*group)),
                            WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Not(
                                None,
                            )),
                        ])
                        .order_by(db::instance::OrderByParam::LibraryPosition(
                            carbon_repos::pcr::Direction::Desc,
                        ))
                        .exec()
                        .await?
                        .and_then(|i| i.library_position);

                    // Also check groups for max libraryPosition
                    let max_group_pos = self
                        .app
                        .prisma_client
                        .instance_group()
                        .find_first(vec![db::instance_group::WhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Not(None),
                        )])
                        .order_by(db::instance_group::OrderByParam::LibraryPosition(
                            carbon_repos::pcr::Direction::Desc,
                        ))
                        .exec()
                        .await?
                        .and_then(|g| g.library_position);

                    let max_pos = max_instance_pos
                        .unwrap_or(0)
                        .max(max_group_pos.unwrap_or(0));
                    Some(max_pos + 1)
                } else {
                    None
                };

                (group, target_idx, lib_pos)
            }
            InstanceMoveTarget::BeforeGroup(group_id) => {
                // Position instance before a folder (at library root level)
                // Get the folder's libraryPosition
                let target_folder = self
                    .app
                    .prisma_client
                    .instance_group()
                    .find_unique(db::instance_group::UniqueWhereParam::IdEquals(*group_id))
                    .exec()
                    .await?
                    .ok_or_else(|| anyhow!("GroupId is not in database"))?;

                let lib_pos = target_folder
                    .library_position
                    .ok_or_else(|| anyhow!("Target folder has no libraryPosition"))?;

                // The instance will be moved to the default group with target libraryPosition
                let target_idx = self
                    .app
                    .prisma_client
                    .instance()
                    .count(vec![WhereParam::GroupId(IntFilter::Equals(
                        *default_group_id,
                    ))])
                    .exec()
                    .await? as i32;

                (default_group_id, target_idx, Some(lib_pos))
            }
        };

        let index_shifts = if start_group == target_group {
            vec![match (start_idx, target_idx) {
                (start, target) if start < target => self.app.prisma_client.instance().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(*target_group)),
                        WhereParam::Index(IntFilter::Gt(start)),
                        WhereParam::Index(IntFilter::Lt(target)),
                    ],
                    vec![SetParam::DecrementIndex(1)],
                ),
                (start, target) if start > target => self.app.prisma_client.instance().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(*target_group)),
                        WhereParam::Index(IntFilter::Gte(target)),
                        WhereParam::Index(IntFilter::Lt(start)),
                    ],
                    vec![SetParam::IncrementIndex(1)],
                ),
                _ => return Ok(()),
            }]
        } else {
            vec![
                self.app.prisma_client.instance().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(*start_group)),
                        WhereParam::Index(IntFilter::Gt(start_idx)),
                    ],
                    vec![SetParam::DecrementIndex(1)],
                ),
                self.app.prisma_client.instance().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(*target_group)),
                        WhereParam::Index(IntFilter::Gte(target_idx)),
                    ],
                    vec![SetParam::IncrementIndex(1)],
                ),
            ]
        };

        // When moving forward in the same group, the source ends up at target - 1
        // because we're shifting items in (start, target) down, not including target
        let final_idx = if start_group == target_group && start_idx < target_idx {
            target_idx - 1
        } else {
            target_idx
        };

        // Handle libraryPosition updates
        let mut update_params = vec![
            SetParam::SetGroupId(*target_group),
            SetParam::SetIndex(final_idx),
        ];

        // Determine the new libraryPosition
        let new_library_pos = if target_group == default_group_id {
            // Moving to default group: need to set libraryPosition
            target_library_pos
        } else {
            // Moving to a folder: clear libraryPosition
            None
        };

        update_params.push(SetParam::SetLibraryPosition(new_library_pos));

        // If moving TO default group and inserting before an item, shift library positions
        if target_group == default_group_id {
            if let Some(target_lib_pos) = target_library_pos {
                // Only shift if we have a target position (not end of group with no items)
                if start_library_pos != Some(target_lib_pos) {
                    // Shift libraryPosition of items at or after target_lib_pos
                    self.app
                        .prisma_client
                        .instance()
                        .update_many(
                            vec![
                                WhereParam::GroupId(IntFilter::Equals(*default_group_id)),
                                WhereParam::LibraryPosition(
                                    db::read_filters::IntNullableFilter::Gte(target_lib_pos),
                                ),
                                // Don't shift the instance we're moving
                                WhereParam::Id(db::read_filters::IntFilter::Not(*instance)),
                            ],
                            vec![SetParam::IncrementLibraryPosition(1)],
                        )
                        .exec()
                        .await?;

                    // Also shift groups
                    self.app
                        .prisma_client
                        .instance_group()
                        .update_many(
                            vec![db::instance_group::WhereParam::LibraryPosition(
                                db::read_filters::IntNullableFilter::Gte(target_lib_pos),
                            )],
                            vec![db::instance_group::SetParam::IncrementLibraryPosition(1)],
                        )
                        .exec()
                        .await?;
                }
            }
        }

        // If moving FROM default group, shift library positions to fill the gap
        if start_group == default_group_id && target_group != default_group_id {
            if let Some(start_lib_pos) = start_library_pos {
                // Decrement libraryPosition of items after start_lib_pos
                self.app
                    .prisma_client
                    .instance()
                    .update_many(
                        vec![
                            WhereParam::GroupId(IntFilter::Equals(*default_group_id)),
                            WhereParam::LibraryPosition(db::read_filters::IntNullableFilter::Gt(
                                start_lib_pos,
                            )),
                        ],
                        vec![SetParam::DecrementLibraryPosition(1)],
                    )
                    .exec()
                    .await?;

                // Also shift groups
                self.app
                    .prisma_client
                    .instance_group()
                    .update_many(
                        vec![db::instance_group::WhereParam::LibraryPosition(
                            db::read_filters::IntNullableFilter::Gt(start_lib_pos),
                        )],
                        vec![db::instance_group::SetParam::DecrementLibraryPosition(1)],
                    )
                    .exec()
                    .await?;
            }
        }

        self.app
            .prisma_client
            ._batch((
                index_shifts,
                self.app
                    .prisma_client
                    .instance()
                    .update(UniqueWhereParam::IdEquals(*instance), update_params),
            ))
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        // Auto-delete empty non-default groups after moving instance out
        if start_group != default_group_id && start_group != target_group {
            let remaining_count = self
                .app
                .prisma_client
                .instance()
                .count(vec![WhereParam::GroupId(IntFilter::Equals(*start_group))])
                .exec()
                .await?;

            if remaining_count == 0 {
                // Delete the now-empty group
                self.app
                    .prisma_client
                    .instance_group()
                    .delete(db::instance_group::UniqueWhereParam::IdEquals(*start_group))
                    .exec()
                    .await?;
                // GET_GROUPS already invalidated above, but invalidate again after deletion
                self.app.invalidate(GET_GROUPS, None);
            }
        }

        Ok(())
    }

    pub fn get_default_group(self) -> BoxFuture<'s, anyhow::Result<GroupId>> {
        Box::pin(async move {
            use db::instance_group::WhereParam;

            static DEFAULT_MUTEX: Mutex<()> = Mutex::const_new(());

            let groupid = self
                .app
                .settings_manager()
                .get_settings()
                .await?
                .default_instance_group;

            match groupid {
                Some(groupid) => {
                    let group = self
                        .app
                        .prisma_client
                        .instance_group()
                        .find_first(vec![WhereParam::Id(IntFilter::Equals(groupid))])
                        .exec()
                        .await?;

                    match group {
                        Some(x) => Ok(GroupId(x.id)),
                        None => bail!(
                            "invalid database state: default group specified in configuration, but missing from groups"
                        ),
                    }
                }
                None => {
                    match DEFAULT_MUTEX.try_lock() {
                        Ok(_lock) => {
                            let index = self.next_group_index().await?;

                            self.app
                                .prisma_client
                                ._transaction()
                                .run(|prisma| async move {
                                    let group = prisma
                                        .instance_group()
                                        .create(
                                            String::from("localize➽default"),
                                            index.value,
                                            vec![],
                                        )
                                        .exec()
                                        .await?;

                                    use db::app_configuration::{SetParam, UniqueWhereParam};

                                    prisma
                                        .app_configuration()
                                        .update(
                                            UniqueWhereParam::IdEquals(0),
                                            vec![SetParam::SetDefaultInstanceGroup(Some(group.id))],
                                        )
                                        .exec()
                                        .await?;

                                    Ok(GroupId(group.id))
                                })
                                .await
                        }
                        Err(_) => {
                            // Wait for the lock to finish, some other thread probably
                            // wrote the group to the DB at this point, so just retry getting it from the db.
                            let _ = DEFAULT_MUTEX.lock().await;
                            self.get_default_group().await
                        }
                    }
                }
            }
        })
    }

    pub async fn create_group(self, name: String) -> anyhow::Result<GroupId> {
        use db::instance_group::WhereParam;

        let index = self.next_group_index().await?;

        let group = self
            .app
            .prisma_client
            .instance_group()
            .find_first(vec![WhereParam::Name(StringFilter::Equals(name.clone()))])
            .exec()
            .await?;

        if let Some(group) = group {
            return Ok(GroupId(group.id));
        }

        // Instances in the default group (and only those) have library_position set,
        // so filtering by LibraryPosition IS NOT NULL avoids needing default_group_id
        // here — which would otherwise deadlock via get_default_group re-acquiring
        // index_lock on a fresh database.
        let max_instance_pos: Option<i32> = self
            .app
            .prisma_client
            .instance()
            .find_first(vec![db::instance::WhereParam::LibraryPosition(
                db::read_filters::IntNullableFilter::Not(None),
            )])
            .order_by(db::instance::OrderByParam::LibraryPosition(
                carbon_repos::pcr::Direction::Desc,
            ))
            .exec()
            .await?
            .and_then(|i| i.library_position);

        let max_group_pos: Option<i32> = self
            .app
            .prisma_client
            .instance_group()
            .find_first(vec![WhereParam::LibraryPosition(
                db::read_filters::IntNullableFilter::Not(None),
            )])
            .order_by(db::instance_group::OrderByParam::LibraryPosition(
                carbon_repos::pcr::Direction::Desc,
            ))
            .exec()
            .await?
            .and_then(|g| g.library_position);

        let next_library_pos = max_instance_pos
            .unwrap_or(0)
            .max(max_group_pos.unwrap_or(0))
            + 1;

        let group = self
            .app
            .prisma_client
            .instance_group()
            .create(
                name,
                index.value,
                vec![db::instance_group::SetParam::SetLibraryPosition(Some(
                    next_library_pos,
                ))],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(GroupId(group.id))
    }

    /// Create a group at a specific library position, shifting existing items to make room.
    pub async fn create_group_at_position(
        self,
        name: String,
        target_position: i32,
    ) -> anyhow::Result<GroupId> {
        use db::instance_group::WhereParam;

        let index = self.next_group_index().await?;

        // Shift all items (groups and ungrouped instances) with library_position >= target_position up by 1
        // (library_position is only set on default-group instances, so no need to filter by group_id
        // — which would require materializing the default group and re-acquiring index_lock.)
        self.app
            .prisma_client
            .instance()
            .update_many(
                vec![db::instance::WhereParam::LibraryPosition(
                    db::read_filters::IntNullableFilter::Gte(target_position),
                )],
                vec![db::instance::SetParam::IncrementLibraryPosition(1)],
            )
            .exec()
            .await?;

        self.app
            .prisma_client
            .instance_group()
            .update_many(
                vec![WhereParam::LibraryPosition(
                    db::read_filters::IntNullableFilter::Gte(target_position),
                )],
                vec![db::instance_group::SetParam::IncrementLibraryPosition(1)],
            )
            .exec()
            .await?;

        // Create the group at the target position
        let group = self
            .app
            .prisma_client
            .instance_group()
            .create(
                name,
                index.value,
                vec![db::instance_group::SetParam::SetLibraryPosition(Some(
                    target_position,
                ))],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(GroupId(group.id))
    }

    pub async fn rename_group(self, group: GroupId, name: String) -> anyhow::Result<()> {
        use db::instance_group::{SetParam, UniqueWhereParam};

        self.app
            .prisma_client
            .instance_group()
            .update(
                UniqueWhereParam::IdEquals(*group),
                vec![SetParam::SetName(name)],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(())
    }

    /// Generate a unique folder name by appending (1), (2), etc. if needed.
    async fn generate_unique_folder_name(&self, base_name: &str) -> anyhow::Result<String> {
        use db::instance_group::WhereParam;

        // Check if base name exists
        let existing = self
            .app
            .prisma_client
            .instance_group()
            .find_first(vec![WhereParam::Name(StringFilter::Equals(
                base_name.to_string(),
            ))])
            .exec()
            .await?;

        if existing.is_none() {
            return Ok(base_name.to_string());
        }

        // Find next available number
        let mut counter = 1;
        loop {
            let candidate = format!("{} ({})", base_name, counter);
            let exists = self
                .app
                .prisma_client
                .instance_group()
                .find_first(vec![WhereParam::Name(StringFilter::Equals(
                    candidate.clone(),
                ))])
                .exec()
                .await?;

            if exists.is_none() {
                return Ok(candidate);
            }
            counter += 1;
        }
    }

    /// Create a new folder (group) from a list of instances.
    /// The folder is named "New Folder" by default, with (1), (2), etc. appended if needed.
    /// If target_instance_id is provided and is at the library root, the folder is created
    /// at that instance's position instead of at the end.
    pub async fn create_folder_from_instances(
        self,
        instance_ids: Vec<InstanceId>,
        target_instance_id: Option<InstanceId>,
    ) -> anyhow::Result<GroupId> {
        if instance_ids.is_empty() {
            bail!("Cannot create folder from empty list of instances");
        }

        // Generate a unique folder name
        let folder_name = self.generate_unique_folder_name("New Folder").await?;

        // Determine the target library position if target instance is at library root
        let target_library_pos = if let Some(target_id) = target_instance_id {
            let default_group_id = self.get_default_group().await?;
            let target_instance = self
                .app
                .prisma_client
                .instance()
                .find_unique(db::instance::UniqueWhereParam::IdEquals(*target_id))
                .exec()
                .await?;

            // Only use position if instance exists, is in default group, and has a library_position
            target_instance
                .filter(|i| i.group_id == *default_group_id)
                .and_then(|i| i.library_position)
        } else {
            None
        };

        // Create group at target position or at end
        let group_id = match target_library_pos {
            Some(pos) => self.create_group_at_position(folder_name, pos).await?,
            None => self.create_group(folder_name).await?,
        };

        // Move all instances to the new group
        for instance_id in instance_ids {
            self.move_instance(instance_id, InstanceMoveTarget::EndOfGroup(group_id))
                .await?;
        }

        Ok(group_id)
    }

    /// Arrange all ungrouped instances (in default group) and folders by the given criteria.
    /// This is a one-off arrange operation that reassigns library positions.
    /// Used in folders mode only (instancesGroupBy = null).
    pub async fn arrange_library(self, sort_by: LibrarySortCriteria) -> anyhow::Result<()> {
        let default_group_id = self.get_default_group().await?;

        // Lock indexes while we're changing them
        let _index_lock = self.index_lock.lock().await;

        // Get all instances in the default group (ungrouped instances)
        let instances = self
            .app
            .prisma_client
            .instance()
            .find_many(vec![db::instance::group_id::equals(*default_group_id)])
            .exec()
            .await?;

        // Get instance data for sorting
        let active_instances = self.instances.read().await;

        // Build a sortable list of (instance_id, name, last_played, seconds_played)
        let mut sortable_instances: Vec<(i32, String, Option<DateTime<Utc>>, u32)> = instances
            .iter()
            .map(|inst| {
                let instance_data = active_instances.get(&InstanceId(inst.id));
                let (last_played, seconds_played) = match instance_data {
                    Some(data) => match &data.type_ {
                        InstanceType::Valid(valid) => {
                            (valid.config.last_played, valid.config.seconds_played)
                        }
                        InstanceType::Invalid(_) => (None, 0),
                    },
                    None => (None, 0),
                };
                (inst.id, inst.name.clone(), last_played, seconds_played)
            })
            .collect();

        drop(active_instances);

        // Sort based on criteria
        match sort_by {
            LibrarySortCriteria::Name => {
                sortable_instances.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
            }
            LibrarySortCriteria::LastPlayed => {
                sortable_instances.sort_by(|a, b| match (&b.2, &a.2) {
                    (Some(b_date), Some(a_date)) => b_date.cmp(a_date),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.1.to_lowercase().cmp(&b.1.to_lowercase()),
                });
            }
            LibrarySortCriteria::MostPlayed => {
                sortable_instances.sort_by(|a, b| {
                    b.3.cmp(&a.3)
                        .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
                });
            }
            LibrarySortCriteria::DateCreated => {
                // Use instance id as proxy for creation order (lower id = older)
                sortable_instances.sort_by(|a, b| a.0.cmp(&b.0));
            }
        }

        // Fetch and sort non-default groups (folders) by name. They appear
        // after ungrouped instances in the library listing.
        let groups = self
            .app
            .prisma_client
            .instance_group()
            .find_many(vec![])
            .exec()
            .await?;

        let mut sortable_groups: Vec<(i32, String)> = groups
            .iter()
            .filter(|g| g.id != *default_group_id)
            .map(|g| (g.id, g.name.clone()))
            .collect();
        sortable_groups.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

        // Folders always come first in the library, followed by ungrouped
        // instances. The frontend enforces the folders-first rule at sort
        // time; we mirror it here so the DB positions agree with what the
        // user sees.
        //
        // The frontend sorts top-level items by `library_position ?? index`
        // (or `libraryPosition ?? 10000` for folders). Writing `index`
        // alone is invisible whenever `library_position` is set, so we
        // stamp both: positions `0..M-1` for folders, `M..M+N-1` for
        // ungrouped instances.
        let mut group_updates = Vec::new();
        // Default group has no library_position (it's not rendered as a
        // top-level folder); keep group_index at 0 as before.
        group_updates.push(self.app.prisma_client.instance_group().update(
            db::instance_group::UniqueWhereParam::IdEquals(*default_group_id),
            vec![db::instance_group::group_index::set(0)],
        ));
        for (i, (group_id, _)) in sortable_groups.iter().enumerate() {
            let p = i as i32;
            group_updates.push(self.app.prisma_client.instance_group().update(
                db::instance_group::UniqueWhereParam::IdEquals(*group_id),
                vec![
                    db::instance_group::group_index::set((i + 1) as i32),
                    db::instance_group::library_position::set(Some(p)),
                ],
            ));
        }
        if !group_updates.is_empty() {
            self.app.prisma_client._batch(group_updates).await?;
        }

        let instance_base = sortable_groups.len() as i32;
        let mut updates = Vec::new();
        for (i, (instance_id, _, _, _)) in sortable_instances.iter().enumerate() {
            let p = instance_base + i as i32;
            updates.push(self.app.prisma_client.instance().update(
                db::instance::UniqueWhereParam::IdEquals(*instance_id),
                vec![
                    db::instance::index::set(p),
                    db::instance::library_position::set(Some(p)),
                ],
            ));
        }
        if !updates.is_empty() {
            self.app.prisma_client._batch(updates).await?;
        }

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(())
    }

    /// Arrange all instances within a specific folder by the given criteria.
    /// This is a one-off arrange operation that reassigns indices within the folder.
    /// Used in folders mode only (instancesGroupBy = null).
    pub async fn arrange_group(
        self,
        group_id: GroupId,
        sort_by: LibrarySortCriteria,
    ) -> anyhow::Result<()> {
        use db::instance::{SetParam, UniqueWhereParam, WhereParam};

        // Lock indexes while we're changing them
        let _index_lock = self.index_lock.lock().await;

        // Get all instances in the specified group
        let instances = self
            .app
            .prisma_client
            .instance()
            .find_many(vec![WhereParam::GroupId(IntFilter::Equals(*group_id))])
            .exec()
            .await?;

        // Get instance data for sorting
        let active_instances = self.instances.read().await;

        // Build a sortable list of (instance_id, name, last_played, seconds_played)
        let mut sortable_instances: Vec<(i32, String, Option<DateTime<Utc>>, u32)> = instances
            .iter()
            .map(|inst| {
                let instance_data = active_instances.get(&InstanceId(inst.id));
                let (last_played, seconds_played) = match instance_data {
                    Some(data) => match &data.type_ {
                        InstanceType::Valid(valid) => {
                            (valid.config.last_played, valid.config.seconds_played)
                        }
                        InstanceType::Invalid(_) => (None, 0),
                    },
                    None => (None, 0),
                };
                (inst.id, inst.name.clone(), last_played, seconds_played)
            })
            .collect();

        drop(active_instances);

        // Sort based on criteria
        match sort_by {
            LibrarySortCriteria::Name => {
                sortable_instances.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
            }
            LibrarySortCriteria::LastPlayed => {
                sortable_instances.sort_by(|a, b| match (&b.2, &a.2) {
                    (Some(b_date), Some(a_date)) => b_date.cmp(a_date),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.1.to_lowercase().cmp(&b.1.to_lowercase()),
                });
            }
            LibrarySortCriteria::MostPlayed => {
                sortable_instances.sort_by(|a, b| {
                    b.3.cmp(&a.3)
                        .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
                });
            }
            LibrarySortCriteria::DateCreated => {
                // Use instance id as proxy for creation order (lower id = older)
                sortable_instances.sort_by(|a, b| a.0.cmp(&b.0));
            }
        }

        // Update indices for all instances in the group
        let mut updates = Vec::new();
        for (new_index, (instance_id, _, _, _)) in sortable_instances.iter().enumerate() {
            updates.push(self.app.prisma_client.instance().update(
                UniqueWhereParam::IdEquals(*instance_id),
                vec![SetParam::SetIndex(new_index as i32)],
            ));
        }

        if !updates.is_empty() {
            self.app.prisma_client._batch(updates).await?;
        }

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        Ok(())
    }

    /// Add an instance to the database without checking if it exists.
    /// Does not invalidate.
    async fn add_instance(
        self,
        name: String,
        shortpath: String,
        group: GroupId,
    ) -> anyhow::Result<InstanceId> {
        use db::instance_group::UniqueWhereParam;

        // Materialize the default group before taking index_lock via
        // next_instance_index (see create_group).
        let default_group_id = self.get_default_group().await?;

        let index = self.next_instance_index(group).await?;
        let library_position = if group == default_group_id {
            // New instances/folders appear at the top of the library.
            // Pick a value strictly smaller than every existing
            // library_position across both ungrouped instances and groups.
            let min_instance_pos: Option<i32> = self
                .app
                .prisma_client
                .instance()
                .find_first(vec![
                    db::instance::group_id::equals(*default_group_id),
                    db::instance::library_position::not(None),
                ])
                .order_by(db::instance::OrderByParam::LibraryPosition(
                    carbon_repos::pcr::Direction::Asc,
                ))
                .exec()
                .await?
                .and_then(|i| i.library_position);

            let min_group_pos: Option<i32> = self
                .app
                .prisma_client
                .instance_group()
                .find_first(vec![db::instance_group::library_position::not(None)])
                .order_by(db::instance_group::OrderByParam::LibraryPosition(
                    carbon_repos::pcr::Direction::Asc,
                ))
                .exec()
                .await?
                .and_then(|g| g.library_position);

            let current_min = match (min_instance_pos, min_group_pos) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };

            Some(current_min.map(|n| n - 1).unwrap_or(0))
        } else {
            None
        };

        let create_params = match library_position {
            Some(pos) => vec![db::instance::library_position::set(Some(pos))],
            None => vec![],
        };

        let (_, instance) = self
            .app
            .prisma_client
            ._batch((
                // delete any existing entry at the same shortpath
                self.app
                    .prisma_client
                    .instance()
                    .delete_many(vec![db::instance::shortpath::equals(shortpath.clone())]),
                self.app.prisma_client.instance().create(
                    name,
                    shortpath,
                    index.value,
                    UniqueWhereParam::IdEquals(*group),
                    create_params,
                ),
            ))
            .await?;

        Ok(InstanceId(instance.id))
    }

    /// Remove an instance from the database without checking if it exists.
    /// Does not invalidate.
    async fn remove_instance(self, instance: InstanceId) -> anyhow::Result<()> {
        use db::instance::UniqueWhereParam;

        self.app
            .prisma_client
            .instance()
            .delete(UniqueWhereParam::IdEquals(*instance))
            .exec()
            .await?;

        self.app.meta_cache_manager().gc_mod_metadata().await;

        Ok(())
    }

    pub async fn set_favorite(self, instance_id: InstanceId, favorite: bool) -> anyhow::Result<()> {
        use db::instance::{SetParam, UniqueWhereParam};

        const MAX_FAVORITES: usize = 10;

        let mut instances = self.instances.write().await;

        // Verify instance exists
        if !instances.contains_key(&instance_id) {
            return Err(InvalidInstanceIdError(instance_id).into());
        }

        // If setting as favorite, check the limit before taking mutable borrow
        if favorite {
            let current_favorite_count = instances
                .iter()
                .filter(|(id, inst)| {
                    **id != instance_id
                        && matches!(&inst.type_, InstanceType::Valid(data) if data.favorite)
                })
                .count();

            if current_favorite_count >= MAX_FAVORITES {
                bail!(
                    "Maximum number of favorites ({}) reached. Remove a favorite before adding a new one.",
                    MAX_FAVORITES
                );
            }
        }

        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;
        let data = instance.data_mut()?;
        data.favorite = favorite;
        drop(instances);

        // Update database for target instance
        self.app
            .prisma_client
            .instance()
            .update(
                UniqueWhereParam::IdEquals(*instance_id),
                vec![SetParam::SetFavorite(favorite)],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));

        Ok(())
    }

    fn next_folder(&self, name: &str) -> anyhow::Result<(String, PathBuf)> {
        if name.is_empty() {
            bail!("Attempted to find an instance directory name for an unnamed instance");
        }

        let mut sanitized_name = sanitize_name(name);
        let instance_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path();

        if !instance_path.exists() {
            return Ok((sanitized_name.clone(), instance_path.join(&sanitized_name)));
        }

        if !instance_path.is_dir() {
            bail!("GDL instances path is not a directory. Please move the file blocking it.");
        }

        sanitized_name = truncate_name(&sanitized_name, &instance_path);

        for i in 0..1000 {
            let current_name = if i == 0 {
                sanitized_name.clone()
            } else {
                format!("{}{}", sanitized_name, i)
            };

            let full_path = instance_path.join(&current_name);
            if !full_path.exists() {
                return Ok((current_name, full_path));
            }
        }

        bail!("Unable to create a unique folder name after 1000 attempts")
    }

    pub async fn load_icon(self, icon: PathBuf) -> anyhow::Result<(String, Vec<u8>)> {
        let data = tokio::fs::read(icon.clone())
            .await
            .with_context(|| format!("Reading file `{}`", icon.to_string_lossy()))?;

        let extension = match icon.extension() {
            Some(ext) => ext,
            None => OsStr::new(""),
        };

        let icon_name = PathBuf::from("icon")
            .with_extension(extension)
            .to_string_lossy()
            .to_string();

        Ok((icon_name, data))
    }

    pub async fn download_icon(self, url: String) -> anyhow::Result<(String, Vec<u8>)> {
        let extension = url
            .rsplit_once('/')
            .map(|(_, name)| name.rsplit_once('.'))
            .flatten()
            .map(|(_, ext)| ext)
            .unwrap_or("png");

        let data = self
            .app
            .reqwest_client
            .get(&url)
            .send()
            .await?
            .bytes()
            .await?;

        Ok((format!("icon.{extension}"), data.to_vec()))
    }

    pub async fn set_loaded_icon(self, icon: (String, Vec<u8>)) {
        *self.loaded_icon.lock().await = Some(icon);
    }

    pub async fn create_instance(
        self,
        group: GroupId,
        name: String,
        use_loaded_icon: bool,
        version: InstanceVersionSource,
        notes: String,
    ) -> anyhow::Result<InstanceId> {
        let icon = match use_loaded_icon {
            true => self.loaded_icon.lock().await.take(),
            false => None,
        };

        self.create_instance_ext(group, name, icon, None, None, version, notes, |_| async {
            Ok(())
        })
        .await
    }

    #[tracing::instrument(skip(self, icon, initializer))]
    pub async fn create_instance_ext<F, I>(
        self,
        group: GroupId,
        name: String,
        icon: Option<(String, Vec<u8>)>,
        seconds_played: Option<u32>,
        last_played: Option<DateTime<Utc>>,
        version: InstanceVersionSource,
        notes: String,
        initializer: F,
    ) -> anyhow::Result<InstanceId>
    where
        F: FnOnce(PathBuf) -> I,
        I: Future<Output = anyhow::Result<()>>,
    {
        trace!("Creating instance");

        let tmpdir = self
            .app
            .settings_manager()
            .runtime_path
            .get_temp()
            .maketmpdir()
            .await?;

        tokio::fs::create_dir(tmpdir.join("instance")).await?;

        let icon = match icon {
            Some((path, data)) => {
                tokio::fs::write(tmpdir.join(&path), data)
                    .await
                    .context("saving instance icon")?;

                InstanceIcon::RelativePath(path)
            }
            None => InstanceIcon::Default,
        };

        let (version, modpack, pack_locked) = match version {
            InstanceVersionSource::Version(version) => (Some(version), None, false),
            InstanceVersionSource::Modpack(modpack, locked) => (None, Some(modpack), locked),
            InstanceVersionSource::ModpackWithKnownVersion(version, modpack, locked) => {
                (Some(version), Some(modpack), locked)
            }
        };

        let info = info::Instance {
            name: name.clone(),
            icon,
            date_created: Utc::now(),
            date_updated: Utc::now(),
            last_played,
            seconds_played: seconds_played.unwrap_or(0),
            modpack: modpack.clone().map(|modpack| info::ModpackInfo {
                modpack,
                locked: pack_locked,
            }),
            game_configuration: info::GameConfig {
                version,
                global_java_args: true,
                extra_java_args: None,
                memory: None,
                java_override: None,
                game_resolution: None,
            },
            pre_launch_hook: None,
            post_exit_hook: None,
            wrapper_command: None,
            mod_sources: None,
            notes,
        };

        let json = schema::make_instance_config(info.clone())?;
        tokio::fs::write(tmpdir.join("instance.json"), json)
            .await
            .context("writing instance json")?;

        let setup_path = tmpdir.join(".setup");
        tokio::fs::create_dir(&setup_path)
            .await
            .context("writing setup marker")?;

        if let Some(modpack) = modpack {
            let pack_version_text = serde_json::to_string(&PackVersionFile::from(modpack))?;
            tokio::fs::write(
                setup_path.join("change-pack-version.json"),
                &pack_version_text,
            )
            .await?;
        }

        trace!("Running extended instance initializer");
        initializer(tmpdir.to_path_buf()).await?;
        trace!("Finished extended instance initializer");

        tokio::fs::create_dir_all(
            self.app
                .settings_manager()
                .runtime_path
                .get_instances()
                .to_path(),
        )
        .await?;

        trace!("Locking path_lock");
        let path_lock = self.path_lock.lock().await;
        let (shortpath, path) = self.next_folder(&name)?;

        tmpdir
            .try_rename_or_move(&path)
            .await
            .context("moving tmpdir to instance location")?;

        trace!("Created instance folder at '{path:?}'. Unlocking path_lock");
        drop(path_lock);

        let id = self
            .add_instance(name.clone(), shortpath.clone(), group)
            .await?;

        trace!("Adding instance to instances list");

        let icon_revision = match &info.icon {
            InstanceIcon::Default => None,
            InstanceIcon::RelativePath(_) => Some(1),
        };

        self.instances.write().await.insert(
            id,
            Instance {
                shortpath: shortpath.clone(),
                type_: InstanceType::Valid(InstanceData {
                    favorite: false,
                    config: info,
                    state: run::LaunchState::Inactive { failed_task: None },
                    modpack_update_curseforge: None,
                    modpack_update_modrinth: None,
                    icon_revision,
                }),
            },
        );

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);

        info!({ shortpath = ?shortpath }, "Created new instance '{name}' (id {})", *id);

        Ok(id)
    }

    pub async fn update_instance(
        self,
        update: domain::InstanceSettingsUpdate,
    ) -> anyhow::Result<()> {
        use db::instance::{SetParam, UniqueWhereParam};

        // Acquire per-instance operation lock to prevent concurrent updates to same instance
        // while allowing parallel updates to different instances
        let op_lock = self
            .instance_op_locks
            .entry(update.instance_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _instance_guard = op_lock.lock().await;

        // Phase 1: Read instance data under write lock
        let (mut shortpath, config, mut path, icon_revision) = {
            let instances = self.instances.write().await;
            let instance = instances
                .get(&update.instance_id)
                .ok_or(InvalidInstanceIdError(update.instance_id))?;

            let data = instance.type_.data()?;

            let path = self
                .app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&instance.shortpath)
                .get_root();

            (
                instance.shortpath.clone(),
                data.config.clone(),
                path,
                data.icon_revision,
            )
            // Write lock released here - other instances can now operate
        };

        // Phase 2: Apply all updates WITHOUT holding global lock
        let mut info = config;

        // Check what changed before consuming values (for conditional invalidation)
        let name_or_icon_changed = update.name.is_some() || update.use_loaded_icon.is_some();
        let modpack_lock_changed = update.modpack_locked.is_some();

        let mut new_icon_revision = icon_revision;

        if let Some(use_loaded_icon) = update.use_loaded_icon {
            let icon = match (use_loaded_icon, self.loaded_icon.lock().await.take()) {
                (true, Some((ipath, data))) => {
                    tokio::fs::write(path.join(&ipath), data)
                        .await
                        .context("saving instance icon")?;

                    if let InstanceIcon::RelativePath(oldpath) = &info.icon {
                        if *oldpath != ipath {
                            tokio::fs::remove_file(path.join(oldpath))
                                .await
                                .context("removing old instance icon")?;
                        }
                    }

                    InstanceIcon::RelativePath(ipath)
                }
                _ => InstanceIcon::Default,
            };

            info.icon = icon;
            new_icon_revision = match info.icon {
                InstanceIcon::Default => None,
                InstanceIcon::RelativePath(_) => Some(icon_revision.unwrap_or(1) + 1),
            };
        }

        if let Some(name) = update.name.clone() {
            info.name = name;
        }

        if let Some(notes) = update.notes {
            info.notes = notes;
        }

        if let Some(pre_launch_hook) = update.pre_launch_hook {
            info.pre_launch_hook = pre_launch_hook;
        }

        if let Some(post_exit_hook) = update.post_exit_hook {
            info.post_exit_hook = post_exit_hook;
        }

        if let Some(wrapper_command) = update.wrapper_command {
            info.wrapper_command = wrapper_command;
        }

        if let Some(java_override) = update.java_override {
            info!(?java_override, "Updating java override");
            info.game_configuration.java_override = java_override;
        }

        let mut need_reinstall = false;

        if let Some(version) = update.version {
            info.game_configuration.version =
                Some(info::GameVersion::Standard(info::StandardVersion {
                    release: version,
                    modloaders: match &info.game_configuration.version {
                        Some(info::GameVersion::Standard(info::StandardVersion {
                            modloaders,
                            ..
                        })) => modloaders.clone(),
                        _ => HashSet::new(),
                    },
                }));
            need_reinstall = true;
        }

        if let Some(modloader) = update.modloader {
            info.game_configuration.version =
                Some(info::GameVersion::Standard(info::StandardVersion {
                    release: match &info.game_configuration.version {
                        Some(info::GameVersion::Standard(info::StandardVersion {
                            release,
                            ..
                        })) => release.clone(),
                        _ => bail!("custom versions are not yet supported"),
                    },
                    modloaders: match modloader {
                        Some(modloader) => HashSet::from([modloader]),
                        None => HashSet::new(),
                    },
                }));
            need_reinstall = true;
        }

        if let Some(global_java_args) = update.global_java_args {
            info.game_configuration.global_java_args = global_java_args;
        }

        if let Some(extra_java_args) = update.extra_java_args {
            info.game_configuration.extra_java_args = extra_java_args;
        }

        if let Some(game_resolution) = update.game_resolution {
            info.game_configuration.game_resolution = game_resolution;
        }

        if let Some(memory) = update.memory {
            info.game_configuration.memory = memory;
        }

        if let Some(mod_sources) = update.mod_sources {
            info.mod_sources = mod_sources;
        }

        if let Some(modpack_locked) = update.modpack_locked {
            if let Some(modpack_locked) = modpack_locked {
                if let Some(modpack) = &mut info.modpack {
                    modpack.locked = modpack_locked;
                }
            } else {
                info.modpack = None;
            }
        }

        info.date_updated = Utc::now();

        let json = schema::make_instance_config(info.clone())?;

        self.app
            .settings_manager()
            .runtime_path
            .get_temp()
            .write_file_atomic(path.join("instance.json"), json)
            .await?;

        // Handle instance rename with path_lock
        if let Some(name) = update.name {
            let name_changed = shortpath != name;
            if name_changed {
                let _path_guard = self.path_lock.lock().await;
                let (new_shortpath, new_path) = self.next_folder(&name)?;
                tokio::fs::rename(&path, &new_path).await?;

                self.app
                    .prisma_client
                    .instance()
                    .update(
                        UniqueWhereParam::IdEquals(*update.instance_id),
                        vec![
                            SetParam::SetName(name),
                            SetParam::SetShortpath(new_shortpath.clone()),
                        ],
                    )
                    .exec()
                    .await?;

                shortpath = new_shortpath;
                path = new_path;
            }
        }

        // Phase 3: Update in-memory state under write lock (FAST - just assignment)
        {
            let mut instances = self.instances.write().await;
            let instance = instances
                .get_mut(&update.instance_id)
                .ok_or(InvalidInstanceIdError(update.instance_id))?;

            instance.shortpath = shortpath;

            if let InstanceType::Valid(data) = &mut instance.type_ {
                data.config = info;
                data.icon_revision = new_icon_revision;
            }
            // Write lock released here
        }

        // Send conditional invalidations (no lock needed)
        // Only invalidate queries that are actually affected by the changes
        if name_or_icon_changed || modpack_lock_changed {
            // Name/icon/lock changes affect instance list display
            // (lock status shows in ListInstance.locked field)
            self.app.invalidate(GET_GROUPS, None);
            self.app.invalidate(GET_ALL_INSTANCES, None);
        }

        // Always invalidate instance details since settings changed
        self.app
            .invalidate(INSTANCE_DETAILS, Some(update.instance_id.0.into()));

        if need_reinstall {
            let setup = path.join(".setup");
            tokio::fs::create_dir_all(&setup)
                .await
                .context("writing incomplete instance marker")?;
            // tokio::fs::create_dir_all(setup.join("modpack-complete"))
            //     .await
            //     .context("writing modpack complete")?;

            let app = self.app.clone();
            tokio::spawn(async move {
                app.instance_manager()
                    .prepare_game(InstanceId(*update.instance_id), None, None, true)
                    .await?;

                Ok(()) as anyhow::Result<()>
            });
        }

        Ok(())
    }

    pub async fn update_playtime(
        self,
        instance_id: InstanceId,
        added_seconds: u32,
    ) -> anyhow::Result<()> {
        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let shortpath = &mut instance.shortpath;
        let data = instance.type_.data_mut()?;

        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path()
            .join(shortpath as &str);

        data.config.last_played = Some(Utc::now());
        data.config.seconds_played += added_seconds;

        let json = schema::make_instance_config(data.config.clone())?;

        self.app
            .settings_manager()
            .runtime_path
            .get_temp()
            .write_file_atomic(path.join("instance.json"), json)
            .await?;

        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));

        Ok(())
    }

    pub async fn delete_instance(&self, instance_id: InstanceId) -> anyhow::Result<()> {
        let app = self.app.clone();

        tokio::spawn(async move {
            app.instance_manager()._delete_instance(instance_id).await?;

            Ok::<_, anyhow::Error>(())
        });

        Ok(())
    }

    async fn _delete_instance(self, instance_id: InstanceId) -> anyhow::Result<()> {
        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &mut instance.type_ else {
            return Err(anyhow!("Instance {instance_id} is not in a valid state"));
        };

        data.state = LaunchState::Deleting;

        let instance_shortpath = instance.shortpath.clone();
        drop(instances);

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));

        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path()
            .join(&instance_shortpath as &str);

        let should_go_to_trash = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .deletion_through_recycle_bin;

        tokio::task::spawn_blocking(move || {
            if should_go_to_trash {
                trash::delete(&path)?;
            } else {
                std::fs::remove_dir_all(&path)?;
            }

            Ok::<_, anyhow::Error>(())
        })
        .await??;

        // Get the instance's group_id before deleting, so we can check if the group becomes empty
        let group_id = self
            .app
            .prisma_client
            .instance()
            .find_unique(db::instance::UniqueWhereParam::IdEquals(*instance_id))
            .exec()
            .await?
            .map(|inst| GroupId(inst.group_id));

        let mut instances = self.instances.write().await;

        instances.remove(&instance_id);
        self.remove_instance(instance_id).await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));

        // Auto-delete empty non-default groups after deleting the last instance
        if let Some(group_id) = group_id {
            let default_group_id = self.get_default_group().await?;

            if group_id != default_group_id {
                let remaining_count = self
                    .app
                    .prisma_client
                    .instance()
                    .count(vec![db::instance::WhereParam::GroupId(IntFilter::Equals(
                        *group_id,
                    ))])
                    .exec()
                    .await?;

                if remaining_count == 0 {
                    self.app
                        .prisma_client
                        .instance_group()
                        .delete(db::instance_group::UniqueWhereParam::IdEquals(*group_id))
                        .exec()
                        .await?;
                    self.app.invalidate(GET_GROUPS, None);
                }
            }
        }

        Ok(())
    }

    /// # Locks
    /// - [InstanceManager::instances] (w)
    pub async fn duplicate_instance(
        self,
        instance_id: InstanceId,
        name: String,
    ) -> anyhow::Result<InstanceId> {
        let mut instances = self.instances.write().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let group_id = self
            .app
            .prisma_client
            .instance()
            .find_unique(db::instance::UniqueWhereParam::IdEquals(*instance_id))
            .exec()
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "instance was not listed in db while being present in internal list"
                )
            })?
            .group_id;

        let mut new_info = instance.data()?.config.clone();
        let (new_shortpath, new_path) = self.next_folder(&instance.shortpath)?;
        new_info.name = name;

        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath)
            .get_root();

        let tmpdir = self
            .app
            .settings_manager()
            .runtime_path
            .get_temp()
            .maketmpdir()
            .await?;

        let path2 = path.clone();
        let tmpdir2 = tmpdir.to_path_buf();
        let tmppath = tmpdir.join(
            path.file_name()
                .expect("instance path cannot end in .. or be empty"),
        );
        tokio::task::spawn_blocking(move || {
            fs_extra::dir::copy(path2, tmpdir2, &CopyOptions::new())
        })
        .await??;

        let json = schema::make_instance_config(new_info.clone())?;

        let icon_revision = match &new_info.icon {
            InstanceIcon::Default => None,
            InstanceIcon::RelativePath(_) => Some(1),
        };

        tokio::fs::write(&tmpdir.join("instance.json"), json).await?;

        tokio::fs::rename(&tmppath, new_path).await?;
        let id = self
            .add_instance(
                new_info.name.clone(),
                new_shortpath.clone(),
                GroupId(group_id),
            )
            .await?;

        instances.insert(
            id,
            Instance {
                shortpath: new_shortpath,
                type_: InstanceType::Valid(InstanceData {
                    favorite: false,
                    config: new_info,
                    state: run::LaunchState::Inactive { failed_task: None },
                    modpack_update_curseforge: None,
                    modpack_update_modrinth: None,
                    icon_revision,
                }),
            },
        );

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        self.app
            .meta_cache_manager()
            .queue_caching(
                crate::managers::metadata::cache::CacheEntityId::Instance(id),
                false,
            )
            .await;

        Ok(id)
    }

    pub async fn open_folder(
        self,
        instance_id: InstanceId,
        folder: InstanceFolder,
    ) -> anyhow::Result<()> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&instance.shortpath);

        let path = match folder {
            InstanceFolder::Root => path.get_root().to_path_buf(),
            InstanceFolder::Data => path.get_data_path().to_path_buf(),
            InstanceFolder::Mods => path.get_mods_path().to_path_buf(),
            InstanceFolder::Configs => path.get_config_path().to_path_buf(),
            InstanceFolder::Screenshots => path.get_screenshots_path().to_path_buf(),
            InstanceFolder::Saves => path.get_saves_path().to_path_buf(),
            InstanceFolder::Logs => path.get_logs_path().to_path_buf(),
            InstanceFolder::CrashReports => path.get_crash_reports_path().to_path_buf(),
            InstanceFolder::ResourcePacks => path.get_resourcepacks_path().to_path_buf(),
            InstanceFolder::TexturePacks => path.get_texturepacks_path().to_path_buf(),
            InstanceFolder::ShaderPacks => path.get_shaderpacks_path().to_path_buf(),
        };

        if !path.is_file() && !path.is_dir() {
            tokio::fs::create_dir_all(&path).await.with_context(|| {
                format!("Creating instance folder at `{}`", path.to_string_lossy())
            })?;
        } else if path.is_file() && !path.is_dir() {
            bail!("Path is a file, not a directory");
        }

        opener::open(path)?;

        Ok(())
    }

    /// Delete an instance group and move all contained instances into the default group.
    // TODO: handle deleting the default group while it has instances.
    pub async fn delete_group(self, group: GroupId) -> anyhow::Result<()> {
        use db::{instance, instance_group};

        // lock indexes before checking for instances to make sure none can be moved or created.
        let _index_lock = self.index_lock.lock().await;

        let any_instances = self
            .app
            .prisma_client
            .instance()
            .count(vec![instance::WhereParam::GroupId(IntFilter::Equals(
                *group,
            ))])
            .exec()
            .await?
            != 0;

        // a default group will be created if get_default_group is called, so
        // we check if any instances exist before creating it to avoid making an
        // empty group every time a group is deleted.
        if any_instances {
            let default_group = self.get_default_group().await?;

            // next_instance_index can't be used due to _index_lock, and dropping it
            // first would be a race condition.
            let base_index = self
                .app
                .prisma_client
                .instance()
                .count(vec![instance::WhereParam::GroupId(IntFilter::Equals(
                    *group,
                ))])
                .exec()
                .await?;

            self.app
                .prisma_client
                ._batch((
                    self.app.prisma_client.instance().update_many(
                        vec![instance::WhereParam::GroupId(IntFilter::Equals(*group))],
                        vec![
                            instance::SetParam::SetGroupId(*default_group),
                            instance::SetParam::IncrementIndex(base_index as i32),
                        ],
                    ),
                    self.app
                        .prisma_client
                        .instance_group()
                        .delete(instance_group::UniqueWhereParam::IdEquals(*group)),
                ))
                .await?;
        } else {
            self.app
                .prisma_client
                .instance_group()
                .delete(instance_group::UniqueWhereParam::IdEquals(*group))
                .exec()
                .await?;
        }

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        Ok(())
    }

    /// Delete an instance group and all instances it contains.
    pub async fn delete_group_with_instances(self, group: GroupId) -> anyhow::Result<()> {
        use db::{instance, instance_group};

        // Get all instances in the group
        let instances_in_group = self
            .app
            .prisma_client
            .instance()
            .find_many(vec![instance::WhereParam::GroupId(IntFilter::Equals(
                *group,
            ))])
            .exec()
            .await?;

        // Delete all instances in the group (spawn async tasks for each)
        let app = self.app.clone();
        for instance in instances_in_group {
            let instance_id = InstanceId(instance.id);
            let app_clone = app.clone();
            tokio::spawn(async move {
                if let Err(e) = app_clone
                    .instance_manager()
                    ._delete_instance(instance_id)
                    .await
                {
                    tracing::error!("Failed to delete instance {:?}: {:?}", instance_id, e);
                }
            });
        }

        // Delete the group record
        self.app
            .prisma_client
            .instance_group()
            .delete(instance_group::UniqueWhereParam::IdEquals(*group))
            .exec()
            .await?;

        self.app.invalidate(GET_GROUPS, None);
        self.app.invalidate(GET_ALL_INSTANCES, None);
        Ok(())
    }

    pub async fn instance_details(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<domain::InstanceDetails> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let instance = match &instance.type_ {
            InstanceType::Invalid(_) => bail!(InvalidInstanceDataError),
            InstanceType::Valid(x) => x,
        };

        let icon_revision = match &instance.config.icon {
            InstanceIcon::Default => None,
            InstanceIcon::RelativePath(_) => instance.icon_revision,
        };

        let mc_version = match &instance.config.game_configuration.version {
            Some(info::GameVersion::Standard(version)) => Some(version.release.clone()),
            Some(info::GameVersion::Custom(custom)) => Some(custom.clone()),
            None => None,
        };

        let mut mc_manifest = None;

        if let Some(mc_version) = &mc_version {
            let manifest = self.app.minecraft_manager().get_minecraft_manifest().await;
            mc_manifest = manifest.ok();
        }

        let required_java_profile = mc_version.clone().and_then(|version| {
            let Some(manifest) = mc_manifest else {
                return None;
            };
            let java = manifest
                .versions
                .iter()
                .find(|profile| profile.id == version)
                .and_then(|version| version.java_profile.clone());

            let Some(required_java) = java else {
                return None;
            };

            SystemJavaProfileName::try_from(required_java)
                .map(|v| v.to_string())
                .ok()
        });

        Ok(domain::InstanceDetails {
            id: instance_id,
            favorite: instance.favorite,
            name: instance.config.name.clone(),
            version: mc_version,
            // is_being_cached: self
            //     .app
            //     .meta_cache_manager()
            //     .is_instance_being_cached(instance_id)
            //     .await,
            modpack: instance.config.modpack.clone(),
            global_java_args: instance.config.game_configuration.global_java_args,
            extra_java_args: instance.config.game_configuration.extra_java_args.clone(),
            memory: instance.config.game_configuration.memory,
            game_resolution: instance.config.game_configuration.game_resolution.clone(),
            last_played: instance.config.last_played,
            seconds_played: instance.config.seconds_played as u32,
            modloaders: match &instance.config.game_configuration.version {
                Some(info::GameVersion::Standard(version)) => {
                    version.modloaders.iter().map(Clone::clone).collect()
                }
                Some(info::GameVersion::Custom(_)) => Vec::new(), // todo
                None => Vec::new(),
            },
            java_override: instance.config.game_configuration.java_override.clone(),
            required_java_profile,
            state: (&instance.state).into(),
            notes: instance.config.notes.clone(),
            icon_revision,
            has_pack_update: instance.modpack_update_curseforge.unwrap_or(false)
                || instance.modpack_update_modrinth.unwrap_or(false),
            pre_launch_hook: instance.config.pre_launch_hook.clone(),
            post_exit_hook: instance.config.post_exit_hook.clone(),
            wrapper_command: instance.config.wrapper_command.clone(),
        })
    }

    pub async fn get_modpack_info(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<Option<InstanceModpackInfo>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let instance = match &instance.type_ {
            InstanceType::Invalid(_) => bail!(InvalidInstanceDataError),
            InstanceType::Valid(x) => x,
        };

        let modpack = match &instance.config.modpack {
            Some(modpack) => modpack.clone(),
            None => {
                return Ok(None);
            }
        };

        drop(instances);

        let _guard = self.modpack_info_semaphore.lock().await;

        let modpack_info = match modpack.modpack {
            info::Modpack::Curseforge(curseforge) => {
                cache::curseforge::modpack::get_modpack_metadata(&self.app, curseforge).await?
            }
            info::Modpack::Modrinth(modrinth) => {
                cache::modrinth::modpack::get_modpack_metadata(&self.app, modrinth).await?
            }
        };

        Ok(Some(modpack_info))
    }

    pub async fn get_modpack_icon(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let instance = match &instance.type_ {
            InstanceType::Invalid(_) => bail!(InvalidInstanceDataError),
            InstanceType::Valid(x) => x,
        };

        let modpack = match &instance.config.modpack {
            Some(modpack) => modpack.clone(),
            None => {
                return Ok(None);
            }
        };

        drop(instances);

        let _guard = self.modpack_info_semaphore.lock().await;

        let modpack_info = match modpack.modpack {
            info::Modpack::Curseforge(curseforge) => {
                cache::curseforge::modpack::get_modpack_icon(&self.app, curseforge).await?
            }
            info::Modpack::Modrinth(modrinth) => {
                cache::modrinth::modpack::get_modpack_icon(&self.app, modrinth).await?
            }
        };

        Ok(Some(modpack_info))
    }

    pub async fn instance_icon(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<Option<(String, Vec<u8>)>> {
        let instances = self.instances.read().await;

        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let InstanceType::Valid(data) = &instance.type_ else {
            return Ok(None);
        };

        match &data.config.icon {
            InstanceIcon::Default => Ok(None),
            InstanceIcon::RelativePath(icon_path) => {
                let path = self
                    .app
                    .settings_manager()
                    .runtime_path
                    .get_instances()
                    .to_path()
                    .join(&instance.shortpath)
                    .join(icon_path);
                let icon = tokio::fs::read(path).await?;

                Ok(Some((icon_path.clone(), icon)))
            }
        }
    }

    async fn next_group_index(self) -> anyhow::Result<IdLock<'s, i32>> {
        let guard = self.manager.index_lock.lock().await;

        let count = self
            .app
            .prisma_client
            .instance_group()
            .count(vec![])
            .exec()
            .await?;

        Ok(IdLock {
            value: count as i32,
            guard,
        })
    }

    async fn next_instance_index(self, group: GroupId) -> anyhow::Result<IdLock<'s, i32>> {
        let guard = self.manager.index_lock.lock().await;

        // Newly created instances appear at the TOP of their group, so we
        // pick an index strictly smaller than the current minimum. Sort is
        // ascending on `index`, so a smaller value sorts first.
        let min_index: Option<i32> = self
            .app
            .prisma_client
            .instance()
            .find_first(vec![db::instance::group_id::equals(*group)])
            .order_by(db::instance::OrderByParam::Index(
                carbon_repos::pcr::Direction::Asc,
            ))
            .exec()
            .await?
            .map(|i| i.index);

        Ok(IdLock {
            value: min_index.map(|n| n - 1).unwrap_or(0),
            guard,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListGroup {
    pub id: GroupId,
    pub name: String,
    pub library_position: Option<i32>,
    pub instances: Vec<ListInstance>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListInstance {
    pub id: InstanceId,
    pub group_id: GroupId,
    pub index: i32,
    pub library_position: Option<i32>,
    pub name: String,
    pub favorite: bool,
    pub status: ListInstanceStatus,
    pub icon_revision: Option<u32>,
    pub last_played: Option<DateTime<Utc>>,
    pub locked: bool,
    pub date_created: DateTime<Utc>,
    pub date_updated: DateTime<Utc>,
    pub seconds_played: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ListInstanceStatus {
    Valid(ValidListInstance),
    Invalid(InvalidListInstance),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ValidListInstance {
    pub mc_version: Option<String>,
    pub modloader: Option<info::ModLoaderType>,
    pub modloader_version: Option<String>,
    pub modpack: Option<Modpack>,
    pub state: domain::LaunchState,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidListInstance {
    JsonMissing,
    JsonError(ConfigurationParseError),
    Other(String),
}

/// Lock used to prevent race conditions when modifying group or instance indexes
struct IdLock<'a, V: Copy + Clone> {
    value: V,
    guard: MutexGuard<'a, ()>,
}

impl Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for InstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for GroupId {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for InstanceId {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum InstanceMoveTarget {
    Before(InstanceId),
    BeginningOfGroup(GroupId),
    EndOfGroup(GroupId),
    BeforeGroup(GroupId), // Position instance before a folder (at library root level)
}

pub enum GroupMoveTarget {
    BeforeGroup(GroupId),
    BeforeInstance(InstanceId), // Instance must be in default group (ungrouped)
    EndOfLibrary,
}

#[derive(Debug, Clone, Copy)]
pub enum LibrarySortCriteria {
    Name,
    LastPlayed,
    MostPlayed,
    DateCreated,
}

#[derive(Debug)]
pub struct Instance {
    pub shortpath: String,
    pub type_: InstanceType,
}

#[derive(Debug)]
pub enum InstanceType {
    Valid(InstanceData),
    Invalid(InvalidConfiguration),
}

impl InstanceType {
    pub fn data(&self) -> Result<&InstanceData, InvalidInstanceDataError> {
        match self {
            Self::Valid(data) => Ok(data),
            Self::Invalid(_) => Err(InvalidInstanceDataError),
        }
    }

    pub fn data_mut(&mut self) -> Result<&mut InstanceData, InvalidInstanceDataError> {
        match self {
            Self::Valid(data) => Ok(data),
            Self::Invalid(_) => Err(InvalidInstanceDataError),
        }
    }
}

impl Instance {
    pub fn data(&self) -> Result<&InstanceData, InvalidInstanceDataError> {
        self.type_.data()
    }

    pub fn data_mut(&mut self) -> Result<&mut InstanceData, InvalidInstanceDataError> {
        self.type_.data_mut()
    }
}

#[derive(Debug)]
pub enum InvalidConfiguration {
    NoFile,
    Invalid(ConfigurationParseError),
    IoError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationParseError {
    pub type_: ConfigurationParseErrorType,
    pub message: String,
    pub line: u32,
    pub config_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Type, Serialize)]
pub enum ConfigurationParseErrorType {
    Syntax,
    Data,
    Eof,
    Unknown,
}

#[derive(Debug)]
pub enum Late<T> {
    Loading,
    Ready(T),
}

#[derive(Debug)]
pub struct InstanceData {
    favorite: bool,
    config: info::Instance,
    state: run::LaunchState,
    modpack_update_curseforge: Option<bool>,
    modpack_update_modrinth: Option<bool>,
    icon_revision: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Mod {
    id: String,
    filename: OsString,
    enabled: bool,
    modloaders: Vec<domain::info::ModLoaderType>,
    metadata: domain::ModFileMetadata,
}

#[derive(Debug)]
pub enum InstanceVersionSource {
    Version(info::GameVersion),
    /// (version, modpack, locked)
    Modpack(info::Modpack, bool),
    /// (version, modpack, locked)
    ModpackWithKnownVersion(info::GameVersion, info::Modpack, bool),
}

#[derive(Error, Debug)]
#[error("attempted to use invalid InstanceId {0}")]
pub struct InvalidInstanceIdError(InstanceId);

#[derive(Error, Debug)]
#[error("attempted to use invalid GroupId {0}")]
pub struct InvalidGroupIdError(GroupId);

#[derive(Error, Debug)]
#[error("attempted to get data of an invalid instance")]
pub struct InvalidInstanceDataError;

#[cfg(test)]
mod test {
    use std::{collections::HashSet, time::Duration};

    use super::domain;
    use carbon_repos::{
        db::{PrismaClient, read_filters::IntFilter},
        pcr::Direction,
    };
    use unicode_segmentation::UnicodeSegmentation;

    use crate::{
        domain::instance::{InstanceSettingsUpdate, info},
        managers::instance::{
            GroupId, GroupMoveTarget, InstanceId, InstanceMoveTarget, ListGroup, ListInstance,
            ListInstanceStatus, ValidListInstance,
        },
    };

    use super::InstanceVersionSource;

    #[tokio::test]
    async fn move_groups() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        async fn get_ordered_groups(prisma_client: &PrismaClient) -> anyhow::Result<Vec<GroupId>> {
            use carbon_repos::db::instance_group::OrderByParam;

            Ok(prisma_client
                .instance_group()
                .find_many(vec![])
                .order_by(OrderByParam::GroupIndex(Direction::Asc))
                .exec()
                .await?
                .into_iter()
                .map(|group| GroupId(group.id))
                .collect())
        }

        let mut groups = [
            app.instance_manager()
                .create_group(String::from("move0"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move1"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move2"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move3"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move4"))
                .await?,
        ];

        // move 1 to 1 (do nothing)
        app.instance_manager()
            .move_group(groups[1], GroupMoveTarget::BeforeGroup(groups[1]))
            .await?;
        assert_eq!(
            groups[..],
            get_ordered_groups(&app.prisma_client).await?[..]
        );

        // move 1 to 3 as if dragged
        app.instance_manager()
            .move_group(groups[1], GroupMoveTarget::BeforeGroup(groups[3]))
            .await?;
        groups = [groups[0], groups[2], groups[1], groups[3], groups[4]];
        assert_eq!(
            groups[..],
            get_ordered_groups(&app.prisma_client).await?[..]
        );

        // move 3 back to 1
        app.instance_manager()
            .move_group(groups[3], GroupMoveTarget::BeforeGroup(groups[1]))
            .await?;
        groups = [groups[0], groups[3], groups[1], groups[2], groups[4]];
        assert_eq!(
            groups[..],
            get_ordered_groups(&app.prisma_client).await?[..]
        );

        // move 1 to end of list
        app.instance_manager()
            .move_group(groups[1], GroupMoveTarget::EndOfLibrary)
            .await?;
        groups = [groups[0], groups[2], groups[3], groups[4], groups[1]];
        assert_eq!(
            groups[..],
            get_ordered_groups(&app.prisma_client).await?[..]
        );

        // move 4 to beginning of list
        app.instance_manager()
            .move_group(groups[4], GroupMoveTarget::BeforeGroup(groups[0]))
            .await?;
        groups = [groups[4], groups[0], groups[1], groups[2], groups[3]];
        assert_eq!(
            groups[..],
            get_ordered_groups(&app.prisma_client).await?[..]
        );

        Ok(())
    }

    #[tokio::test]
    async fn move_instances() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        async fn get_ordered_instances(
            prisma_client: &PrismaClient,
            group: GroupId,
        ) -> anyhow::Result<Vec<InstanceId>> {
            use carbon_repos::db::instance::{OrderByParam, WhereParam};

            Ok(prisma_client
                .instance()
                .find_many(vec![WhereParam::GroupId(IntFilter::Equals(*group))])
                .order_by(OrderByParam::Index(Direction::Asc))
                .exec()
                .await?
                .into_iter()
                .map(|instance| InstanceId(instance.id))
                .collect())
        }

        let [group0, group1] = [
            app.instance_manager()
                .create_group(String::from("group0"))
                .await?,
            app.instance_manager()
                .create_group(String::from("group1"))
                .await?,
        ];

        let mk_instance = |shortpath: &'static str, group| {
            let app = &app;
            async move {
                let id = app
                    .instance_manager()
                    .add_instance(shortpath.to_string(), shortpath.to_string(), group)
                    .await?;

                Ok::<_, anyhow::Error>(id)
            }
        };

        let mut group0_instances = [
            mk_instance("g0i0", group0.clone()).await?,
            mk_instance("g0i1", group0.clone()).await?,
            mk_instance("g0i2", group0.clone()).await?,
        ];
        // New instances prepend within their group, so DB-ascending order
        // is the reverse of creation order.
        group0_instances.reverse();

        let mut group1_instances = [
            mk_instance("g1i0", group1.clone()).await?,
            mk_instance("g1i1", group1.clone()).await?,
        ];
        group1_instances.reverse();

        // move 1 to 1 (do nothing)
        app.instance_manager()
            .move_instance(
                group0_instances[1],
                InstanceMoveTarget::Before(group0_instances[1]),
            )
            .await?;

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        // move 1 to end of list
        app.instance_manager()
            .move_instance(group0_instances[1], InstanceMoveTarget::EndOfGroup(group0))
            .await?;

        group0_instances = [
            group0_instances[0],
            group0_instances[2],
            group0_instances[1],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        // move 0 to end of list
        app.instance_manager()
            .move_instance(group0_instances[0], InstanceMoveTarget::EndOfGroup(group0))
            .await?;

        group0_instances = [
            group0_instances[1],
            group0_instances[2],
            group0_instances[0],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        // move 2 back to 0
        app.instance_manager()
            .move_instance(
                group0_instances[2],
                InstanceMoveTarget::Before(group0_instances[0]),
            )
            .await?;

        group0_instances = [
            group0_instances[2],
            group0_instances[0],
            group0_instances[1],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        app.instance_manager()
            .move_instance(
                group0_instances[2],
                InstanceMoveTarget::BeginningOfGroup(group0),
            )
            .await?;

        group0_instances = [
            group0_instances[2],
            group0_instances[0],
            group0_instances[1],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        // move 0:1 to 1:1
        app.instance_manager()
            .move_instance(
                group0_instances[1],
                InstanceMoveTarget::Before(group1_instances[1]),
            )
            .await?;

        let group1_instances = [
            group1_instances[0],
            group0_instances[1],
            group1_instances[1],
        ];

        let group0_instances = [group0_instances[0], group0_instances[2]];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        assert_eq!(
            group1_instances[..],
            get_ordered_instances(&app.prisma_client, group1).await?[..],
        );

        // move 0:0 to end of group 1
        app.instance_manager()
            .move_instance(group0_instances[0], InstanceMoveTarget::EndOfGroup(group1))
            .await?;

        let group1_instances = [
            group1_instances[0],
            group1_instances[1],
            group1_instances[2],
            group0_instances[0],
        ];

        let group0_instances = [group0_instances[1]];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        assert_eq!(
            group1_instances[..],
            get_ordered_instances(&app.prisma_client, group1).await?[..],
        );

        Ok(())
    }

    #[tokio::test]
    async fn delete_group() -> anyhow::Result<()> {
        use carbon_repos::db::instance::UniqueWhereParam::ShortpathEquals;

        let app = crate::setup_managers_for_test().await;

        let default_group = app.instance_manager().get_default_group().await?;
        let group = app
            .instance_manager()
            .create_group(String::from("foo"))
            .await?;
        app.instance_manager()
            .add_instance(String::from("baz"), String::from("baz"), default_group)
            .await?;
        app.instance_manager()
            .add_instance(String::from("bar"), String::from("bar"), group)
            .await?;

        let instance = app
            .prisma_client
            .instance()
            .find_unique(ShortpathEquals(String::from("bar")))
            .exec()
            .await?
            .unwrap();

        assert_eq!(instance.index, 0);
        assert_eq!(instance.group_id, *group);

        app.instance_manager().delete_group(group).await?;

        let instance = app
            .prisma_client
            .instance()
            .find_unique(ShortpathEquals(String::from("bar")))
            .exec()
            .await?
            .unwrap();

        // index should be `1` due to instance already present in default group.
        assert_eq!(instance.index, 1);
        assert_eq!(
            instance.group_id,
            *app.instance_manager().get_default_group().await?
        );

        Ok(())
    }

    #[tokio::test]
    async fn delete_group_empty() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        let group_count = app
            .prisma_client
            .instance_group()
            .count(vec![])
            .exec()
            .await?;

        // assert no default group exists
        assert_eq!(group_count, 0);

        let group = app
            .instance_manager()
            .create_group(String::from("foo"))
            .await?;

        let group_count = app
            .prisma_client
            .instance_group()
            .count(vec![])
            .exec()
            .await?;

        // assert only the created group exists
        assert_eq!(group_count, 1);

        app.instance_manager().delete_group(group).await?;

        let group_count = app
            .prisma_client
            .instance_group()
            .count(vec![])
            .exec()
            .await?;

        // assert the default group was not created while deleting the new group
        assert_eq!(group_count, 0);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "currently failing intermittently (probably due to restart_in_place)"]
    async fn instance_crud() -> anyhow::Result<()> {
        let mut app = crate::setup_managers_for_test().await;

        // create
        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];
        let instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                String::from("test"),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        let mut list = app.instance_manager().list_groups().await?;
        let mut expected = vec![ListGroup {
            id: default_group.id,
            name: default_group.name.clone(),
            library_position: None, // Default group has no library position
            instances: vec![ListInstance {
                id: instance_id,
                group_id: default_group.id,
                index: 0,
                library_position: list[0].instances[0].library_position, // Use actual value from DB
                name: String::from("test"),
                favorite: false,
                icon_revision: None,
                status: ListInstanceStatus::Valid(ValidListInstance {
                    mc_version: Some(String::from("1.7.10")),
                    modloader: None,
                    modloader_version: None,
                    modpack: None,
                    state: domain::LaunchState::Inactive { failed_task: None },
                }),
                locked: false,
                last_played: None,
                date_created: list[0].instances[0].date_created,
                date_updated: list[0].instances[0].date_updated,
                seconds_played: 0,
            }],
        }];

        assert_eq!(list, expected);

        // check that it was persisted
        app.restart_in_place().await;

        // wait for instance scan
        tokio::time::sleep(Duration::from_millis(100)).await;

        list = app.instance_manager().list_groups().await?;
        assert_eq!(list, expected);

        // update
        app.instance_manager()
            .update_instance(InstanceSettingsUpdate {
                instance_id,
                name: Some(String::from("test2")),
                use_loaded_icon: None,
                notes: None,
                version: None,
                modloader: None,
                global_java_args: None,
                extra_java_args: None,
                memory: None,
                java_override: None,
                pre_launch_hook: None,
                post_exit_hook: None,
                wrapper_command: None,
                game_resolution: None,
                modpack_locked: None,
                mod_sources: None,
            })
            .await?;

        expected[0].instances[0].name = String::from("test2");

        list = app.instance_manager().list_groups().await?;
        assert_eq!(list, expected);

        // check that it was persisted
        app.restart_in_place().await;

        // wait for instance scan
        tokio::time::sleep(Duration::from_millis(100)).await;

        list = app.instance_manager().list_groups().await?;
        assert_eq!(list, expected);

        // delete
        app.instance_manager().delete_instance(instance_id).await?;
        expected[0].instances.clear();

        list = app.instance_manager().list_groups().await?;
        assert_eq!(list, expected);

        // check that it was persisted
        app.restart_in_place().await;

        // wait for instance scan
        tokio::time::sleep(Duration::from_millis(100)).await;

        list = app.instance_manager().list_groups().await?;
        assert_eq!(list, expected);

        Ok(())
    }

    #[tokio::test]
    async fn test_modpack_info() -> anyhow::Result<()> {
        let mut app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];
        let curseforge_instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                String::from("curseforge instance"),
                false,
                InstanceVersionSource::Modpack(
                    info::Modpack::Curseforge(info::CurseforgeModpack {
                        // RLCraft
                        project_id: 285109,
                        file_id: 4612979,
                    }),
                    true,
                ),
                String::new(),
            )
            .await?;

        let modrinth_instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                String::from("modrinth instance"),
                false,
                InstanceVersionSource::Modpack(
                    info::Modpack::Modrinth(info::ModrinthModpack {
                        // Fabulously Optimized
                        project_id: String::from("1KVo5zza"),
                        version_id: String::from("HH3vor7X"),
                    }),
                    true,
                ),
                String::new(),
            )
            .await?;

        assert_eq!(
            app.prisma_client
                .curse_forge_modpack_cache()
                .find_many(vec![])
                .exec()
                .await?
                .len(),
            0
        );

        assert_eq!(
            app.prisma_client
                .modrinth_modpack_cache()
                .find_many(vec![])
                .exec()
                .await?
                .len(),
            0
        );

        app.instance_manager()
            .get_modpack_info(curseforge_instance_id)
            .await?;

        app.instance_manager()
            .get_modpack_info(modrinth_instance_id)
            .await?;

        assert_eq!(
            app.prisma_client
                .curse_forge_modpack_cache()
                .find_many(vec![])
                .exec()
                .await?
                .len(),
            1
        );

        assert_eq!(
            app.prisma_client
                .modrinth_modpack_cache()
                .find_many(vec![])
                .exec()
                .await?
                .len(),
            1
        );

        assert_eq!(
            app.prisma_client
                .curse_forge_modpack_image_cache()
                .find_many(vec![])
                .exec()
                .await?
                .len(),
            1
        );

        assert_eq!(
            app.prisma_client
                .modrinth_modpack_image_cache()
                .find_many(vec![])
                .exec()
                .await?
                .len(),
            1
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_next_folder_ascii() -> anyhow::Result<()> {
        let mut app = crate::setup_managers_for_test().await;

        let (instance_name, _) = app.instance_manager().next_folder("some_instance")?;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];
        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name, "some_instance");

        let (instance_name, _) = app.instance_manager().next_folder("some_instance")?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name, "some_instance1");

        let (instance_name, _) = app.instance_manager().next_folder("some_instance")?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name, "some_instance2");

        Ok(())
    }

    #[tokio::test]
    async fn text_next_folder_basic_unicode() -> anyhow::Result<()> {
        let mut app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];

        let (instance_name, _) = app.instance_manager().next_folder("ɀɃɏɔɮ˞˳̸")?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name, "ɀɃɏɔɮ˞˳̸");

        let (instance_name, _) = app
            .instance_manager()
            .next_folder("Cozy Cottage 𝘸𝘪𝘵𝘩 𝘴𝘢𝘶𝘤𝘦 🧂")?;

        assert_eq!(instance_name, "Cozy Cottage 𝘸𝘪𝘵𝘩 𝘴𝘢𝘶𝘤𝘦 🧂");

        Ok(())
    }

    #[tokio::test]
    async fn test_next_folder_unicode() -> anyhow::Result<()> {
        let mut app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];

        // Although the following two strings look the same, they are not.
        // Different filesystems handle it differently

        let e_with_1_byte = "é"; // precomposed (U+00E9)
        let e_with_2_bytes = "é"; // decomposed (U+0065 U+0301)

        let (instance_name, _) = app.instance_manager().next_folder(e_with_1_byte)?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name, e_with_1_byte);

        let (instance_name, _) = app.instance_manager().next_folder(e_with_2_bytes)?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        let comparison = if std::env::consts::OS == "macos" {
            format!("{}{}", e_with_2_bytes, "1") // macos saves as decomposed
        } else {
            e_with_2_bytes.to_string()
        };

        assert_eq!(instance_name, comparison);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "TODO: fix"]
    async fn test_next_folder_long_input() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;
        let default_group_id = app.instance_manager().get_default_group().await?;
        let default_group = &app.instance_manager().list_groups().await?[0];

        let e_with_2_bytes = "é"; // decomposed (U+0065 U+0301)

        // long string should be truncated with graphemes and not code points
        let mut long_string = String::new();
        for _ in 0..100 {
            long_string.push_str(e_with_2_bytes);
        }

        let (instance_name, _) = app.instance_manager().next_folder(&*long_string)?;

        app.instance_manager()
            .create_instance(
                default_group_id,
                instance_name.clone(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        assert_eq!(instance_name.graphemes(true).count(), 28);
        assert_eq!(instance_name.len(), 84); // UTF8 3 bytes * 28 allowed graphemes

        Ok(())
    }
}
