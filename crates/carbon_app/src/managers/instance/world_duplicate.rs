//! Duplicating a world, as a task the player can follow and cancel. The copy
//! is made in a staging folder beside `saves` while the original stays free to
//! play, and goes into `saves` in one rename only when the original did not
//! change meanwhile. Nothing here writes to the original world.
use super::{
    InstanceId, InstanceManager, InstanceRootGuard,
    datapacks::world_identity,
    installer::publication,
    mods::validate_addon_basename,
    world_copy::{Cancelled, Survey, copy_tree, free_space, survey, unchanged_since},
    world_folder_name::{free_folder_name, safe_folder_name},
    world_level::{read_level_name, write_level_name},
    world_refusal::WorldRefusal,
    world_rename::world_name,
};
use crate::{
    api::{
        keys::instance::{GET_WORLD_COPY, GET_WORLDS},
        translation::Translation,
    },
    domain::{instance::AddonType, vtask::VisualTaskId},
    managers::{
        ManagerRef,
        metadata::cache::CacheEntityId,
        runtime_lifecycle::{ActivityPolicy, spawn_retained},
        vtask::{Subtask, TaskState, VisualTask},
    },
    util::publish::rename_dir_noclobber,
};
use anyhow::{Context, bail};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Staging folders are named this, then the instance id, `-` and a random
/// part. The id lets each instance sweep only its own leftovers: a `saves`
/// linked into a shared folder sits beside other instances' copies.
const STAGING_PREFIX: &str = ".gdl-world-copy-";

/// The start of the names of `id`'s staging folders.
fn staging_prefix(id: InstanceId) -> String {
    format!("{STAGING_PREFIX}{}-", *id)
}

/// The instance a staging folder named `name` was made for, when its name
/// carries one.
fn staging_owner(name: &std::ffi::OsStr) -> Option<InstanceId> {
    let (id, _) = name
        .to_str()?
        .strip_prefix(STAGING_PREFIX)?
        .split_once('-')?;
    id.parse().ok().map(InstanceId)
}

/// Whether `name`, in an instance's data folder, is the launcher's own copy
/// of a world in the making, which is no content of the instance: duplicating
/// or exporting the instance leaves it out.
pub(crate) fn is_world_copy_staging(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .is_some_and(|name| name.starts_with(STAGING_PREFIX))
}

/// How many folder names a copy tries when the one it picked turns out taken
/// at the last moment (a name the file system folds differently).
const PUBLISH_TRIES: usize = 5;

/// Room the drive keeps beyond the world's own size, so a copy never fills it.
const SPACE_MARGIN: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DuplicateWorld {
    pub instance_id: i32,
    /// The save's folder name.
    pub world: String,
    /// The world's identity as the world list gave it.
    pub identity: String,
    /// The copy's in-game name.
    pub name: String,
}

/// The copy an instance is making.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCopy {
    /// The folder name of the world being copied.
    pub source: String,
    /// The copy's in-game name.
    pub name: String,
    pub task: VisualTaskId,
}

/// A copy that is running, with what stops it and what tells it ended.
#[derive(Debug)]
pub(super) struct RunningCopy {
    copy: WorldCopy,
    cancel: CancellationToken,
    /// Cancelled once the copy is off the list and its staging folder gone.
    ended: CancellationToken,
}

/// The copy each instance is making; registered and checked under the gate.
pub(super) type WorldCopies = dashmap::DashMap<InstanceId, RunningCopy>;

/// Takes the copy off the list when it ends, however it ends.
struct Registration<'a> {
    copies: &'a WorldCopies,
    id: InstanceId,
    task: VisualTaskId,
}

impl Drop for Registration<'_> {
    fn drop(&mut self) {
        if let Some((_, running)) = self
            .copies
            .remove_if(&self.id, |_, running| running.copy.task == self.task)
        {
            running.ended.cancel();
        }
    }
}

/// What the task works with once the gate is released.
struct Admitted {
    task: VisualTask,
    task_id: VisualTaskId,
    subtask: Subtask,
    survey: Survey,
    world: PathBuf,
    staging: PathBuf,
    name: String,
    root_hold: InstanceRootGuard,
    cancel: CancellationToken,
}

impl ManagerRef<'_, InstanceManager> {
    /// Starts a copy of a world under the in-game name `request.name`, as a
    /// visual task whose id this returns at once. What makes the copy
    /// impossible is refused before the task starts.
    pub async fn duplicate_world(self, request: DuplicateWorld) -> anyhow::Result<VisualTaskId> {
        let activity = self
            .app
            .runtime_lifecycle
            .admit_owned("duplicate a world", ActivityPolicy::Installer)?;
        // Stopped by the player, or with the activity when the launcher closes.
        let cancel = activity.cancellation().child_token();
        let id = InstanceId(request.instance_id);
        let admitted = match self.admit_world_copy(id, &request, cancel).await {
            Ok(admitted) => admitted,
            Err(error) => {
                // Nothing was written.
                activity.finish();
                return Err(error);
            }
        };
        let task_id = admitted.task_id;
        self.app.invalidate(GET_WORLD_COPY, Some((*id).into()));
        self.app.invalidate(GET_WORLDS, Some((*id).into()));

        let app = self.app.clone();
        spawn_retained(activity, move |_| {
            Box::pin(async move {
                app.instance_manager()
                    .run_world_copy(id, request, admitted)
                    .await
            })
        });
        Ok(task_id)
    }

    /// The copy the instance is making, if any.
    pub fn world_copy(self, id: InstanceId) -> Option<WorldCopy> {
        self.world_copies
            .get(&id)
            .map(|running| running.copy.clone())
    }

    /// Stops the instance's copy, if one runs; it leaves nothing behind.
    pub fn cancel_world_copy(self, id: InstanceId) {
        if let Some(running) = self.world_copies.get(&id) {
            running.cancel.cancel();
        }
    }

    /// Refuses a change to `world` while it is being copied: the copy would
    /// be thrown away, or hold half of the change. Call under the gate.
    pub(super) fn ensure_world_not_copied(
        &self,
        id: InstanceId,
        world: &str,
    ) -> anyhow::Result<()> {
        if self
            .world_copies
            .get(&id)
            .is_some_and(|running| running.copy.source == world)
        {
            bail!(WorldRefusal::BeingCopied);
        }
        Ok(())
    }

    /// Under the gate: the refusals, then the task and the registration.
    async fn admit_world_copy(
        self,
        id: InstanceId,
        request: &DuplicateWorld,
        cancel: CancellationToken,
    ) -> anyhow::Result<Admitted> {
        let operation = self.lock_instance(id).await;
        self.ensure_world_datapacks_may_change(id).await?;
        let name = world_name(&request.name)?.to_owned();
        validate_addon_basename(&request.world)?;
        let path = publication::current_path(&self.app, id).await?;
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let world = saves.join(&request.world);
        if world_identity(id, &path.get_root(), &world).await? != request.identity {
            bail!(WorldRefusal::WorldChanged);
        }
        let title = {
            let world = world.clone();
            tokio::task::spawn_blocking(move || read_level_name(&world)).await?
        };
        let title = match title {
            Ok(title) => title,
            Err(error) => {
                info!(
                    ?error,
                    "Instance {id}: could not read the name of {}",
                    world.display()
                );
                bail!(WorldRefusal::NameUnreadable);
            }
        };
        if self.world_copies.contains_key(&id) {
            bail!(WorldRefusal::BeingCopied);
        }
        let parent = staging_parent(&saves)?;
        let (survey, free) = {
            let (world, parent) = (world.clone(), parent.clone());
            tokio::task::spawn_blocking(move || {
                Ok::<_, anyhow::Error>((survey(&world)?, free_space(&parent)))
            })
            .await??
        };
        if !has_room(free, survey.bytes) {
            bail!(WorldRefusal::NoSpace);
        }
        let staging = parent.join(format!(
            "{}{}",
            staging_prefix(id),
            uuid::Uuid::new_v4().simple()
        ));

        let label = Translation::InstanceTaskDuplicateWorld {
            world_name: title.unwrap_or_else(|| request.world.clone()),
            instance_name: self.instance_name(id).await?,
        };
        let task = VisualTask::new(label.clone());
        task.require_explicit_completion();
        let subtask = task.subtask(label);
        task.edit(|data| data.state = TaskState::KnownProgress)
            .await;
        subtask.update_download(0, survey.bytes, false);
        let task_id = self.app.task_manager().spawn_task(&task).await;
        // Taken under the gate, so it never waits; a delete or a rename that
        // moves the instance folder stops the copy first (see
        // `stop_world_copy`).
        let root_hold = self.hold_root(&operation, false).await;
        // Nothing waits between this and the task's start, where the
        // registration is taken off again however the copy ends.
        self.world_copies.insert(
            id,
            RunningCopy {
                copy: WorldCopy {
                    source: request.world.clone(),
                    name: name.clone(),
                    task: task_id,
                },
                cancel: cancel.clone(),
                ended: CancellationToken::new(),
            },
        );
        // The copy does not hold up Play, scans or other changes.
        drop(operation);
        Ok(Admitted {
            task,
            task_id,
            subtask,
            survey,
            world,
            staging,
            name,
            root_hold,
            cancel,
        })
    }

    /// The task's work and its end: a cancel or a failure leaves nothing.
    async fn run_world_copy(self, id: InstanceId, request: DuplicateWorld, admitted: Admitted) {
        let registration = Registration {
            copies: &self.world_copies,
            id,
            task: admitted.task_id,
        };
        let Admitted {
            task,
            task_id,
            subtask,
            survey,
            world,
            mut staging,
            name,
            root_hold,
            cancel,
        } = admitted;
        let result = self
            .copy_and_publish(
                id,
                &request,
                CopyJob {
                    world,
                    name,
                    survey,
                    subtask,
                    root_hold,
                },
                &mut staging,
                &cancel,
            )
            .await;
        if result.is_err() {
            remove_staging(&staging).await;
        }
        drop(registration);
        self.app.invalidate(GET_WORLD_COPY, Some((*id).into()));
        match result {
            Ok(_) => {
                self.refresh_worlds(id, "a duplicate").await;
                task.complete();
            }
            Err(error) => {
                self.app.invalidate(GET_WORLDS, Some((*id).into()));
                if error.downcast_ref::<Cancelled>().is_some() {
                    self.app.task_manager().withdraw_task(task_id).await;
                } else {
                    // One line, with the code of a refusal in it.
                    task.fail(crate::error::flattened(&error)).await;
                }
            }
        }
    }

    /// Copies the world into `staging`, then, under the gate and only when
    /// the original is as it was, moves the copy into `saves`. Returns the
    /// copy's folder name. `staging` is where the copy is now: a rename of
    /// the instance the copy waited for may have moved it.
    async fn copy_and_publish(
        self,
        id: InstanceId,
        request: &DuplicateWorld,
        job: CopyJob,
        staging: &mut PathBuf,
        cancel: &CancellationToken,
    ) -> anyhow::Result<String> {
        let CopyJob {
            world,
            name,
            survey,
            subtask,
            root_hold,
        } = job;
        #[cfg(test)]
        self.test_hooks.pause("world_copy_before_files", id).await;
        let survey = {
            let (world, staging, name, cancel) =
                (world.clone(), staging.clone(), name.clone(), cancel.clone());
            tokio::task::spawn_blocking(move || {
                let total = survey.bytes;
                let progress = |copied| subtask.update_download(copied, total, false);
                copy_tree(&world, &staging, &survey, &progress, &cancel)?;
                write_level_name(&staging, &name)?;
                Ok::<_, anyhow::Error>(survey)
            })
            .await??
        };
        // Given back before the gate is taken (the gate comes first).
        drop(root_hold);
        #[cfg(test)]
        self.test_hooks.pause("world_copy_before_publish", id).await;

        let operation = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                // A rename of the instance may have moved the folder meanwhile.
                if let Ok(moved) = self.current_staging(id, staging).await {
                    *staging = moved;
                }
                return Err(Cancelled.into());
            }
            operation = self.lock_instance(id) => operation,
        };
        *staging = self.current_staging(id, staging).await?;
        if cancel.is_cancelled() {
            return Err(Cancelled.into());
        }
        let path = publication::current_path(&self.app, id).await?;
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        // A game started meanwhile may be writing the original, so the copy
        // is not kept, and the refusal says why.
        self.ensure_game_inactive(id).await?;
        // The pack may have been locked while the copy ran.
        self.ensure_world_datapacks_may_change(id).await?;
        let original = saves.join(&request.world);
        let untouched = world_identity(id, &path.get_root(), &original)
            .await
            .is_ok_and(|identity| identity == request.identity)
            && tokio::task::spawn_blocking(move || unchanged_since(&original, &survey))
                .await?
                // A world that cannot be read again cannot be shown untouched.
                .unwrap_or(false);
        if !untouched {
            bail!(WorldRefusal::ChangedWhileCopied);
        }

        let taken = self.taken_world_names(id, &saves).await?;
        let mut tried = Vec::new();
        let folder = loop {
            let all: Vec<String> = taken.iter().chain(&tried).cloned().collect();
            let folder = free_folder_name(&safe_folder_name(&name), &all, None);
            let target = saves.join(&folder);
            match rename_dir_noclobber(staging, &target).await {
                Ok(()) => break folder,
                // A name judged free that the file system holds already.
                Err(error)
                    if error.kind() == std::io::ErrorKind::AlreadyExists
                        && tried.len() + 1 < PUBLISH_TRIES =>
                {
                    info!(
                        "Instance {id}: {} is taken, trying another name",
                        target.display()
                    );
                    tried.push(folder);
                }
                Err(error) => {
                    return Err(anyhow::Error::from(error).context(format!(
                        "moving {} to {}",
                        staging.display(),
                        target.display()
                    )));
                }
            }
        };
        // The copy is in place: from here on nothing fails it. A row the
        // scan will make later still lists it.
        let cache = self.app.meta_cache_manager();
        if let Err(error) = cache
            .refresh_published_addon(
                &operation,
                &saves,
                folder.clone(),
                true,
                AddonType::Worlds.to_db_string().to_owned(),
            )
            .await
        {
            warn!(
                ?error,
                "Instance {id}: could not record the copied world {folder}"
            );
            cache
                .queue_caching(CacheEntityId::Instance(id), false)
                .await;
        }
        drop(operation);
        Ok(folder)
    }

    /// Where `staging` is now, beside the instance's current `saves`.
    async fn current_staging(self, id: InstanceId, staging: &Path) -> anyhow::Result<PathBuf> {
        let path = publication::current_path(&self.app, id).await?;
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let folder = staging
            .file_name()
            .with_context(|| format!("{} has no folder name", staging.display()))?;
        Ok(staging_parent(&saves)?.join(folder))
    }

    /// Stops the instance's copy, if one runs, and waits until it ended and
    /// its staging folder is gone. Called under the gate by a delete or a
    /// rename that moves the instance folder, before they take the folder:
    /// the copy would be lost either way, and they need not wait for it.
    pub(super) async fn stop_world_copy(&self, id: InstanceId) {
        let ended = self.world_copies.get(&id).map(|running| {
            running.cancel.cancel();
            running.ended.clone()
        });
        if let Some(ended) = ended {
            ended.cancelled().await;
        }
    }

    /// Removes the instance's staging folders beside `saves` when it runs no
    /// copy: those a crash or a power cut left. A copy registers under the
    /// gate before it makes its folder and this runs under the same gate,
    /// so it never sees one being made. In the instance's own data folder
    /// every staging folder goes, whatever instance id it carries (an
    /// instance duplicate may have brought one along); beside a `saves`
    /// linked elsewhere only the instance's own do. A folder whose id names
    /// an instance copying now stays: another instance's `saves` may link
    /// here. Nothing else is touched.
    pub(super) async fn sweep_world_copy_staging(self, id: InstanceId) -> anyhow::Result<()> {
        let _operation = self.lock_instance(id).await;
        if self.world_copies.contains_key(&id) {
            return Ok(());
        }
        let path = publication::current_path(&self.app, id).await?;
        // A `saves` the list cannot use either has nothing to sweep.
        let Ok(saves) = publication::checked_folder(&path, AddonType::Worlds).await else {
            return Ok(());
        };
        if !publication::followed_folder(&saves).await? {
            return Ok(());
        }
        let parent = staging_parent(&saves)?;
        let own = staging_prefix(id);
        let in_own_data_folder = own_data_folder(&path).is_some_and(|data| data == parent);
        let mut entries = tokio::fs::read_dir(&parent)
            .await
            .with_context(|| format!("listing {}", parent.display()))?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name();
            let is_staging = match in_own_data_folder {
                true => is_world_copy_staging(&name),
                false => name.to_str().is_some_and(|name| name.starts_with(&own)),
            };
            // Not followed: a link is never what a copy made.
            if !is_staging || !entry.file_type().await?.is_dir() {
                continue;
            }
            // A copy registers before it makes its folder, under a new name each
            // time, so a listed folder of an instance not copying now is a leftover.
            if staging_owner(&name).is_some_and(|owner| self.world_copies.contains_key(&owner)) {
                continue;
            }
            info!(
                "Instance {id}: removing {}, left by a world copy that did not finish",
                entry.path().display()
            );
            remove_staging(&entry.path()).await;
        }
        Ok(())
    }
}

/// The copy's own part of the task's work.
struct CopyJob {
    world: PathBuf,
    name: String,
    survey: Survey,
    subtask: Subtask,
    root_hold: InstanceRootGuard,
}

/// Where staging folders go: beside the folder `saves` resolves to, so on
/// its drive even when `saves` is a link to another one.
fn staging_parent(saves: &Path) -> anyhow::Result<PathBuf> {
    let resolved =
        dunce::canonicalize(saves).with_context(|| format!("resolving {}", saves.display()))?;
    Ok(resolved
        .parent()
        .with_context(|| format!("{} has no parent folder", resolved.display()))?
        .to_owned())
}

/// The instance's data folder as resolved, when it is the instance's own:
/// inside its folder, not a link to another place.
fn own_data_folder(path: &carbon_rt_path::InstancePath) -> Option<PathBuf> {
    let root = dunce::canonicalize(path.get_root()).ok()?;
    let data = path.get_data_path();
    let own = root.join(data.file_name()?);
    (dunce::canonicalize(&data).ok()? == own).then_some(own)
}

/// Whether a drive with `free` bytes left takes a copy of `bytes`. A drive
/// whose room cannot be read is not refused.
fn has_room(free: Option<u64>, bytes: u64) -> bool {
    free.is_none_or(|free| {
        bytes
            .checked_add(SPACE_MARGIN)
            .is_some_and(|needed| free >= needed)
    })
}

async fn remove_staging(staging: &Path) {
    let folder = staging.to_owned();
    let removed = tokio::task::spawn_blocking(move || match std::fs::remove_dir_all(&folder) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    })
    .await;
    match removed {
        Ok(Ok(())) => {}
        Ok(Err(error)) => warn!(?error, "could not remove {}", staging.display()),
        Err(error) => warn!(?error, "could not remove {}", staging.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::instance::AddonType;
    use crate::error::coded_cause;
    use crate::managers::App;
    use crate::managers::instance::{
        datapacks::{CopyDatapack, DatapackWorld},
        installer::publication,
        lifecycle_tests::Pause,
        mods::tests::addon_fixture,
        run::{LaunchState, RunningInstance},
        world_datapack_changes::{
            DeleteWorldDatapack, InstallWorldDatapack, WorldDatapackSource,
            tests::{api, modrinth_serving, mr_version, pack_zip, put_pack},
        },
        world_datapacks::World,
        world_level::{level_dat_bytes, read_level_name},
        world_rename::RenameWorld,
    };
    use crate::managers::vtask::TaskOutcome;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    /// An instance with one data pack of its own and the save `New World`,
    /// whose in-game name is "Home", holding one data pack and a region
    /// file. Returns the instance, its `saves` folder, the world as the list
    /// shows it and the id of the instance's data pack.
    async fn world_instance(app: &App, instance: &str) -> (InstanceId, PathBuf, World, String) {
        let (id, row, _) = addon_fixture(
            app,
            instance,
            AddonType::DataPacks,
            "instance.zip",
            &pack_zip("instance"),
        )
        .await;
        let saves = publication::current_path(app, id)
            .await
            .unwrap()
            .get_saves_path();
        std::fs::create_dir_all(&saves).unwrap();
        let world = add_world(app, id, &saves, "New World", "Home").await;
        (id, saves, world, row.id)
    }

    /// Adds the save `folder` named `title`, with a data pack, and its row.
    async fn add_world(
        app: &App,
        id: InstanceId,
        saves: &Path,
        folder: &str,
        title: &str,
    ) -> World {
        let world = saves.join(folder);
        std::fs::create_dir_all(world.join("region")).unwrap();
        std::fs::write(world.join("level.dat"), level_dat_bytes(Some(title))).unwrap();
        std::fs::write(world.join("region/r.0.0.mca"), vec![7u8; 300_000]).unwrap();
        put_pack(&world, "pack.zip", &pack_zip(folder));
        {
            let operation = app.instance_manager().lock_instance(id).await;
            app.meta_cache_manager()
                .refresh_published_addon(
                    &operation,
                    &saves.to_path_buf(),
                    folder.to_owned(),
                    true,
                    AddonType::Worlds.to_db_string().to_owned(),
                )
                .await
                .unwrap();
        }
        listed(app, id)
            .await
            .into_iter()
            .find(|world| world.name == folder)
            .unwrap()
    }

    async fn listed(app: &App, id: InstanceId) -> Vec<World> {
        app.instance_manager().list_worlds(id).await.unwrap()
    }

    fn request(id: InstanceId, world: &World, name: &str) -> DuplicateWorld {
        DuplicateWorld {
            instance_id: *id,
            world: world.name.clone(),
            identity: world.identity.clone().unwrap(),
            name: name.into(),
        }
    }

    async fn duplicate(app: &App, request: DuplicateWorld) -> anyhow::Result<VisualTaskId> {
        app.instance_manager().duplicate_world(request).await
    }

    /// Waits for the task to end.
    async fn wait(app: &App, task: VisualTaskId) -> TaskOutcome {
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                match app.task_manager().get_task_outcome(task).await {
                    TaskOutcome::Running(_) => {}
                    outcome => return outcome,
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the copy did not end")
    }

    /// The code the task failed with.
    fn failed_code(outcome: &TaskOutcome) -> Option<String> {
        match outcome {
            TaskOutcome::Failed(task) => match &task.progress {
                crate::domain::vtask::Progress::Failed(error) => {
                    coded_cause(error).map(|(code, _)| code)
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn code(error: &anyhow::Error) -> Option<String> {
        coded_cause(error).map(|(code, _)| code)
    }

    fn entries(folder: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// The staging folders beside `saves`.
    fn staging_folders(saves: &Path) -> Vec<String> {
        let parent = dunce::canonicalize(saves).unwrap();
        entries(parent.parent().unwrap())
            .into_iter()
            .filter(|name| name.starts_with(".gdl-world-copy-"))
            .collect()
    }

    /// Every path under `folder` with its bytes and modified time.
    fn snapshot(folder: &Path) -> Vec<(String, Vec<u8>, Option<std::time::SystemTime>)> {
        let mut all = Vec::new();
        for entry in walkdir::WalkDir::new(folder).sort_by_file_name() {
            let entry = entry.unwrap();
            let meta = entry.metadata().unwrap();
            let bytes = if meta.is_file() {
                std::fs::read(entry.path()).unwrap()
            } else {
                Vec::new()
            };
            let modified = meta.is_file().then(|| meta.modified().unwrap());
            all.push((entry.path().display().to_string(), bytes, modified));
        }
        all
    }

    async fn set_state(app: &App, id: InstanceId, state: LaunchState) {
        app.instance_manager()
            .instances
            .write()
            .await
            .get_mut(&id)
            .unwrap()
            .data_mut()
            .unwrap()
            .state = state;
    }

    fn running() -> LaunchState {
        LaunchState::Running(RunningInstance::adopted(
            u32::MAX,
            1,
            chrono::Utc::now(),
            chrono::Utc::now(),
        ))
    }

    /// Starts a duplicate held at the test pause `at`.
    async fn held(
        app: &App,
        id: InstanceId,
        at: &'static str,
        request: DuplicateWorld,
    ) -> (Arc<Pause>, VisualTaskId) {
        let pause = app.instance_manager().test_hooks.arm(at, id);
        let task = duplicate(app, request).await.unwrap();
        tokio::time::timeout(Duration::from_secs(10), pause.entered.notified())
            .await
            .unwrap();
        (pause, task)
    }

    #[test]
    fn a_drive_needs_room_for_the_world_and_a_margin() {
        assert!(has_room(None, u64::MAX));
        assert!(has_room(Some(SPACE_MARGIN + 10), 10));
        assert!(!has_room(Some(SPACE_MARGIN + 9), 10));
        assert!(!has_room(Some(u64::MAX), u64::MAX));
    }

    #[tokio::test]
    async fn a_duplicate_is_a_second_world_with_its_data_packs() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-world").await;
        let before = snapshot(&saves.join("New World"));

        let task = duplicate(&app, request(id, &world, "Home (copy)"))
            .await
            .unwrap();

        assert!(
            matches!(wait(&app, task).await, TaskOutcome::Succeeded),
            "the copy failed"
        );
        assert_eq!(entries(&saves), vec!["Home (copy)", "New World"]);
        let worlds = listed(&app, id).await;
        assert_eq!(worlds.len(), 2, "{worlds:?}");
        let copy = worlds.iter().find(|w| w.name == "Home (copy)").unwrap();
        let original = worlds.iter().find(|w| w.name == "New World").unwrap();
        assert_eq!(copy.title.as_deref(), Some("Home (copy)"));
        assert_eq!(original.title.as_deref(), Some("Home"));
        assert_eq!(copy.datapacks.len(), 1, "{copy:?}");
        assert_eq!(copy.datapacks[0].filename, "pack.zip");
        assert_ne!(copy.identity, original.identity);
        assert!(copy.identity.is_some());
        assert_ne!(copy.addon_id, original.addon_id);
        assert_ne!(copy.datapacks[0].id, original.datapacks[0].id);
        assert_eq!(
            std::fs::read(saves.join("Home (copy)/region/r.0.0.mca")).unwrap(),
            vec![7u8; 300_000]
        );
        assert_eq!(snapshot(&saves.join("New World")), before);
        assert!(staging_folders(&saves).is_empty());
        assert_eq!(app.instance_manager().world_copy(id), None);
    }

    #[tokio::test]
    async fn a_taken_folder_name_gets_a_number() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-taken").await;
        add_world(&app, id, &saves, "Home (copy)", "Elsewhere").await;
        let world = listed(&app, id)
            .await
            .into_iter()
            .find(|w| w.name == world.name)
            .unwrap();

        let task = duplicate(&app, request(id, &world, "Home (copy)"))
            .await
            .unwrap();

        assert!(matches!(wait(&app, task).await, TaskOutcome::Succeeded));
        assert_eq!(
            entries(&saves),
            vec!["Home (copy)", "Home (copy) (2)", "New World"]
        );
        assert_eq!(
            read_level_name(&saves.join("Home (copy) (2)"))
                .unwrap()
                .as_deref(),
            Some("Home (copy)")
        );
        assert_eq!(
            read_level_name(&saves.join("Home (copy)"))
                .unwrap()
                .as_deref(),
            Some("Elsewhere")
        );
    }

    #[tokio::test]
    async fn the_copy_is_reported_while_it_runs() {
        let app = crate::setup_managers_for_test().await;
        let (id, _, world, _) = world_instance(&app, "duplicate-reported").await;

        let (pause, task) = held(
            &app,
            id,
            "world_copy_before_files",
            request(id, &world, "Home (copy)"),
        )
        .await;

        assert_eq!(
            app.instance_manager().world_copy(id),
            Some(WorldCopy {
                source: "New World".into(),
                name: "Home (copy)".into(),
                task,
            })
        );
        pause.release.notify_one();
        assert!(matches!(wait(&app, task).await, TaskOutcome::Succeeded));
        assert_eq!(app.instance_manager().world_copy(id), None);
    }

    #[tokio::test]
    async fn the_world_being_copied_cannot_be_changed() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, instance_pack) = world_instance(&app, "duplicate-blocks").await;
        let other = add_world(&app, id, &saves, "Other", "Other").await;
        let world = listed(&app, id)
            .await
            .into_iter()
            .find(|w| w.name == world.name)
            .unwrap();
        let identity = world.identity.clone().unwrap();
        let mut api = api().await;
        let zip = pack_zip("new");
        let url = api.file("new.zip", &zip).await;
        modrinth_serving(
            &mut api,
            vec![mr_version(
                "release",
                10,
                "release",
                &["datapack"],
                ("new.zip", &url, &zip),
            )],
        )
        .await;
        let (pause, task) = held(
            &app,
            id,
            "world_copy_before_files",
            request(id, &world, "Home (copy)"),
        )
        .await;
        let before = snapshot(&saves.join("New World"));
        let manager = app.instance_manager();

        let mut refusals = vec![
            (
                "rename",
                manager
                    .rename_world(RenameWorld {
                        instance_id: *id,
                        world: world.name.clone(),
                        identity: identity.clone(),
                        name: "Elsewhere".into(),
                    })
                    .await
                    .map(|_| ()),
            ),
            (
                "delete",
                manager
                    .delete_mods(id, vec![world.addon_id.clone()])
                    .await
                    .unwrap()
                    .pop()
                    .unwrap()
                    .1
                    .map(|_| ()),
            ),
            (
                "install a data pack",
                manager
                    .install_world_datapack_from(
                        &api.curseforge,
                        &api.modrinth,
                        InstallWorldDatapack {
                            instance_id: *id,
                            world: world.name.clone(),
                            identity: identity.clone(),
                            source: WorldDatapackSource::Modrinth {
                                project_id: "project".into(),
                                version_id: None,
                            },
                            replaces: None,
                        },
                    )
                    .await
                    .map(|_| ()),
            ),
            (
                "remove a data pack",
                manager
                    .delete_world_datapack(DeleteWorldDatapack {
                        instance_id: *id,
                        id: world.datapacks[0].id.clone(),
                    })
                    .await,
            ),
            (
                "add a data pack from the instance",
                manager
                    .copy_datapack(CopyDatapack {
                        instance_id: *id,
                        mod_id: instance_pack,
                        world: DatapackWorld {
                            name: world.name.clone(),
                            identity: identity.clone(),
                        },
                        replace: false,
                    })
                    .await
                    .map(|_| ()),
            ),
        ];
        refusals.push((
            "duplicate",
            duplicate(&app, request(id, &world, "Again"))
                .await
                .map(|_| ()),
        ));
        for (what, result) in refusals {
            let error = result.expect_err(what);
            assert_eq!(
                code(&error).as_deref(),
                Some("WORLD_BEING_COPIED"),
                "{what}: {error:#}"
            );
        }
        assert_eq!(snapshot(&saves.join("New World")), before);
        assert_eq!(listed(&app, id).await.len(), 2);

        // Another world of the instance is not held up.
        manager
            .rename_world(RenameWorld {
                instance_id: *id,
                world: other.name.clone(),
                identity: other.identity.clone().unwrap(),
                name: "Renamed".into(),
            })
            .await
            .unwrap();

        pause.release.notify_one();
        assert!(matches!(wait(&app, task).await, TaskOutcome::Succeeded));
        assert_eq!(entries(&saves), vec!["Home (copy)", "New World", "Renamed"]);
    }

    #[tokio::test]
    async fn a_world_played_while_it_is_copied_is_not_kept() {
        use std::io::Write;
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-played").await;
        let (pause, task) = held(
            &app,
            id,
            "world_copy_before_publish",
            request(id, &world, "Home (copy)"),
        )
        .await;

        std::fs::OpenOptions::new()
            .append(true)
            .open(saves.join("New World/region/r.0.0.mca"))
            .unwrap()
            .write_all(b"played")
            .unwrap();
        pause.release.notify_one();

        let outcome = wait(&app, task).await;
        assert_eq!(
            failed_code(&outcome).as_deref(),
            Some("WORLD_CHANGED_WHILE_COPIED")
        );
        assert_eq!(entries(&saves), vec!["New World"]);
        assert!(staging_folders(&saves).is_empty());
        assert_eq!(app.instance_manager().world_copy(id), None);
    }

    #[tokio::test]
    async fn a_game_running_at_the_end_is_not_kept() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-running").await;
        let (pause, task) = held(
            &app,
            id,
            "world_copy_before_publish",
            request(id, &world, "Home (copy)"),
        )
        .await;

        set_state(&app, id, running()).await;
        pause.release.notify_one();

        let outcome = wait(&app, task).await;
        set_state(&app, id, LaunchState::Inactive { failed_task: None }).await;
        // Said as it is: the world itself may not have changed.
        assert_eq!(
            failed_code(&outcome).as_deref(),
            Some("INSTANCE_GAME_ACTIVE")
        );
        assert_eq!(entries(&saves), vec!["New World"]);
        assert!(staging_folders(&saves).is_empty());
    }

    #[tokio::test]
    async fn a_cancel_leaves_nothing() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-cancel").await;
        let (pause, task) = held(
            &app,
            id,
            "world_copy_before_files",
            request(id, &world, "Home (copy)"),
        )
        .await;

        app.instance_manager().cancel_world_copy(id);
        pause.release.notify_one();

        let outcome = wait(&app, task).await;
        assert!(matches!(outcome, TaskOutcome::Dismissed));
        assert_eq!(entries(&saves), vec!["New World"]);
        assert!(staging_folders(&saves).is_empty());
        assert_eq!(app.instance_manager().world_copy(id), None);

        let task = duplicate(&app, request(id, &world, "Home (copy)"))
            .await
            .unwrap();
        assert!(matches!(wait(&app, task).await, TaskOutcome::Succeeded));
    }

    #[tokio::test]
    async fn refused_before_it_starts() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-refused").await;
        let tasks = app.task_manager().get_tasks().await.len();
        let level = saves.join("New World/level.dat");
        let original = std::fs::read(&level).unwrap();

        let cases: Vec<(&str, DuplicateWorld, Box<dyn Fn()>)> = vec![
            (
                "WORLD_CHANGED",
                DuplicateWorld {
                    identity: "another world".into(),
                    ..request(id, &world, "Home (copy)")
                },
                Box::new(|| {}),
            ),
            (
                "WORLD_NAME_INVALID",
                request(id, &world, "   "),
                Box::new(|| {}),
            ),
            (
                "WORLD_NAME_UNREADABLE",
                request(id, &world, "Home (copy)"),
                Box::new(|| std::fs::write(&level, b"not gzip").unwrap()),
            ),
        ];
        for (expected, request, prepare) in cases {
            prepare();
            let error = duplicate(&app, request).await.unwrap_err();
            std::fs::write(&level, &original).unwrap();
            assert_eq!(code(&error).as_deref(), Some(expected), "{error:#}");
            assert_eq!(entries(&saves), vec!["New World"], "{expected}");
            assert!(staging_folders(&saves).is_empty(), "{expected}");
            assert_eq!(app.instance_manager().world_copy(id), None, "{expected}");
            assert_eq!(
                app.task_manager().get_tasks().await.len(),
                tasks,
                "{expected}"
            );
        }

        set_state(&app, id, running()).await;
        let error = duplicate(&app, request(id, &world, "Home (copy)"))
            .await
            .unwrap_err();
        set_state(&app, id, LaunchState::Inactive { failed_task: None }).await;
        assert_eq!(code(&error).as_deref(), Some("INSTANCE_GAME_ACTIVE"));

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&level, saves.join("New World/link.dat")).unwrap();
            let error = duplicate(&app, request(id, &world, "Home (copy)"))
                .await
                .unwrap_err();
            assert_eq!(code(&error).as_deref(), Some("WORLD_HOLDS_LINK"));
        }
        assert_eq!(entries(&saves), vec!["New World"]);
        assert!(staging_folders(&saves).is_empty());
        assert_eq!(app.instance_manager().world_copy(id), None);
        assert_eq!(app.task_manager().get_tasks().await.len(), tasks);
    }

    #[tokio::test]
    async fn a_staging_folder_left_by_a_crash_is_removed_when_the_worlds_are_listed() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, _, _) = world_instance(&app, "duplicate-leftover").await;
        let parent = dunce::canonicalize(&saves)
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();
        let leftover = parent.join(format!(".gdl-world-copy-{}-left", *id));
        std::fs::create_dir_all(leftover.join("region")).unwrap();
        std::fs::write(leftover.join("region/r.0.0.mca"), b"half").unwrap();
        // In the instance's own folder every staging folder is a leftover,
        // whatever instance it was made for: one an instance duplicate
        // carried over has the old instance's id.
        let foreign = [
            format!(".gdl-world-copy-{}-starting", *id + 1),
            format!(".gdl-world-copy-{}x-left", *id),
            ".gdl-world-copy-left".to_owned(),
        ];
        for name in &foreign {
            std::fs::create_dir(parent.join(name)).unwrap();
        }
        std::fs::create_dir(parent.join("kept")).unwrap();
        let note = parent.join(format!(".gdl-world-copy-{}-note.txt", *id));
        std::fs::write(&note, b"a file").unwrap();

        listed(&app, id).await;

        assert!(!leftover.exists());
        for name in &foreign {
            assert!(!parent.join(name).exists(), "{name}");
        }
        assert!(parent.join("kept").is_dir());
        assert!(note.is_file());
        assert_eq!(entries(&saves), vec!["New World"]);
    }

    /// A `saves` linked into another instance's data folder puts that
    /// instance's staging folder there; the owner's listing leaves it alone
    /// while the copy runs.
    #[tokio::test]
    async fn a_staging_folder_of_another_running_copy_is_kept_in_the_own_data_folder() {
        let app = crate::setup_managers_for_test().await;
        let (owner, owner_saves, _, _) = world_instance(&app, "duplicate-sweep-owner").await;
        let (copier, _, world, _) = world_instance(&app, "duplicate-sweep-copier").await;
        let (pause, task) = held(
            &app,
            copier,
            "world_copy_before_publish",
            request(copier, &world, "Home (copy)"),
        )
        .await;
        let parent = dunce::canonicalize(&owner_saves)
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();
        let running = parent.join(format!(".gdl-world-copy-{}-running", *copier));
        std::fs::create_dir_all(running.join("region")).unwrap();

        listed(&app, owner).await;
        assert!(running.join("region").is_dir());

        pause.release.notify_one();
        assert!(matches!(wait(&app, task).await, TaskOutcome::Succeeded));
        listed(&app, owner).await;
        assert!(!running.exists());
    }

    /// Waits until `work` is done, failing the test if it takes long.
    async fn soon<T>(work: impl std::future::Future<Output = T>, what: &str) -> T {
        tokio::time::timeout(Duration::from_secs(10), work)
            .await
            .unwrap_or_else(|_| panic!("{what} did not end"))
    }

    #[tokio::test]
    async fn an_instance_duplicated_during_a_copy_does_not_take_the_copy() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-instance-during").await;
        let (pause, task) = held(
            &app,
            id,
            "world_copy_before_publish",
            request(id, &world, "Home (copy)"),
        )
        .await;
        // The running copy's folder is not a leftover.
        assert_eq!(listed(&app, id).await.len(), 1);
        let staged = staging_folders(&saves);
        assert_eq!(staged.len(), 1);
        let explored = app
            .instance_manager()
            .explore_data(id, Vec::new())
            .await
            .unwrap();
        assert!(explored.iter().any(|entry| entry.name == "saves"));
        assert!(
            !explored
                .iter()
                .any(|entry| entry.name.starts_with(".gdl-world-copy-")),
            "{explored:?}"
        );

        let copy = soon(
            app.instance_manager()
                .duplicate_instance(id, "Duplicated".into()),
            "the instance duplicate",
        )
        .await
        .unwrap();

        let copied = publication::current_path(&app, copy).await.unwrap();
        assert!(staging_folders(&copied.get_saves_path()).is_empty());
        assert_eq!(entries(&copied.get_saves_path()), vec!["New World"]);
        assert!(
            copied
                .get_data_path()
                .join("saves/New World/level.dat")
                .is_file()
        );
        assert_eq!(staging_folders(&saves), staged);
        pause.release.notify_one();
        assert!(matches!(wait(&app, task).await, TaskOutcome::Succeeded));
        assert_eq!(entries(&saves), vec!["Home (copy)", "New World"]);
    }

    /// A rename that moves the instance folder, or a delete, stops the copy
    /// instead of waiting for it: the copy is lost either way. It ends as
    /// cancelled, with nothing shown and nothing left.
    #[tokio::test]
    async fn moving_or_deleting_the_instance_stops_its_copy() {
        use crate::managers::instance::world_datapack_changes::tests::{
            renamed, without_recycle_bin,
        };
        for (n, at) in ["world_copy_before_files", "world_copy_before_publish"]
            .into_iter()
            .enumerate()
        {
            let app = crate::setup_managers_for_test().await;
            without_recycle_bin(&app).await;
            let instance = format!("duplicate-stopped-{n}");
            let (id, _, world, _) = world_instance(&app, &instance).await;

            let (pause, task) = held(&app, id, at, request(id, &world, "Home (copy)")).await;
            let manager = app.clone();
            let new_name = format!("{instance}-renamed");
            let rename = tokio::spawn(async move {
                manager
                    .instance_manager()
                    .update_instance(renamed(id, &new_name))
                    .await
            });
            tokio::time::sleep(Duration::from_millis(50)).await;
            pause.release.notify_one();
            soon(rename, "the rename").await.unwrap().unwrap();
            assert!(
                matches!(wait(&app, task).await, TaskOutcome::Dismissed),
                "{at}"
            );
            let moved = publication::current_path(&app, id).await.unwrap();
            assert!(moved.get_root().ends_with(format!("{instance}-renamed")));
            assert_eq!(entries(&moved.get_saves_path()), vec!["New World"], "{at}");
            assert!(staging_folders(&moved.get_saves_path()).is_empty(), "{at}");
            assert_eq!(app.instance_manager().world_copy(id), None);

            let world = listed(&app, id).await.pop().unwrap();
            let (pause, task) = held(&app, id, at, request(id, &world, "Home (copy)")).await;
            let manager = app.clone();
            let delete =
                tokio::spawn(async move { manager.instance_manager()._delete_instance(id).await });
            tokio::time::sleep(Duration::from_millis(50)).await;
            pause.release.notify_one();
            soon(delete, "the delete").await.unwrap().unwrap();
            assert!(
                matches!(wait(&app, task).await, TaskOutcome::Dismissed),
                "{at}"
            );
            assert!(!moved.get_root().exists(), "{at}");
            assert_eq!(app.instance_manager().world_copy(id), None);
        }
    }

    #[tokio::test]
    async fn a_copy_held_open_by_a_scanner_for_a_moment_is_published() {
        use crate::util::publish::fault::{Step, inject};
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-scanned").await;
        inject(Step::NoReplace, &saves.join("Home (copy)"), 5);
        inject(Step::NoReplace, &saves.join("Home (copy)"), 5);

        let task = duplicate(&app, request(id, &world, "Home (copy)"))
            .await
            .unwrap();

        assert!(matches!(wait(&app, task).await, TaskOutcome::Succeeded));
        assert_eq!(entries(&saves), vec!["Home (copy)", "New World"]);
        assert!(staging_folders(&saves).is_empty());
    }

    #[tokio::test]
    async fn a_folder_name_taken_at_the_last_moment_gets_the_next_number() {
        use crate::util::publish::fault::{Step, inject, pending};
        const ALREADY_EXISTS: i32 = if cfg!(windows) { 183 } else { 17 };
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-raced").await;
        inject(Step::NoReplace, &saves.join("Home (copy)"), ALREADY_EXISTS);

        let task = duplicate(&app, request(id, &world, "Home (copy)"))
            .await
            .unwrap();

        assert!(matches!(wait(&app, task).await, TaskOutcome::Succeeded));
        assert_eq!(entries(&saves), vec!["Home (copy) (2)", "New World"]);
        assert_eq!(
            read_level_name(&saves.join("Home (copy) (2)"))
                .unwrap()
                .as_deref(),
            Some("Home (copy)")
        );

        // Tried a few times only, then the copy fails and leaves nothing.
        let taken = ["Other", "Other (2)", "Other (3)", "Other (4)", "Other (5)"];
        for name in taken {
            inject(Step::NoReplace, &saves.join(name), ALREADY_EXISTS);
        }
        let task = duplicate(&app, request(id, &world, "Other")).await.unwrap();
        assert!(matches!(wait(&app, task).await, TaskOutcome::Failed(_)));
        for name in taken {
            assert!(!pending(Step::NoReplace, &saves.join(name)), "{name}");
        }
        assert_eq!(entries(&saves), vec!["Home (copy) (2)", "New World"]);
        assert!(staging_folders(&saves).is_empty());
    }

    /// Sets the instance's modpack, locked or not.
    async fn set_pack_locked(app: &App, id: InstanceId, locked: bool) {
        use crate::domain::instance::info;
        app.instance_manager()
            .instances
            .write()
            .await
            .get_mut(&id)
            .unwrap()
            .data_mut()
            .unwrap()
            .config
            .modpack = Some(info::ModpackInfo {
            modpack: info::Modpack::Modrinth(info::ModrinthModpack {
                project_id: "pack".into(),
                version_id: "1".into(),
            }),
            locked,
        });
    }

    #[tokio::test]
    async fn a_pack_locked_while_it_is_copied_is_not_kept() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-locked").await;
        let (pause, task) = held(
            &app,
            id,
            "world_copy_before_publish",
            request(id, &world, "Home (copy)"),
        )
        .await;

        set_pack_locked(&app, id, true).await;
        pause.release.notify_one();

        let outcome = wait(&app, task).await;
        let TaskOutcome::Failed(failed) = &outcome else {
            panic!("the copy did not fail");
        };
        let crate::domain::vtask::Progress::Failed(error) = &failed.progress else {
            panic!("{:?}", failed.progress);
        };
        assert!(
            format!("{error:#}").contains("Modpack is locked"),
            "{error:#}"
        );
        assert_eq!(entries(&saves), vec!["New World"]);
        assert!(staging_folders(&saves).is_empty());
        assert_eq!(app.instance_manager().world_copy(id), None);
    }

    #[tokio::test]
    async fn a_cancel_while_waiting_for_a_moved_instance_leaves_nothing() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world, _) = world_instance(&app, "duplicate-moved").await;
        let (pause, task) = held(
            &app,
            id,
            "world_copy_before_publish",
            request(id, &world, "Home (copy)"),
        )
        .await;
        let manager = app.instance_manager();
        // The copy waits for the gate while the instance folder is renamed,
        // as a rename of the instance does under it.
        let operation = manager.lock_instance(id).await;
        pause.release.notify_one();
        let old_root = publication::current_path(&app, id)
            .await
            .unwrap()
            .get_root();
        let new_root = old_root.with_file_name("duplicate-moved-renamed");
        std::fs::rename(&old_root, &new_root).unwrap();
        manager
            .instances
            .write()
            .await
            .get_mut(&id)
            .unwrap()
            .shortpath = "duplicate-moved-renamed".into();
        let moved_saves = publication::current_path(&app, id)
            .await
            .unwrap()
            .get_saves_path();
        assert_eq!(staging_folders(&moved_saves).len(), 1);

        manager.cancel_world_copy(id);
        let outcome = wait(&app, task).await;
        drop(operation);

        assert!(matches!(outcome, TaskOutcome::Dismissed));
        assert!(!saves.exists());
        assert_eq!(entries(&moved_saves), vec!["New World"]);
        assert!(staging_folders(&moved_saves).is_empty());
        assert_eq!(manager.world_copy(id), None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_linked_saves_folder_gets_its_staging_beside_the_real_one() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, _, _) = world_instance(&app, "duplicate-linked").await;
        // On the drive of the instance, so the folder can be moved there.
        let elsewhere = tempfile::tempdir_in(saves.ancestors().nth(4).unwrap()).unwrap();
        let real = elsewhere.path().join("real saves");
        std::fs::rename(&saves, &real).unwrap();
        std::os::unix::fs::symlink(&real, &saves).unwrap();
        // Beside a shared folder only this instance's leftovers go.
        let others = elsewhere
            .path()
            .join(format!(".gdl-world-copy-{}-other", *id + 1));
        std::fs::create_dir(&others).unwrap();
        let own = elsewhere
            .path()
            .join(format!(".gdl-world-copy-{}-left", *id));
        std::fs::create_dir(&own).unwrap();
        let world = listed(&app, id).await.pop().unwrap();
        assert!(others.is_dir());
        assert!(!own.exists());
        std::fs::remove_dir(&others).unwrap();

        let (pause, task) = held(
            &app,
            id,
            "world_copy_before_publish",
            request(id, &world, "Home (copy)"),
        )
        .await;
        let staged: Vec<_> = entries(elsewhere.path())
            .into_iter()
            .filter(|name| name.starts_with(".gdl-world-copy-"))
            .collect();
        assert_eq!(staged.len(), 1, "{:?}", entries(elsewhere.path()));
        assert!(
            staged[0].starts_with(&format!(".gdl-world-copy-{}-", *id)),
            "{staged:?}"
        );
        // A listing meanwhile leaves the running copy's folder alone.
        let staging = elsewhere.path().join(&staged[0]);
        let copied = snapshot(&staging);
        assert_eq!(listed(&app, id).await.len(), 1);
        assert_eq!(snapshot(&staging), copied);
        assert!(staging.join("level.dat").is_file());
        let instance_folder = saves.parent().unwrap();
        assert!(
            !entries(instance_folder)
                .iter()
                .any(|name| name.starts_with(".gdl-world-copy-"))
        );
        pause.release.notify_one();

        assert!(matches!(wait(&app, task).await, TaskOutcome::Succeeded));
        assert_eq!(entries(&real), vec!["Home (copy)", "New World"]);
        assert_eq!(entries(elsewhere.path()), vec!["real saves"]);
    }
}
