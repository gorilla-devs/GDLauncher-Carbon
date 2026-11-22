//! Renaming a world: the name the game shows, its folder in `saves` and the
//! launcher's records of it, all or nothing.
use super::{
    InstanceId, InstanceManager,
    datapacks::world_identity,
    installer::publication,
    mods::validate_addon_basename,
    world_folder_name::{free_folder_name, safe_folder_name},
    world_level::{read_level_name, write_level_name},
    world_refusal::WorldRefusal,
};
use crate::{
    domain::instance::AddonType,
    managers::{
        ManagerRef,
        runtime_lifecycle::{ActivityPolicy, spawn_retained},
    },
    util::publish::rename_dir,
};
use anyhow::{Context, anyhow, bail};
use carbon_repos::{
    dbtypes::DbDateTime,
    repos::{mod_file_cache as mfcdb, world_datapack_cache as wdb},
};
use chrono::Utc;
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// The longest in-game name a rename or a duplicate accepts, in characters.
const MAX_NAME_CHARS: usize = 64;

/// The in-game name a world is given: `name` trimmed, refused unless it has
/// 1 to [`MAX_NAME_CHARS`] characters.
pub(super) fn world_name(name: &str) -> Result<&str, WorldRefusal> {
    let name = name.trim();
    if !(1..=MAX_NAME_CHARS).contains(&name.chars().count()) {
        return Err(WorldRefusal::InvalidName);
    }
    Ok(name)
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct RenameWorld {
    pub instance_id: i32,
    /// The save's folder name.
    pub world: String,
    /// The world's identity as the world list gave it.
    pub identity: String,
    /// The new in-game name, as typed.
    pub name: String,
}

/// A folder move that failed, with where the world is now.
struct Stuck {
    error: anyhow::Error,
    /// The folder name the world has after the failure.
    at: String,
}

impl ManagerRef<'_, InstanceManager> {
    /// Renames a world; returns the folder name it now has.
    pub async fn rename_world(self, request: RenameWorld) -> anyhow::Result<String> {
        let activity = self
            .app
            .runtime_lifecycle
            .admit_owned("rename a world", ActivityPolicy::CoreWork)?;
        // Its own task, so a request dropped midway (the window closed)
        // never leaves a moved folder without its records or its roll back.
        let app = self.app.clone();
        spawn_retained(activity, move |_| {
            Box::pin(async move { app.instance_manager().rename_world_owned(request).await })
        })
        .await
        .map_err(|error| anyhow!("The world rename stopped: {error}"))?
    }

    async fn rename_world_owned(self, request: RenameWorld) -> anyhow::Result<String> {
        let id = InstanceId(request.instance_id);
        let operation = self.lock_instance(id).await;
        let renamed = self.rename_world_gated(id, &request).await;
        drop(operation);
        match &renamed {
            Ok(_) => self.refresh_worlds(id, "a rename").await,
            Err(error)
                if error.downcast_ref::<WorldRefusal>().is_some_and(|refusal| {
                    matches!(refusal, WorldRefusal::RenameIncomplete { .. })
                }) =>
            {
                self.refresh_worlds(id, "a rename that was not finished")
                    .await
            }
            Err(_) => {}
        }
        renamed
    }

    async fn rename_world_gated(
        self,
        id: InstanceId,
        request: &RenameWorld,
    ) -> anyhow::Result<String> {
        self.ensure_world_datapacks_may_change(id).await?;
        self.ensure_world_not_copied(id, &request.world)?;
        let name = world_name(&request.name)?;
        validate_addon_basename(&request.world)?;
        let path = publication::current_path(&self.app, id).await?;
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let world = saves.join(&request.world);
        if world_identity(id, &path.get_root(), &world).await? != request.identity {
            bail!(WorldRefusal::WorldChanged);
        }
        let original = {
            let world = world.clone();
            tokio::task::spawn_blocking(move || {
                read_level_name(&world)?;
                Ok::<_, anyhow::Error>(std::fs::read(world.join("level.dat"))?)
            })
            .await?
        };
        let original = match original {
            Ok(bytes) => bytes,
            Err(error) => {
                info!(
                    ?error,
                    "Instance {id}: could not read the name of {}",
                    world.display()
                );
                bail!(WorldRefusal::NameUnreadable);
            }
        };

        let folder = free_folder_name(
            &safe_folder_name(name),
            &self.taken_world_names(id, &saves).await?,
            Some(&request.world),
        );
        let moved = folder != request.world;
        if moved {
            #[cfg(test)]
            let first = match self.test_hooks.fail("world_rename_folder", id) {
                Ok(()) => move_folder(&saves, &request.world, &folder).await,
                Err(error) => Err(Stuck {
                    error,
                    at: request.world.clone(),
                }),
            };
            #[cfg(not(test))]
            let first = move_folder(&saves, &request.world, &folder).await;
            if let Err(stuck) = first {
                if stuck.at == request.world {
                    return Err(WorldRefusal::NotRenamed.caused_by(stuck.error));
                }
                error!(error = ?stuck.error, "Instance {id}: a world was left in {}", stuck.at);
                bail!(WorldRefusal::RenameIncomplete { folder: stuck.at });
            }
        }

        let renamed = saves.join(&folder);
        let written = {
            let (renamed, name) = (renamed.clone(), name.to_owned());
            tokio::task::spawn_blocking(move || write_level_name(&renamed, &name))
                .await
                .map_err(anyhow::Error::from)
                .and_then(|written| written)
        };
        // Without a folder move there are no records to move, and a failed
        // write leaves level.dat as it was.
        if !moved {
            return written
                .map(|()| folder)
                .map_err(|error| WorldRefusal::NotRenamed.caused_by(error));
        }
        let failure = match written {
            Err(error) => Some((error, false)),
            Ok(()) => self
                .move_world_records(id, &saves, &request.world, &folder)
                .await
                .err()
                .map(|error| (error, true)),
        };
        let Some((error, name_written)) = failure else {
            return Ok(folder);
        };

        // Put back what was done: the name the game shows, then the folder.
        // A world whose name stays changed still goes back to its folder,
        // where its records are.
        let mut incomplete = false;
        if name_written {
            let (world, original) = (renamed.clone(), original.clone());
            let restored =
                tokio::task::spawn_blocking(move || put_back_level_dat(&world, &original))
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|restored| restored);
            if let Err(restore) = restored {
                error!(error = ?restore, "Instance {id}: could not put back the name of {}", renamed.display());
                incomplete = true;
            }
        }
        #[cfg(test)]
        let back = match self.test_hooks.fail("world_rename_back", id) {
            Ok(()) => move_folder(&saves, &folder, &request.world).await,
            Err(error) => Err(Stuck {
                error,
                at: folder.clone(),
            }),
        };
        #[cfg(not(test))]
        let back = move_folder(&saves, &folder, &request.world).await;
        let now_in = match back {
            Ok(()) => request.world.clone(),
            Err(stuck) => {
                error!(error = ?stuck.error, "Instance {id}: could not rename a world back, it is in {}", stuck.at);
                incomplete = true;
                stuck.at
            }
        };
        if incomplete {
            error!(?error, "Instance {id}: the rename of a world failed");
            bail!(WorldRefusal::RenameIncomplete { folder: now_in });
        }
        Err(WorldRefusal::NotRenamed.caused_by(error))
    }

    /// The names a renamed or copied world may not take: every entry of
    /// `saves` and every world the launcher has a record of, or data pack
    /// records under, whose folder may be gone before a scan removes the
    /// records; taking such a name would merge those records into the world.
    pub(super) async fn taken_world_names(
        self,
        id: InstanceId,
        saves: &Path,
    ) -> anyhow::Result<Vec<String>> {
        let mut taken = Vec::new();
        let mut entries = tokio::fs::read_dir(saves)
            .await
            .with_context(|| format!("listing {}", saves.display()))?;
        while let Some(entry) = entries.next_entry().await? {
            taken.push(entry.file_name().to_string_lossy().into_owned());
        }
        for row in mfcdb::get_mod_files_by_instance(&self.app.db, *id).await? {
            if row.addon_type == AddonType::Worlds.to_db_string() {
                taken.push(row.filename);
            }
        }
        for row in wdb::get_world_datapacks_by_instance(&self.app.db, *id).await? {
            taken.push(row.world);
        }
        taken.sort();
        taken.dedup();
        Ok(taken)
    }

    /// Moves the world's own row and its data packs' rows to the folder name
    /// `to`, in one transaction; they keep their ids.
    async fn move_world_records(
        self,
        id: InstanceId,
        saves: &Path,
        from: &str,
        to: &str,
    ) -> anyhow::Result<()> {
        let cache = self.app.meta_cache_manager();
        let parsed = cache.world_dir_parse_result(saves, to).await?;
        let metadata_id = cache.ensure_mod_metadata(&parsed, to).await?;
        #[cfg(test)]
        let fail = self.test_hooks.fail("world_rename_records", id).is_err();
        let (instance, from, to) = (*id, from.to_owned(), to.to_owned());
        let now = DbDateTime(Utc::now().fixed_offset());
        self.app
            .db
            .write(move |mut conn| {
                let tx = conn.transaction()?;
                mfcdb::rename_world_file_conn(&tx, instance, &from, &to, &metadata_id, now)?;
                #[cfg(test)]
                if fail {
                    return Err(carbon_repos::db_error::DbError::Conversion(
                        "world_rename_records failed (test)".into(),
                    ));
                }
                wdb::rename_world_of_datapacks_conn(&tx, instance, &from, &to, now)?;
                tx.commit()?;
                Ok(())
            })
            .await?;
        Ok(())
    }
}

/// Fails when anything is at `path`: a rename onto a folder could replace it.
async fn ensure_free(path: &Path) -> anyhow::Result<()> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(_) => bail!("{} already exists", path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("checking {}", path.display())),
    }
}

/// Renames the folder `from` in `saves` to `to`. A change of letter case
/// only goes through a temporary name, which a system that ignores case
/// needs to see a change at all.
async fn move_folder(saves: &Path, from: &str, to: &str) -> Result<(), Stuck> {
    let stuck = |at: &str| {
        let at = at.to_owned();
        move |error: anyhow::Error| Stuck { error, at }
    };
    let source = saves.join(from);
    let target = saves.join(to);
    if from.to_lowercase() != to.to_lowercase() {
        ensure_free(&target).await.map_err(stuck(from))?;
        return rename_dir(&source, &target)
            .await
            .with_context(|| format!("renaming {} to {}", source.display(), target.display()))
            .map_err(stuck(from));
    }

    let (step_name, step) = free_step(saves, to).await.map_err(stuck(from))?;
    rename_dir(&source, &step)
        .await
        .with_context(|| format!("renaming {} to {}", source.display(), step.display()))
        .map_err(stuck(from))?;
    if let Err(error) = rename_dir(&step, &target).await {
        let error = anyhow::Error::from(error).context(format!(
            "renaming {} to {}",
            step.display(),
            target.display()
        ));
        return match rename_dir(&step, &source).await {
            Ok(()) => Err(Stuck {
                error,
                at: from.to_owned(),
            }),
            Err(back) => {
                error!(error = ?back, "could not rename {} back", step.display());
                Err(Stuck {
                    error,
                    at: step_name,
                })
            }
        };
    }
    Ok(())
}

/// The first free temporary name for a change of letter case to `to`, so a
/// folder left by an earlier rename that stopped midway never blocks it.
async fn free_step(saves: &Path, to: &str) -> anyhow::Result<(String, PathBuf)> {
    for n in 1..=100 {
        let name = match n {
            1 => format!("{to}.gdl-renaming"),
            n => format!("{to}.gdl-renaming-{n}"),
        };
        let path = saves.join(&name);
        if ensure_free(&path).await.is_ok() {
            return Ok((name, path));
        }
    }
    bail!("no free temporary name for {to} in {}", saves.display())
}

/// Puts the bytes `original` back as the world's `level.dat`, in one step.
fn put_back_level_dat(world: &Path, original: &[u8]) -> anyhow::Result<()> {
    let target = world.join("level.dat");
    let mut tmp = tempfile::Builder::new()
        .prefix("level.dat.")
        .suffix(".tmp")
        .tempfile_in(world)
        .context("creating the restored level.dat")?;
    tmp.write_all(original)?;
    let permissions = std::fs::metadata(&target)
        .with_context(|| format!("reading the permissions of {}", target.display()))?
        .permissions();
    tmp.as_file().set_permissions(permissions)?;
    tmp.as_file().sync_all()?;
    tmp.persist(&target)
        .map_err(|e| e.error)
        .context("putting back level.dat")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::coded_cause;
    use crate::managers::App;
    use crate::managers::instance::{
        mods::tests::addon_fixture,
        run::LaunchState,
        world_datapack_changes::tests::{pack_zip, put_pack, scanned_rows},
        world_datapacks::World,
        world_level::level_dat_bytes,
    };

    /// An instance with the save `folder`, whose in-game name is `title`, its
    /// addon row, and one data pack recorded by a scan. Returns the instance,
    /// its `saves` folder and the world as the list shows it.
    async fn world_instance(
        app: &App,
        instance: &str,
        folder: &str,
        title: Option<&str>,
    ) -> (InstanceId, PathBuf, World) {
        let (id, _, world) = addon_fixture(
            app,
            instance,
            AddonType::Worlds,
            folder,
            &level_dat_bytes(title),
        )
        .await;
        put_pack(&world, "pack.zip", &pack_zip("kept"));
        assert_eq!(scanned_rows(app, id).await.len(), 1);
        let listed = listed(app, id).await.pop().unwrap();
        (id, world.parent().unwrap().to_owned(), listed)
    }

    async fn listed(app: &App, id: InstanceId) -> Vec<World> {
        app.instance_manager().list_worlds(id).await.unwrap()
    }

    fn request(id: InstanceId, world: &World, name: &str) -> RenameWorld {
        RenameWorld {
            instance_id: *id,
            world: world.name.clone(),
            identity: world.identity.clone().unwrap(),
            name: name.into(),
        }
    }

    async fn rename(app: &App, request: RenameWorld) -> anyhow::Result<String> {
        app.instance_manager().rename_world(request).await
    }

    fn code(error: &anyhow::Error) -> Option<String> {
        coded_cause(error).map(|(code, _)| code)
    }

    fn entries(saves: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(saves)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn title(world: &Path) -> Option<String> {
        read_level_name(world).unwrap()
    }

    /// The entries of `saves` and every file in each world, to show that a
    /// refused rename changes nothing.
    fn snapshot(saves: &Path) -> Vec<(String, Vec<u8>)> {
        let mut all = Vec::new();
        for entry in walkdir::WalkDir::new(saves).sort_by_file_name() {
            let entry = entry.unwrap();
            let bytes = if entry.file_type().is_file() {
                std::fs::read(entry.path()).unwrap()
            } else {
                Vec::new()
            };
            all.push((entry.path().display().to_string(), bytes));
        }
        all
    }

    /// The world's own row and its data packs' rows, as (id, name) pairs.
    async fn records(app: &App, id: InstanceId) -> Vec<(String, String)> {
        let mut rows = Vec::new();
        for row in mfcdb::get_mod_files_by_instance(&app.db, *id)
            .await
            .unwrap()
        {
            rows.push((row.id, row.filename));
        }
        for row in wdb::get_world_datapacks_by_instance(&app.db, *id)
            .await
            .unwrap()
        {
            rows.push((row.id, format!("{}/{}", row.world, row.filename)));
        }
        rows.sort();
        rows
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

    #[tokio::test]
    async fn a_rename_moves_the_folder_the_name_and_the_records() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) = world_instance(&app, "rename-moves", "New World", None).await;

        let folder = rename(&app, request(id, &world, "Skyblock: season 2"))
            .await
            .unwrap();

        assert_eq!(folder, "Skyblock_ season 2");
        assert_eq!(entries(&saves), vec!["Skyblock_ season 2"]);
        assert_eq!(
            title(&saves.join(&folder)).as_deref(),
            Some("Skyblock: season 2")
        );
        let worlds = listed(&app, id).await;
        assert_eq!(worlds.len(), 1, "{worlds:?}");
        assert_eq!(worlds[0].name, "Skyblock_ season 2");
        assert_eq!(worlds[0].title.as_deref(), Some("Skyblock: season 2"));
        assert_eq!(worlds[0].addon_id, world.addon_id);
        let packs: Vec<_> = worlds[0].datapacks.iter().map(|pack| &pack.id).collect();
        assert_eq!(packs, vec![&world.datapacks[0].id]);
        // The row has the metadata a scan would give the new folder name.
        let cache = app.meta_cache_manager();
        let parsed = cache.world_dir_parse_result(&saves, &folder).await.unwrap();
        let row = mfcdb::get_mod_file_cache_by_id(&app.db, &world.addon_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            row.metadata_id,
            cache.ensure_mod_metadata(&parsed, &folder).await.unwrap()
        );
    }

    #[tokio::test]
    async fn a_name_that_gives_the_same_folder_changes_only_the_in_game_name() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) =
            world_instance(&app, "rename-same-folder", "New World", Some("Old")).await;
        let before = records(&app, id).await;

        let folder = rename(&app, request(id, &world, "New World"))
            .await
            .unwrap();

        assert_eq!(folder, "New World");
        assert_eq!(entries(&saves), vec!["New World"]);
        assert_eq!(records(&app, id).await, before);
        assert_eq!(
            title(&saves.join("New World")).as_deref(),
            Some("New World")
        );
    }

    #[tokio::test]
    async fn a_change_of_letter_case_renames_the_folder() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) = world_instance(&app, "rename-case", "world", None).await;

        let folder = rename(&app, request(id, &world, "World")).await.unwrap();

        assert_eq!(folder, "World");
        assert_eq!(entries(&saves), vec!["World"]);
        assert_eq!(title(&saves.join("World")).as_deref(), Some("World"));
        assert_eq!(listed(&app, id).await[0].addon_id, world.addon_id);
    }

    #[tokio::test]
    async fn a_taken_name_gets_a_number() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) = world_instance(&app, "rename-taken", "New World", None).await;
        std::fs::create_dir(saves.join("Skyblock")).unwrap();
        std::fs::write(saves.join("Skyblock/level.dat"), level_dat_bytes(None)).unwrap();

        let folder = rename(&app, request(id, &world, "Skyblock")).await.unwrap();

        assert_eq!(folder, "Skyblock (2)");
        assert_eq!(
            title(&saves.join("Skyblock (2)")).as_deref(),
            Some("Skyblock")
        );
        assert_eq!(title(&saves.join("Skyblock")), None);
    }

    #[tokio::test]
    async fn a_rename_is_refused_while_the_game_is_active_and_changes_nothing() {
        use crate::domain::vtask::VisualTaskId as TaskId;
        use crate::managers::instance::run::RunningInstance;
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) = world_instance(&app, "rename-active", "New World", None).await;
        let before = (snapshot(&saves), records(&app, id).await);

        for state in [
            LaunchState::Queued(TaskId(900)),
            LaunchState::Preparing(TaskId(900)),
            LaunchState::Running(RunningInstance::adopted(
                u32::MAX,
                1,
                chrono::Utc::now(),
                chrono::Utc::now(),
            )),
        ] {
            let name = format!("{state:?}");
            set_state(&app, id, state).await;
            let error = rename(&app, request(id, &world, "Elsewhere"))
                .await
                .unwrap_err();
            assert_eq!(
                code(&error).as_deref(),
                Some("INSTANCE_GAME_ACTIVE"),
                "{name}"
            );
            assert_eq!(
                (snapshot(&saves), records(&app, id).await),
                before,
                "{name}"
            );
        }
        set_state(&app, id, LaunchState::Inactive { failed_task: None }).await;
    }

    #[tokio::test]
    async fn a_rename_is_refused_when_the_world_is_not_the_one_listed() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) = world_instance(&app, "rename-changed", "New World", None).await;
        let before = (snapshot(&saves), records(&app, id).await);

        let error = rename(
            &app,
            RenameWorld {
                identity: "another world".into(),
                ..request(id, &world, "Elsewhere")
            },
        )
        .await
        .unwrap_err();

        assert_eq!(code(&error).as_deref(), Some("WORLD_CHANGED"));
        assert_eq!((snapshot(&saves), records(&app, id).await), before);
    }

    #[tokio::test]
    async fn a_rename_is_refused_when_its_name_cannot_be_read() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) = world_instance(&app, "rename-unreadable", "New World", None).await;
        std::fs::write(saves.join("New World/level.dat"), b"not gzip").unwrap();
        let before = (snapshot(&saves), records(&app, id).await);

        let error = rename(&app, request(id, &world, "Elsewhere"))
            .await
            .unwrap_err();

        assert_eq!(code(&error).as_deref(), Some("WORLD_NAME_UNREADABLE"));
        assert_eq!((snapshot(&saves), records(&app, id).await), before);
    }

    #[tokio::test]
    async fn a_rename_is_refused_for_an_empty_or_too_long_name() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) = world_instance(&app, "rename-invalid", "New World", None).await;
        let before = snapshot(&saves);

        for name in [String::new(), "   ".into(), "a".repeat(65)] {
            let error = rename(&app, request(id, &world, &name)).await.unwrap_err();
            assert_eq!(
                code(&error).as_deref(),
                Some("WORLD_NAME_INVALID"),
                "{name:?}"
            );
            assert_eq!(snapshot(&saves), before, "{name:?}");
        }

        let longest = "é".repeat(64);
        let folder = rename(&app, request(id, &world, &longest)).await.unwrap();
        assert_eq!(title(&saves.join(&folder)), Some(longest));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_name_that_cannot_be_written_puts_the_folder_back() {
        use std::os::unix::fs::PermissionsExt;
        // A folder mode does not stop root, so the write would succeed.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) = world_instance(&app, "rename-unwritable", "New World", None).await;
        let folder = saves.join("New World");
        let original = std::fs::read(folder.join("level.dat")).unwrap();
        let before = records(&app, id).await;
        // No new level.dat can be made beside the old one; the folder can
        // still be renamed, as its parent is writable.
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o555)).unwrap();

        let error = rename(&app, request(id, &world, "Elsewhere")).await;

        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
        let error = error.unwrap_err();
        // The rename got as far as the write, and keeps why it stopped.
        assert!(format!("{error:#}").contains("level.dat"), "{error:#}");
        assert_eq!(
            code(&error).as_deref(),
            Some("WORLD_NOT_RENAMED"),
            "{error:#}"
        );
        assert_eq!(entries(&saves), vec!["New World"]);
        assert_eq!(std::fs::read(folder.join("level.dat")).unwrap(), original);
        assert_eq!(records(&app, id).await, before);
    }

    #[tokio::test]
    async fn records_that_cannot_move_put_the_folder_and_the_name_back() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) =
            world_instance(&app, "rename-records", "New World", Some("Old")).await;
        let original = std::fs::read(saves.join("New World/level.dat")).unwrap();
        let before = records(&app, id).await;
        app.instance_manager()
            .test_hooks
            .arm_failure("world_rename_records", id);

        let error = rename(&app, request(id, &world, "Elsewhere"))
            .await
            .unwrap_err();

        // The rename got as far as the records, after the name was written.
        assert!(
            format!("{error:#}").contains("world_rename_records failed (test)"),
            "{error:#}"
        );
        assert_eq!(
            code(&error).as_deref(),
            Some("WORLD_NOT_RENAMED"),
            "{error:#}"
        );
        assert_eq!(entries(&saves), vec!["New World"]);
        assert_eq!(title(&saves.join("New World")).as_deref(), Some("Old"));
        assert_eq!(
            std::fs::read(saves.join("New World/level.dat")).unwrap(),
            original
        );
        assert_eq!(records(&app, id).await, before);
    }

    #[tokio::test]
    async fn a_world_that_cannot_be_renamed_back_says_which_folder_it_is_in() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) =
            world_instance(&app, "rename-stuck", "New World", Some("Old")).await;
        let hooks = &app.instance_manager().test_hooks;
        hooks.arm_failure("world_rename_records", id);
        hooks.arm_failure("world_rename_back", id);

        let error = rename(&app, request(id, &world, "Elsewhere"))
            .await
            .unwrap_err();

        assert_eq!(
            coded_cause(&error),
            Some((
                "WORLD_RENAME_INCOMPLETE".to_owned(),
                Some(serde_json::json!({ "folder": "Elsewhere" }))
            ))
        );
        assert_eq!(entries(&saves), vec!["Elsewhere"]);
        assert_eq!(title(&saves.join("Elsewhere")).as_deref(), Some("Old"));
    }

    #[tokio::test]
    async fn a_folder_that_cannot_be_renamed_changes_nothing() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) =
            world_instance(&app, "rename-folder-fails", "New World", Some("Old")).await;
        let before = (snapshot(&saves), records(&app, id).await);
        app.instance_manager()
            .test_hooks
            .arm_failure("world_rename_folder", id);

        let error = rename(&app, request(id, &world, "Elsewhere"))
            .await
            .unwrap_err();

        assert!(
            format!("{error:#}").contains("world_rename_folder failed (test)"),
            "{error:#}"
        );
        assert_eq!(
            code(&error).as_deref(),
            Some("WORLD_NOT_RENAMED"),
            "{error:#}"
        );
        assert_eq!((snapshot(&saves), records(&app, id).await), before);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_name_that_cannot_be_written_in_its_own_folder_changes_nothing() {
        use std::os::unix::fs::PermissionsExt;
        // A folder mode does not stop root, so the write would succeed.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) =
            world_instance(&app, "rename-in-place-unwritable", "New World", Some("Old")).await;
        let folder = saves.join("New World");
        let before = (snapshot(&saves), records(&app, id).await);
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o555)).unwrap();

        let error = rename(&app, request(id, &world, "New World")).await;

        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
        let error = error.unwrap_err();
        assert!(format!("{error:#}").contains("level.dat"), "{error:#}");
        assert_eq!(
            code(&error).as_deref(),
            Some("WORLD_NOT_RENAMED"),
            "{error:#}"
        );
        assert_eq!((snapshot(&saves), records(&app, id).await), before);
    }

    #[tokio::test]
    async fn a_leftover_temporary_folder_does_not_block_a_change_of_letter_case() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) = world_instance(&app, "rename-case-leftover", "world", None).await;
        let leftover = saves.join("World.gdl-renaming");
        std::fs::create_dir(&leftover).unwrap();
        std::fs::write(leftover.join("kept.txt"), b"left by a crash").unwrap();

        let folder = rename(&app, request(id, &world, "World")).await.unwrap();

        assert_eq!(folder, "World");
        assert_eq!(entries(&saves), vec!["World", "World.gdl-renaming"]);
        assert_eq!(title(&saves.join("World")).as_deref(), Some("World"));
        assert_eq!(entries(&leftover), vec!["kept.txt"]);
        assert_eq!(
            std::fs::read(leftover.join("kept.txt")).unwrap(),
            b"left by a crash"
        );
    }

    #[tokio::test]
    async fn a_name_with_stale_data_pack_records_gets_a_number() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves, world) =
            world_instance(&app, "rename-stale-packs", "New World", None).await;
        // Data pack rows of a world whose folder is gone.
        wdb::upsert_world_datapack(
            &app.db,
            *id,
            "Skyblock",
            "stale.zip",
            0,
            false,
            None,
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();

        let folder = rename(&app, request(id, &world, "Skyblock")).await.unwrap();

        assert_eq!(folder, "Skyblock (2)");
        assert_eq!(entries(&saves), vec!["Skyblock (2)"]);
        let worlds = listed(&app, id).await;
        let packs: Vec<_> = worlds[0].datapacks.iter().map(|pack| &pack.id).collect();
        assert_eq!(packs, vec![&world.datapacks[0].id]);
    }
}
