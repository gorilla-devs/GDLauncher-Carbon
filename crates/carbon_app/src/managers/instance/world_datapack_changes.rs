//! Installing a data pack from CurseForge or Modrinth straight into one world
//! of an instance. Only `saves/<world>/datapacks` and a private download in
//! the instance root are written; the instance's own addon folders, the
//! installer and the addon scanner are not involved.
use super::{
    InstanceId, InstanceManager, InstanceRootGuard, InvalidInstanceIdError,
    datapacks::world_identity,
    installer::{
        dependency::{LoaderFit, best_candidate},
        publication,
    },
    mods::{addon_metadata, remove_addon_entry, validate_addon_basename},
    run::LaunchState,
    world_datapacks::world_datapacks_folder,
    world_refusal::WorldRefusal,
};
use crate::{
    api::{
        keys::instance::{GET_WORLDS, INSTANCE_MODS},
        translation::Translation,
    },
    domain::{instance::AddonType, vtask::VisualTaskId},
    managers::{
        ManagerRef,
        metadata::cache::{
            CacheEntityId, curseforge::record_installed_file, modrinth::record_installed_version,
        },
        modplatforms::{curseforge::CurseForge, modrinth::Modrinth},
        runtime_lifecycle::{ActivityPolicy, spawn_retained},
        vtask::{TaskState, VisualTask},
    },
    util::{
        publish::{crosses_devices, long_path, rename_noclobber},
        recycle_bin::Removal,
    },
};
use anyhow::{Context, anyhow, bail};
use carbon_net::{Checksum, DownloadOptions, Downloadable};
use carbon_platforms::{
    ModChannel,
    curseforge::{
        self, ClassId,
        filters::{ModFileParameters, ModFilesParameters, ModFilesParametersQuery, ModParameters},
    },
    modrinth::{
        project::{Project, ProjectVersionsFilters},
        search::{ProjectID, VersionID},
        version::{Version, VersionFile},
    },
};
use carbon_repos::repos::world_datapack_cache::{self as wdb, WorldDatapackRow};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tracing::warn;

/// Where the data pack comes from; no version or file means the newest data
/// pack build for the instance's game version.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub enum WorldDatapackSource {
    Modrinth {
        project_id: String,
        version_id: Option<String>,
    },
    Curseforge {
        project_id: u32,
        file_id: Option<u32>,
    },
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct InstallWorldDatapack {
    pub instance_id: i32,
    /// The save's folder name.
    pub world: String,
    /// The world's identity as the world list gave it.
    pub identity: String,
    pub source: WorldDatapackSource,
    /// The id of the world data pack the install replaces.
    pub replaces: Option<String>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DeleteWorldDatapack {
    pub instance_id: i32,
    /// The world data pack's id.
    pub id: String,
}

/// Packs moved out of the game's sight until their replacement is in place:
/// then [`TakenAway::send_on`] removes them as the user's Delete would, or
/// [`TakenAway::put_back`] returns them under their names. Dropped
/// unsettled, the world is put back as it was: the replacement
/// [`TakenAway::publish`] put in place is withdrawn and the packs return.
/// When one cannot return, `keep` is set, so the folder it waits in stays
/// and the warning names it.
pub(super) struct TakenAway {
    removal: Removal,
    /// Held while a publication, removal or put-back runs on the blocking
    /// pool, so a drop waits for that step and then sees what it did.
    settlement: Arc<std::sync::Mutex<Settlement>>,
    keep: Arc<AtomicBool>,
}

/// Where a pack taken away was, and where it waits.
#[derive(Clone)]
struct Aside {
    original: PathBuf,
    aside: PathBuf,
}

#[derive(Default)]
struct Settlement {
    settled: bool,
    /// Each entry is moved aside and recorded here in one step under the
    /// lock: a drop meanwhile waits for that step, and so never misses an
    /// entry waiting in a private folder that is then removed whole.
    entries: Vec<Aside>,
    /// The replacement, once it is in place.
    published: Option<PathBuf>,
    /// How many entries, from the first, were removed already.
    sent: usize,
}

/// The name a pack waits under beside itself when the private folder is on
/// another drive. No `.zip` suffix and no folder: the game does not load it
/// and the scanner does not list it.
const WAITING: &str = ".gdl-replaced";

/// Moves the entry at `path` into `aside_folder` under its own name, so the
/// Recycle Bin later shows that name. A rename only, so the entry keeps its
/// bytes and times; across drives, where a rename cannot reach, it is
/// renamed beside itself instead. A folder is never left beside itself: the
/// game would load it under any name.
pub(super) async fn take_away(
    path: &Path,
    aside_folder: &Path,
    removal: Removal,
    keep: Arc<AtomicBool>,
) -> anyhow::Result<TakenAway> {
    take_away_all(&[path.to_owned()], aside_folder, removal, keep).await
}

/// [`take_away`] for several entries, settled together. When one cannot be
/// moved, those moved before it are put back.
pub(super) async fn take_away_all(
    paths: &[PathBuf],
    aside_folder: &Path,
    removal: Removal,
    keep: Arc<AtomicBool>,
) -> anyhow::Result<TakenAway> {
    let taken = TakenAway {
        removal,
        settlement: Default::default(),
        keep,
    };
    for path in paths {
        let (original, folder) = (path.clone(), aside_folder.to_owned());
        // A drop that went first settled the entries: this one stays put.
        taken
            .settle(move |settlement| {
                let aside = set_aside(&original, &folder)?;
                #[cfg(test)]
                set_aside_pause::pause(&original);
                settlement.entries.push(Aside { original, aside });
                Ok(())
            })
            .await?;
    }
    Ok(taken)
}

fn set_aside(path: &Path, folder: &Path) -> anyhow::Result<PathBuf> {
    let name = path.file_name().context("a data pack has a name")?;
    std::fs::create_dir_all(folder)?;
    let aside = folder.join(name);
    match rename_noclobber(path, &aside) {
        Ok(()) => return Ok(aside),
        Err(error) if crosses_devices(&error) => {}
        Err(error) => {
            return Err(error).with_context(|| format!("Could not move {} aside", path.display()));
        }
    }
    if std::fs::metadata(path).is_ok_and(|meta| meta.is_dir()) {
        bail!(
            "{} is a folder on another drive than the instance, so it cannot be set aside",
            path.display()
        );
    }
    let beside = path.parent().context("a data pack is in a folder")?;
    for attempt in 0..100 {
        let aside = beside.join(format!("{WAITING}-{attempt}"));
        match rename_noclobber(path, &aside) {
            Ok(()) => return Ok(aside),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Could not move {} aside", path.display()));
            }
        }
    }
    bail!("Could not find a free name to set {} aside", path.display())
}

/// Holds a set-aside right after its rename, for tests that drop what
/// awaits it at that moment.
#[cfg(test)]
pub(crate) mod set_aside_pause {
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

    type Pause = (PathBuf, SyncSender<()>, Receiver<()>);
    static PAUSES: std::sync::Mutex<Vec<Pause>> = std::sync::Mutex::new(Vec::new());

    /// The next set-aside of `path` tells `entered` once renamed, then
    /// waits for `release`.
    pub(crate) fn arm(path: &Path) -> (Receiver<()>, SyncSender<()>) {
        let (entered_tx, entered) = sync_channel(1);
        let (release, release_rx) = sync_channel(1);
        PAUSES
            .lock()
            .unwrap()
            .push((path.to_owned(), entered_tx, release_rx));
        (entered, release)
    }

    pub(super) fn pause(path: &Path) {
        let pause = {
            let mut pauses = PAUSES.lock().unwrap();
            let index = pauses.iter().position(|(armed, ..)| armed == path);
            index.map(|index| pauses.remove(index))
        };
        if let Some((_, entered, release)) = pause {
            let _ = entered.send(());
            let _ = release.recv();
        }
    }
}

impl TakenAway {
    /// Runs `step` on the blocking pool with the settlement held, unless
    /// the entries are settled already.
    async fn settle<T: Send + 'static>(
        &self,
        step: impl FnOnce(&mut Settlement) -> anyhow::Result<T> + Send + 'static,
    ) -> anyhow::Result<T> {
        let settlement = self.settlement.clone();
        tokio::task::spawn_blocking(move || {
            let mut settlement = settlement.lock().unwrap_or_else(|held| held.into_inner());
            if settlement.settled {
                bail!("The data pack set aside was already put back or removed");
            }
            step(&mut settlement)
        })
        .await?
    }

    /// Publishes `file` at `destination` without replacing anything, as
    /// the replacement of the entries taken away.
    pub(super) async fn publish(
        &self,
        file: tempfile::NamedTempFile,
        destination: &Path,
    ) -> anyhow::Result<()> {
        let destination = destination.to_owned();
        self.settle(move |settlement| {
            crate::util::publish::persist_noclobber(file, &destination)?;
            settlement.published = Some(destination);
            Ok(())
        })
        .await
    }

    /// Withdraws the replacement when one was published (it is this
    /// install's own download), then returns the entries not removed yet
    /// under their original names, never over something that took a name
    /// meanwhile. An entry that cannot return stays where it waits, and when
    /// the replacement cannot be withdrawn (two builds of one pack would
    /// then be in the world) they all do; `keep` is set then.
    pub(super) async fn put_back(self) -> anyhow::Result<()> {
        let keep = self.keep.clone();
        self.settle(move |settlement| {
            settlement.settled = true;
            Ok(settle_back(settlement, &keep))
        })
        .await?
        .into_result()
    }

    /// Removes the entries as the "Deletion through Recycle Bin" setting
    /// asked, in order. On failure, the entries not removed yet are still
    /// where they waited and can be put back.
    pub(super) async fn send_on(&mut self) -> anyhow::Result<()> {
        let removal = self.removal;
        self.settle(move |settlement| {
            while let Some(entry) = settlement.entries.get(settlement.sent) {
                remove_set_aside(&entry.aside, removal)?;
                settlement.sent += 1;
            }
            settlement.settled = true;
            Ok(())
        })
        .await
    }
}

/// Removes the entry at `path` the way `removal` says, a link by itself.
/// One already gone is fine.
fn remove_set_aside(path: &Path, removal: Removal) -> anyhow::Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if meta.file_type().is_symlink() {
        // Windows removes a link to a folder (or a junction) as a folder.
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            if meta.file_type().is_symlink_dir() {
                return Ok(std::fs::remove_dir(path)?);
            }
        }
        return Ok(std::fs::remove_file(path)?);
    }
    if !meta.is_dir() && !meta.is_file() {
        bail!(
            "{} is no file or folder that can be deleted",
            path.display()
        );
    }
    match removal {
        Removal::RecycleBin => crate::util::recycle_bin::move_to_recycle_bin_blocking(path),
        Removal::Permanent if meta.is_dir() => Ok(std::fs::remove_dir_all(path)?),
        Removal::Permanent => Ok(std::fs::remove_file(path)?),
    }
}

/// How entries taken away ended when they were put back.
enum Returned {
    Back,
    /// These could not return under their names.
    Stuck(Vec<(Aside, std::io::Error)>),
    /// The replacement could not be withdrawn, so none was put back.
    NotWithdrawn {
        published: PathBuf,
        error: std::io::Error,
        waiting: Vec<Aside>,
    },
}

/// That `entry` waits at its place aside, and why it is not back.
fn kept_at(entry: &Aside, error: &dyn std::fmt::Display) -> String {
    format!(
        "{} could not be put back and is kept at {}: {error}",
        entry.original.display(),
        entry.aside.display()
    )
}

impl Returned {
    fn into_result(self) -> anyhow::Result<()> {
        match self {
            Returned::Back => Ok(()),
            Returned::Stuck(stuck) => Err(anyhow!(
                "{}",
                stuck
                    .iter()
                    .map(|(entry, error)| kept_at(entry, error))
                    .collect::<Vec<_>>()
                    .join("; ")
            )),
            Returned::NotWithdrawn {
                published,
                error,
                waiting,
            } => Err(anyhow!(
                "{} could not be removed: {error}. {}",
                published.display(),
                waiting
                    .iter()
                    .map(|entry| kept_at(entry, &"the replacement is still in its place"))
                    .collect::<Vec<_>>()
                    .join("; ")
            )),
        }
    }
}

/// Withdraws the published replacement, if any, then returns the entries
/// not removed yet to their original names. The replacement is this
/// install's own download; while it stays, the entries stay aside, so one
/// build of the pack is in the world. Sets `keep` when an entry stays aside.
fn settle_back(settlement: &mut Settlement, keep: &AtomicBool) -> Returned {
    let waiting = settlement
        .entries
        .get(settlement.sent..)
        .unwrap_or_default()
        .to_vec();
    if let Some(published) = settlement.published.take()
        && let Err(error) = std::fs::remove_file(&published)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        keep.store(true, Ordering::SeqCst);
        return Returned::NotWithdrawn {
            published,
            error,
            waiting,
        };
    }
    let stuck: Vec<_> = waiting
        .iter()
        .filter_map(|entry| {
            rename_noclobber(&entry.aside, &entry.original)
                .err()
                .map(|error| (entry.clone(), error))
        })
        .collect();
    if stuck.is_empty() {
        Returned::Back
    } else {
        keep.store(true, Ordering::SeqCst);
        Returned::Stuck(stuck)
    }
}

impl Drop for TakenAway {
    fn drop(&mut self) {
        let settlement = self.settlement.clone();
        let mut settlement = settlement.lock().unwrap_or_else(|held| held.into_inner());
        if settlement.settled {
            return;
        }
        settlement.settled = true;
        if let Err(error) = settle_back(&mut settlement, &self.keep).into_result() {
            warn!("{error:#}");
        }
    }
}

/// Refuses when the entry at `path` (lstat `meta`) is not the kind of entry
/// its row lists: a folder for a folder row, a file otherwise. A link is
/// judged by what it points to; a dangling one passes, as only the link
/// itself is ever removed.
async fn ensure_listed_kind(
    path: &Path,
    meta: &std::fs::Metadata,
    is_folder: bool,
) -> anyhow::Result<()> {
    let kind = if meta.file_type().is_symlink() {
        match tokio::fs::metadata(path).await {
            Ok(followed) => followed,
            Err(_) => return Ok(()),
        }
    } else {
        meta.clone()
    };
    let listed = if is_folder {
        kind.is_dir()
    } else {
        kind.is_file()
    };
    if !listed {
        bail!(WorldRefusal::PackChanged);
    }
    Ok(())
}

/// The refusal of an install into a world when the project has no data pack
/// build for the instance's game version. Nothing changed.
#[derive(Debug, thiserror::Error)]
#[error("This project has no data pack for Minecraft {version}")]
pub struct NoDatapackForVersion {
    pub version: String,
}

impl crate::error::FeErrorCode for NoDatapackForVersion {
    fn error_code(&self) -> &'static str {
        "NO_DATAPACK_FOR_VERSION"
    }

    fn error_data(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "version": self.version }))
    }
}

fn no_data_pack(game: &str) -> anyhow::Error {
    NoDatapackForVersion {
        version: game.to_owned(),
    }
    .into()
}

/// The file of a Modrinth version that is a data pack: its primary file
/// (the first when none is flagged), when the version is tagged `datapack`
/// and that file is a zip. The game loads nothing else from a world's
/// `datapacks` folder. A version the user picked needs no more than this.
/// Modrinth lists a data pack project as a mod, so this is also what tells
/// its data pack builds from its mod jars when one is installed into an
/// instance.
pub(crate) fn modrinth_datapack_file(version: &Version) -> Option<&VersionFile> {
    if !version.loaders.iter().any(|loader| loader == "datapack") {
        return None;
    }
    version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .filter(|file| file.filename.to_ascii_lowercase().ends_with(".zip"))
}

/// [`modrinth_datapack_file`] of a version that also lists `game`: the
/// builds the newest one and an update are chosen among, and the only ones
/// a world's pack shows an update for.
pub(crate) fn modrinth_world_file<'a>(version: &'a Version, game: &str) -> Option<&'a VersionFile> {
    if !version.game_versions.iter().any(|listed| listed == game) {
        return None;
    }
    modrinth_datapack_file(version)
}

/// Whether a CurseForge file of `project` is a data pack: the project is a
/// data pack one and the file is a zip.
fn is_curseforge_datapack_file(project: &curseforge::Mod, file: &curseforge::File) -> bool {
    project.class_id == Some(ClassId::Datapacks)
        && file.file_name.to_ascii_lowercase().ends_with(".zip")
}

/// [`is_curseforge_datapack_file`] for a file that also lists `game` (see
/// [`modrinth_world_file`]).
pub(super) fn is_curseforge_world_file(
    project: &curseforge::Mod,
    file: &curseforge::File,
    game: &str,
) -> bool {
    is_curseforge_datapack_file(project, file)
        && file.game_versions.iter().any(|listed| listed == game)
}

/// The newest data pack build of a Modrinth project for `game`, releases
/// before betas before alphas.
pub(super) async fn latest_world_datapack_modrinth(
    modrinth: &Modrinth,
    project: &Project,
    game: &str,
) -> anyhow::Result<Version> {
    let mut versions = modrinth
        .get_project_versions(ProjectVersionsFilters {
            project_id: ProjectID(project.id.clone()),
            game_versions: Some(vec![game.to_owned()]),
            loaders: Some(vec!["datapack".to_owned()]),
            limit: None,
            offset: None,
        })
        .await?
        .0;
    versions.sort_by(|a, b| b.date_published.cmp(&a.date_published));
    best_candidate(&versions, ModChannel::Stable, |version| {
        modrinth_world_file(version, game).map(|_| (LoaderFit::Tagged, version.version_type.into()))
    })
    .cloned()
    .ok_or_else(|| no_data_pack(game))
}

/// [`latest_world_datapack_modrinth`] for a CurseForge project.
pub(super) async fn latest_world_datapack_curseforge(
    curseforge: &CurseForge,
    project: &curseforge::Mod,
    game: &str,
) -> anyhow::Result<curseforge::File> {
    if project.class_id != Some(ClassId::Datapacks) {
        return Err(no_data_pack(game));
    }
    let mut files = curseforge
        .get_mod_files(ModFilesParameters {
            mod_id: project.id,
            query: ModFilesParametersQuery {
                game_version: Some(game.to_owned()),
                game_version_type_id: None,
                mod_loader_type: None,
                index: None,
                page_size: Some(200),
            },
        })
        .await?
        .data;
    files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
    best_candidate(&files, ModChannel::Stable, |file| {
        is_curseforge_world_file(project, file, game)
            .then(|| (LoaderFit::Tagged, ModChannel::from(file.release_type)))
    })
    .cloned()
    .ok_or_else(|| no_data_pack(game))
}

/// What the platform says about the build being installed, kept to record
/// its identity once it is in the world.
enum Identity {
    Modrinth {
        project: Project,
        version: Version,
        file: VersionFile,
    },
    Curseforge {
        project: curseforge::Mod,
        file: curseforge::File,
    },
}

/// A data pack build chosen for a world.
struct WorldDatapackBuild {
    name: String,
    filename: String,
    download: Downloadable,
    identity: Identity,
}

async fn resolve_modrinth(
    modrinth: &Modrinth,
    project_id: &str,
    version_id: Option<&str>,
    game: &str,
) -> anyhow::Result<WorldDatapackBuild> {
    let project = modrinth
        .get_project(ProjectID(project_id.to_owned()))
        .await?;
    let (version, file) = match version_id {
        Some(id) => {
            let version = modrinth.get_version(VersionID(id.to_owned())).await?;
            if version.project_id != project.id {
                bail!(
                    "Modrinth version {} is not of project {}",
                    version.id,
                    project.id
                );
            }
            // The user's own pick may list another game version.
            let file = modrinth_datapack_file(&version)
                .cloned()
                .ok_or(WorldRefusal::NotADatapackVersion)?;
            (version, file)
        }
        None => {
            let version = latest_world_datapack_modrinth(modrinth, &project, game).await?;
            let file = modrinth_world_file(&version, game)
                .cloned()
                .ok_or_else(|| no_data_pack(game))?;
            (version, file)
        }
    };
    let download = Downloadable::new(&file.url, PathBuf::new())
        .with_checksum(Some(Checksum::Sha512(file.hashes.sha512.clone())))
        .with_size(file.size.into());
    Ok(WorldDatapackBuild {
        name: project.title.clone(),
        filename: file.filename.clone(),
        download,
        identity: Identity::Modrinth {
            project,
            version,
            file,
        },
    })
}

async fn resolve_curseforge(
    curseforge: &CurseForge,
    project_id: u32,
    file_id: Option<u32>,
    game: &str,
) -> anyhow::Result<WorldDatapackBuild> {
    let project = curseforge
        .get_mod(ModParameters {
            mod_id: project_id.try_into()?,
        })
        .await?
        .data;
    let file = match file_id {
        Some(id) => {
            if project.class_id != Some(ClassId::Datapacks) {
                return Err(no_data_pack(game));
            }
            let file = curseforge
                .get_mod_file(ModFileParameters {
                    mod_id: project.id,
                    file_id: id.try_into()?,
                })
                .await?
                .data;
            if file.mod_id != project.id {
                bail!(
                    "CurseForge file {} is not of project {}",
                    file.id,
                    project.id
                );
            }
            // The user's own pick may list another game version.
            if !is_curseforge_datapack_file(&project, &file) {
                bail!(WorldRefusal::NotADatapackVersion);
            }
            file
        }
        None => latest_world_datapack_curseforge(curseforge, &project, game).await?,
    };
    let url = file
        .download_url
        .clone()
        .ok_or(WorldRefusal::NotDownloadable)?;
    let sha1 = file
        .hashes
        .iter()
        .find(|hash| matches!(hash.algo, curseforge::HashAlgo::Sha1))
        .with_context(|| format!("CurseForge gives no SHA-1 for file {}", file.id))?;
    let download = Downloadable::new(url, PathBuf::new())
        .with_checksum(Some(Checksum::Sha1(sha1.value.clone())))
        .with_size(file.file_length.into());
    Ok(WorldDatapackBuild {
        name: project.name.clone(),
        filename: file.file_name.clone(),
        download,
        identity: Identity::Curseforge { project, file },
    })
}

impl ManagerRef<'_, InstanceManager> {
    /// Installs a data pack from CurseForge or Modrinth into one world of an
    /// instance, as a visual task whose id this returns at once. What makes
    /// the install impossible is refused before the task starts, leaving
    /// the world as it is; a failed download or publication fails the task.
    pub async fn install_world_datapack(
        self,
        request: InstallWorldDatapack,
    ) -> anyhow::Result<VisualTaskId> {
        let platforms = self.app.modplatforms_manager();
        let (task, _) = self
            .install_world_datapack_from(&platforms.curseforge, &platforms.modrinth, request)
            .await?;
        Ok(task)
    }

    /// [`Self::install_world_datapack`] asking the given platform clients,
    /// with the handle of the work behind the task.
    pub(super) async fn install_world_datapack_from(
        self,
        curseforge: &CurseForge,
        modrinth: &Modrinth,
        request: InstallWorldDatapack,
    ) -> anyhow::Result<(
        VisualTaskId,
        tokio::task::JoinHandle<anyhow::Result<PathBuf>>,
    )> {
        let activity = self.app.runtime_lifecycle.admit_owned(
            "install a data pack into a world",
            ActivityPolicy::Installer,
        )?;
        let (task, task_id, build, staging, root_hold) = match self
            .admit_world_datapack_install(curseforge, modrinth, &request)
            .await
        {
            Ok(admitted) => admitted,
            Err(error) => {
                // Nothing was written: the activity is complete, not
                // left for a quit or move to wait on.
                activity.finish();
                return Err(error);
            }
        };
        let id = InstanceId(request.instance_id);

        let app = self.app.clone();
        let handle = spawn_retained(activity, move |slot| {
            Box::pin(async move {
                let cancel = slot.as_ref().unwrap().cancellation();
                // Stopped by a quit only before it takes the gate, so the
                // task reports what happened to the world.
                let result = app
                    .instance_manager()
                    .finish_world_datapack_install(
                        id, &task, &cancel, build, staging, root_hold, request,
                    )
                    .await;
                match &result {
                    Ok(_) => task.complete(),
                    // One line, with the code of a refusal in it.
                    Err(error) => task.clone().fail(crate::error::flattened(error)).await,
                }
                result
            })
        });
        Ok((task_id, handle))
    }

    /// Removes one data pack from a world, as the "Deletion through Recycle
    /// Bin" setting asks. Refused, leaving the world as it is, when the
    /// instance's addons may not change now or the row does not name an
    /// entry of the `datapacks` folder of one of the instance's saves.
    pub async fn delete_world_datapack(self, request: DeleteWorldDatapack) -> anyhow::Result<()> {
        let _activity = self
            .app
            .runtime_lifecycle
            .admit("delete a data pack from a world")?;
        let id = InstanceId(request.instance_id);
        {
            let _operation = self.lock_instance(id).await;
            self.ensure_world_datapacks_may_change(id).await?;
            let row = self
                .instance_world_datapack(id, &request.id)
                .await?
                .ok_or(WorldRefusal::PackGone)?;
            let removal = self.addon_removal().await?;
            // None: the `datapacks` folder is gone, and the pack with it.
            if let Some(path) = self.world_datapack_entry(id, &row).await?
                && let Some(meta) = addon_metadata(&path).await?
            {
                ensure_listed_kind(&path, &meta, row.is_folder).await?;
                // A link is removed by itself, never what it points to.
                let is_directory = row.is_folder && !meta.file_type().is_symlink();
                if !remove_addon_entry(&path, &meta, is_directory, removal).await? {
                    bail!(WorldRefusal::PackChanged);
                }
            }
            wdb::delete_world_datapack_by_id(&self.app.db, &row.id).await?;
        }
        self.refresh_worlds(id, "a removal").await;
        Ok(())
    }

    /// The world data pack `row_id` when it is one of the instance's.
    pub(super) async fn instance_world_datapack(
        self,
        id: InstanceId,
        row_id: &str,
    ) -> anyhow::Result<Option<WorldDatapackRow>> {
        Ok(wdb::get_world_datapack_by_id(&self.app.db, row_id)
            .await?
            .filter(|row| row.instance_id == *id))
    }

    /// Where the entry `row` lists is: `saves/<world>/datapacks/<filename>`,
    /// with each a single name, the world an existing save and its
    /// `datapacks` folder followed when it is a link, so the entry is a
    /// direct child of that folder. `None` when there is no `datapacks`
    /// folder. Call with the instance gate held.
    pub(super) async fn world_datapack_entry(
        self,
        id: InstanceId,
        row: &WorldDatapackRow,
    ) -> anyhow::Result<Option<PathBuf>> {
        validate_addon_basename(&row.world)?;
        validate_addon_basename(&row.filename)?;
        let path = publication::current_path(&self.app, id).await?;
        let root = path.get_root();
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        world_identity(id, &root, &saves.join(&row.world)).await?;
        let folder = world_datapacks_folder(&saves, &row.world);
        if !publication::followed_folder(&folder).await? {
            return Ok(None);
        }
        Ok(Some(folder.join(&row.filename)))
    }

    /// The row an install into `world` replaces, refused when it is in
    /// another world or a folder. `None` when it is no longer there.
    async fn replaceable_row(
        self,
        id: InstanceId,
        world: &str,
        row_id: &str,
    ) -> anyhow::Result<Option<WorldDatapackRow>> {
        let Some(row) = self.instance_world_datapack(id, row_id).await? else {
            return Ok(None);
        };
        if row.world != world {
            bail!(WorldRefusal::OtherWorld);
        }
        if row.is_folder {
            bail!(WorldRefusal::FolderNotReplaced);
        }
        Ok(Some(row))
    }

    /// The file a change of `world` replaces, as it is now under the gate.
    /// `None` when it went since the row was read: nothing to take away.
    /// Refused when the file there is no longer the one the row recorded:
    /// one put under that name since (the user's own build, say) is not the
    /// launcher's to remove.
    pub(super) async fn replaced_entry(
        self,
        id: InstanceId,
        world: &str,
        row_id: &str,
    ) -> anyhow::Result<Option<PathBuf>> {
        let Some(row) = self.replaceable_row(id, world, row_id).await? else {
            return Ok(None);
        };
        let Some(path) = self.world_datapack_entry(id, &row).await? else {
            return Ok(None);
        };
        let Some(meta) = addon_metadata(&path).await? else {
            return Ok(None);
        };
        ensure_listed_kind(&path, &meta, false).await?;
        if !self.holds_recorded_file(id, &row).await? {
            bail!(WorldRefusal::PackChanged);
        }
        Ok(Some(path))
    }

    /// Rescans the worlds after a change by `what`, and tells the frontend.
    pub(super) async fn refresh_worlds(self, id: InstanceId, what: &str) {
        match self.scan_world_datapacks(id).await {
            Ok(true) => {
                self.app
                    .meta_cache_manager()
                    .queue_platform_passes(CacheEntityId::Instance(id))
                    .await;
            }
            Ok(false) => {}
            Err(error) => warn!(
                ?error,
                "Instance {id}: could not scan the data packs in its worlds after {what}"
            ),
        }
        // Also when the scan saw no change: a recorded identity is new.
        self.app.invalidate(GET_WORLDS, Some((*id).into()));
        self.app.invalidate(INSTANCE_MODS, Some((*id).into()));
    }

    /// Everything before the download: the refusals, the build to install,
    /// its private folder and the visual task.
    async fn admit_world_datapack_install(
        self,
        curseforge: &CurseForge,
        modrinth: &Modrinth,
        request: &InstallWorldDatapack,
    ) -> anyhow::Result<(
        VisualTask,
        VisualTaskId,
        WorldDatapackBuild,
        Staging,
        InstanceRootGuard,
    )> {
        let id = InstanceId(request.instance_id);
        validate_addon_basename(&request.world)?;
        let game = self.runs(id).await?.release;
        let build = match &request.source {
            WorldDatapackSource::Modrinth {
                project_id,
                version_id,
            } => resolve_modrinth(modrinth, project_id, version_id.as_deref(), &game).await?,
            WorldDatapackSource::Curseforge {
                project_id,
                file_id,
            } => resolve_curseforge(curseforge, *project_id, *file_id, &game).await?,
        };
        validate_addon_basename(&build.filename)?;

        let operation = self.lock_instance(id).await;
        self.ensure_world_datapacks_may_change(id).await?;
        let replaced = match &request.replaces {
            Some(row_id) => Some(
                self.replaceable_row(id, &request.world, row_id)
                    .await?
                    .ok_or(WorldRefusal::PackGone)?,
            ),
            None => None,
        };
        let path = publication::current_path(&self.app, id).await?;
        let root = path.get_root();
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let world = saves.join(&request.world);
        if world_identity(id, &root, &world).await? != request.identity {
            bail!(WorldRefusal::WorldChanged);
        }
        let folder = world_datapacks_folder(&saves, &request.world);
        // An update keeps the file name: the name is its own, not taken.
        let own_name = replaced
            .as_ref()
            .is_some_and(|row| row.filename == build.filename);
        if !own_name
            && publication::followed_folder(&folder).await?
            && publication::selected_file(&folder, &build.filename)
                .await?
                .is_some()
        {
            bail!(WorldRefusal::NameTaken {
                filename: build.filename.clone(),
            });
        }
        // A private folder in the instance root: the download and what the
        // downloader keeps beside it go when it is dropped.
        let staging = Staging::new(&root)?;
        let instance_name = {
            let instances = self.instances.read().await;
            instances
                .get(&id)
                .ok_or(InvalidInstanceIdError(id))?
                .data()?
                .config
                .name
                .clone()
        };
        let task = VisualTask::new(Translation::InstanceTaskStageDatapack {
            mod_name: build.name.clone(),
            instance_name,
        });
        task.require_explicit_completion();
        let task_id = self.app.task_manager().spawn_task(&task).await;
        // Taken under the gate, so it never waits; a delete or a rename that
        // moves the folder waits for it, as for a duplicate's copy.
        let root_hold = self.hold_root(&operation, false).await;
        // The download does not hold up Play, scans or other changes.
        drop(operation);

        Ok((task, task_id, build, staging, root_hold))
    }

    /// Refuses while the instance's addons may not change or its game is
    /// queued, preparing, running or being deleted.
    pub(super) async fn ensure_world_datapacks_may_change(
        self,
        id: InstanceId,
    ) -> anyhow::Result<()> {
        self.ensure_modpack_not_locked(id).await?;
        self.ensure_game_inactive(id).await
    }

    /// Refuses while the instance's game is queued, preparing or running:
    /// it has the worlds open and writes them when it closes.
    pub(super) async fn ensure_game_inactive(self, id: InstanceId) -> anyhow::Result<()> {
        let inactive = matches!(
            self.instances
                .read()
                .await
                .get(&id)
                .ok_or(InvalidInstanceIdError(id))?
                .data()?
                .state,
            LaunchState::Inactive { .. }
        );
        if !inactive {
            bail!(WorldRefusal::GameActive);
        }
        Ok(())
    }

    /// The task's work: download, publish, record what the platform said
    /// about the file and rescan the worlds. `root_hold` keeps a delete or a
    /// rename that moves the instance folder waiting while the download
    /// writes into `staging`.
    #[allow(clippy::too_many_arguments)]
    async fn finish_world_datapack_install(
        self,
        id: InstanceId,
        task: &VisualTask,
        cancel: &tokio_util::sync::CancellationToken,
        build: WorldDatapackBuild,
        staging: Staging,
        root_hold: InstanceRootGuard,
        request: InstallWorldDatapack,
    ) -> anyhow::Result<PathBuf> {
        let mut download = build.download;
        download.path = staging.path().join(ARTIFACT);
        let subtask = task.subtask(Translation::InstanceTaskInstallModDownloadFile);
        task.edit(|data| data.state = TaskState::KnownProgress)
            .await;
        let (progress_tx, mut progress_rx) =
            tokio::sync::watch::channel(carbon_net::Progress::new());
        let progress = async move {
            while progress_rx.changed().await.is_ok() {
                subtask.update_download_progress(&progress_rx.borrow_and_update(), false);
            }
            subtask
        };
        let transfer = carbon_net::download_multiple(
            std::slice::from_ref(&download),
            DownloadOptions::builder()
                .concurrency(1)
                .cancel_token(cancel.clone())
                .progress_sender(progress_tx)
                .build(),
        );
        let (result, subtask) = tokio::select! {
            biased;
            _ = cancel.cancelled() => bail!(CLOSING),
            done = async { tokio::join!(transfer, progress) } => done,
        };
        result.map_err(|error| WorldRefusal::DownloadFailed {
            filename: build.filename.clone(),
            cause: format!("{:#}", anyhow::Error::from(error)),
        })?;
        subtask.complete_download();

        #[cfg(test)]
        self.test_hooks.pause("world_datapack_downloaded", id).await;
        // Given back before the gate is taken (the gate comes first). The
        // downloader is done with the folder: a delete that goes first
        // removes it with the instance, a rename moves it with the root,
        // and both are seen under the gate.
        drop(root_hold);
        let destination = self
            .publish_world_datapack_download(id, cancel, &build.filename, staging, &request)
            .await?;
        // The pack is in the world: from here on nothing fails the install.
        if let Err(error) = self
            .record_world_datapack_identity(&destination, &build.filename, &build.identity)
            .await
        {
            warn!(
                ?error,
                "Instance {id}: could not record the platform identity of data pack {}",
                build.filename
            );
        }
        self.refresh_worlds(id, "an install").await;
        Ok(destination)
    }

    /// Under the instance gate: the re-checks, then the publication, with
    /// the pack the install replaces taken away first. The private folder
    /// goes before the gate on every return, so a delete or a rename queued
    /// behind the gate never meets it half removed. A quit stops it only
    /// while it waits for the gate.
    async fn publish_world_datapack_download(
        self,
        id: InstanceId,
        cancel: &tokio_util::sync::CancellationToken,
        filename: &str,
        staging: Staging,
        request: &InstallWorldDatapack,
    ) -> anyhow::Result<PathBuf> {
        // The last point a quit stops the install. From here on it runs to
        // its end, and the quit waits for it: once the old pack is taken
        // away, it is either back or sent on its way.
        let operation = tokio::select! {
            biased;
            _ = cancel.cancelled() => bail!(CLOSING),
            operation = self.lock_instance(id) => operation,
        };
        // Declared after the gate, so dropped before it on every return.
        let mut staging = staging;
        #[cfg(test)]
        self.test_hooks.pause("world_datapack_gate_taken", id).await;
        // Removed from wherever the root is now, also when refused or
        // stopped: a rename the install waited for may have moved it.
        if let Ok(path) = publication::current_path(&self.app, id).await {
            staging.root = long_path(&path.get_root());
        }
        if cancel.is_cancelled() {
            bail!(CLOSING);
        }
        self.ensure_world_datapacks_may_change(id).await?;
        let path = publication::current_path(&self.app, id).await?;
        let root = path.get_root();
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let world = saves.join(&request.world);
        // Before the folder is made: never in a save put in its place.
        // A moved root also gives the save another identity.
        if world_identity(id, &root, &world).await? != request.identity {
            bail!(WorldRefusal::WorldChangedDuringCopy);
        }
        let artifact = staging.path().join(ARTIFACT);
        let temporary = tempfile::NamedTempFile::from_parts(
            std::fs::File::open(&artifact)
                .with_context(|| format!("The download of {filename} is gone"))?,
            tempfile::TempPath::from_path(&artifact),
        );
        let folder = world_datapacks_folder(&saves, &request.world);
        if !publication::followed_folder(&folder).await? {
            tokio::fs::create_dir(&folder).await?;
        }
        let replaced = match &request.replaces {
            Some(row_id) => self
                .replaced_entry(id, &request.world, row_id)
                .await?
                .into_iter()
                .collect(),
            None => Vec::new(),
        };
        let removal = self.addon_removal().await?;
        let published = self
            .publish_replacing(
                id,
                &root,
                temporary,
                &world,
                &request.identity,
                filename,
                replaced,
                &staging,
                removal,
            )
            .await;
        drop(staging);
        drop(operation);
        published
    }

    /// [`Self::publish_replacing`] for a copy into a world, which has no
    /// private folder of its own yet: the old packs wait in one made for
    /// it. Call with the instance gate held.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn publish_into_world_replacing(
        self,
        id: InstanceId,
        root: &Path,
        temporary: tempfile::NamedTempFile,
        world: &Path,
        identity: &str,
        filename: &str,
        replaced: Vec<PathBuf>,
    ) -> anyhow::Result<PathBuf> {
        if replaced.is_empty() {
            return self
                .publish_into_world(id, root, temporary, world, identity, filename)
                .await;
        }
        let removal = self.addon_removal().await?;
        let staging = Staging::new(root)?;
        self.publish_replacing(
            id, root, temporary, world, identity, filename, replaced, &staging, removal,
        )
        .await
    }

    /// [`Self::publish_into_world`], after taking `replaced` away. The old
    /// packs are sent on their way with `removal` only once the new one is
    /// in place; when publishing fails, or the Recycle Bin refuses an old
    /// pack, the world is put back as it was, but for old packs the Recycle
    /// Bin already took. When an old pack cannot go back, the error says
    /// where it is, and `staging` is kept so it stays there.
    #[allow(clippy::too_many_arguments)]
    async fn publish_replacing(
        self,
        id: InstanceId,
        root: &Path,
        temporary: tempfile::NamedTempFile,
        world: &Path,
        identity: &str,
        filename: &str,
        replaced: Vec<PathBuf>,
        staging: &Staging,
        removal: Removal,
    ) -> anyhow::Result<PathBuf> {
        if replaced.is_empty() {
            return self
                .publish_into_world(id, root, temporary, world, identity, filename)
                .await;
        }
        let mut taken = take_away_all(
            &replaced,
            &staging.path().join(REPLACED),
            removal,
            staging.keeper(),
        )
        .await?;
        #[cfg(test)]
        self.test_hooks.pause("world_datapack_taken_away", id).await;
        let error = match self
            .world_destination(id, root, &temporary, world, identity, filename)
            .await
        {
            Err(error) => error,
            Ok(destination) => match taken.publish(temporary, &destination).await {
                Err(error) => error,
                Ok(()) => match taken.send_on().await {
                    Ok(()) => return Ok(destination),
                    Err(error) => error,
                },
            },
        };
        match taken.put_back().await {
            Ok(()) => Err(error),
            Err(put_back) => Err(anyhow!("{error:#}. {put_back:#}")),
        }
    }

    /// The platform's identity of a file just published, recorded at once:
    /// the lookup the platform passes make later can miss it.
    async fn record_world_datapack_identity(
        self,
        published: &Path,
        filename: &str,
        identity: &Identity,
    ) -> anyhow::Result<()> {
        let folder = published
            .parent()
            .context("a published data pack is in a folder")?
            .to_path_buf();
        let cache = self.app.meta_cache_manager();
        let parsed = cache
            .hash_and_parse_mod_file(&folder, filename, true)
            .await?;
        let metadata_id = cache.ensure_mod_metadata(&parsed, filename).await?;
        match identity {
            Identity::Modrinth {
                project,
                version,
                file,
            } => record_installed_version(self.app, &metadata_id, version, file, project).await,
            Identity::Curseforge { project, file } => {
                record_installed_file(self.app, &metadata_id, file, project).await
            }
        }
    }
}

const CLOSING: &str = "Data pack install stopped because the launcher is closing";

/// The name of the download inside the install's private folder.
const ARTIFACT: &str = "artifact";
/// The folder inside the install's private folder where the pack it
/// replaces waits for the new one.
const REPLACED: &str = "replaced";

/// An install's private folder in the instance root, removed when dropped
/// from under `root`, which follows the instance's folder if a rename moved
/// it.
struct Staging {
    root: PathBuf,
    name: std::ffi::OsString,
    /// Set when the folder holds a pack an error or a warning names.
    kept: Arc<AtomicBool>,
}

impl Staging {
    fn new(root: &Path) -> std::io::Result<Self> {
        let root = long_path(root);
        let folder = tempfile::Builder::new()
            .prefix(".tmp")
            .tempdir_in(&root)?
            .keep();
        let name = folder
            .file_name()
            .expect("a temporary folder has a name")
            .to_owned();
        Ok(Self {
            root,
            name,
            kept: Default::default(),
        })
    }

    fn path(&self) -> PathBuf {
        self.root.join(&self.name)
    }

    /// The flag that leaves the folder in place once set.
    fn keeper(&self) -> Arc<AtomicBool> {
        self.kept.clone()
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if self.kept.load(Ordering::SeqCst) {
            return;
        }
        match std::fs::remove_dir_all(self.path()) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => warn!(
                ?error,
                "Could not remove the private download folder {}",
                self.path().display()
            ),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::managers::App;
    use crate::managers::instance::mods::tests::addon_fixture;
    use carbon_repos::repos::{mod_metadata as metarepo, world_datapack_cache as wdb};
    use mockito::Matcher;
    use serde_json::json;
    use sha2::Digest;
    use std::io::Write;

    pub(crate) const GAME: &str = "1.20.1";
    pub(crate) const CF_PROJECT: i32 = 4242;

    pub(crate) struct Api {
        server: mockito::ServerGuard,
        pub(crate) curseforge: CurseForge,
        pub(crate) modrinth: Modrinth,
    }

    pub(crate) async fn api() -> Api {
        let server = mockito::Server::new_async().await;
        let base = format!("{}/", server.url());
        let client = || reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build();
        Api {
            curseforge: CurseForge::with_base_url(client(), &base),
            modrinth: Modrinth::with_base_url(client(), &base),
            server,
        }
    }

    impl Api {
        pub(crate) async fn get(&mut self, path: &str, body: serde_json::Value) {
            self.server
                .mock("GET", path)
                .match_query(Matcher::Any)
                .with_body(body.to_string())
                .create_async()
                .await;
        }

        /// Serves `bytes` as the download at `/files/<name>`; returns its URL.
        pub(crate) async fn file(&mut self, name: &str, bytes: &[u8]) -> String {
            let path = format!("/files/{name}");
            self.server
                .mock("GET", path.as_str())
                .with_body(bytes)
                .create_async()
                .await;
            format!("{}{path}", self.server.url())
        }
    }

    pub(crate) fn pack_zip(description: &str) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("pack.mcmeta", zip::write::SimpleFileOptions::default())
            .unwrap();
        write!(
            zip,
            r#"{{"pack":{{"pack_format":15,"description":"{description}"}}}}"#
        )
        .unwrap();
        zip.finish().unwrap().into_inner()
    }

    fn mr_project() -> serde_json::Value {
        json!({
            "slug": "world-pack", "title": "World Pack", "description": "", "categories": [], "client_side": "optional", "server_side": "optional",
            "body": "", "additional_categories": [], "donation_urls": [], "project_type": "mod", "downloads": 0, "id": "project", "team": "fixture",
            "published": "2026-01-01T00:00:00Z", "updated": "2026-01-01T00:00:00Z", "followers": 0, "status": "approved",
            "license": {"id": "MIT", "name": "MIT", "url": null}, "versions": [], "game_versions": [], "loaders": []
        })
    }

    /// A Modrinth version whose one file, `filename` at `url`, holds `bytes`.
    pub(crate) fn mr_version(
        id: &str,
        day: u32,
        version_type: &str,
        loaders: &[&str],
        (filename, url, bytes): (&str, &str, &[u8]),
    ) -> serde_json::Value {
        json!({
            "id": id, "project_id": "project", "name": format!("{id} name"), "version_number": id,
            "dependencies": [], "game_versions": [GAME], "version_type": version_type,
            "loaders": loaders, "featured": false, "author_id": "author",
            "date_published": format!("2026-01-{day:02}T00:00:00Z"), "downloads": 0,
            "files": [{
                "hashes": {
                    "sha512": hex::encode(sha2::Sha512::digest(bytes)),
                    "sha1": hex::encode(sha1::Sha1::digest(bytes)),
                },
                "url": url, "filename": filename, "primary": true,
                "size": bytes.len(), "file_type": null
            }]
        })
    }

    pub(crate) fn cf_project(class: i32) -> serde_json::Value {
        json!({
            "id": CF_PROJECT, "gameId": 432, "name": "Curse Pack", "slug": "curse-pack", "summary": "", "status": 4,
            "links": {"websiteUrl": null, "wikiUrl": null, "issuesUrl": null, "sourceUrl": null},
            "downloadCount": 0, "isFeatured": false, "primaryCategoryId": 0, "categories": [], "classId": class,
            "authors": [], "logo": null, "screenshots": [], "mainFileId": 0, "latestFiles": [], "latestFilesIndexes": [],
            "dateCreated": "2026-01-01T00:00:00Z", "dateModified": "2026-01-01T00:00:00Z", "dateReleased": "2026-01-01T00:00:00Z",
            "allowModDistribution": true, "gamePopularityRank": 0, "isAvailable": true, "thumbsUpCount": 0
        })
    }

    /// `release_type`: 1 release, 2 beta, 3 alpha.
    pub(crate) fn cf_file(
        id: i32,
        day: u32,
        release_type: u8,
        (filename, url, bytes): (&str, &str, &[u8]),
        fingerprint: u32,
    ) -> serde_json::Value {
        json!({
            "id": id, "gameId": 432, "modId": CF_PROJECT, "isAvailable": true,
            "displayName": format!("file {id}"), "fileName": filename,
            "releaseType": release_type, "fileStatus": 4,
            "hashes": [{"value": hex::encode(sha1::Sha1::digest(bytes)), "algo": 1}],
            "fileDate": format!("2026-01-{day:02}T00:00:00Z"), "fileLength": bytes.len(),
            "downloadCount": 0, "downloadUrl": url, "gameVersions": [GAME],
            "sortableGameVersions": [], "dependencies": [], "fileFingerprint": fingerprint
        })
    }

    /// The CurseForge fingerprint of `bytes`, as the scanner computes it.
    async fn fingerprint(app: &App, bytes: &[u8]) -> u32 {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("pack.zip"), bytes).unwrap();
        let cache = app.meta_cache_manager();
        let parsed = cache
            .hash_and_parse_mod_file(&folder.path().to_path_buf(), "pack.zip", true)
            .await
            .unwrap();
        let metadata_id = cache
            .ensure_mod_metadata(&parsed, "pack.zip")
            .await
            .unwrap();
        metarepo::get_metadata_by_id(&app.db, &metadata_id)
            .await
            .unwrap()
            .unwrap()
            .murmur2 as u32
    }

    /// An instance at 1.20.1 with the save `World` and its addon row. Returns
    /// the instance, the world's identity and the world's folder.
    async fn world_instance(app: &App, name: &str) -> (InstanceId, String, PathBuf) {
        let (id, _, world) = addon_fixture(app, name, AddonType::Worlds, "World", b"save").await;
        let identity = app
            .instance_manager()
            .datapack_worlds(id)
            .await
            .unwrap()
            .pop()
            .unwrap()
            .identity;
        (id, identity, world)
    }

    fn request(
        id: InstanceId,
        identity: &str,
        source: WorldDatapackSource,
    ) -> InstallWorldDatapack {
        InstallWorldDatapack {
            instance_id: *id,
            world: "World".into(),
            identity: identity.into(),
            source,
            replaces: None,
        }
    }

    fn modrinth(version_id: Option<&str>) -> WorldDatapackSource {
        WorldDatapackSource::Modrinth {
            project_id: "project".into(),
            version_id: version_id.map(str::to_owned),
        }
    }

    fn curseforge(file_id: Option<u32>) -> WorldDatapackSource {
        WorldDatapackSource::Curseforge {
            project_id: CF_PROJECT as u32,
            file_id,
        }
    }

    /// Runs the install to its end: the task's outcome.
    async fn install(
        app: &App,
        api: &Api,
        request: InstallWorldDatapack,
    ) -> anyhow::Result<(VisualTaskId, PathBuf)> {
        let (task, work) = app
            .instance_manager()
            .install_world_datapack_from(&api.curseforge, &api.modrinth, request)
            .await?;
        Ok((task, work.await??))
    }

    pub(crate) fn entries(folder: &Path) -> Vec<(String, Vec<u8>)> {
        let mut entries: Vec<_> = match std::fs::read_dir(folder) {
            Ok(entries) => entries
                .map(|entry| {
                    let entry = entry.unwrap();
                    let bytes = std::fs::read(entry.path()).unwrap_or_default();
                    (entry.file_name().to_string_lossy().into_owned(), bytes)
                })
                .collect(),
            Err(_) => Vec::new(),
        };
        entries.sort();
        entries
    }

    /// The instance root's entries and the world's data packs, to show that
    /// a refused or failed install leaves both as they were.
    pub(crate) fn snapshot(app_root: &Path, world: &Path) -> (Vec<String>, Vec<(String, Vec<u8>)>) {
        let mut root: Vec<_> = std::fs::read_dir(app_root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        root.sort();
        (root, entries(&world.join("datapacks")))
    }

    async fn instance_root(app: &App, id: InstanceId) -> PathBuf {
        publication::current_path(app, id).await.unwrap().get_root()
    }

    /// An API whose Modrinth project lists `versions` and serves each of
    /// them alone too.
    pub(crate) async fn modrinth_serving(api: &mut Api, versions: Vec<serde_json::Value>) {
        api.get("/project/project", mr_project()).await;
        for version in &versions {
            api.get(
                &format!("/version/{}", version["id"].as_str().unwrap()),
                version.clone(),
            )
            .await;
        }
        api.get("/project/project/version", json!(versions)).await;
    }

    #[tokio::test]
    async fn the_latest_build_is_the_newest_release_before_a_newer_beta() {
        let mut api = api().await;
        let zip = pack_zip("latest");
        let url = api.file("pack.zip", &zip).await;
        let file = ("pack.zip", url.as_str(), zip.as_slice());
        api.get(
            "/project/project/version",
            json!([
                mr_version("beta", 20, "beta", &["datapack"], file),
                mr_version("release", 10, "release", &["datapack"], file),
                mr_version("older", 5, "release", &["datapack"], file),
            ]),
        )
        .await;
        let project = serde_json::from_value(mr_project()).unwrap();
        let version = latest_world_datapack_modrinth(&api.modrinth, &project, GAME)
            .await
            .unwrap();
        assert_eq!(version.id, "release");

        api.get(
            &format!("/mods/{CF_PROJECT}/files"),
            json!({ "data": [
                cf_file(30, 20, 2, file, 0),
                cf_file(20, 10, 1, file, 0),
                cf_file(10, 5, 1, file, 0),
            ]}),
        )
        .await;
        let project = serde_json::from_value(cf_project(6945)).unwrap();
        let file = latest_world_datapack_curseforge(&api.curseforge, &project, GAME)
            .await
            .unwrap();
        assert_eq!(file.id, 20);
    }

    #[tokio::test]
    async fn a_version_without_the_datapack_tag_or_with_a_jar_file_is_no_data_pack() {
        let mut api = api().await;
        let zip = pack_zip("tagged");
        api.get(
            "/project/project/version",
            json!([
                mr_version(
                    "fabric-only",
                    20,
                    "release",
                    &["fabric"],
                    ("pack.zip", "https://cdn.invalid/pack.zip", &zip)
                ),
                mr_version(
                    "jar",
                    10,
                    "release",
                    &["datapack", "fabric"],
                    ("pack.jar", "https://cdn.invalid/pack.jar", &zip)
                ),
            ]),
        )
        .await;
        let project = serde_json::from_value(mr_project()).unwrap();
        let error = latest_world_datapack_modrinth(&api.modrinth, &project, GAME)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "This project has no data pack for Minecraft 1.20.1"
        );
    }

    #[tokio::test]
    async fn a_chosen_version_tagged_only_fabric_is_refused() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "chosen-fabric").await;
        let mut api = api().await;
        let zip = pack_zip("fabric");
        let url = api.file("pack.zip", &zip).await;
        modrinth_serving(
            &mut api,
            vec![mr_version(
                "fabric-only",
                20,
                "release",
                &["fabric"],
                ("pack.zip", &url, &zip),
            )],
        )
        .await;
        let root = instance_root(&app, id).await;
        let before = snapshot(&root, &world);
        let error = install(
            &app,
            &api,
            request(id, &identity, modrinth(Some("fabric-only"))),
        )
        .await
        .unwrap_err();
        // The project may hold a data pack beside the build that was picked.
        assert_eq!(
            error.downcast_ref(),
            Some(&WorldRefusal::NotADatapackVersion)
        );
        assert_eq!(snapshot(&root, &world), before);

        // Asked for its newest build, the project has none for the world.
        let error = install(&app, &api, request(id, &identity, modrinth(None)))
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "This project has no data pack for Minecraft 1.20.1"
        );
        // The frontend words the refusal from its code and the version.
        let sent = serde_json::to_value(crate::error::FeError::from_anyhow(&error)).unwrap();
        let cause = &sent["cause"][0];
        assert_eq!(cause["code"], "NO_DATAPACK_FOR_VERSION", "{sent}");
        assert_eq!(cause["data"], json!({ "version": "1.20.1" }), "{sent}");
        assert_eq!(snapshot(&root, &world), before);
    }

    /// A build tagged as a data pack whose file is the mod jar is refused
    /// when picked, as the newest one is skipped.
    #[tokio::test]
    async fn a_chosen_version_whose_file_is_a_jar_is_refused() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "chosen-jar").await;
        let mut api = api().await;
        let jar = pack_zip("jar");
        let url = api.file("pack.jar", &jar).await;
        modrinth_serving(
            &mut api,
            vec![mr_version(
                "jar",
                20,
                "release",
                &["datapack", "fabric"],
                ("pack.jar", &url, &jar),
            )],
        )
        .await;
        let root = instance_root(&app, id).await;
        let before = snapshot(&root, &world);
        let error = install(&app, &api, request(id, &identity, modrinth(Some("jar"))))
            .await
            .unwrap_err();
        assert_eq!(
            error.downcast_ref(),
            Some(&WorldRefusal::NotADatapackVersion)
        );
        assert_eq!(snapshot(&root, &world), before);
    }

    /// A version the user picks only has to be a data pack: one that lists
    /// another game version than the instance's is installed. The newest
    /// build is still only one for the instance's game version.
    #[tokio::test]
    async fn a_chosen_modrinth_version_for_another_game_version_is_installed() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "chosen-other-game-mr").await;
        let mut api = api().await;
        let zip = pack_zip("for 1.20");
        let url = api.file("pack.zip", &zip).await;
        let mut version = mr_version(
            "for-1.20",
            20,
            "release",
            &["datapack"],
            ("pack.zip", &url, &zip),
        );
        version["game_versions"] = json!(["1.20"]);
        modrinth_serving(&mut api, vec![version]).await;

        let error = install(&app, &api, request(id, &identity, modrinth(None)))
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "This project has no data pack for Minecraft 1.20.1"
        );
        assert!(entries(&world.join("datapacks")).is_empty());

        let (_, published) = install(
            &app,
            &api,
            request(id, &identity, modrinth(Some("for-1.20"))),
        )
        .await
        .unwrap();
        assert_eq!(published, world.join("datapacks").join("pack.zip"));
        assert_eq!(
            entries(&world.join("datapacks")),
            vec![("pack.zip".to_owned(), zip)]
        );
    }

    /// [`a_chosen_modrinth_version_for_another_game_version_is_installed`]
    /// for a CurseForge file; one that is no zip is refused.
    #[tokio::test]
    async fn a_chosen_curseforge_file_for_another_game_version_is_installed() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "chosen-other-game-cf").await;
        let mut api = api().await;
        let zip = pack_zip("curse, for 1.20");
        let url = api.file("curse.zip", &zip).await;
        let fingerprint = fingerprint(&app, &zip).await;
        let mut file = cf_file(20, 10, 1, ("curse.zip", &url, &zip), fingerprint);
        file["gameVersions"] = json!(["1.20"]);
        let mut jar = cf_file(21, 11, 1, ("curse.jar", &url, &zip), 0);
        jar["gameVersions"] = json!(["1.20"]);
        api.get(
            &format!("/mods/{CF_PROJECT}"),
            json!({ "data": cf_project(6945) }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files"),
            json!({ "data": [jar.clone(), file.clone()] }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files/20"),
            json!({ "data": file }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files/21"),
            json!({ "data": jar }),
        )
        .await;

        let error = install(&app, &api, request(id, &identity, curseforge(None)))
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "This project has no data pack for Minecraft 1.20.1"
        );
        let error = install(&app, &api, request(id, &identity, curseforge(Some(21))))
            .await
            .unwrap_err();
        assert_eq!(
            error.downcast_ref(),
            Some(&WorldRefusal::NotADatapackVersion)
        );
        assert!(entries(&world.join("datapacks")).is_empty());

        install(&app, &api, request(id, &identity, curseforge(Some(20))))
            .await
            .unwrap();
        assert_eq!(
            entries(&world.join("datapacks")),
            vec![("curse.zip".to_owned(), zip)]
        );
    }

    #[tokio::test]
    async fn a_curseforge_project_of_class_mods_is_refused() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "cf-mods-class").await;
        let mut api = api().await;
        let zip = pack_zip("mods class");
        let url = api.file("pack.zip", &zip).await;
        let file = cf_file(20, 10, 1, ("pack.zip", &url, &zip), 0);
        api.get(
            &format!("/mods/{CF_PROJECT}"),
            json!({ "data": cf_project(6) }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files"),
            json!({ "data": [file.clone()] }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files/20"),
            json!({ "data": file }),
        )
        .await;
        let root = instance_root(&app, id).await;
        let before = snapshot(&root, &world);
        for file_id in [None, Some(20)] {
            let error = install(&app, &api, request(id, &identity, curseforge(file_id)))
                .await
                .unwrap_err();
            assert_eq!(
                error.to_string(),
                "This project has no data pack for Minecraft 1.20.1",
                "{file_id:?}"
            );
        }
        assert_eq!(snapshot(&root, &world), before);
    }

    #[tokio::test]
    async fn a_download_that_fails_its_checksum_publishes_nothing() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "checksum").await;
        let mut api = api().await;
        let listed = pack_zip("listed");
        let served = pack_zip("served");
        assert_eq!(served.len(), listed.len(), "only the bytes differ");
        let url = api.file("pack.zip", &served).await;
        modrinth_serving(
            &mut api,
            vec![mr_version(
                "release",
                10,
                "release",
                &["datapack"],
                ("pack.zip", &url, &listed),
            )],
        )
        .await;
        let root = instance_root(&app, id).await;
        let before = snapshot(&root, &world);
        let (task, work) = app
            .instance_manager()
            .install_world_datapack_from(
                &api.curseforge,
                &api.modrinth,
                request(id, &identity, modrinth(None)),
            )
            .await
            .unwrap();
        let error = work.await.unwrap().unwrap_err();
        assert!(
            format!("{error:#}").to_lowercase().contains("checksum"),
            "{error:#}"
        );
        // The failed task shows the same line, and says what failed by its
        // code, so the desktop can word it.
        let crate::managers::vtask::TaskOutcome::Failed(failed) =
            app.task_manager().get_task_outcome(task).await
        else {
            panic!("the task failed");
        };
        let crate::domain::vtask::Progress::Failed(shown) = &failed.progress else {
            panic!("a failed task shows its error");
        };
        let sent = serde_json::to_value(crate::error::FeError::from_anyhow(shown)).unwrap();
        let cause = &sent["cause"][0];
        assert_eq!(cause["display"], format!("{error:#}"), "{sent}");
        assert_eq!(cause["code"], "DATAPACK_DOWNLOAD_FAILED", "{sent}");
        assert_eq!(cause["data"]["filename"], "pack.zip", "{sent}");
        assert_eq!(snapshot(&root, &world), before);
        assert!(
            wdb::get_world_datapacks_by_instance(&app.db, *id)
                .await
                .unwrap()
                .is_empty()
        );
    }

    /// Each refusal leaves the world's data packs and the instance root as
    /// they were.
    #[tokio::test]
    async fn an_install_the_instance_cannot_take_now_changes_nothing() {
        use crate::domain::vtask::VisualTaskId as TaskId;
        use crate::managers::instance::modpack::session::{Kind, RECORD, Session};
        use crate::managers::instance::run::RunningInstance;
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "refusals").await;
        let mut api = api().await;
        let zip = pack_zip("refused");
        let url = api.file("pack.zip", &zip).await;
        modrinth_serving(
            &mut api,
            vec![mr_version(
                "release",
                10,
                "release",
                &["datapack"],
                ("pack.zip", &url, &zip),
            )],
        )
        .await;
        let root = instance_root(&app, id).await;
        std::fs::create_dir_all(world.join("datapacks")).unwrap();
        std::fs::write(world.join("datapacks/kept.zip"), b"kept").unwrap();
        let before = snapshot(&root, &world);
        let set_state = |state: LaunchState| {
            let app = app.clone();
            async move {
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
        };
        let refused = |request: InstallWorldDatapack| {
            let (app, api) = (app.clone(), &api);
            async move { install(&app, api, request).await.unwrap_err().to_string() }
        };

        let running = "This instance is running or being installed. Close the game or wait for it to finish, then try again.";
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
            set_state(state).await;
            assert_eq!(
                refused(request(id, &identity, modrinth(None))).await,
                running,
                "{name}"
            );
            assert_eq!(snapshot(&root, &world), before, "{name}");
        }
        set_state(LaunchState::Deleting).await;
        assert!(
            refused(request(id, &identity, modrinth(None)))
                .await
                .contains("being deleted")
        );
        set_state(LaunchState::Inactive { failed_task: None }).await;

        assert_eq!(
            refused(request(id, "another world", modrinth(None))).await,
            "Selected world changed. Refresh the world list and select it again"
        );
        assert_eq!(
            refused(InstallWorldDatapack {
                replaces: Some("row".into()),
                ..request(id, &identity, modrinth(None))
            })
            .await,
            WorldRefusal::PackGone.to_string()
        );
        assert_eq!(snapshot(&root, &world), before);

        // A pending pack setup holds the instance's files.
        let mut pack = Session::load(&root).await.unwrap().unwrap();
        pack.kind = Kind::Pack;
        let record = root.join(".setup").join(RECORD);
        let original = std::fs::read(&record).unwrap();
        std::fs::write(&record, serde_json::to_vec(&pack).unwrap()).unwrap();
        install(&app, &api, request(id, &identity, modrinth(None)))
            .await
            .expect_err("a pending pack setup refuses");
        assert_eq!(snapshot(&root, &world), before);
        std::fs::write(&record, original).unwrap();

        // A custom game version file names no version to choose a build for.
        let version = {
            let manager = app.instance_manager();
            let mut instances = manager.instances.write().await;
            let config = &mut instances.get_mut(&id).unwrap().data_mut().unwrap().config;
            config.game_configuration.version.replace(
                crate::domain::instance::info::GameVersion::Custom("custom.json".into()),
            )
        };
        assert!(
            refused(request(id, &identity, modrinth(None)))
                .await
                .contains("custom game version")
        );
        assert_eq!(snapshot(&root, &world), before);
        app.instance_manager()
            .instances
            .write()
            .await
            .get_mut(&id)
            .unwrap()
            .data_mut()
            .unwrap()
            .config
            .game_configuration
            .version = version;
        assert!(
            !install_left_behind(&app),
            "a refusal leaves no activity behind"
        );
    }

    #[tokio::test]
    async fn a_name_the_world_already_holds_is_refused_and_kept() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "name-taken").await;
        let mut api = api().await;
        let zip = pack_zip("new");
        let url = api.file("pack.zip", &zip).await;
        modrinth_serving(
            &mut api,
            vec![mr_version(
                "release",
                10,
                "release",
                &["datapack"],
                ("pack.zip", &url, &zip),
            )],
        )
        .await;
        std::fs::create_dir_all(world.join("datapacks")).unwrap();
        std::fs::write(world.join("datapacks/pack.zip"), b"the user's own").unwrap();
        let root = instance_root(&app, id).await;
        let before = snapshot(&root, &world);
        let error = install(&app, &api, request(id, &identity, modrinth(None)))
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "This world already contains that datapack filename; existing content was retained"
        );
        assert_eq!(
            std::fs::read(world.join("datapacks/pack.zip")).unwrap(),
            b"the user's own"
        );
        assert_eq!(snapshot(&root, &world), before);
    }

    /// A save put in the selected one's place while the pack downloads gets
    /// nothing, and the moved one gets no pack either.
    #[tokio::test]
    async fn a_world_swapped_during_the_install_gets_nothing() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "swapped").await;
        let mut api = api().await;
        let zip = pack_zip("swapped");
        let url = api.file("pack.zip", &zip).await;
        modrinth_serving(
            &mut api,
            vec![mr_version(
                "release",
                10,
                "release",
                &["datapack"],
                ("pack.zip", &url, &zip),
            )],
        )
        .await;
        let pause = app
            .instance_manager()
            .test_hooks
            .arm("world_datapack_downloaded", id);
        let (_, work) = app
            .instance_manager()
            .install_world_datapack_from(
                &api.curseforge,
                &api.modrinth,
                request(id, &identity, modrinth(None)),
            )
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), pause.entered.notified())
            .await
            .unwrap();
        let moved = world.with_file_name("Moved");
        std::fs::rename(&world, &moved).unwrap();
        std::fs::create_dir(&world).unwrap();
        std::fs::write(world.join("level.dat"), b"save").unwrap();
        pause.release.notify_one();
        assert!(work.await.unwrap().is_err());
        assert!(
            !install_left_behind(&app),
            "a failed install leaves no activity behind"
        );
        assert!(!world.join("datapacks").exists());
        assert!(entries(&moved.join("datapacks")).is_empty());
    }

    /// The private folders of installs in an instance root.
    pub(crate) fn staging_folders(root: &Path) -> Vec<String> {
        std::fs::read_dir(root)
            .map(|entries| {
                entries
                    .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                    .filter(|name| name.starts_with(".tmp"))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// An install of a served Modrinth data pack, stopped once its download
    /// is done and before it publishes. Returns the work behind its task.
    async fn install_held_after_download(
        app: &App,
        api: &mut Api,
        id: InstanceId,
        identity: &str,
    ) -> (
        std::sync::Arc<crate::managers::instance::lifecycle_tests::Pause>,
        tokio::task::JoinHandle<anyhow::Result<PathBuf>>,
    ) {
        let zip = pack_zip("held");
        let url = api.file("pack.zip", &zip).await;
        modrinth_serving(
            api,
            vec![mr_version(
                "release",
                10,
                "release",
                &["datapack"],
                ("pack.zip", &url, &zip),
            )],
        )
        .await;
        let pause = app
            .instance_manager()
            .test_hooks
            .arm("world_datapack_downloaded", id);
        let (_, work) = app
            .instance_manager()
            .install_world_datapack_from(
                &api.curseforge,
                &api.modrinth,
                request(id, identity, modrinth(None)),
            )
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), pause.entered.notified())
            .await
            .unwrap();
        (pause, work)
    }

    pub(crate) async fn without_recycle_bin(app: &App) {
        app.settings_manager()
            .set(
                carbon_repos::repos::app_configuration::AppConfigurationPatch {
                    deletion_through_recycle_bin: Some(false),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
    }

    /// A delete asked for while the pack downloads waits for the download,
    /// then removes the instance with nothing of the install left.
    #[tokio::test]
    async fn deleting_the_instance_waits_for_the_download() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, identity, _) = world_instance(&app, "deleted-while-downloading").await;
        let root = instance_root(&app, id).await;
        let mut api = api().await;
        let (pause, work) = install_held_after_download(&app, &mut api, id, &identity).await;
        assert_eq!(
            staging_folders(&root).len(),
            1,
            "the download is in the root"
        );

        let deleting = app.clone();
        let mut delete =
            tokio::spawn(async move { deleting.instance_manager()._delete_instance(id).await });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(150), &mut delete)
                .await
                .is_err(),
            "the folder was deleted while the pack downloaded"
        );
        assert!(root.exists());
        pause.release.notify_one();
        assert!(work.await.unwrap().is_err(), "the instance is gone");
        delete.await.unwrap().unwrap();
        assert!(!root.exists());
        assert!(!install_left_behind(&app));
    }

    /// A rename that moves the folder, asked for while the pack downloads,
    /// waits for the download. The world is then another save as far as its
    /// identity goes, so the install is refused, and the private folder is
    /// removed from the renamed root.
    #[tokio::test]
    async fn renaming_the_instance_waits_for_the_download() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, _) = world_instance(&app, "renamed-while-downloading").await;
        let root = instance_root(&app, id).await;
        let mut api = api().await;
        let (pause, work) = install_held_after_download(&app, &mut api, id, &identity).await;

        let renaming = app.clone();
        let mut rename = tokio::spawn(async move {
            renaming
                .instance_manager()
                .update_instance(crate::domain::instance::InstanceSettingsUpdate {
                    instance_id: id,
                    name: Some("renamed-later".into()),
                    use_loaded_icon: None,
                    notes: None,
                    version: None,
                    modloader: None,
                    java_override: None,
                    global_java_args: None,
                    extra_java_args: None,
                    memory: None,
                    pre_launch_hook: None,
                    post_exit_hook: None,
                    wrapper_command: None,
                    game_resolution: None,
                    mod_sources: None,
                    modpack_locked: None,
                })
                .await
        });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(150), &mut rename)
                .await
                .is_err(),
            "the folder moved while the pack downloaded"
        );
        assert!(root.exists());
        pause.release.notify_one();
        let error = work.await.unwrap().unwrap_err();
        assert_eq!(
            error.to_string(),
            "Selected world changed during copying; no datapack published"
        );
        rename.await.unwrap().unwrap();
        let renamed = instance_root(&app, id).await;
        assert_ne!(renamed, root);
        assert!(!root.exists());
        assert!(staging_folders(&renamed).is_empty(), "{renamed:?}");
        let world = renamed.join("instance/saves/World");
        assert!(entries(&world.join("datapacks")).is_empty());
        assert!(!install_left_behind(&app));
    }

    /// Once the pack is in the world, failing to record its platform
    /// identity is no failed install: the task succeeds and the list shows
    /// the pack by its file name.
    #[tokio::test]
    async fn a_pack_whose_identity_cannot_be_recorded_is_still_installed() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "unrecorded").await;
        let mut api = api().await;
        let zip = pack_zip("unrecorded");
        let url = api.file("pack.zip", &zip).await;
        // A version that names another project: `record_installed_version`
        // refuses it.
        let mut version = mr_version(
            "release",
            10,
            "release",
            &["datapack"],
            ("pack.zip", &url, &zip),
        );
        version["project_id"] = json!("another");
        modrinth_serving(&mut api, vec![version]).await;
        let mut events = app.invalidation_channel.subscribe();
        let (task, published) = install(&app, &api, request(id, &identity, modrinth(None)))
            .await
            .unwrap();
        assert_eq!(published, world.join("datapacks/pack.zip"));
        assert_eq!(std::fs::read(&published).unwrap(), zip);
        assert!(worlds_invalidated(&mut events, id));
        let listed = app.instance_manager().list_worlds(id).await.unwrap();
        let pack = &listed[0].datapacks[0];
        assert_eq!(pack.filename, "pack.zip");
        assert_eq!(pack.name, None);
        assert!(pack.modrinth.is_none());
        for _ in 0..50 {
            if matches!(
                app.task_manager().get_task_outcome(task).await,
                crate::managers::vtask::TaskOutcome::Succeeded
            ) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("the task did not succeed");
    }

    /// Whether a quit or move would still wait on an install into a world.
    fn install_left_behind(app: &App) -> bool {
        app.runtime_lifecycle
            .relocation_blockers("probe")
            .iter()
            .any(|blocker| blocker.name.contains("data pack into a world"))
    }

    fn worlds_invalidated(
        events: &mut tokio::sync::broadcast::Receiver<crate::api::InvalidationEvent>,
        id: InstanceId,
    ) -> bool {
        let mut seen = false;
        while let Ok(event) = events.try_recv() {
            seen |= event.key == GET_WORLDS.full && event.args == Some((*id).into());
        }
        seen
    }

    #[tokio::test]
    async fn a_modrinth_data_pack_lands_in_the_world_with_its_platform_identity() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "mr-install").await;
        let mut api = api().await;
        let zip = pack_zip("installed");
        let url = api.file("pack.zip", &zip).await;
        modrinth_serving(
            &mut api,
            vec![mr_version(
                "release",
                10,
                "release",
                &["datapack"],
                ("pack.zip", &url, &zip),
            )],
        )
        .await;
        let root = instance_root(&app, id).await;
        let (root_before, _) = snapshot(&root, &world);
        let mut events = app.invalidation_channel.subscribe();
        let (task, published) = install(&app, &api, request(id, &identity, modrinth(None)))
            .await
            .unwrap();
        assert_eq!(published, world.join("datapacks").join("pack.zip"));
        assert_eq!(std::fs::read(&published).unwrap(), zip);
        assert_eq!(
            snapshot(&root, &world).0,
            root_before,
            "no private file left"
        );
        assert!(worlds_invalidated(&mut events, id));

        let rows = wdb::get_world_datapacks_by_instance(&app.db, *id)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            (rows[0].world.as_str(), rows[0].filename.as_str()),
            ("World", "pack.zip")
        );
        let metadata_id = rows[0].metadata_id.clone().unwrap();
        let recorded = metarepo::get_mr_cache_by_metadata(&app.db, &metadata_id)
            .await
            .unwrap()
            .expect("the Modrinth identity is recorded");
        assert_eq!(recorded.version_id, "release");

        // No platform pass has answered: the name comes from the record.
        let listed = app.instance_manager().list_worlds(id).await.unwrap();
        let pack = &listed[0].datapacks[0];
        assert_eq!(pack.name.as_deref(), Some("World Pack"));
        assert_eq!(pack.modrinth.as_ref().unwrap().project_id, "project");
        for _ in 0..50 {
            if matches!(
                app.task_manager().get_task_outcome(task).await,
                crate::managers::vtask::TaskOutcome::Succeeded
            ) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("the task did not succeed");
    }

    #[tokio::test]
    async fn a_curseforge_data_pack_lands_in_the_world_with_its_platform_identity() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "cf-install").await;
        let mut api = api().await;
        let zip = pack_zip("curse");
        let url = api.file("curse.zip", &zip).await;
        let fingerprint = fingerprint(&app, &zip).await;
        let file = cf_file(20, 10, 1, ("curse.zip", &url, &zip), fingerprint);
        api.get(
            &format!("/mods/{CF_PROJECT}"),
            json!({ "data": cf_project(6945) }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files/20"),
            json!({ "data": file }),
        )
        .await;
        install(&app, &api, request(id, &identity, curseforge(Some(20))))
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(world.join("datapacks/curse.zip")).unwrap(),
            zip
        );
        let listed = app.instance_manager().list_worlds(id).await.unwrap();
        let pack = &listed[0].datapacks[0];
        assert_eq!(pack.name.as_deref(), Some("Curse Pack"));
        assert_eq!(pack.curseforge.as_ref().unwrap().file_id, 20);
    }

    /// Every entry under `folder` with its bytes (none for a folder or a
    /// link), to show a refused change touched nothing.
    pub(crate) fn tree(folder: &Path) -> Vec<(String, Option<Vec<u8>>)> {
        let mut found = Vec::new();
        let mut pending = vec![folder.to_path_buf()];
        while let Some(next) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&next) else {
                continue;
            };
            for entry in entries {
                let path = entry.unwrap().path();
                let meta = std::fs::symlink_metadata(&path).unwrap();
                let name = path
                    .strip_prefix(folder)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                if meta.is_dir() {
                    pending.push(path);
                    found.push((name, None));
                } else if meta.is_file() {
                    found.push((name, Some(std::fs::read(&path).unwrap())));
                } else {
                    found.push((name, None));
                }
            }
        }
        found.sort();
        found
    }

    /// A save `name` beside `world`, holding `level.dat`.
    fn add_world(world: &Path, name: &str) -> PathBuf {
        let other = world.with_file_name(name);
        std::fs::create_dir_all(other.join("datapacks")).unwrap();
        std::fs::write(other.join("level.dat"), b"save").unwrap();
        other
    }

    /// `bytes` as `filename` in the `datapacks` folder of `world`.
    pub(crate) fn put_pack(world: &Path, filename: &str, bytes: &[u8]) -> PathBuf {
        let path = world.join("datapacks").join(filename);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        path
    }

    /// The instance's world data pack rows after a scan.
    pub(crate) async fn scanned_rows(app: &App, id: InstanceId) -> Vec<WorldDatapackRow> {
        app.instance_manager()
            .scan_world_datapacks(id)
            .await
            .unwrap();
        let mut rows = wdb::get_world_datapacks_by_instance(&app.db, *id)
            .await
            .unwrap();
        rows.sort_by(|a, b| (&a.world, &a.filename).cmp(&(&b.world, &b.filename)));
        rows
    }

    pub(crate) fn row_of<'a>(
        rows: &'a [WorldDatapackRow],
        world: &str,
        filename: &str,
    ) -> &'a WorldDatapackRow {
        rows.iter()
            .find(|row| row.world == world && row.filename == filename)
            .unwrap_or_else(|| panic!("no row for {world}/{filename} in {rows:?}"))
    }

    async fn delete(app: &App, id: InstanceId, row: &str) -> anyhow::Result<()> {
        app.instance_manager()
            .delete_world_datapack(DeleteWorldDatapack {
                instance_id: *id,
                id: row.to_owned(),
            })
            .await
    }

    async fn row_exists(app: &App, row: &str) -> bool {
        wdb::get_world_datapack_by_id(&app.db, row)
            .await
            .unwrap()
            .is_some()
    }

    /// Two worlds holding the same file: removing it from one leaves the
    /// other's file and row as they were.
    #[tokio::test]
    async fn removing_a_pack_leaves_the_same_file_in_another_world() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, _, world) = world_instance(&app, "remove-one").await;
        let other = add_world(&world, "Other");
        let zip = pack_zip("shared");
        let removed = put_pack(&world, "pack.zip", &zip);
        let kept = put_pack(&other, "pack.zip", &zip);
        let rows = scanned_rows(&app, id).await;
        let (target, other_row) = (
            row_of(&rows, "World", "pack.zip").clone(),
            row_of(&rows, "Other", "pack.zip").clone(),
        );
        let mut events = app.invalidation_channel.subscribe();

        delete(&app, id, &target.id).await.unwrap();

        assert!(!removed.exists());
        assert!(!crate::util::recycle_bin::bin_under_test(&removed).exists());
        assert_eq!(std::fs::read(&kept).unwrap(), zip);
        assert_eq!(scanned_rows(&app, id).await, vec![other_row]);
        assert!(!row_exists(&app, &target.id).await);
        assert!(worlds_invalidated(&mut events, id));
    }

    #[tokio::test]
    async fn a_removed_pack_goes_to_the_recycle_bin_by_default() {
        let app = crate::setup_managers_for_test().await;
        let (id, _, world) = world_instance(&app, "remove-binned").await;
        let zip = pack_zip("binned");
        let path = put_pack(&world, "pack.zip", &zip);
        let rows = scanned_rows(&app, id).await;

        delete(&app, id, &rows[0].id).await.unwrap();

        assert!(!path.exists());
        assert_eq!(
            std::fs::read(crate::util::recycle_bin::bin_under_test(&path).join("pack.zip"))
                .unwrap(),
            zip
        );
        assert!(scanned_rows(&app, id).await.is_empty());
    }

    #[tokio::test]
    async fn a_folder_pack_is_removed() {
        let app = crate::setup_managers_for_test().await;
        let (id, _, world) = world_instance(&app, "remove-folder").await;
        let folder = world.join("datapacks/unpacked");
        std::fs::create_dir_all(folder.join("data")).unwrap();
        std::fs::write(
            folder.join("pack.mcmeta"),
            br#"{"pack":{"pack_format":15}}"#,
        )
        .unwrap();
        std::fs::write(folder.join("data/function.mcfunction"), b"say hi").unwrap();
        let kept = put_pack(&world, "kept.zip", &pack_zip("kept"));
        let rows = scanned_rows(&app, id).await;
        let target = row_of(&rows, "World", "unpacked");
        assert!(target.is_folder);

        delete(&app, id, &target.id).await.unwrap();

        assert!(!folder.exists());
        let binned = crate::util::recycle_bin::bin_under_test(&folder).join("unpacked");
        assert_eq!(
            std::fs::read(binned.join("data/function.mcfunction")).unwrap(),
            b"say hi"
        );
        assert!(kept.exists());
        let rows = scanned_rows(&app, id).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].filename, "kept.zip");
    }

    #[tokio::test]
    async fn a_pack_already_gone_loses_its_row() {
        let app = crate::setup_managers_for_test().await;
        let (id, _, world) = world_instance(&app, "remove-gone").await;
        let path = put_pack(&world, "pack.zip", &pack_zip("gone"));
        let rows = scanned_rows(&app, id).await;
        std::fs::remove_file(&path).unwrap();

        delete(&app, id, &rows[0].id).await.unwrap();

        assert!(!row_exists(&app, &rows[0].id).await);
    }

    /// A row whose world or file name is no single name inside the save's
    /// `datapacks` folder is refused, and nothing anywhere in the saves is
    /// touched.
    #[tokio::test]
    async fn a_row_naming_a_path_outside_the_datapacks_folder_is_refused() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, _, world) = world_instance(&app, "remove-escape").await;
        put_pack(&world, "pack.zip", &pack_zip("kept"));
        std::fs::create_dir_all(world.join("datapacks/sub")).unwrap();
        std::fs::write(world.join("datapacks/sub/pack.zip"), pack_zip("nested")).unwrap();
        let saves = world.parent().unwrap().to_path_buf();
        let before = tree(&saves);
        for (world_name, filename) in [
            ("World", "../level.dat"),
            ("World", "sub/pack.zip"),
            ("World", ".."),
            ("World", ""),
            ("..", "World"),
            ("World/datapacks", "pack.zip"),
            ("", "pack.zip"),
        ] {
            wdb::upsert_world_datapack(
                &app.db,
                *id,
                world_name,
                filename,
                0,
                filename == "..",
                None,
                None,
                carbon_repos::dbtypes::DbDateTime(chrono::Utc::now().fixed_offset()),
            )
            .await
            .unwrap();
            let row = wdb::get_world_datapacks_by_instance(&app.db, *id)
                .await
                .unwrap()
                .into_iter()
                .find(|row| row.world == world_name && row.filename == filename)
                .unwrap();
            let error = delete(&app, id, &row.id).await.unwrap_err().to_string();
            assert_eq!(error, "invalid addon filename", "{world_name}/{filename}");
            assert_eq!(tree(&saves), before, "{world_name}/{filename}");
            assert!(row_exists(&app, &row.id).await);
        }
    }

    #[tokio::test]
    async fn a_row_of_another_instance_is_refused() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, _, _) = world_instance(&app, "remove-asking").await;
        let (owner, _, world) = world_instance(&app, "remove-owning").await;
        let path = put_pack(&world, "pack.zip", &pack_zip("owned"));
        let rows = scanned_rows(&app, owner).await;

        let error = delete(&app, id, &rows[0].id).await.unwrap_err();

        assert_eq!(error.downcast_ref(), Some(&WorldRefusal::PackGone));
        assert!(path.exists());
        assert!(row_exists(&app, &rows[0].id).await);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_linked_datapacks_folder_loses_only_the_entry_inside() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, _, world) = world_instance(&app, "remove-linked").await;
        let shared = tempfile::tempdir().unwrap();
        std::fs::write(shared.path().join("a.zip"), pack_zip("a")).unwrap();
        std::fs::write(shared.path().join("b.zip"), pack_zip("b")).unwrap();
        std::os::unix::fs::symlink(shared.path(), world.join("datapacks")).unwrap();
        let rows = scanned_rows(&app, id).await;

        delete(&app, id, &row_of(&rows, "World", "a.zip").id)
            .await
            .unwrap();

        assert!(!shared.path().join("a.zip").exists());
        assert!(shared.path().join("b.zip").exists());
        let link = std::fs::symlink_metadata(world.join("datapacks")).unwrap();
        assert!(link.file_type().is_symlink());
        let rows = scanned_rows(&app, id).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].filename, "b.zip");
    }

    /// A zip row whose name is now a folder (or the other way round) is
    /// refused: it is not what the user chose to remove.
    #[tokio::test]
    async fn an_entry_that_is_no_longer_the_listed_kind_is_refused() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, _, world) = world_instance(&app, "remove-kind").await;
        let path = put_pack(&world, "pack.zip", &pack_zip("zip"));
        let rows = scanned_rows(&app, id).await;
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("level.dat"), b"not a pack").unwrap();
        let before = tree(&world);

        let error = delete(&app, id, &rows[0].id).await.unwrap_err();

        assert_eq!(error.downcast_ref(), Some(&WorldRefusal::PackChanged));
        assert_eq!(tree(&world), before);
    }

    #[tokio::test]
    async fn a_delete_the_instance_cannot_take_now_changes_nothing() {
        use crate::managers::instance::run::RunningInstance;
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, _, world) = world_instance(&app, "remove-running").await;
        put_pack(&world, "pack.zip", &pack_zip("running"));
        let rows = scanned_rows(&app, id).await;
        let before = tree(&world);
        let set_state = |state: LaunchState| {
            let app = app.clone();
            async move {
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
        };
        set_state(LaunchState::Running(RunningInstance::adopted(
            u32::MAX,
            1,
            chrono::Utc::now(),
            chrono::Utc::now(),
        )))
        .await;
        let error = delete(&app, id, &rows[0].id).await.unwrap_err();
        assert!(error.to_string().contains("running"), "{error}");
        set_state(LaunchState::Inactive { failed_task: None }).await;
        assert_eq!(tree(&world), before);
        assert!(row_exists(&app, &rows[0].id).await);

        // A world that is no save any more.
        std::fs::remove_file(world.join("level.dat")).unwrap();
        let before = tree(&world);
        delete(&app, id, &rows[0].id)
            .await
            .expect_err("a folder without level.dat is no save");
        assert_eq!(tree(&world), before);
    }

    /// An API serving one Modrinth data pack version, `filename` holding
    /// `bytes`.
    async fn serving(api: &mut Api, filename: &str, bytes: &[u8]) {
        let url = api.file(filename, bytes).await;
        modrinth_serving(
            api,
            vec![mr_version(
                "release",
                10,
                "release",
                &["datapack"],
                (filename, &url, bytes),
            )],
        )
        .await;
    }

    fn replacing(id: InstanceId, identity: &str, world: &str, row: &str) -> InstallWorldDatapack {
        InstallWorldDatapack {
            world: world.into(),
            replaces: Some(row.into()),
            ..request(id, identity, modrinth(None))
        }
    }

    /// An update under the same name leaves the new file alone in the
    /// world, and the same file in another world with its row.
    #[tokio::test]
    async fn an_update_replaces_the_pack_under_the_same_name() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "replace-same").await;
        let other = add_world(&world, "Other");
        let old = pack_zip("old");
        put_pack(&world, "pack.zip", &old);
        let other_pack = put_pack(&other, "pack.zip", &old);
        let rows = scanned_rows(&app, id).await;
        let other_row = row_of(&rows, "Other", "pack.zip").clone();
        let mut api = api().await;
        let new = pack_zip("new");
        serving(&mut api, "pack.zip", &new).await;
        let root = instance_root(&app, id).await;

        install(
            &app,
            &api,
            replacing(
                id,
                &identity,
                "World",
                &row_of(&rows, "World", "pack.zip").id,
            ),
        )
        .await
        .unwrap();

        assert_eq!(
            entries(&world.join("datapacks")),
            vec![("pack.zip".to_owned(), new)]
        );
        assert_eq!(std::fs::read(&other_pack).unwrap(), old);
        assert!(staging_folders(&root).is_empty());
        let rows = scanned_rows(&app, id).await;
        assert_eq!(rows.len(), 2);
        assert_eq!(row_of(&rows, "Other", "pack.zip"), &other_row);
    }

    #[tokio::test]
    async fn a_replace_by_another_file_name_leaves_only_the_new_one() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, identity, world) = world_instance(&app, "replace-renamed").await;
        put_pack(&world, "pack-1.zip", &pack_zip("one"));
        let rows = scanned_rows(&app, id).await;
        let mut api = api().await;
        let new = pack_zip("two");
        serving(&mut api, "pack-2.zip", &new).await;
        let root = instance_root(&app, id).await;

        install(&app, &api, replacing(id, &identity, "World", &rows[0].id))
            .await
            .unwrap();

        assert_eq!(
            entries(&world.join("datapacks")),
            vec![("pack-2.zip".to_owned(), new)]
        );
        assert!(staging_folders(&root).is_empty());
        let rows = scanned_rows(&app, id).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].filename, "pack-2.zip");
    }

    #[tokio::test]
    async fn a_replace_whose_new_name_holds_a_third_file_is_refused() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "replace-taken").await;
        put_pack(&world, "pack-1.zip", &pack_zip("one"));
        put_pack(&world, "pack-2.zip", b"the user's own");
        let rows = scanned_rows(&app, id).await;
        let mut api = api().await;
        serving(&mut api, "pack-2.zip", &pack_zip("two")).await;
        let root = instance_root(&app, id).await;
        let before = snapshot(&root, &world);

        let error = install(
            &app,
            &api,
            replacing(
                id,
                &identity,
                "World",
                &row_of(&rows, "World", "pack-1.zip").id,
            ),
        )
        .await
        .unwrap_err();

        assert_eq!(
            error.downcast_ref(),
            Some(&WorldRefusal::NameTaken {
                filename: "pack-2.zip".into()
            })
        );
        assert_eq!(snapshot(&root, &world), before);
    }

    #[tokio::test]
    async fn a_replace_across_two_worlds_or_of_a_folder_is_refused() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "replace-refused").await;
        let other = add_world(&world, "Other");
        put_pack(&other, "pack.zip", &pack_zip("other"));
        std::fs::create_dir_all(world.join("datapacks/unpacked")).unwrap();
        std::fs::write(
            world.join("datapacks/unpacked/pack.mcmeta"),
            br#"{"pack":{"pack_format":15}}"#,
        )
        .unwrap();
        let rows = scanned_rows(&app, id).await;
        let mut api = api().await;
        serving(&mut api, "pack.zip", &pack_zip("new")).await;
        let saves = world.parent().unwrap().to_path_buf();
        let before = tree(&saves);

        let error = install(
            &app,
            &api,
            replacing(
                id,
                &identity,
                "World",
                &row_of(&rows, "Other", "pack.zip").id,
            ),
        )
        .await
        .unwrap_err();
        assert_eq!(error.downcast_ref(), Some(&WorldRefusal::OtherWorld));
        assert_eq!(tree(&saves), before);

        let error = install(
            &app,
            &api,
            replacing(
                id,
                &identity,
                "World",
                &row_of(&rows, "World", "unpacked").id,
            ),
        )
        .await
        .unwrap_err();
        assert_eq!(error.downcast_ref(), Some(&WorldRefusal::FolderNotReplaced));
        assert_eq!(tree(&saves), before);
    }

    fn modified(path: &Path) -> std::time::SystemTime {
        std::fs::symlink_metadata(path).unwrap().modified().unwrap()
    }

    fn set_modified(path: &Path) -> std::time::SystemTime {
        let time =
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000);
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(time)
            .unwrap();
        time
    }

    /// The new file cannot be published: the old one is back under its
    /// name with its bytes and time, and keeps its row.
    #[tokio::test]
    async fn a_failed_publish_puts_the_old_pack_back() {
        use crate::util::publish::fault::{IN_USE, Step, inject};
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "replace-put-back").await;
        let old = pack_zip("old");
        let path = put_pack(&world, "pack.zip", &old);
        let time = set_modified(&path);
        let rows = scanned_rows(&app, id).await;
        let mut api = api().await;
        serving(&mut api, "pack.zip", &pack_zip("new")).await;
        let root = instance_root(&app, id).await;
        let before = snapshot(&root, &world);
        inject(Step::NoReplace, &path, IN_USE);

        install(&app, &api, replacing(id, &identity, "World", &rows[0].id))
            .await
            .expect_err("the publication fails");
        assert!(
            !crate::util::publish::fault::pending(Step::NoReplace, &path),
            "the publication was tried"
        );

        assert_eq!(snapshot(&root, &world), before);
        assert_eq!(modified(&path), time);
        // The same row with the same metadata. Its stamp moves on: a
        // rename changes the inode change time, so the next scan reads the
        // file once more.
        let after = scanned_rows(&app, id).await;
        let identity = |rows: &[WorldDatapackRow]| {
            rows.iter()
                .map(|row| {
                    (
                        row.id.clone(),
                        row.filename.clone(),
                        row.metadata_id.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(identity(&after), identity(&rows));
    }

    /// The user puts their own build under the replaced pack's name while
    /// the download runs: under the gate it is no longer the file the row
    /// recorded, so it is neither set aside nor removed.
    #[tokio::test]
    async fn a_file_put_under_the_replaced_name_during_the_download_is_kept() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "replace-swapped").await;
        let path = put_pack(&world, "pack.zip", &pack_zip("old"));
        let rows = scanned_rows(&app, id).await;
        let mut api = api().await;
        serving(&mut api, "pack.zip", &pack_zip("new")).await;
        let root = instance_root(&app, id).await;
        let pause = app
            .instance_manager()
            .test_hooks
            .arm("world_datapack_downloaded", id);
        let (_, work) = app
            .instance_manager()
            .install_world_datapack_from(
                &api.curseforge,
                &api.modrinth,
                replacing(id, &identity, "World", &rows[0].id),
            )
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), pause.entered.notified())
            .await
            .unwrap();
        std::fs::write(&path, b"the user's own build").unwrap();
        pause.release.notify_one();

        let error = work.await.unwrap().unwrap_err();

        assert_eq!(error.downcast_ref(), Some(&WorldRefusal::PackChanged));
        assert_eq!(
            entries(&world.join("datapacks")),
            vec![("pack.zip".to_owned(), b"the user's own build".to_vec())]
        );
        assert!(staging_folders(&root).is_empty());
    }

    /// The Recycle Bin refuses the old pack once the new one is in place:
    /// the new one is withdrawn and the old one is back, as before.
    #[tokio::test]
    async fn a_recycle_bin_that_refuses_the_old_pack_undoes_the_replace() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "replace-bin-refuses").await;
        let old = pack_zip("old");
        put_pack(&world, "pack.zip", &old);
        let rows = scanned_rows(&app, id).await;
        let mut api = api().await;
        serving(&mut api, "pack.zip", &pack_zip("new")).await;
        let root = instance_root(&app, id).await;
        let before = snapshot(&root, &world);
        let pause = app
            .instance_manager()
            .test_hooks
            .arm("world_datapack_downloaded", id);
        let (_, work) = app
            .instance_manager()
            .install_world_datapack_from(
                &api.curseforge,
                &api.modrinth,
                replacing(id, &identity, "World", &rows[0].id),
            )
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), pause.entered.notified())
            .await
            .unwrap();
        // A file where the Recycle Bin under test would put the old pack.
        let staging = staging_folders(&root);
        std::fs::write(
            crate::util::recycle_bin::bin_under_test(
                &root.join(&staging[0]).join(REPLACED).join("pack.zip"),
            ),
            b"",
        )
        .unwrap();
        pause.release.notify_one();

        let error = work.await.unwrap().unwrap_err().to_string();

        assert!(error.contains("Nothing was deleted"), "{error}");
        assert_eq!(snapshot(&root, &world), before);
    }

    /// When the old pack cannot go back either, the error says where it
    /// is, and it is still there.
    #[tokio::test]
    async fn a_pack_that_cannot_be_put_back_is_named_where_it_waits() {
        use crate::util::publish::fault::{IN_USE, Step, inject};
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "replace-stuck").await;
        let old = pack_zip("old");
        let path = put_pack(&world, "pack.zip", &old);
        let rows = scanned_rows(&app, id).await;
        let mut api = api().await;
        serving(&mut api, "pack.zip", &pack_zip("new")).await;
        let root = instance_root(&app, id).await;
        inject(Step::NoReplace, &path, IN_USE);
        inject(Step::NoReplace, &path, IN_USE);

        let error = install(&app, &api, replacing(id, &identity, "World", &rows[0].id))
            .await
            .unwrap_err()
            .to_string();

        let staging = staging_folders(&root);
        assert_eq!(staging.len(), 1, "the waiting pack's folder is kept");
        let waiting = root.join(&staging[0]).join("replaced").join("pack.zip");
        assert!(error.contains(&path.display().to_string()), "{error}");
        assert!(error.contains(&waiting.display().to_string()), "{error}");
        assert_eq!(std::fs::read(&waiting).unwrap(), old);
        assert!(!path.exists());
    }

    async fn quit(app: &App) {
        app.runtime_lifecycle
            .prepare_shutdown(
                "world-datapack-quit",
                crate::managers::runtime_lifecycle::ShutdownPurpose::Quit,
                async { Ok(()) },
            )
            .unwrap();
    }

    async fn succeeded(app: &App, task: VisualTaskId) -> bool {
        for _ in 0..50 {
            if matches!(
                app.task_manager().get_task_outcome(task).await,
                crate::managers::vtask::TaskOutcome::Succeeded
            ) {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        false
    }

    /// A quit that comes once the old pack is taken away waits for the
    /// replace, which ends as it would have: the new pack in place, the old
    /// one removed, and the task a success.
    #[tokio::test]
    async fn a_quit_after_the_old_pack_is_taken_away_waits_for_the_replace() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, identity, world) = world_instance(&app, "replace-quit").await;
        put_pack(&world, "pack-1.zip", &pack_zip("old"));
        let rows = scanned_rows(&app, id).await;
        let mut api = api().await;
        let new = pack_zip("new");
        serving(&mut api, "pack-2.zip", &new).await;
        let root = instance_root(&app, id).await;
        let pause = app
            .instance_manager()
            .test_hooks
            .arm("world_datapack_taken_away", id);
        let (task, work) = app
            .instance_manager()
            .install_world_datapack_from(
                &api.curseforge,
                &api.modrinth,
                replacing(id, &identity, "World", &rows[0].id),
            )
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), pause.entered.notified())
            .await
            .unwrap();
        quit(&app).await;
        assert!(install_left_behind(&app), "the quit waits for the replace");
        pause.release.notify_one();

        let published = work.await.unwrap().unwrap();

        assert_eq!(published, world.join("datapacks/pack-2.zip"));
        assert_eq!(
            entries(&world.join("datapacks")),
            vec![("pack-2.zip".to_owned(), new)]
        );
        assert!(staging_folders(&root).is_empty());
        assert!(succeeded(&app, task).await);
        assert!(!install_left_behind(&app));
    }

    /// A quit while the pack is still downloading stops the install, and the
    /// world is as it was.
    #[tokio::test]
    async fn a_quit_before_the_gate_stops_the_replace() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, world) = world_instance(&app, "replace-quit-early").await;
        put_pack(&world, "pack.zip", &pack_zip("old"));
        let rows = scanned_rows(&app, id).await;
        let mut api = api().await;
        serving(&mut api, "pack.zip", &pack_zip("new")).await;
        let root = instance_root(&app, id).await;
        let before = snapshot(&root, &world);
        let pause = app
            .instance_manager()
            .test_hooks
            .arm("world_datapack_downloaded", id);
        let (_, work) = app
            .instance_manager()
            .install_world_datapack_from(
                &api.curseforge,
                &api.modrinth,
                replacing(id, &identity, "World", &rows[0].id),
            )
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), pause.entered.notified())
            .await
            .unwrap();
        quit(&app).await;
        pause.release.notify_one();

        let error = work.await.unwrap().unwrap_err();

        assert_eq!(error.to_string(), CLOSING);
        assert_eq!(snapshot(&root, &world), before);
    }

    /// Dropped unsettled while another file holds its name, the old pack
    /// stays where it waits, the private folder with it, at the path the
    /// warning names.
    #[tokio::test]
    async fn a_pack_dropped_where_it_cannot_go_back_keeps_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("saves/World/datapacks/pack.zip");
        std::fs::create_dir_all(pack.parent().unwrap()).unwrap();
        std::fs::write(&pack, b"old").unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        let staging = Staging::new(&root).unwrap();
        let taken = take_away(
            &pack,
            &staging.path().join(REPLACED),
            Removal::Permanent,
            staging.keeper(),
        )
        .await
        .unwrap();
        let waiting = taken.settlement.lock().unwrap().entries[0].aside.clone();
        std::fs::write(&pack, b"another file").unwrap();

        drop(taken);
        drop(staging);

        assert_eq!(std::fs::read(&waiting).unwrap(), b"old");
        assert_eq!(std::fs::read(&pack).unwrap(), b"another file");
    }

    /// Dropped once its replacement is in place, under the same name or
    /// another, the old pack is back and the replacement gone: one build
    /// stays.
    #[tokio::test]
    async fn a_pack_dropped_after_its_replacement_is_published_comes_back_alone() {
        for name in ["pack.zip", "pack-2.zip"] {
            let (_dir, pack, aside) = take_away_fixture(b"old");
            let taken = take_away(&pack, &aside, Removal::Permanent, Default::default())
                .await
                .unwrap();
            let folder = pack.parent().unwrap().to_path_buf();
            let mut file = crate::util::publish::temp_in(&folder).unwrap();
            std::io::Write::write_all(&mut file, b"new").unwrap();
            taken.publish(file, &folder.join(name)).await.unwrap();
            assert!(folder.join(name).exists());

            drop(taken);

            assert_eq!(
                entries(&folder),
                vec![("pack.zip".to_owned(), b"old".to_vec())],
                "{name}"
            );
        }
    }

    /// A world and a private folder, two folders deep each, as an instance
    /// root holds them.
    fn take_away_fixture(bytes: &[u8]) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("saves/World/datapacks/pack.zip");
        std::fs::create_dir_all(pack.parent().unwrap()).unwrap();
        std::fs::write(&pack, bytes).unwrap();
        let aside = dir.path().join("root/.tmpstaging/replaced");
        (dir, pack, aside)
    }

    #[tokio::test]
    async fn a_pack_taken_away_comes_back_with_its_bytes_and_time() {
        let (_dir, pack, aside) = take_away_fixture(b"old");
        let time = set_modified(&pack);

        let taken = take_away(&pack, &aside, Removal::Permanent, Default::default())
            .await
            .unwrap();
        assert!(!pack.exists());
        assert!(entries(pack.parent().unwrap()).is_empty());
        assert_eq!(std::fs::read(aside.join("pack.zip")).unwrap(), b"old");
        taken.put_back().await.unwrap();

        assert_eq!(std::fs::read(&pack).unwrap(), b"old");
        assert_eq!(modified(&pack), time);
        assert!(!aside.join("pack.zip").exists());
    }

    #[tokio::test]
    async fn a_pack_taken_away_and_dropped_comes_back() {
        let (_dir, pack, aside) = take_away_fixture(b"old");
        drop(
            take_away(&pack, &aside, Removal::Permanent, Default::default())
                .await
                .unwrap(),
        );
        assert_eq!(std::fs::read(&pack).unwrap(), b"old");
    }

    /// What awaits the set-aside is dropped right after the rename, before
    /// the set-aside returned (a request whose window closed): the pack is
    /// back under its name, not erased with the private folder it waited in.
    #[tokio::test]
    async fn a_set_aside_whose_caller_is_dropped_after_the_rename_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("saves/World/datapacks/pack.zip");
        std::fs::create_dir_all(pack.parent().unwrap()).unwrap();
        std::fs::write(&pack, b"old").unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        let staging = Staging::new(&root).unwrap();
        let (entered, release) = set_aside_pause::arm(&pack);
        let caller = {
            let (pack, aside, keep) = (
                pack.clone(),
                staging.path().join(REPLACED),
                staging.keeper(),
            );
            tokio::spawn(async move {
                take_away(&pack, &aside, Removal::Permanent, keep)
                    .await
                    .map(drop)
            })
        };
        tokio::task::spawn_blocking(move || entered.recv().unwrap())
            .await
            .unwrap();
        assert!(!pack.exists(), "the pack was moved aside");

        caller.abort();
        release.send(()).unwrap();

        assert!(caller.await.unwrap_err().is_cancelled());
        drop(staging);
        assert_eq!(std::fs::read(&pack).unwrap(), b"old");
        assert!(std::fs::read_dir(&root).unwrap().next().is_none());
    }

    #[tokio::test]
    async fn a_pack_sent_on_follows_the_recycle_bin_setting() {
        use crate::util::recycle_bin::bin_under_test;
        let (_dir, pack, aside) = take_away_fixture(b"binned");
        let mut taken = take_away(&pack, &aside, Removal::RecycleBin, Default::default())
            .await
            .unwrap();
        taken.send_on().await.unwrap();
        let waited = aside.join("pack.zip");
        assert!(!waited.exists() && !pack.exists());
        assert_eq!(
            std::fs::read(bin_under_test(&waited).join("pack.zip")).unwrap(),
            b"binned"
        );

        let (_dir, pack, aside) = take_away_fixture(b"erased");
        let mut taken = take_away(&pack, &aside, Removal::Permanent, Default::default())
            .await
            .unwrap();
        taken.send_on().await.unwrap();
        let waited = aside.join("pack.zip");
        assert!(!waited.exists() && !pack.exists());
        assert!(!bin_under_test(&waited).exists());
    }

    /// With the private folder on another drive the pack waits beside
    /// itself, under a name the game does not load and the scanner does not
    /// list.
    #[tokio::test]
    async fn a_pack_taken_away_across_drives_waits_beside_itself() {
        use crate::util::publish::fault::{CROSS_DEVICE, Step, inject};
        let (_dir, pack, aside) = take_away_fixture(b"old");
        inject(Step::NoReplace, &aside.join("pack.zip"), CROSS_DEVICE);

        let mut taken = take_away(&pack, &aside, Removal::Permanent, Default::default())
            .await
            .unwrap();

        let beside = entries(pack.parent().unwrap());
        assert_eq!(beside.len(), 1);
        assert!(
            !beside[0].0.to_ascii_lowercase().ends_with(".zip"),
            "{beside:?}"
        );
        assert_eq!(beside[0].1, b"old");
        assert!(!aside.join("pack.zip").exists());
        taken.send_on().await.unwrap();
        assert!(entries(pack.parent().unwrap()).is_empty());

        let (_dir, pack, aside) = take_away_fixture(b"old");
        inject(Step::NoReplace, &aside.join("pack.zip"), CROSS_DEVICE);
        let taken = take_away(&pack, &aside, Removal::Permanent, Default::default())
            .await
            .unwrap();
        taken.put_back().await.unwrap();
        assert_eq!(
            entries(pack.parent().unwrap()),
            vec![("pack.zip".to_owned(), b"old".to_vec())]
        );
    }

    fn renamed(id: InstanceId, name: &str) -> crate::domain::instance::InstanceSettingsUpdate {
        crate::domain::instance::InstanceSettingsUpdate {
            instance_id: id,
            name: Some(name.into()),
            use_loaded_icon: None,
            notes: None,
            version: None,
            modloader: None,
            java_override: None,
            global_java_args: None,
            extra_java_args: None,
            memory: None,
            pre_launch_hook: None,
            post_exit_hook: None,
            wrapper_command: None,
            game_resolution: None,
            mod_sources: None,
            modpack_locked: None,
        }
    }

    /// A rename the install waited for moves the root; a quit that comes
    /// once the install holds the gate stops it, and its private folder is
    /// removed from the renamed root.
    #[tokio::test]
    async fn a_quit_once_the_gate_is_taken_after_a_rename_leaves_no_download_behind() {
        let app = crate::setup_managers_for_test().await;
        let (id, identity, _) = world_instance(&app, "renamed-then-quit").await;
        let root = instance_root(&app, id).await;
        let mut api = api().await;
        let (pause, work) = install_held_after_download(&app, &mut api, id, &identity).await;
        let gate = app
            .instance_manager()
            .test_hooks
            .arm("world_datapack_gate_taken", id);
        let renaming = app.clone();
        let mut rename = tokio::spawn(async move {
            renaming
                .instance_manager()
                .update_instance(renamed(id, "renamed-then-quit-later"))
                .await
        });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(150), &mut rename)
                .await
                .is_err(),
            "the rename waits for the download"
        );
        pause.release.notify_one();
        tokio::time::timeout(std::time::Duration::from_secs(10), gate.entered.notified())
            .await
            .unwrap();
        rename.await.unwrap().unwrap();
        let moved = instance_root(&app, id).await;
        assert_ne!(moved, root);
        assert_eq!(staging_folders(&moved).len(), 1, "the download moved along");
        quit(&app).await;
        gate.release.notify_one();

        let error = work.await.unwrap().unwrap_err();

        assert_eq!(error.to_string(), CLOSING);
        assert!(staging_folders(&moved).is_empty(), "{moved:?}");
        assert!(!install_left_behind(&app));
    }

    /// The replacement cannot be withdrawn: the old pack stays where it
    /// waits, its folder kept, and the error names the replacement, the
    /// pack's own path and where it waits. A drop does the same.
    #[tokio::test]
    async fn a_replacement_that_cannot_be_withdrawn_keeps_the_old_pack_aside() {
        for dropped in [false, true] {
            let (_dir, pack, aside) = take_away_fixture(b"old");
            let keep = Arc::new(AtomicBool::new(false));
            let taken = take_away(&pack, &aside, Removal::Permanent, keep.clone())
                .await
                .unwrap();
            let folder = pack.parent().unwrap().to_path_buf();
            let replacement = folder.join("pack-2.zip");
            let mut file = crate::util::publish::temp_in(&folder).unwrap();
            std::io::Write::write_all(&mut file, b"new").unwrap();
            taken.publish(file, &replacement).await.unwrap();
            // A folder in its place cannot be removed as a file.
            std::fs::remove_file(&replacement).unwrap();
            std::fs::create_dir(&replacement).unwrap();
            std::fs::write(replacement.join("inside"), b"x").unwrap();
            let waiting = aside.join("pack.zip");

            if dropped {
                drop(taken);
            } else {
                let error = taken.put_back().await.unwrap_err().to_string();
                for path in [&replacement, &pack, &waiting] {
                    assert!(error.contains(&path.display().to_string()), "{error}");
                }
            }

            assert!(!pack.exists(), "dropped: {dropped}");
            assert_eq!(std::fs::read(&waiting).unwrap(), b"old");
            assert!(keep.load(Ordering::SeqCst));
        }
    }
}
