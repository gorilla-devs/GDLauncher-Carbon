//! Installing a data pack from CurseForge or Modrinth straight into one world
//! of an instance. Only `saves/<world>/datapacks` and a private download in
//! the instance root are written; the instance's own addon folders, the
//! installer and the addon scanner are not involved.
use super::{
    InstanceId, InstanceManager, InstanceRootGuard, InvalidInstanceIdError,
    datapacks::{NAME_TAKEN, WORLD_CHANGED, world_identity},
    installer::{
        dependency::{LoaderFit, best_candidate},
        publication,
    },
    mods::validate_addon_basename,
    run::LaunchState,
    world_datapacks::world_datapacks_folder,
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
use serde::Deserialize;
use std::path::{Path, PathBuf};
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

fn no_data_pack(game: &str) -> anyhow::Error {
    anyhow!("This project has no data pack for Minecraft {game}")
}

/// The file of a Modrinth version that goes into a world: its primary file
/// (the first when none is flagged), when the version is tagged `datapack`,
/// lists `game` and that file is a zip. The game loads nothing else from a
/// world's `datapacks` folder.
fn modrinth_world_file<'a>(version: &'a Version, game: &str) -> Option<&'a VersionFile> {
    if !version.loaders.iter().any(|loader| loader == "datapack")
        || !version.game_versions.iter().any(|listed| listed == game)
    {
        return None;
    }
    version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .filter(|file| file.filename.to_ascii_lowercase().ends_with(".zip"))
}

/// Whether a CurseForge file of `project` goes into a world: the project is a
/// data pack one, and the file is a zip that lists `game`.
fn is_curseforge_world_file(
    project: &curseforge::Mod,
    file: &curseforge::File,
    game: &str,
) -> bool {
    project.class_id == Some(ClassId::Datapacks)
        && file.file_name.to_ascii_lowercase().ends_with(".zip")
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
    let version = match version_id {
        Some(id) => {
            let version = modrinth.get_version(VersionID(id.to_owned())).await?;
            if version.project_id != project.id {
                bail!(
                    "Modrinth version {} is not of project {}",
                    version.id,
                    project.id
                );
            }
            version
        }
        None => latest_world_datapack_modrinth(modrinth, &project, game).await?,
    };
    let file = modrinth_world_file(&version, game)
        .cloned()
        .ok_or_else(|| no_data_pack(game))?;
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
            curseforge
                .get_mod_file(ModFileParameters {
                    mod_id: project.id,
                    file_id: id.try_into()?,
                })
                .await?
                .data
        }
        None => latest_world_datapack_curseforge(curseforge, &project, game).await?,
    };
    if file.mod_id != project.id || !is_curseforge_world_file(&project, &file, game) {
        return Err(no_data_pack(game));
    }
    let url = file
        .download_url
        .clone()
        .context("mod cannot be downloaded without privileged api key")?;
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
    async fn install_world_datapack_from(
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
                let work = app.instance_manager().finish_world_datapack_install(
                    id, &task, &cancel, build, staging, root_hold, request,
                );
                let result = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => Err(anyhow!(
                        "Data pack install stopped because the launcher is closing"
                    )),
                    result = work => result,
                };
                match &result {
                    Ok(_) => task.complete(),
                    Err(error) => task.clone().fail(anyhow!("{error:#}")).await,
                }
                result
            })
        });
        Ok((task_id, handle))
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
        if request.replaces.is_some() {
            bail!("Replacing a data pack in a world is not supported yet");
        }
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
        let path = publication::current_path(&self.app, id).await?;
        let root = path.get_root();
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let world = saves.join(&request.world);
        if world_identity(id, &root, &world).await? != request.identity {
            bail!(WORLD_CHANGED);
        }
        let folder = world_datapacks_folder(&saves, &request.world);
        if publication::followed_folder(&folder).await?
            && publication::selected_file(&folder, &build.filename)
                .await?
                .is_some()
        {
            bail!(NAME_TAKEN);
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
    async fn ensure_world_datapacks_may_change(self, id: InstanceId) -> anyhow::Result<()> {
        self.ensure_modpack_not_locked(id).await?;
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
            bail!(
                "This instance is running or being installed. Close the game or wait for it to finish, then try again."
            );
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
        mut staging: Staging,
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
        let (result, subtask) = tokio::join!(transfer, progress);
        result.with_context(|| format!("Failed to download data pack {}", build.filename))?;
        subtask.complete_download();

        #[cfg(test)]
        self.test_hooks.pause("world_datapack_downloaded", id).await;
        // Given back before the gate is taken (the gate comes first). The
        // downloader is done with the folder: a delete that goes first
        // removes it with the instance, a rename moves it with the root,
        // and both are seen under the gate below.
        drop(root_hold);
        let destination = {
            let _operation = self.lock_instance(id).await;
            // Removed from wherever the root is now, also when refused.
            if let Ok(path) = publication::current_path(&self.app, id).await {
                staging.root = crate::util::publish::long_path(&path.get_root());
            }
            self.ensure_world_datapacks_may_change(id).await?;
            let path = publication::current_path(&self.app, id).await?;
            let root = path.get_root();
            let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
            let world = saves.join(&request.world);
            // Before the folder is made: never in a save put in its place.
            // A moved root also gives the save another identity.
            if world_identity(id, &root, &world).await? != request.identity {
                bail!("Selected world changed during copying; no datapack published");
            }
            let artifact = staging.path().join(ARTIFACT);
            let temporary = tempfile::NamedTempFile::from_parts(
                std::fs::File::open(&artifact)
                    .with_context(|| format!("The download of {} is gone", build.filename))?,
                tempfile::TempPath::from_path(&artifact),
            );
            let folder = world_datapacks_folder(&saves, &request.world);
            if !publication::followed_folder(&folder).await? {
                tokio::fs::create_dir(&folder).await?;
            }
            let destination = self
                .publish_into_world(
                    id,
                    &root,
                    temporary,
                    &world,
                    &request.identity,
                    &build.filename,
                )
                .await?;
            // Nothing of the install is left in the root once the gate goes.
            drop(staging);
            destination
        };
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
                "Instance {id}: could not scan the data packs in its worlds after an install"
            ),
        }
        // Also when the scan saw no change: the recorded identity is new.
        self.app.invalidate(GET_WORLDS, Some((*id).into()));
        self.app.invalidate(INSTANCE_MODS, Some((*id).into()));
        Ok(destination)
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

/// The name of the download inside the install's private folder.
const ARTIFACT: &str = "artifact";

/// An install's private folder in the instance root, removed when dropped
/// from under `root`, which follows the instance's folder if a rename moved
/// it.
struct Staging {
    root: PathBuf,
    name: std::ffi::OsString,
}

impl Staging {
    fn new(root: &Path) -> std::io::Result<Self> {
        let root = crate::util::publish::long_path(root);
        let folder = tempfile::Builder::new()
            .prefix(".tmp")
            .tempdir_in(&root)?
            .keep();
        let name = folder
            .file_name()
            .expect("a temporary folder has a name")
            .to_owned();
        Ok(Self { root, name })
    }

    fn path(&self) -> PathBuf {
        self.root.join(&self.name)
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
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
mod tests {
    use super::*;
    use crate::managers::App;
    use crate::managers::instance::mods::tests::addon_fixture;
    use carbon_repos::repos::{mod_metadata as metarepo, world_datapack_cache as wdb};
    use mockito::Matcher;
    use serde_json::json;
    use sha2::Digest;
    use std::io::Write;

    const GAME: &str = "1.20.1";
    const CF_PROJECT: i32 = 4242;

    struct Api {
        server: mockito::ServerGuard,
        curseforge: CurseForge,
        modrinth: Modrinth,
    }

    async fn api() -> Api {
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
        async fn get(&mut self, path: &str, body: serde_json::Value) {
            self.server
                .mock("GET", path)
                .match_query(Matcher::Any)
                .with_body(body.to_string())
                .create_async()
                .await;
        }

        /// Serves `bytes` as the download at `/files/<name>`; returns its URL.
        async fn file(&mut self, name: &str, bytes: &[u8]) -> String {
            let path = format!("/files/{name}");
            self.server
                .mock("GET", path.as_str())
                .with_body(bytes)
                .create_async()
                .await;
            format!("{}{path}", self.server.url())
        }
    }

    fn pack_zip(description: &str) -> Vec<u8> {
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
            "body": "", "additional_categories": [], "donation_urls": [], "project_type": "datapack", "downloads": 0, "id": "project", "team": "fixture",
            "published": "2026-01-01T00:00:00Z", "updated": "2026-01-01T00:00:00Z", "followers": 0, "status": "approved",
            "license": {"id": "MIT", "name": "MIT", "url": null}, "versions": [], "game_versions": [], "loaders": []
        })
    }

    /// A Modrinth version whose one file, `filename` at `url`, holds `bytes`.
    fn mr_version(
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

    fn cf_project(class: i32) -> serde_json::Value {
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
    fn cf_file(
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

    fn entries(folder: &Path) -> Vec<(String, Vec<u8>)> {
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
    fn snapshot(app_root: &Path, world: &Path) -> (Vec<String>, Vec<(String, Vec<u8>)>) {
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
    async fn modrinth_serving(api: &mut Api, versions: Vec<serde_json::Value>) {
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
        assert_eq!(
            error.to_string(),
            "This project has no data pack for Minecraft 1.20.1"
        );
        assert_eq!(snapshot(&root, &world), before);
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
        assert!(matches!(
            app.task_manager().get_task_outcome(task).await,
            crate::managers::vtask::TaskOutcome::Failed(_)
        ));
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
        assert!(
            refused(InstallWorldDatapack {
                replaces: Some("row".into()),
                ..request(id, &identity, modrinth(None))
            })
            .await
            .contains("not supported")
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
    fn staging_folders(root: &Path) -> Vec<String> {
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

    async fn without_recycle_bin(app: &App) {
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
}
