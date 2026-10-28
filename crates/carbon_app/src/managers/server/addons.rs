//! Server-specific planning uses the real server game/loader and persisted owners.
use super::*;
use crate::managers::AppInner;
use crate::managers::instance::installer::publication::{matches_download, selected_file};
use carbon_net::{Checksum, DownloadOptions, Downloadable};
use carbon_platforms::{
    curseforge::{
        FileRelationType,
        filters::{ModFileParameters, ModFilesParameters, ModFilesParametersQuery},
    },
    modrinth::{
        project::{ProjectSupportRange, ProjectVersionsFilters},
        search::{ProjectID, VersionID},
        version::DependencyType,
    },
};
use sha2::{Digest, Sha512};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub(super) enum Selection {
    Curseforge(u32, Option<u32>),
    Modrinth(String, Option<String>),
}
#[derive(Clone)]
struct Artifact {
    key: String,
    version: String,
    filename: String,
    url: String,
    size: u64,
    checksum: Checksum,
    required: bool,
}
struct Prepared {
    artifact: Artifact,
    location: String,
    old: Option<(mfcdb::ServerAddonFullRow, Vec<u8>)>,
    download: Downloadable,
    satisfied: bool,
    reuse: Option<PathBuf>,
}
fn basename(filename: &str) -> anyhow::Result<()> {
    world::validate_relative(filename)?;
    if filename.contains('/') {
        bail!("Addon filename is not a basename");
    }
    Ok(())
}
pub(super) async fn digest(path: &Path) -> anyhow::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut digest = Sha512::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(digest.finalize().to_vec())
}
/// No-replace move through the shared publication helper. It copies when the
/// server folder is on another drive than staging, so it runs on the
/// blocking pool.
async fn move_owned(source: &Path, destination: &Path) -> anyhow::Result<()> {
    crate::util::publish::move_noclobber_async(source, destination).await?;
    Ok(())
}

async fn plan(
    app: &Arc<AppInner>,
    server: &server_repo::ServerRow,
    selection: Selection,
) -> anyhow::Result<Vec<Artifact>> {
    let dependencies = app
        .settings_manager()
        .get_settings()
        .await?
        .download_dependencies;
    plan_graph(selection, dependencies, |selection| {
        resolve(app, server, selection, dependencies)
    })
    .await
}

async fn plan_graph<F, Fut>(
    selection: Selection,
    dependencies: bool,
    mut resolve: F,
) -> anyhow::Result<Vec<Artifact>>
where
    F: FnMut(Selection) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<(Artifact, Vec<Selection>)>>,
{
    let mut pending = VecDeque::from([(selection, false)]);
    let mut selected = HashMap::<String, String>::new();
    let mut result = Vec::new();
    while let Some((selection, required)) = pending.pop_front() {
        let (key, pin) = match &selection {
            Selection::Curseforge(project, pin) => {
                (format!("cf:{project}"), pin.map(|pin| pin.to_string()))
            }
            Selection::Modrinth(project, pin) => (format!("mr:{project}"), pin.clone()),
        };
        if let Some(chosen) = selected.get(&key) {
            if pin.as_ref().is_some_and(|pin| pin != chosen) {
                bail!("Conflicting required versions of {key}");
            }
            continue;
        }
        let (mut artifact, children) = resolve(selection).await?;
        if artifact.key != key || pin.as_ref().is_some_and(|pin| *pin != artifact.version) {
            bail!("Provider returned a different pinned artifact");
        }
        artifact.required = required;
        basename(&artifact.filename)?;
        selected.insert(artifact.key.clone(), artifact.version.clone());
        if dependencies {
            pending.extend(children.into_iter().map(|child| (child, true)));
        }
        result.push(artifact);
        if result.len() > 512 {
            bail!("Server dependency graph exceeds the supported bound");
        }
    }
    Ok(result)
}

async fn resolve(
    app: &Arc<AppInner>,
    server: &server_repo::ServerRow,
    selection: Selection,
    dependencies: bool,
) -> anyhow::Result<(Artifact, Vec<Selection>)> {
    let (artifact, children) = match selection {
        Selection::Curseforge(project, pin) => {
            let key = format!("cf:{project}");
            let file = if let Some(pin) = pin {
                app.modplatforms_manager()
                    .curseforge
                    .get_mod_file(ModFileParameters {
                        mod_id: project as i32,
                        file_id: pin as i32,
                    })
                    .await?
                    .data
            } else {
                let files = app
                    .modplatforms_manager()
                    .curseforge
                    .get_mod_files(ModFilesParameters {
                        mod_id: project as i32,
                        query: ModFilesParametersQuery {
                            game_version: Some(server.game_version.clone()),
                            game_version_type_id: None,
                            mod_loader_type: curseforge_modloader_filter(
                                None,
                                server.modloader_type.as_deref(),
                            ),
                            index: None,
                            page_size: Some(200),
                        },
                    })
                    .await?;
                files
                    .data
                    .into_iter()
                    .find(|f| f.game_versions.contains(&server.game_version))
                    .context("Required server dependency has no compatible version")?
            };
            if file.mod_id != project as i32 || !file.game_versions.contains(&server.game_version) {
                bail!("Selected CurseForge file does not match this server/project");
            }
            let is_jar = file.file_name.to_ascii_lowercase().ends_with(".jar");
            if is_jar
                && server.modloader_type.as_ref().is_none_or(|loader| {
                    !file
                        .game_versions
                        .iter()
                        .any(|v| v.eq_ignore_ascii_case(loader))
                })
            {
                bail!("Selected server mod does not match its loader");
            }
            let hash = file
                .hashes
                .iter()
                .find(|h| matches!(h.algo, carbon_platforms::curseforge::HashAlgo::Sha1))
                .or(file.hashes.first())
                .context("Server artifact has no verification hash")?;
            let checksum = match hash.algo {
                carbon_platforms::curseforge::HashAlgo::Sha1 => Checksum::Sha1(hash.value.clone()),
                carbon_platforms::curseforge::HashAlgo::Md5 => Checksum::Md5(hash.value.clone()),
            };
            let children = file
                .dependencies
                .iter()
                .filter(|d| matches!(d.relation_type, FileRelationType::RequiredDependency))
                .map(|d| Selection::Curseforge(d.mod_id as u32, None))
                .collect::<Vec<_>>();
            (
                Artifact {
                    key,
                    version: file.id.to_string(),
                    filename: file.file_name,
                    url: file
                        .download_url
                        .context("Required server artifact has no download URL")?,
                    size: file.file_length as u64,
                    checksum,
                    required: false,
                },
                children,
            )
        }
        Selection::Modrinth(project, pin) => {
            let key = format!("mr:{project}");
            let metadata = app
                .modplatforms_manager()
                .modrinth
                .get_project(ProjectID(project.clone()))
                .await?;
            if matches!(
                metadata.server_side,
                ProjectSupportRange::Unsupported | ProjectSupportRange::Unknown
            ) {
                bail!("Required addon server compatibility is not established: {project}");
            }
            let loaders =
                modrinth_loader_filter(&metadata.project_type, server.modloader_type.as_deref());
            let version = if let Some(pin) = pin {
                app.modplatforms_manager()
                    .modrinth
                    .get_version(VersionID(pin))
                    .await?
            } else {
                app.modplatforms_manager()
                    .modrinth
                    .get_project_versions(ProjectVersionsFilters {
                        project_id: ProjectID(project.clone()),
                        game_versions: Some(vec![server.game_version.clone()]),
                        loaders: loaders.clone(),
                        limit: None,
                        offset: None,
                    })
                    .await?
                    .into_iter()
                    .next()
                    .context("Required server dependency has no compatible version")?
            };
            if version.project_id != project
                || !version.game_versions.contains(&server.game_version)
            {
                bail!("Selected Modrinth version does not match this server/project");
            }
            let file = version
                .files
                .iter()
                .find(|file| file.primary)
                .or(version.files.first())
                .context("Required version has no artifact")?;
            validate_selected_loader(
                &file.filename,
                &version.loaders,
                loaders.as_ref(),
                server.modloader_type.as_deref(),
            )?;
            let mut children = Vec::new();
            if dependencies {
                for dependency in version
                    .dependencies
                    .iter()
                    .filter(|d| matches!(d.dependency_type, DependencyType::Required))
                {
                    let project = if let Some(project) = &dependency.project_id {
                        project.clone()
                    } else if let Some(version) = &dependency.version_id {
                        app.modplatforms_manager()
                            .modrinth
                            .get_version(VersionID(version.clone()))
                            .await?
                            .project_id
                    } else {
                        bail!(
                            "Required dependency is identified only by filename; install it manually before retrying"
                        );
                    };
                    children.push(Selection::Modrinth(project, dependency.version_id.clone()));
                }
            }
            (
                Artifact {
                    key,
                    version: version.id,
                    filename: file.filename.clone(),
                    url: file.url.clone(),
                    size: file.size as u64,
                    checksum: Checksum::Sha512(file.hashes.sha512.clone()),
                    required: false,
                },
                children,
            )
        }
    };
    Ok((artifact, children))
}

pub(super) async fn install(
    app: &Arc<AppInner>,
    id: ServerId,
    selection: Selection,
) -> anyhow::Result<VisualTaskId> {
    let mut activity = app
        .runtime_lifecycle
        .admit(format!("server {} addon installation", id.0))?;
    let server = server_repo::get_server(&app.db, id.0)
        .await?
        .context("Server not found")?;
    // Reservation prevents two whole addon plans from superseding each other;
    // the server gate remains available during all metadata and downloads.
    let reservation = app.server_manager.addon_installations.clone();
    match reservation.entry(id) {
        dashmap::mapref::entry::Entry::Occupied(_) => {
            bail!("A server addon installation is already in progress")
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(());
        }
    }
    struct Reservation(Arc<DashMap<ServerId, ()>>, ServerId);
    impl Drop for Reservation {
        fn drop(&mut self) {
            self.0.remove(&self.1);
        }
    }
    let reservation = Reservation(reservation, id);
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::ServerTaskInstallMod {
            mod_name: "Server addon and required dependencies".into(),
            server_name: server.name.clone(),
        },
    );
    let task_id = app.task_manager().spawn_task(&task).await;
    activity.external(); // completion belongs to the detached producer, not RPC Drop
    let app = app.clone();
    tokio::spawn(async move {
        let _reservation = reservation;
        let result = async {
            let artifacts = plan(&app, &server, selection).await?;
            let root = app.settings_manager().runtime_path.to_path_buf();
            let staging = tempfile::Builder::new()
                .prefix(".server-addon-")
                .tempdir_in(root)?
                .keep();
            let server_path = app
                .settings_manager()
                .runtime_path
                .get_servers()
                .get_server_path(&server.shortpath);
            let data = server_path.get_data_path();
            let mut prepared = Vec::new();
            {
                let gate = app.server_manager().get_op_lock(id);
                let _guard = gate.lock().await;
                app.server_manager()
                    .ensure_addon_publication(id, &server_path.get_root())
                    .await?;
                let cached = mfcdb::get_server_mods_full(&app.db, id.0).await?;
                for (index, artifact) in artifacts.into_iter().enumerate() {
                    let folder = server_addon_dir(&server_path, &artifact.filename)?;
                    let location = folder
                        .join(&artifact.filename)
                        .strip_prefix(&data)?
                        .to_str()
                        .context("Invalid addon path")?
                        .replace(std::path::MAIN_SEPARATOR, "/");
                    let kind = if artifact.filename.to_ascii_lowercase().ends_with(".zip") {
                        "datapacks"
                    } else {
                        "mods"
                    };
                    let stage_path = staging.join(format!("{index}-{}", artifact.filename));
                    if let Some(item) =
                        prepare_artifact(&data, &folder, artifact, location, &cached, &stage_path)
                            .await?
                    {
                        prepared.push(item);
                    }
                }
            }
            for item in &prepared {
                if let Some(source) = &item.reuse {
                    tokio::fs::copy(source, &item.download.path)
                        .await
                        .context("Satisfied bytes changed while staging")?;
                }
            }
            let downloads: Vec<_> = prepared
                .iter()
                .filter(|item| !item.satisfied && item.reuse.is_none())
                .map(|item| item.download.clone())
                .collect();
            carbon_net::download_multiple(
                &downloads,
                DownloadOptions::builder().concurrency(4).build(),
            )
            .await
            .with_context(|| {
                format!(
                    "Addon download failed; staged evidence retained at {}",
                    staging.display()
                )
            })?;
            let gate = app.server_manager().get_op_lock(id);
            let _guard = gate.lock().await;
            app.server_manager()
                .ensure_addon_publication(id, &server_path.get_root())
                .await?;
            let current = server_repo::get_server(&app.db, id.0)
                .await?
                .context("Server was deleted during download")?;
            if current.shortpath != server.shortpath
                || current.game_version != server.game_version
                || current.modloader_type != server.modloader_type
            {
                bail!("Server owner or version changed during download");
            }
            validate_plan(
                &data,
                &prepared,
                &mfcdb::get_server_mods_full(&app.db, id.0).await?,
            )
            .await?;
            for (index, item) in prepared.into_iter().enumerate() {
                if item.satisfied {
                    continue;
                }
                if !matches_download(&item.download.path, &item.download).await? {
                    bail!("Staged server addon failed verification");
                }
                let folder = server_addon_dir(&server_path, &item.artifact.filename)?;
                let current_location = folder
                    .join(&item.artifact.filename)
                    .strip_prefix(&data)?
                    .to_str()
                    .context("Invalid addon path")?
                    .replace(std::path::MAIN_SEPARATOR, "/");
                if current_location != item.location {
                    bail!("Selected world changed during download");
                }
                let mut old_path = None;
                let mut enabled = true;
                if let Some((row, hash)) = &item.old {
                    let now = mfcdb::get_server_mod_file_cache_by_id(&app.db, &row.id)
                        .await?
                        .context("Replacement owner disappeared")?;
                    if now.server_id != id.0
                        || now.location != row.location
                        || now.addon_type != row.addon_type
                    {
                        bail!("Replacement owner changed");
                    }
                    let physical =
                        world::owned_addon(&data, &row.addon_type, &row.location, &row.filename)?;
                    if digest(&physical).await? != *hash {
                        bail!("Old addon changed during download");
                    }
                    enabled = !physical.to_string_lossy().ends_with(".disabled");
                    if item.artifact.required && !enabled {
                        bail!("Required dependency was disabled during download");
                    }
                    old_path = Some(physical);
                }
                let kind = if item
                    .artifact
                    .filename
                    .to_ascii_lowercase()
                    .ends_with(".zip")
                {
                    "datapacks"
                } else {
                    "mods"
                };
                selected_source(&item.artifact)?; // source feasibility before retiring any live bytes
                let metadata_id = app
                    .meta_cache_manager()
                    .prepare_server_addon_metadata(&item.download.path, &item.artifact.filename)
                    .await?;
                let identity_artifact = item.artifact.clone();
                let location = item.location.clone();
                let retired = item
                    .old
                    .as_ref()
                    .filter(|(row, _)| row.location != item.location)
                    .map(|(row, _)| row.id.clone());
                let backup = staging.join(format!("previous-{index}"));
                let db = &app.db;
                publish_with_metadata(
                    &item.download,
                    &folder,
                    &item.artifact.filename,
                    old_path.as_deref(),
                    item.old.as_ref().map(|(_, hash)| hash.as_slice()),
                    enabled,
                    &backup,
                    async move {
                        db.write(move |mut conn| {
                            Ok(publish_selection(
                                &mut conn,
                                id.0,
                                kind,
                                &location,
                                enabled,
                                &metadata_id,
                                &identity_artifact,
                                retired.as_deref(),
                            ))
                        })
                        .await?
                    },
                )
                .await?;
                if item.old.is_some() {
                    tokio::fs::remove_file(&backup).await?;
                }
            }
            tokio::fs::remove_dir(&staging).await?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if result.is_ok() {
            app.meta_cache_manager()
                .queue_caching(
                    crate::managers::metadata::cache::CacheEntityId::Server(id.0),
                    true,
                )
                .await;
        }
        retain_outcome(task, result).await;
        app.invalidate(GET_SERVER_ADDONS, None);
        activity.finish();
    });
    Ok(task_id)
}

fn replacement_candidates<'a>(
    cached: &'a [mfcdb::ServerAddonFullRow],
    kind: &str,
    data: &Path,
    folder: &Path,
    artifact: &Artifact,
) -> Vec<&'a mfcdb::ServerAddonFullRow> {
    let Ok(namespace) = folder.strip_prefix(data) else {
        return Vec::new();
    };
    let namespace = namespace
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    cached
        .iter()
        .filter(|row| {
            row.addon_type == kind
                && row
                    .location
                    .rsplit_once('/')
                    .is_some_and(|(parent, _)| parent == namespace)
                && same_project(row, artifact)
        })
        .collect()
}
fn same_selection(row: &mfcdb::ServerAddonFullRow, artifact: &Artifact) -> bool {
    same_project(row, artifact)
        && if artifact.key.starts_with("cf:") {
            row.cf_file_id.map(|id| id.to_string()).as_ref() == Some(&artifact.version)
        } else {
            row.mr_version_id.as_ref() == Some(&artifact.version)
        }
}
async fn prepare_artifact(
    data: &Path,
    folder: &Path,
    artifact: Artifact,
    location: String,
    cached: &[mfcdb::ServerAddonFullRow],
    staging: &Path,
) -> anyhow::Result<Option<Prepared>> {
    let kind = if artifact.filename.to_ascii_lowercase().ends_with(".zip") {
        "datapacks"
    } else {
        "mods"
    };
    let candidates = replacement_candidates(cached, kind, data, folder, &artifact);
    if candidates.len() > 1 {
        bail!(
            "Multiple installed owners of {}; resolve duplicates before replacing",
            artifact.key
        );
    }
    selected_source(&artifact)?;
    let download = Downloadable::new(&artifact.url, staging)
        .with_size(artifact.size)
        .with_checksum(Some(artifact.checksum.clone()));
    let mut reuse = None;
    let mut satisfied = false;
    let old = if let Some(row) = candidates.first() {
        let physical = world::owned_addon(data, &row.addon_type, &row.location, &row.filename)?;
        if artifact.required && physical.to_string_lossy().ends_with(".disabled") {
            bail!("Required dependency is disabled; enable it explicitly");
        }
        let hash = digest(&physical).await?;
        if hash != row.sha512 {
            bail!("Installed addon changed since scan; refresh before replacing");
        }
        if matches_download(&physical, &download).await? {
            satisfied = artifact.required
                && same_selection(row, &artifact)
                && row.filename == artifact.filename;
            if !satisfied {
                reuse = Some(physical);
            }
        }
        Some(((*row).clone(), hash))
    } else {
        None
    };
    Ok(Some(Prepared {
        artifact,
        location,
        old,
        download,
        satisfied,
        reuse,
    }))
}
async fn validate_plan(
    data: &Path,
    plan: &[Prepared],
    current: &[mfcdb::ServerAddonFullRow],
) -> anyhow::Result<()> {
    for item in plan {
        if let Some((row, expected)) = &item.old {
            let now = current
                .iter()
                .find(|now| now.id == row.id)
                .context("Replacement owner disappeared")?;
            if now.location != row.location || now.addon_type != row.addon_type {
                bail!("Replacement owner changed");
            }
            if item.satisfied && !same_selection(now, &item.artifact) {
                bail!("Required dependency selection changed during download");
            }
            let physical = world::owned_addon(data, &row.addon_type, &row.location, &row.filename)?;
            if digest(&physical).await? != *expected {
                bail!("Old addon changed during download");
            }
            if item.artifact.required && physical.to_string_lossy().ends_with(".disabled") {
                bail!("Required dependency was disabled during download");
            }
        }
    }
    Ok(())
}
fn validate_selected_loader(
    filename: &str,
    actual: &[String],
    _query: Option<&Vec<String>>,
    server_loader: Option<&str>,
) -> anyhow::Result<()> {
    if filename.to_ascii_lowercase().ends_with(".jar") {
        let loader = server_loader
            .context("A loader-packaged JAR requires a compatible server modloader")?;
        if !actual
            .iter()
            .any(|actual| actual.eq_ignore_ascii_case(loader))
        {
            bail!("Selected JAR is incompatible with this server loader");
        }
    }
    Ok(())
}

fn same_project(row: &mfcdb::ServerAddonFullRow, artifact: &Artifact) -> bool {
    row.cf_project_id.map(|id| format!("cf:{id}")) == Some(artifact.key.clone())
        || row.mr_project_id.as_ref().map(|id| format!("mr:{id}")) == Some(artifact.key.clone())
}

fn selected_source(artifact: &Artifact) -> anyhow::Result<(&str, &str)> {
    let (platform, project) = artifact
        .key
        .split_once(':')
        .context("Missing selected source")?;
    if !matches!(platform, "cf" | "mr") || project.is_empty() || artifact.version.is_empty() {
        bail!("Invalid selected source");
    }
    if platform == "cf" {
        project.parse::<u32>()?;
        artifact.version.parse::<u32>()?;
    }
    basename(&artifact.filename)?;
    Ok((platform, project))
}
fn publish_identity(
    conn: &impl carbon_repos::db_exec::WriteAccess,
    server_id: i32,
    kind: &str,
    location: &str,
    metadata_id: &str,
    artifact: &Artifact,
) -> anyhow::Result<()> {
    let (platform, project) = selected_source(artifact)?;
    let metadata = carbon_repos::repos::mod_metadata::get_metadata_by_id_conn(conn, metadata_id)?
        .context("Verified metadata disappeared")?;
    if mfcdb::publish_server_source_conn(
        conn,
        server_id,
        kind,
        location,
        platform,
        project,
        &artifact.version,
        &metadata.sha512,
        DbDateTime(Utc::now().fixed_offset()),
    )? != 1
    {
        bail!("Selected source no longer matches the current owner/hash");
    }
    Ok(())
}
fn publish_selection(
    conn: &mut carbon_repos::db_exec::WriteGuard<'_>,
    server_id: i32,
    kind: &str,
    location: &str,
    enabled: bool,
    metadata_id: &str,
    artifact: &Artifact,
    retired: Option<&str>,
) -> anyhow::Result<()> {
    selected_source(artifact)?;
    let transaction = conn.transaction()?;
    mfcdb::upsert_server_mod_file_cache_at_location_conn(
        &transaction,
        server_id,
        &artifact.filename,
        location,
        artifact.size.try_into()?,
        enabled,
        kind,
        metadata_id,
        DbDateTime(Utc::now().fixed_offset()),
    )?;
    publish_identity(
        &transaction,
        server_id,
        kind,
        location,
        metadata_id,
        artifact,
    )?;
    if let Some(retired) = retired {
        mfcdb::delete_server_mod_file_cache_by_id_conn(&transaction, retired)?;
    }
    transaction.commit()?;
    Ok(())
}
async fn publish_with_metadata<F>(
    download: &Downloadable,
    folder: &Path,
    filename: &str,
    old: Option<&Path>,
    expected_old: Option<&[u8]>,
    enabled: bool,
    backup: &Path,
    publication: F,
) -> anyhow::Result<()>
where
    F: std::future::Future<Output = anyhow::Result<()>>,
{
    publish_staged(
        download,
        folder,
        filename,
        old,
        expected_old,
        enabled,
        backup,
    )
    .await?;
    if let Err(error) = publication.await {
        let destination = folder.join(if enabled {
            filename.to_owned()
        } else {
            format!("{filename}.disabled")
        });
        let restored = async {
            if !matches_download(&destination, download).await? {
                bail!("Published bytes changed before recovery");
            }
            move_owned(&destination, &download.path)?;
            if let Some(old) = old {
                move_owned(backup, old)?;
            }
            Ok::<_, anyhow::Error>(())
        }
        .await;
        return match restored {
            Ok(()) => Err(error).context("Addon metadata publication failed; original files/rows restored and staged bytes retained"),
            Err(recovery) => Err(error).context(format!("Addon metadata publication failed; recovery unresolved at {}: {recovery:#}", backup.display())),
        };
    }
    Ok(())
}

async fn retain_outcome(task: crate::managers::vtask::VisualTask, result: anyhow::Result<()>) {
    if let Err(error) = result {
        task.fail(error).await;
    } else {
        drop(task);
    }
}
async fn publish_staged(
    download: &Downloadable,
    folder: &Path,
    filename: &str,
    old: Option<&Path>,
    expected_old: Option<&[u8]>,
    enabled: bool,
    backup: &Path,
) -> anyhow::Result<()> {
    basename(filename)?;
    if !matches_download(&download.path, download).await? {
        bail!("Staged server addon failed verification");
    }
    tokio::fs::create_dir_all(folder).await?;
    if let Some((occupied, _)) = selected_file(folder, filename).await? {
        if old != Some(occupied.as_path()) {
            bail!("Server addon destination collision");
        }
    }
    let destination = folder.join(if enabled {
        filename.to_owned()
    } else {
        format!("{filename}.disabled")
    });
    if let Some(old) = old {
        if Some(digest(old).await?.as_slice()) != expected_old {
            bail!("Old server addon changed before publication");
        }
        move_owned(old, backup)?;
        if Some(digest(backup).await?.as_slice()) != expected_old {
            let _ = move_owned(backup, old);
            bail!(
                "Old server addon changed at publication; recovery retained at {}",
                backup.display()
            );
        }
    }
    #[tokio::test]
    async fn task07_replacement_rejects_changed_old_and_unrelated_collision() {
        let root = tempfile::tempdir().unwrap();
        let staged = root.path().join("staged");
        std::fs::write(&staged, b"new").unwrap();
        let download = Downloadable::new("https://fixture.invalid/not-requested", &staged)
            .with_size(3)
            .with_checksum(Some(Checksum::Sha512(hex::encode(Sha512::digest(b"new")))));
        let old = root.path().join("old.jar");
        std::fs::write(&old, b"old").unwrap();
        let expected = digest(&old).await.unwrap();
        std::fs::write(&old, b"edit").unwrap();
        assert!(
            publish_staged(
                &download,
                root.path(),
                "new.jar",
                Some(&old),
                Some(&expected),
                true,
                &root.path().join("backup")
            )
            .await
            .is_err()
        );
        std::fs::write(root.path().join("new.jar"), b"unrelated").unwrap();
        assert!(
            publish_staged(
                &download,
                root.path(),
                "new.jar",
                None,
                None,
                true,
                &root.path().join("backup")
            )
            .await
            .is_err()
        );
        assert_eq!(std::fs::read(&old).unwrap(), b"edit");
        assert_eq!(
            std::fs::read(root.path().join("new.jar")).unwrap(),
            b"unrelated"
        );
    }
    #[tokio::test]
    async fn task07_artifact_failure_is_retained_in_actual_visual_task() {
        let task = crate::managers::vtask::VisualTask::new(
            crate::api::translation::Translation::ServerTaskInstallMod {
                mod_name: "fixture".into(),
                server_name: "fixture".into(),
            },
        );
        let observer = task.clone();
        retain_outcome(task, Err(anyhow!("fixture artifact checksum failed"))).await;
        let actual = observer.make_domain_task(VisualTaskId(7)).await;
        assert!(
            matches!(actual.progress, crate::domain::vtask::Progress::Failed(error) if error.to_string().contains("checksum failed"))
        );
    }
    Ok(())
}

#[cfg(test)]
mod task07_tests {
    use super::*;
    fn artifact(key: &str, version: &str) -> Artifact {
        Artifact {
            key: format!("mr:{key}"),
            version: version.into(),
            filename: format!("{key}.jar"),
            url: "https://fixture.invalid/never-requested".into(),
            size: 3,
            checksum: Checksum::Sha512(hex::encode(Sha512::digest(b"new"))),
            required: false,
        }
    }
    fn review_fixture(
        data: &Path,
        location: &str,
        filename: &str,
        kind: &str,
    ) -> (rusqlite::Connection, mfcdb::ServerAddonFullRow) {
        use carbon_repos::db_exec::test_support::wg;
        let mut conn = rusqlite::Connection::open(data.join("fixture.db")).unwrap();
        carbon_repos::get_migrations()
            .0
            .to_latest(&mut conn)
            .unwrap();
        conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        carbon_repos::repos::mod_metadata::insert_metadata_conn(
            &wg(&mut conn),
            "meta",
            1,
            &Sha512::digest(b"new"),
            &[0; 20],
            "forge",
            None,
            None,
            None,
            None,
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .unwrap();
        mfcdb::upsert_server_mod_file_cache_at_location_conn(
            &wg(&mut conn),
            7,
            filename,
            location,
            3,
            true,
            kind,
            "meta",
            DbDateTime(Utc::now().fixed_offset()),
        )
        .unwrap();
        let mut selected = artifact("23", "1");
        selected.filename = filename.into();
        publish_identity(&wg(&mut conn), 7, kind, location, "meta", &selected).unwrap();
        let row = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7)
            .unwrap()
            .remove(0);
        (conn, row)
    }
    #[tokio::test]
    async fn task07r1_r2_satisfied_dependency_revalidated_after_operator_mutation() {
        for action in ["unchanged", "disable", "delete", "change"] {
            let root = tempfile::tempdir().unwrap();
            let folder = root.path().join("mods");
            std::fs::create_dir(&folder).unwrap();
            let file = folder.join("dependency.jar");
            std::fs::write(&file, b"new").unwrap();
            let (mut conn, row) =
                review_fixture(root.path(), "mods/dependency.jar", "dependency.jar", "mods");
            let mut dependency = artifact("23", "1");
            dependency.filename = "dependency.jar".into();
            dependency.required = true;
            // Production planner completes under the gate, then releases it for network.
            let gate = tokio::sync::Mutex::new(());
            let owner = gate.lock().await;
            let plan: Vec<_> = prepare_artifact(
                root.path(),
                &folder,
                dependency,
                "mods/dependency.jar".into(),
                &[row.clone()],
                &root.path().join("stage"),
            )
            .await
            .unwrap()
            .into_iter()
            .collect();
            drop(owner);
            tokio::task::yield_now().await;
            let operator = gate.lock().await;
            match action {
                "disable" => {
                    std::fs::rename(&file, folder.join("dependency.jar.disabled")).unwrap()
                }
                "delete" => std::fs::remove_file(&file).unwrap(),
                "change" => std::fs::write(&file, b"bad").unwrap(),
                _ => (),
            }
            use carbon_repos::db_exec::test_support::wg;
            if action == "disable" {
                mfcdb::update_server_mod_file_enabled_conn(
                    &wg(&mut conn),
                    &row.id,
                    false,
                    DbDateTime(Utc::now().fixed_offset()),
                )
                .unwrap();
            }
            if action == "delete" {
                mfcdb::delete_server_mod_file_cache_by_id_conn(&wg(&mut conn), &row.id).unwrap();
            }
            drop(operator);
            let publication = gate.lock().await;
            let current = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
            let result = validate_plan(root.path(), &plan, &current).await;
            assert_eq!(
                result.is_ok(),
                action == "unchanged",
                "accepted dependency closure lost {action} mutation"
            );
        }
    }
    #[test]
    fn task07r1_r3_inactive_world_is_not_replacement_candidate() {
        let root = tempfile::tempdir().unwrap();
        let (_conn, mut row) =
            review_fixture(root.path(), "A/datapacks/pack.zip", "pack.zip", "datapacks");
        let mut selected = artifact("23", "2");
        selected.filename = "pack.zip".into();
        let b = root.path().join("B/datapacks");
        assert_eq!(
            replacement_candidates(&[row.clone()], "datapacks", root.path(), &b, &selected).len(),
            0,
            "inactive A must not block first B installation"
        );
        let a = row.clone();
        row.id = "B-owner".into();
        row.location = "B/datapacks/pack.zip".into();
        let rows = [a, row];
        let found = replacement_candidates(&rows, "datapacks", root.path(), &b, &selected);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "B-owner");
    }
    #[tokio::test]
    async fn task07r1_r3_installs_and_replaces_b_while_a_owner_and_file_survive() {
        use carbon_repos::{db_exec::test_support::wg, repos::mod_metadata as metadata};
        let root = tempfile::tempdir().unwrap();
        for name in ["A", "B"] {
            std::fs::create_dir_all(root.path().join(format!("{name}/datapacks"))).unwrap();
        }
        std::fs::write(root.path().join("server.properties"), "level-name=B").unwrap();
        let a_path = root.path().join("A/datapacks/pack.zip");
        std::fs::write(&a_path, b"new").unwrap();
        let (mut conn, a) =
            review_fixture(root.path(), "A/datapacks/pack.zip", "pack.zip", "datapacks");
        let folder = root.path().join("B/datapacks");
        let mut b_id = None;
        for (version, bytes) in [("1", b"new"), ("2", b"two")] {
            let mut selected = artifact("23", version);
            selected.filename = "pack.zip".into();
            selected.checksum = Checksum::Sha512(hex::encode(Sha512::digest(bytes)));
            let cached = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
            let staged = root.path().join(format!("stage-{version}.zip"));
            let item = prepare_artifact(
                root.path(),
                &folder,
                selected.clone(),
                "B/datapacks/pack.zip".into(),
                &cached,
                &staged,
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(item.old.is_some(), version == "2");
            std::fs::write(&staged, bytes).unwrap();
            validate_plan(root.path(), &[item], &cached).await.unwrap();
            let meta = if version == "1" { "meta" } else { "two" };
            if version == "2" {
                metadata::insert_metadata_conn(
                    &wg(&mut conn),
                    meta,
                    2,
                    &Sha512::digest(bytes),
                    &[1; 20],
                    "",
                    None,
                    None,
                    None,
                    None,
                    None,
                    DbDateTime(Utc::now().fixed_offset()),
                )
                .unwrap();
            }
            let old = (version == "2").then(|| folder.join("pack.zip"));
            let old_hash = Sha512::digest(b"new");
            let download = Downloadable::new(&selected.url, &staged)
                .with_size(3)
                .with_checksum(Some(selected.checksum.clone()));
            publish_with_metadata(
                &download,
                &folder,
                "pack.zip",
                old.as_deref(),
                old.as_ref().map(|_| old_hash.as_slice()),
                true,
                &root.path().join(format!("backup-{version}")),
                async {
                    publish_selection(
                        &mut wg(&mut conn),
                        7,
                        "datapacks",
                        "B/datapacks/pack.zip",
                        true,
                        meta,
                        &selected,
                        None,
                    )
                },
            )
            .await
            .unwrap();
            let now = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
            assert_eq!(now.len(), 2);
            assert!(
                now.iter()
                    .any(|row| row.id == a.id && row.location == a.location)
            );
            let b = now
                .iter()
                .find(|row| row.location == "B/datapacks/pack.zip")
                .unwrap();
            if let Some(id) = &b_id {
                assert_eq!(&b.id, id);
            } else {
                b_id = Some(b.id.clone());
            }
            assert_eq!(std::fs::read(&a_path).unwrap(), b"new");
            assert_eq!(std::fs::read(folder.join("pack.zip")).unwrap(), bytes);
        }
        assert!(world::owned_addon(root.path(), "datapacks", &a.location, &a.filename).is_err());
    }

    #[test]
    fn task07r1_r4_identical_bytes_accept_distinct_selected_versions() {
        use carbon_repos::db_exec::test_support::wg;
        for provider in ["cf", "mr"] {
            let root = tempfile::tempdir().unwrap();
            let (mut conn, _row) = review_fixture(root.path(), "mods/one.jar", "one.jar", "mods");
            let mut first = artifact("23", "1");
            first.key = format!("{provider}:23");
            publish_identity(&wg(&mut conn), 7, "mods", "mods/one.jar", "meta", &first).unwrap();
            let mut second = first.clone();
            second.version = "2".into();
            assert!(
                publish_identity(&wg(&mut conn), 7, "mods", "mods/one.jar", "meta", &second)
                    .is_ok(),
                "same physical metadata must support a legitimate second selected version"
            );
        }
    }
    #[test]
    fn task07r1_r5_selected_jar_loader_is_independent_of_discovery_filter() {
        let forge = vec!["forge".into()];
        let filter = vec!["forge".into()];
        // Both specific/latest resolution branches converge on this selected-file validator.
        for query in [None, Some(&filter)] {
            assert!(
                validate_selected_loader("pack.jar", &forge, query, None).is_err(),
                "vanilla cannot load a Forge jar"
            );
            assert!(validate_selected_loader("pack.jar", &forge, query, Some("fabric")).is_err());
            validate_selected_loader("pack.jar", &forge, query, Some("forge")).unwrap();
            validate_selected_loader("pack.zip", &["datapack".into()], query, None).unwrap();
        }
    }

    #[tokio::test]
    async fn task07r1_r4_shared_hash_selection_and_sql_failure_restore_current_owner() {
        use carbon_repos::{db_exec::test_support::wg, repos::mod_metadata as metadata};
        for provider in ["cf", "mr"] {
            for same_name in [false, true] {
                for equal_bytes in [false, true] {
                    for enabled in [false, true] {
                        for fault in ["none", "row", "source"] {
                            let root = tempfile::tempdir().unwrap();
                            let folder = root.path().join("mods");
                            std::fs::create_dir(&folder).unwrap();
                            let mut conn =
                                rusqlite::Connection::open(root.path().join("fixture.db")).unwrap();
                            carbon_repos::get_migrations()
                                .0
                                .to_latest(&mut conn)
                                .unwrap();
                            conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
                            let old_bytes = if equal_bytes { b"new" } else { b"old" };
                            let old_meta = if equal_bytes { "shared" } else { "old" };
                            metadata::insert_metadata_conn(
                                &wg(&mut conn),
                                "shared",
                                1,
                                &Sha512::digest(b"new"),
                                &[0; 20],
                                "forge",
                                None,
                                None,
                                None,
                                None,
                                None,
                                DbDateTime(Utc::now().fixed_offset()),
                            )
                            .unwrap();
                            if !equal_bytes {
                                metadata::insert_metadata_conn(
                                    &wg(&mut conn),
                                    "old",
                                    2,
                                    &Sha512::digest(old_bytes),
                                    &[1; 20],
                                    "forge",
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    DbDateTime(Utc::now().fixed_offset()),
                                )
                                .unwrap();
                            }
                            // This is the actual hash lookup used by ensure_mod_metadata, with a
                            // pre-existing legitimate v3 owner. Identical bytes reuse that ID.
                            let shared = metadata::find_metadata_by_hashes_conn(
                                &wg(&mut conn),
                                &Sha512::digest(b"new"),
                                1,
                            )
                            .unwrap()
                            .unwrap();
                            assert_eq!(shared.id, "shared");
                            let mut first = artifact("23", "1");
                            first.key = format!("{provider}:23");
                            first.filename = "old.jar".into();
                            publish_selection(
                                &mut wg(&mut conn),
                                7,
                                "mods",
                                "mods/old.jar",
                                enabled,
                                old_meta,
                                &first,
                                None,
                            )
                            .unwrap();
                            let mut other = artifact("23", "3");
                            other.key = first.key.clone();
                            other.filename = "shared.jar".into();
                            publish_selection(
                                &mut wg(&mut conn),
                                8,
                                "mods",
                                "mods/shared.jar",
                                true,
                                "shared",
                                &other,
                                None,
                            )
                            .unwrap();
                            let old_path = folder.join(if enabled {
                                "old.jar"
                            } else {
                                "old.jar.disabled"
                            });
                            std::fs::write(&old_path, old_bytes).unwrap();
                            std::fs::write(root.path().join("other-server.jar"), b"new").unwrap();
                            let before = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7)
                                .unwrap()
                                .remove(0);
                            let other_before =
                                mfcdb::get_server_mod_files_by_server_conn(&wg(&mut conn), 8)
                                    .unwrap();
                            let mut selected = artifact("23", "2");
                            selected.key = first.key.clone();
                            selected.filename =
                                if same_name { "old.jar" } else { "new.jar" }.into();
                            let location = format!("mods/{}", selected.filename);
                            let stage = root.path().join("staged.jar");
                            let item = prepare_artifact(
                                root.path(),
                                &folder,
                                selected.clone(),
                                location.clone(),
                                &[before.clone()],
                                &stage,
                            )
                            .await
                            .unwrap()
                            .unwrap();
                            assert!(
                                !item.satisfied,
                                "explicit same-byte selection must publish its new owner source"
                            );
                            if equal_bytes {
                                assert!(item.reuse.is_some());
                            }
                            std::fs::write(&stage, b"new").unwrap();
                            if fault == "row" {
                                conn.execute_batch("CREATE TRIGGER fail_row BEFORE INSERT ON ServerModFileCache WHEN NEW.serverId=7 BEGIN SELECT RAISE(ABORT,'injected row failure'); END").unwrap();
                            }
                            if fault == "source" {
                                conn.execute_batch("CREATE TRIGGER fail_source BEFORE UPDATE OF sourceVersionId ON ServerModFileCache WHEN NEW.serverId=7 BEGIN SELECT RAISE(ABORT,'injected source failure'); END").unwrap();
                            }
                            let backup = root.path().join("backup");
                            let expected = Sha512::digest(old_bytes);
                            let retired = (!same_name).then_some(before.id.as_str());
                            let result = publish_with_metadata(
                                &item.download,
                                &folder,
                                &selected.filename,
                                Some(&old_path),
                                Some(&expected),
                                enabled,
                                &backup,
                                async {
                                    publish_selection(
                                        &mut wg(&mut conn),
                                        7,
                                        "mods",
                                        &location,
                                        enabled,
                                        &shared.id,
                                        &selected,
                                        retired,
                                    )
                                },
                            )
                            .await;
                            let current =
                                mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
                            assert_eq!(current.len(), 1);
                            if fault == "none" {
                                result.unwrap();
                                assert_eq!(current[0].source_version_id.as_deref(), Some("2"));
                                assert_eq!(current[0].filename, selected.filename);
                                assert_eq!(current[0].enabled, enabled);
                                if same_name {
                                    assert_eq!(current[0].id, before.id);
                                }
                                assert_eq!(
                                    std::fs::read(folder.join(if enabled {
                                        selected.filename.clone()
                                    } else {
                                        format!("{}.disabled", selected.filename)
                                    }))
                                    .unwrap(),
                                    b"new"
                                );
                            } else {
                                assert!(result.is_err());
                                assert_eq!(current[0].id, before.id);
                                assert_eq!(current[0].source_version_id.as_deref(), Some("1"));
                                assert_eq!(std::fs::read(&old_path).unwrap(), old_bytes);
                                assert!(stage.exists());
                                assert!(!backup.exists());
                                if !same_name {
                                    assert!(!folder.join("new.jar").exists());
                                    assert!(!folder.join("new.jar.disabled").exists());
                                }
                            }
                            assert_eq!(
                                mfcdb::get_server_mod_files_by_server_conn(&wg(&mut conn), 8)
                                    .unwrap(),
                                other_before
                            );
                            assert_eq!(
                                std::fs::read(root.path().join("other-server.jar")).unwrap(),
                                b"new"
                            );
                        }
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn task07_immediate_versions_replace_without_background_enrichment() {
        use carbon_repos::{db_exec::test_support::wg, repos::mod_metadata as metadata};
        for provider in ["mr", "cf"] {
            for same_name in [false, true] {
                let root = tempfile::tempdir().unwrap();
                let mut conn = rusqlite::Connection::open(root.path().join("fixture.db")).unwrap();
                carbon_repos::get_migrations()
                    .0
                    .to_latest(&mut conn)
                    .unwrap();
                conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
                let folder = root.path().join("mods");
                std::fs::create_dir(&folder).unwrap();
                let mut previous_id = None;
                for (version, bytes) in [("1", b"one"), ("2", b"two")] {
                    let mut selected = artifact("23", version);
                    selected.key = format!("{provider}:23");
                    selected.filename = if same_name {
                        "same.jar".into()
                    } else {
                        format!("version-{version}.jar")
                    };
                    selected.checksum = Checksum::Sha512(hex::encode(Sha512::digest(bytes)));
                    let rows = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
                    let old = rows.iter().find(|row| same_project(row, &selected));
                    if version == "2" {
                        assert!(
                            old.is_some(),
                            "known source identity must be visible without any enrichment worker"
                        );
                    }
                    let old_path = old.map(|row| folder.join(format!("{}.disabled", row.filename)));
                    let staged = root.path().join(format!("stage-{version}"));
                    std::fs::write(&staged, bytes).unwrap();
                    let download = Downloadable::new(&selected.url, &staged)
                        .with_size(3)
                        .with_checksum(Some(selected.checksum.clone()));
                    let backup = root.path().join(format!("backup-{version}"));
                    publish_staged(
                        &download,
                        &folder,
                        &selected.filename,
                        old_path.as_deref(),
                        old.map(|row| row.sha512.as_slice()),
                        false,
                        &backup,
                    )
                    .await
                    .unwrap();
                    let meta_id = format!("meta-{version}");
                    metadata::insert_metadata_conn(
                        &wg(&mut conn),
                        &meta_id,
                        version.parse().unwrap(),
                        &Sha512::digest(bytes),
                        &[0; 20],
                        "forge",
                        None,
                        None,
                        None,
                        None,
                        None,
                        DbDateTime(Utc::now().fixed_offset()),
                    )
                    .unwrap();
                    let location = format!("mods/{}", selected.filename);
                    mfcdb::upsert_server_mod_file_cache_at_location_conn(
                        &wg(&mut conn),
                        7,
                        &selected.filename,
                        &location,
                        3,
                        false,
                        "mods",
                        &meta_id,
                        DbDateTime(Utc::now().fixed_offset()),
                    )
                    .unwrap();
                    publish_identity(&wg(&mut conn), 7, "mods", &location, &meta_id, &selected)
                        .unwrap();
                    if let Some(old) = old {
                        if old.location != location {
                            mfcdb::delete_server_mod_file_cache_by_id_conn(&wg(&mut conn), &old.id)
                                .unwrap();
                        }
                        std::fs::remove_file(backup).unwrap();
                    }
                    let current = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
                    assert_eq!(current.len(), 1);
                    assert!(!current[0].enabled);
                    assert!(same_project(&current[0], &selected));
                    if same_name && version == "2" {
                        assert_eq!(Some(&current[0].id), previous_id.as_ref());
                    }
                    previous_id = Some(current[0].id.clone());
                    assert_eq!(
                        std::fs::read(folder.join(format!("{}.disabled", selected.filename)))
                            .unwrap(),
                        bytes
                    );
                    // A different selected source belongs to this owner, without modifying shared hash metadata.
                    let mut conflict = selected.clone();
                    conflict.version = "3".into();
                    assert!(
                        publish_identity(&wg(&mut conn), 7, "mods", &location, &meta_id, &conflict)
                            .is_ok()
                    );
                    assert!(same_project(
                        &mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap()[0],
                        &selected
                    ));
                }
                assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
                assert_eq!(
                    conn.query_row("SELECT count(*) FROM ModMetadata", [], |row| row
                        .get::<_, i64>(0))
                        .unwrap(),
                    2
                );
                let before_cf =
                    metadata::get_cf_cache_by_metadata_conn(&wg(&mut conn), "meta-2").unwrap();
                let before_mr =
                    metadata::get_mr_cache_by_metadata_conn(&wg(&mut conn), "meta-2").unwrap();
                metadata::insert_metadata_conn(
                    &wg(&mut conn),
                    "foreign",
                    99,
                    &[99; 64],
                    &[99; 20],
                    "forge",
                    None,
                    None,
                    None,
                    None,
                    None,
                    DbDateTime(Utc::now().fixed_offset()),
                )
                .unwrap();
                let mut colliding = artifact("23", "2");
                colliding.key = format!("{provider}:23");
                assert!(
                    publish_identity(
                        &wg(&mut conn),
                        7,
                        "mods",
                        "mods/missing.jar",
                        "foreign",
                        &colliding
                    )
                    .is_err()
                );
                assert_eq!(
                    metadata::get_cf_cache_by_metadata_conn(&wg(&mut conn), "meta-2").unwrap(),
                    before_cf
                );
                assert_eq!(
                    metadata::get_mr_cache_by_metadata_conn(&wg(&mut conn), "meta-2").unwrap(),
                    before_mr
                );
                assert_eq!(
                    mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7)
                        .unwrap()
                        .len(),
                    1
                );
            }
        }
    }

    #[tokio::test]
    async fn task07_required_graph_deduplicates_cycles_and_honors_setting() {
        let plan = plan_graph(
            Selection::Modrinth("a".into(), Some("v1".into())),
            true,
            |selection| async move {
                let Selection::Modrinth(key, _) = selection else {
                    unreachable!()
                };
                Ok((
                    artifact(&key, "v1"),
                    vec![Selection::Modrinth(
                        if key == "a" { "b" } else { "a" }.into(),
                        Some("v1".into()),
                    )],
                ))
            },
        )
        .await
        .unwrap();
        assert_eq!(
            plan.iter()
                .map(|a| (a.key.as_str(), a.required))
                .collect::<Vec<_>>(),
            vec![("mr:a", false), ("mr:b", true)]
        );
        let plan = plan_graph(Selection::Modrinth("a".into(), None), false, |_| async {
            Ok((
                artifact("a", "v1"),
                vec![Selection::Modrinth("b".into(), None)],
            ))
        })
        .await
        .unwrap();
        assert_eq!(plan.len(), 1);
    }
    #[tokio::test]
    async fn task07_dependency_pin_conflict_and_metadata_failure_are_errors() {
        let result = plan_graph(
            Selection::Modrinth("a".into(), Some("v1".into())),
            true,
            |_| async {
                Ok((
                    artifact("a", "v1"),
                    vec![Selection::Modrinth("a".into(), Some("v2".into()))],
                ))
            },
        )
        .await;
        assert!(result.is_err());
        let result = plan_graph(
            Selection::Modrinth("a".into(), None),
            true,
            |selection| async move {
                let Selection::Modrinth(key, _) = selection else {
                    unreachable!()
                };
                if key == "b" {
                    bail!("fixture required metadata missing");
                }
                Ok((
                    artifact("a", "v1"),
                    vec![Selection::Modrinth("b".into(), None)],
                ))
            },
        )
        .await;
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("metadata missing")
        );
    }
    #[tokio::test]
    async fn task07_replacement_preserves_disabled_and_same_path_and_failure() {
        for same_path in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let folder = root.path().join("mods");
            std::fs::create_dir(&folder).unwrap();
            let old = folder.join("v1.jar.disabled");
            std::fs::write(&old, b"old").unwrap();
            let staged = root.path().join("staged");
            std::fs::write(&staged, b"bad").unwrap();
            let backup = root.path().join("backup");
            let download = Downloadable::new("https://fixture.invalid/not-requested", &staged)
                .with_size(3)
                .with_checksum(Some(Checksum::Sha512(hex::encode(Sha512::digest(b"new")))));
            let filename = if same_path { "v1.jar" } else { "v2.jar" };
            let expected = digest(&old).await.unwrap();
            assert!(
                publish_staged(
                    &download,
                    &folder,
                    filename,
                    Some(&old),
                    Some(&expected),
                    false,
                    &backup
                )
                .await
                .is_err()
            );
            assert_eq!(std::fs::read(&old).unwrap(), b"old");
            std::fs::write(&staged, b"new").unwrap();
            publish_staged(
                &download,
                &folder,
                filename,
                Some(&old),
                Some(&expected),
                false,
                &backup,
            )
            .await
            .unwrap();
            std::fs::remove_file(&backup).unwrap(); // DB publication has completed
            assert_eq!(
                std::fs::read(folder.join(format!("{filename}.disabled"))).unwrap(),
                b"new"
            );
            assert!(!folder.join(filename).exists());
            assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
        }
    }
    #[tokio::test]
    async fn task07_replacement_rejects_changed_old_and_unrelated_collision() {
        let root = tempfile::tempdir().unwrap();
        let staged = root.path().join("staged");
        std::fs::write(&staged, b"new").unwrap();
        let download = Downloadable::new("https://fixture.invalid/not-requested", &staged)
            .with_size(3)
            .with_checksum(Some(Checksum::Sha512(hex::encode(Sha512::digest(b"new")))));
        let old = root.path().join("old.jar");
        std::fs::write(&old, b"old").unwrap();
        let expected = digest(&old).await.unwrap();
        std::fs::write(&old, b"edit").unwrap();
        assert!(
            publish_staged(
                &download,
                root.path(),
                "new.jar",
                Some(&old),
                Some(&expected),
                true,
                &root.path().join("backup")
            )
            .await
            .is_err()
        );
        std::fs::write(root.path().join("new.jar"), b"unrelated").unwrap();
        assert!(
            publish_staged(
                &download,
                root.path(),
                "new.jar",
                None,
                None,
                true,
                &root.path().join("backup")
            )
            .await
            .is_err()
        );
        assert_eq!(std::fs::read(&old).unwrap(), b"edit");
        assert_eq!(
            std::fs::read(root.path().join("new.jar")).unwrap(),
            b"unrelated"
        );
    }
    #[tokio::test]
    async fn task07_artifact_failure_is_retained_in_actual_visual_task() {
        let task = crate::managers::vtask::VisualTask::new(
            crate::api::translation::Translation::ServerTaskInstallMod {
                mod_name: "fixture".into(),
                server_name: "fixture".into(),
            },
        );
        let observer = task.clone();
        retain_outcome(task, Err(anyhow!("fixture artifact checksum failed"))).await;
        let actual = observer.make_domain_task(VisualTaskId(7)).await;
        assert!(
            matches!(actual.progress, crate::domain::vtask::Progress::Failed(error) if error.to_string().contains("checksum failed"))
        );
    }
}
