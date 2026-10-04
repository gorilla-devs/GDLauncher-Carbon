//! Local addon publication. Provider acquisition stays outside the instance gate.
use super::super::mods::{addon_metadata, twin_copies_refusal, validate_addon_basename};
use super::*;
use crate::domain::instance::AddonType;
use anyhow::anyhow;
use sha2::{Digest, Sha512};
use std::path::Path;

pub(crate) const WORLD_INCOMPLETE: &str = ".gdl-world-install-incomplete";

/// Prefix of an instance addon install attempt's private folder in the data
/// folder: its download, and the replaced version while an update commits.
pub(crate) const STAGING_PREFIX: &str = ".addon-install-";
/// The same for a server addon installation.
pub(crate) const SERVER_STAGING_PREFIX: &str = ".server-addon-";
/// What a staging folder may hold that is not a download: the replaced
/// version of an addon (`previous-addon`, a server's `previous-<n>`), kept
/// until its update commits, or for good when putting it back failed.
const RECOVERY_PREFIX: &str = "previous-";

/// Clears an install attempt's staging folder once nothing works in it any
/// more: every download and extraction goes, and the folder too unless it
/// still holds a replaced addon (see [`RECOVERY_PREFIX`]), which stays where
/// the error that named it said. Best effort: a leftover is logged, and the
/// startup sweep ([`sweep_stale_staging`]) tries again. Returns whether a
/// replaced addon was kept.
pub(crate) async fn settle_staging(staging: &Path) -> bool {
    let staging = staging.to_owned();
    tokio::task::spawn_blocking(move || settle_staging_blocking(&staging))
        .await
        .unwrap_or(true)
}

fn settle_staging_blocking(staging: &Path) -> bool {
    let entries = match std::fs::read_dir(staging) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return false,
        Err(error) => {
            tracing::warn!(
                "Could not clear addon staging {}: {error}",
                staging.display()
            );
            return true;
        }
    };
    let mut kept = false;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(
                    "Could not clear addon staging {}: {error}",
                    staging.display()
                );
                kept = true;
                continue;
            }
        };
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(RECOVERY_PREFIX)
        {
            kept = true;
            continue;
        }
        let path = entry.path();
        let removed = match entry.file_type() {
            Ok(kind) if kind.is_dir() => std::fs::remove_dir_all(&path),
            _ => std::fs::remove_file(&path),
        };
        if let Err(error) = removed {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("Could not remove {}: {error}", path.display());
            }
        }
    }
    if kept {
        // Waiting for its update to commit, or one that was never put back.
        tracing::info!(
            "Kept addon staging {}: it holds a replaced addon",
            staging.display()
        );
        return true;
    }
    match std::fs::remove_dir(staging) {
        Ok(()) => false,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            tracing::warn!(
                "Could not remove addon staging {}: {error}",
                staging.display()
            );
            true
        }
    }
}

/// Removes `dir`, a staging folder, once it is empty, as after an update
/// committed or a replaced addon was put back. Only a folder an install
/// attempt made: a caller's own folder is never touched. Best effort.
async fn remove_empty_staging(dir: Option<&Path>) {
    let Some(dir) = dir else { return };
    let ours = dir.file_name().is_some_and(|name| {
        let name = name.to_string_lossy();
        name.starts_with(STAGING_PREFIX) || name.starts_with(SERVER_STAGING_PREFIX)
    });
    if !ours {
        return;
    }
    if let Err(error) = tokio::fs::remove_dir(dir).await {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::debug!("Addon staging {} kept: {error}", dir.display());
        }
    }
}

/// At startup no install runs, so every staging folder in the data folder is
/// left over from an earlier session (one that was quit or crashed mid-way,
/// or an older build that kept them). Their downloads go; a replaced addon
/// that was never put back stays with its folder. Best effort.
pub(crate) async fn sweep_stale_staging(data_root: &Path) {
    let root = data_root.to_owned();
    let _ = tokio::task::spawn_blocking(move || {
        let Ok(entries) = std::fs::read_dir(&root) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let staging =
                name.starts_with(STAGING_PREFIX) || name.starts_with(SERVER_STAGING_PREFIX);
            // A link is never ours: tempfile makes plain folders.
            if staging && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                settle_staging_blocking(&entry.path());
            }
        }
    })
    .await;
}

pub(crate) async fn world_install_incomplete(world: &Path) -> anyhow::Result<bool> {
    let marker = world.join(WORLD_INCOMPLETE);
    let Some(meta) = addon_metadata(&marker).await? else {
        return Ok(false);
    };
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Ok(false);
    }
    let value = tokio::fs::read_to_string(marker).await?;
    Ok(value
        .strip_prefix(MARKER_PREFIX)
        .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok()))
}

pub(crate) async fn current_path(
    app: &Arc<AppInner>,
    id: InstanceId,
) -> anyhow::Result<InstancePath> {
    let manager = app.instance_manager();
    let instances = manager.instances.read().await;
    let instance = instances.get(&id).ok_or(InvalidInstanceIdError(id))?;
    instance.data()?;
    Ok(app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&instance.shortpath))
}

pub(crate) fn filename(
    resource: &dyn ResourceInstaller,
    path: &InstancePath,
) -> anyhow::Result<String> {
    let destination = resource.get_install_path(path);
    let folder = resource.addon_type().get_folder_path(path);
    let name = destination
        .strip_prefix(&folder)?
        .to_str()
        .context("invalid addon filename")?;
    validate_addon_basename(name)?;
    Ok(name.to_owned())
}

/// Whether a folder the user may have linked elsewhere is there. A symlink,
/// or a junction on Windows, is followed and only its target has to be a
/// folder: people link an instance, its `instance` folder or one addon
/// folder to share packs or saves between instances or to move a big
/// instance to another drive, and 2.0.40, Prism and server addons follow
/// such links. Publication copies when the target is on another drive
/// (`util::publish`). `Ok(false)` when nothing is there yet. This is the
/// rule for containers only: the entry an action renames, replaces or
/// deletes keeps its own checks ([`selected_file`], `remove_addon_from_disk`).
pub(crate) async fn followed_folder(path: &Path) -> anyhow::Result<bool> {
    match tokio::fs::metadata(path).await {
        Ok(meta) if meta.is_dir() => Ok(true),
        Ok(_) => bail!(
            "{} is a file, not a folder. Move or rename that file, then try again.",
            path.display()
        ),
        // lstat still sees an entry: a link whose target is gone.
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                && addon_metadata(path).await?.is_some() =>
        {
            bail!(
                "{} is a link to a folder that cannot be found. Reconnect the drive \
                 or folder it points to, or remove the link, then try again.",
                path.display()
            )
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => {
            Err(error).with_context(|| format!("Could not open the folder {}", path.display()))
        }
    }
}

/// The folder for `kind`, after checking the instance folder, its
/// `instance` folder and the addon folder with [`followed_folder`]. The path
/// is returned as configured, not resolved, so receipts and rows keep
/// comparing equal paths.
pub(crate) async fn checked_folder(
    path: &InstancePath,
    kind: AddonType,
) -> anyhow::Result<PathBuf> {
    let folder = kind.get_folder_path(path);
    for entry in [path.get_root(), path.get_data_path(), folder.clone()] {
        followed_folder(&entry).await?;
    }
    Ok(folder)
}

/// Why [`selected_file`] cannot name the one file an addon is.
#[derive(Debug, thiserror::Error)]
pub(crate) enum UnclearAddonFile {
    /// Both `name` and `name.disabled` are there.
    #[error("ambiguous enabled and disabled addon copies")]
    Both,
    /// The path is a link or a folder; `enabled` tells which spelling.
    #[error("addon is not a regular file")]
    NotAFile { enabled: bool },
}

pub(crate) async fn selected_file(
    folder: &Path,
    name: &str,
) -> anyhow::Result<Option<(PathBuf, bool)>> {
    validate_addon_basename(name)?;
    let enabled = folder.join(name);
    let disabled = folder.join(format!("{name}.disabled"));
    let a = addon_metadata(&enabled).await?;
    let b = addon_metadata(&disabled).await?;
    if a.is_some() && b.is_some() {
        bail!(UnclearAddonFile::Both);
    }
    for (path, meta, enabled) in [(enabled, a, true), (disabled, b, false)] {
        if let Some(meta) = meta {
            if !meta.is_file() || meta.file_type().is_symlink() {
                bail!(UnclearAddonFile::NotAFile { enabled });
            }
            return Ok(Some((path, enabled)));
        }
    }
    Ok(None)
}

/// [`selected_file`] for an addon an action may replace, with its refusals
/// in words. The rule stays: a link is not GDLauncher's to replace (the file
/// it points to may be shared with other instances), and of two copies none
/// is picked. `verb` is what the action would do to a copy.
pub(crate) async fn replaceable_file(
    folder: &Path,
    name: &str,
    verb: &str,
) -> anyhow::Result<Option<(PathBuf, bool)>> {
    match selected_file(folder, name).await {
        Err(error) => Err(explain_unclear_file(error, folder, name, verb).await),
        found => found,
    }
}

/// An [`UnclearAddonFile`] as what is there and what to do; other errors
/// pass through.
async fn explain_unclear_file(
    error: anyhow::Error,
    folder: &Path,
    name: &str,
    verb: &str,
) -> anyhow::Error {
    match error.downcast_ref::<UnclearAddonFile>() {
        Some(UnclearAddonFile::Both) => twin_copies_refusal(folder, name, verb),
        Some(UnclearAddonFile::NotAFile { enabled }) => {
            let file = if *enabled {
                name.to_owned()
            } else {
                format!("{name}.disabled")
            };
            let link = addon_metadata(&folder.join(&file))
                .await
                .ok()
                .flatten()
                .is_some_and(|meta| meta.file_type().is_symlink());
            if link {
                anyhow!(
                    "{file} in {} is a link to a file stored elsewhere, so GDLauncher will not \
                     replace it. Delete it in the Addons tab or your file manager (only the link \
                     is removed; the file it points to is kept), then install the version you want.",
                    folder.display()
                )
            } else {
                anyhow!(
                    "{file} in {} is a folder or a special file, not an addon file, so GDLauncher \
                     will not replace it. Move or rename it in your file manager, then try again.",
                    folder.display()
                )
            }
        }
        None => error,
    }
}

pub(crate) async fn matches_download(path: &Path, download: &Downloadable) -> anyhow::Result<bool> {
    use tokio::io::AsyncReadExt;
    let Some(meta) = addon_metadata(path).await? else {
        return Ok(false);
    };
    if !meta.is_file()
        || meta.file_type().is_symlink()
        || download.size.is_some_and(|s| s != meta.len())
    {
        return Ok(false);
    }
    let Some(checksum) = &download.checksum else {
        return Ok(false);
    };
    let mut file = tokio::fs::File::open(path).await?;
    let mut bytes = vec![0u8; 65536];
    // Dynamic digest keeps memory bounded for large packs.
    let (mut digest, expected): (Box<dyn sha2::digest::DynDigest + Send>, &str) = match checksum {
        Checksum::Sha1(value) => (Box::new(sha1::Sha1::new()), value),
        Checksum::Sha256(value) => (Box::new(sha2::Sha256::new()), value),
        Checksum::Sha512(value) => (Box::new(Sha512::new()), value),
        Checksum::Md5(value) => (Box::new(md5::Md5::new()), value),
    };
    loop {
        let n = file.read(&mut bytes).await?;
        if n == 0 {
            break;
        }
        digest.update(&bytes[..n]);
    }
    Ok(hex::encode(digest.finalize()).eq_ignore_ascii_case(expected))
}

pub(super) async fn verify_acquired(path: &Path, download: &Downloadable) -> anyhow::Result<()> {
    let metadata = addon_metadata(path)
        .await?
        .context("staged addon is missing")?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || download.size.is_some_and(|s| s != metadata.len())
    {
        bail!("staged addon has unexpected type or size");
    }
    if download.checksum.is_some() && !matches_download(path, download).await? {
        bail!("staged addon checksum mismatch");
    }
    Ok(())
}

pub(super) async fn is_satisfied(
    resource: &dyn ResourceInstaller,
    app: &Arc<AppInner>,
    id: InstanceId,
) -> anyhow::Result<bool> {
    if resource.addon_type() == AddonType::Worlds {
        return Ok(false);
    }
    let path = current_path(app, id).await?;
    let name = filename(resource, &path)?;
    let folder = checked_folder(&path, resource.addon_type()).await?;
    let rows = mfcdb::get_instance_mods_full(&app.db, *id).await?;
    if !rows
        .iter()
        .any(|r| r.filename == name && r.addon_type == resource.addon_type().to_db_string())
    {
        return Ok(false);
    }
    let Some((physical, _)) = replaceable_file(&folder, &name, "use").await? else {
        return Ok(false);
    };
    let Some(download) = resource.downloadable(&path).await else {
        return Ok(false);
    };
    matches_download(&physical, &download).await
}

#[derive(Clone)]
pub(super) struct ReplacementPlan {
    row: mfcdb::ModFileCacheRow,
    original: ReplacementOriginal,
    /// A file addon's state on disk when captured, which publication keeps;
    /// the row can lag behind a rename the scan has not recorded yet.
    enabled: bool,
}

#[derive(Clone)]
enum ReplacementOriginal {
    FileDigest(String),
    WorldDirectory(PathBuf),
}

async fn original_world(folder: &Path, name: &str) -> anyhow::Result<PathBuf> {
    validate_addon_basename(name)?;
    let path = folder.join(name);
    let metadata = addon_metadata(&path)
        .await?
        .context("original world is missing")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("original world is not a regular directory");
    }
    if world_install_incomplete(&path).await? {
        bail!("original world installation is incomplete");
    }
    Ok(path)
}

pub(crate) async fn file_digest(path: &Path) -> anyhow::Result<String> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut digest = Sha512::new();
    let mut buffer = vec![0u8; 65536];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(hex::encode(digest.finalize()))
}

impl ReplacementPlan {
    /// Whether the copy this replaces is enabled, as its file was when captured.
    pub(super) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(super) async fn capture(
        app: &Arc<AppInner>,
        id: InstanceId,
        row_id: &str,
        kind: AddonType,
    ) -> anyhow::Result<Self> {
        let row = app
            .instance_manager()
            .get_owned_mod_file(id, row_id)
            .await?;
        if row.addon_type != kind.to_db_string() {
            bail!("replacement addon kind mismatch");
        }
        let path = current_path(app, id).await?;
        let folder = checked_folder(&path, kind).await?;
        if kind == AddonType::Worlds {
            let original =
                ReplacementOriginal::WorldDirectory(original_world(&folder, &row.filename).await?);
            let enabled = row.enabled;
            return Ok(Self {
                row,
                original,
                enabled,
            });
        }
        let (physical, enabled) = selected_file(&folder, &row.filename)
            .await?
            .context("replacement file is missing")?;
        Ok(Self {
            row,
            original: ReplacementOriginal::FileDigest(file_digest(&physical).await?),
            enabled,
        })
    }
}

/// Rename-without-replacement through the shared publication helper, which
/// also copies across volumes (staging lives in the data folder, the
/// instance may be on another drive). Failure retains the source. The moved
/// file may be a copy, so stamps are taken from where it ends up, and the
/// move runs on the blocking pool so a large copy never stalls the runtime.
async fn move_noclobber(source: &Path, destination: &Path) -> std::io::Result<()> {
    crate::util::publish::move_noclobber_async(source, destination).await
}

pub(super) async fn publish_file(
    resource: &dyn ResourceInstaller,
    app: &Arc<AppInner>,
    id: InstanceId,
    staged: &Path,
    replacement: Option<&ReplacementPlan>,
    required: Option<&super::super::shader_loader::ShaderLoaderVerification>,
) -> anyhow::Result<()> {
    let receipt = Arc::new(Mutex::new(None));
    publish_file_owned(
        resource,
        app,
        id,
        staged,
        replacement,
        required,
        &receipt,
        None,
    )
    .await?;
    if let Some(receipt) = receipt.lock().await.as_ref() {
        receipt.commit().await?;
    }
    Ok(())
}

#[async_recursion::async_recursion]
pub(super) async fn publish_file_owned(
    resource: &dyn ResourceInstaller,
    app: &Arc<AppInner>,
    id: InstanceId,
    staged: &Path,
    replacement: Option<&ReplacementPlan>,
    required: Option<&super::super::shader_loader::ShaderLoaderVerification>,
    receipt: &ReceiptSlot,
    graph: Option<&InstallGraph>,
) -> anyhow::Result<()> {
    let manager = app.instance_manager();
    let _operation = manager.lock_instance(id).await;
    manager.ensure_modpack_not_locked(id).await?;
    if let Some(target) = required {
        target.validate_target(app, id).await?;
    }
    if let Some(graph) = graph {
        graph.validate_target(app, id).await?;
        for (identity, dependency) in graph.requirements.lock().await.iter() {
            if graph.pending(identity) {
                continue;
            }
            if identity == &resource.id() {
                validate_required_resource(resource, app, id).await?;
            } else {
                validate_required_resource(&**dependency.lock().await, app, id).await?;
            }
        }
    }
    let path = current_path(app, id).await?;
    let kind = resource.addon_type();
    let download = resource
        .downloadable(&path)
        .await
        .context("addon has no artifact")?;
    verify_acquired(staged, &download).await?;
    let folder = checked_folder(&path, kind).await?;
    let name = filename(resource, &path)?;
    tokio::fs::create_dir_all(&folder).await?;
    let mut old = None;
    let mut enabled = true;
    if let Some(plan) = replacement {
        let row = manager.get_owned_mod_file(id, &plan.row.id).await?;
        if row.metadata_id != plan.row.metadata_id
            || row.filename != plan.row.filename
            || row.addon_type != plan.row.addon_type
        {
            bail!("replacement identity changed while preparing download");
        }
        let (physical, current_enabled) = replaceable_file(&folder, &row.filename, "update")
            .await?
            .context("replacement file disappeared")?;
        let ReplacementOriginal::FileDigest(digest) = &plan.original else {
            bail!("world copy plan cannot replace a file addon");
        };
        if file_digest(&physical).await? != *digest {
            bail!("replacement bytes changed while preparing download");
        }
        enabled = current_enabled;
        old = Some((physical, row));
    }
    if let Some((occupied, _)) = replaceable_file(&folder, &name, "replace").await? {
        if old.as_ref().is_none_or(|(path, _)| *path != occupied) {
            if replacement.is_none() && is_satisfied(resource, app, id).await? {
                if required.is_some() {
                    enable_satisfied(resource, app, &_operation).await?;
                }
                return Ok(());
            }
            bail!("addon destination is occupied by another artifact");
        }
    }
    let destination = folder.join(if enabled {
        name.clone()
    } else {
        format!("{name}.disabled")
    });
    // No-clobber publication, copied when the instance is on another volume. Keep the
    // original recoverable until local cache finalization succeeds; failure never
    // deletes new public bytes.
    let backup = staged
        .parent()
        .context("missing staging directory")?
        .join("previous-addon");
    // Read before the old file moves aside: from then on every exit has to
    // put it back.
    let published_digest = file_digest(staged).await?;
    let mut previous = None;
    if let Some((physical, row)) = &old {
        let ReplacementOriginal::FileDigest(digest) = &replacement
            .context("missing file replacement plan")?
            .original
        else {
            bail!("world copy plan cannot replace a file addon");
        };
        let original_digest = file_digest(physical).await?;
        move_noclobber(physical, &backup).await?;
        // An unreadable backup is a changed one: it goes back.
        let unchanged = file_digest(&backup)
            .await
            .is_ok_and(|moved| moved == *digest);
        // A move across volumes leaves a copy, so stamp the backup itself.
        let stamp = match file_stamp(&backup) {
            Ok(stamp) if unchanged => stamp,
            _ => {
                // Keep the moved bytes recoverable, without overwriting a later arrival.
                return Err(match move_noclobber(&backup, physical).await {
                    Ok(()) => anyhow!(
                        "{} changed while it was being replaced, so it was left as it is. \
                         Try the update again.",
                        physical.display()
                    ),
                    Err(error) => anyhow!(
                        "old addon changed at publication; recovery: {} ({error})",
                        backup.display()
                    ),
                });
            }
        };
        previous = Some((
            backup.clone(),
            physical.clone(),
            original_digest,
            stamp,
            row.clone(),
        ));
    }
    #[cfg(test)]
    manager.test_hooks.pause("addon_before_publish", id).await;
    let publish = move_noclobber(staged, &destination).await;
    if let Err(error) = publish {
        if let Some((physical, _)) = &old {
            let _ = move_noclobber(&backup, physical).await;
        }
        return Err(error).with_context(|| {
            format!(
                "publication failed; recovery (if needed): {}",
                backup.display()
            )
        });
    }
    // Stamped where it landed: a move across volumes publishes a copy.
    let published_stamp = file_stamp(&destination).with_context(|| {
        format!(
            "Published addon changed during publication; recovery (if needed): {}",
            backup.display()
        )
    })?;
    *receipt.lock().await = Some(PublicationReceipt::File(FileReceipt {
        root: path.get_root(),
        destination: destination.clone(),
        kind,
        name: name.clone(),
        enabled,
        digest: published_digest,
        stamp: published_stamp,
        previous,
    }));
    receipt
        .lock()
        .await
        .as_ref()
        .context("Missing publication receipt")?
        .validate_commit()
        .await?;
    selected_file(&folder, &name)
        .await?
        .context("published addon disappeared")?;
    app.meta_cache_manager()
        .refresh_published_addon(
            &_operation,
            &folder,
            name.clone(),
            enabled,
            kind.to_db_string().to_owned(),
        )
        .await?;
    if let Some((_, row)) = old {
        if row.filename != name {
            mfcdb::delete_mod_file_cache_by_id(&app.db, &row.id).await?;
        }
    }
    Ok(())
}

/// Fully extract privately, accepting either a root save or one wrapped save.
/// No archive entry is ever extracted directly into the user's saves directory.
pub(super) async fn publish_world(
    archive: &Path,
    saves: &Path,
    default_name: &str,
) -> anyhow::Result<()> {
    prepare_world(archive, default_name)
        .await?
        .reserve(saves)
        .await?
        .finish()
        .await
}

/// The save name of a world at an archive's root: the archive's file name
/// without its extension.
pub(super) fn world_archive_stem(
    resource: &dyn ResourceInstaller,
    path: &InstancePath,
) -> anyhow::Result<String> {
    let name = filename(resource, path)?;
    Path::new(&name)
        .file_stem()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .context("Invalid world archive name")
}

/// Explicit world versions are new saves. The guard binds the current instance;
/// this plan never grants deletion authority over the mutable original world.
pub(super) async fn publish_world_version(
    resource: &dyn ResourceInstaller,
    app: &Arc<AppInner>,
    ownership: &crate::managers::instance::InstanceOperationGuard,
    staged: &Path,
    plan: &ReplacementPlan,
) -> anyhow::Result<Vec<PathBuf>> {
    let current = current_path(app, ownership.instance_id()).await?;
    let download = resource
        .downloadable(&current)
        .await
        .context("world has no artifact")?;
    verify_acquired(staged, &download).await?;
    let prepared = prepare_world(staged, &world_archive_stem(resource, &current)?).await?;
    publish_world_version_owned(resource, app, ownership, prepared, plan, None).await
}

/// [`publish_world_version`] for a world the caller extracted before taking
/// the instance gate. Returns the new saves.
pub(super) async fn publish_world_version_owned(
    resource: &dyn ResourceInstaller,
    app: &Arc<AppInner>,
    ownership: &crate::managers::instance::InstanceOperationGuard,
    prepared: PreparedWorld,
    plan: &ReplacementPlan,
    receipt: Option<&ReceiptSlot>,
) -> anyhow::Result<Vec<PathBuf>> {
    let id = ownership.instance_id();
    app.instance_manager().ensure_modpack_not_locked(id).await?;
    let row = app
        .instance_manager()
        .get_owned_mod_file(id, &plan.row.id)
        .await?;
    if resource.addon_type() != AddonType::Worlds
        || row.addon_type != AddonType::Worlds.to_db_string()
        || row.filename != plan.row.filename
        || row.metadata_id != plan.row.metadata_id
    {
        bail!("original world identity changed while preparing download");
    }
    let ReplacementOriginal::WorldDirectory(original) = &plan.original else {
        bail!("file replacement plan cannot publish a world copy");
    };
    let current = current_path(app, id).await?;
    let saves = checked_folder(&current, AddonType::Worlds).await?;
    if original_world(&saves, &row.filename).await? != *original {
        bail!("original world location changed while preparing download");
    }
    filename(resource, &current)?;
    #[cfg(test)]
    app.instance_manager()
        .test_hooks
        .pause("world_copy_before_reserve", id)
        .await;
    let reserved = prepared.reserve_copy(&saves).await?;
    let destinations = reserved.destinations();
    let names = reserved.names();
    reserved
        .finish_recorded(receipt.map(|slot| (current.get_root(), slot)))
        .await?;
    for name in names {
        app.meta_cache_manager()
            .refresh_published_addon(
                ownership,
                &saves,
                name,
                true,
                AddonType::Worlds.to_db_string().to_owned(),
            )
            .await?;
    }
    Ok(destinations)
}

/// A downloaded archive, extracted privately, and the worlds found in it.
pub(super) struct PreparedWorld {
    _extraction: tempfile::TempDir,
    worlds: Vec<WorldSource>,
}

/// One world of an extracted archive and what it holds, listed once so that
/// publication never walks a folder it is moving files out of.
struct WorldSource {
    source: PathBuf,
    name: String,
    /// Paths relative to `source`, each folder before what it holds; `true`
    /// marks a folder.
    entries: Vec<(PathBuf, bool)>,
}

/// Folders of an archive that are never a world, whatever they hold:
/// `__MACOSX` (the resource forks Finder's Compress adds) and hidden ones.
fn archive_clutter(name: &std::ffi::OsStr) -> bool {
    name == "__MACOSX" || name.to_string_lossy().starts_with('.')
}

fn regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file())
}

/// How many folders below an archive's root a world is looked for: a map in
/// a wrapper folder (`Map/level.dat`), or up to two more folders around it.
const WORLD_SEARCH_DEPTH: usize = 3;

/// The worlds in an extracted archive. The archive is one world when
/// `level.dat` is at its root. Otherwise each folder holding a `level.dat`
/// at the shallowest depth that has one is a world of its own, as 2.0.40
/// installed every save of a map pack, and wrapper folders around them are
/// looked through. A README, credits, a bundled resource pack, `__MACOSX` and
/// other folders without a world are left out. A `level.dat` deeper inside a
/// world (a backup, say) is part of that world.
fn world_roots(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    if regular_file(&root.join("level.dat")) {
        return Ok(vec![root.to_owned()]);
    }
    let mut level = vec![root.to_owned()];
    for _ in 0..WORLD_SEARCH_DEPTH {
        let (mut worlds, mut next) = (Vec::new(), Vec::new());
        for folder in &level {
            for entry in std::fs::read_dir(folder)? {
                let entry = entry?;
                if !entry.file_type()?.is_dir() || archive_clutter(&entry.file_name()) {
                    continue;
                }
                let path = entry.path();
                if regular_file(&path.join("level.dat")) {
                    worlds.push(path);
                } else {
                    next.push(path);
                }
            }
        }
        if !worlds.is_empty() {
            worlds.sort();
            return Ok(worlds);
        }
        level = next;
    }
    Ok(Vec::new())
}

/// The worlds [`world_roots`] finds, each with its save name and contents.
fn survey_worlds(root: &Path, default_name: &str) -> anyhow::Result<Vec<WorldSource>> {
    let roots = world_roots(root)?;
    if roots.is_empty() {
        bail!(
            "This download has no Minecraft world in it: none of its folders holds a level.dat \
             file. Nothing was installed."
        );
    }
    let mut worlds = Vec::with_capacity(roots.len());
    for source in roots {
        let name = if source == root {
            default_name.to_owned()
        } else {
            source
                .file_name()
                .and_then(|name| name.to_str())
                .context("A world in this download has a folder name that is not valid text")?
                .to_owned()
        };
        if validate_addon_basename(&name).is_err() {
            bail!(
                "The world \"{name}\" in this download has a folder name that cannot be used \
                 for a save here. Nothing was installed."
            );
        }
        let mut entries = Vec::new();
        for entry in walkdir::WalkDir::new(&source)
            .min_depth(1)
            .sort_by_file_name()
        {
            let entry = entry?;
            let relative = entry.path().strip_prefix(&source)?.to_owned();
            let kind = entry.file_type();
            if kind.is_symlink() || (!kind.is_dir() && !kind.is_file()) {
                bail!(
                    "{} in the world \"{name}\" is a link or a special file, which a save cannot \
                     hold. Nothing was installed.",
                    relative.display()
                );
            }
            if entry.file_name() == WORLD_INCOMPLETE {
                bail!(
                    "The world \"{name}\" holds a file named {WORLD_INCOMPLETE}, which GDLauncher \
                     uses to mark an unfinished install. Nothing was installed."
                );
            }
            entries.push((relative, kind.is_dir()));
        }
        worlds.push(WorldSource {
            source,
            name,
            entries,
        });
    }
    Ok(worlds)
}

/// Extracts `archive` next to it (in the install attempt's private folder)
/// and finds the worlds in it; `default_name` names a world at the archive's
/// root. Needs no instance gate: nothing of the instance is touched.
pub(super) async fn prepare_world(
    archive: &Path,
    default_name: &str,
) -> anyhow::Result<PreparedWorld> {
    let extraction = tempfile::tempdir_in(archive.parent().context("archive has no parent")?)?;
    carbon_compression::decompress(archive, extraction.path()).await?;
    let root = extraction.path().to_owned();
    let default_name = default_name.to_owned();
    let worlds = tokio::task::spawn_blocking(move || survey_worlds(&root, &default_name))
        .await
        .context("Reading the world archive stopped unexpectedly")??;
    Ok(PreparedWorld {
        _extraction: extraction,
        worlds,
    })
}

const MARKER_PREFIX: &str = "gdl-world-install-v1:";

/// Marker tokens of the reservations this process holds. A marked save whose
/// token is not here was left by an install that no longer runs.
static LIVE_RESERVATIONS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::LazyLock::new(Default::default);

fn live_reservations() -> std::sync::MutexGuard<'static, std::collections::HashSet<String>> {
    LIVE_RESERVATIONS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Keeps a reservation's token in [`LIVE_RESERVATIONS`] while it lives.
struct LiveReservation(String);

impl LiveReservation {
    fn new(token: &str) -> Self {
        live_reservations().insert(token.to_owned());
        Self(token.to_owned())
    }
}

impl Drop for LiveReservation {
    fn drop(&mut self) {
        live_reservations().remove(&self.0);
    }
}

/// The token in a save's install marker, when it is one GDLauncher wrote.
fn marker_token(marker: &Path) -> Option<String> {
    let meta = std::fs::symlink_metadata(marker).ok()?;
    if !meta.is_file() || meta.len() > 256 {
        return None;
    }
    let value = std::fs::read_to_string(marker).ok()?;
    value
        .strip_prefix(MARKER_PREFIX)
        .filter(|id| uuid::Uuid::parse_str(id).is_ok())?;
    Some(value)
}

/// Whether `stale` holds the first bytes of `source`: all of it, or a copy
/// that was cut short.
fn prefix_of(stale: &Path, source: &Path) -> std::io::Result<bool> {
    use std::io::Read;
    let (stale_meta, source_meta) = (
        std::fs::symlink_metadata(stale)?,
        std::fs::symlink_metadata(source)?,
    );
    if !source_meta.is_file() || stale_meta.len() > source_meta.len() {
        return Ok(false);
    }
    let (mut stale, mut source) = (std::fs::File::open(stale)?, std::fs::File::open(source)?);
    let (mut left, mut right) = (vec![0u8; 65536], vec![0u8; 65536]);
    loop {
        let n = stale.read(&mut left)?;
        if n == 0 {
            return Ok(true);
        }
        source.read_exact(&mut right[..n])?;
        if left[..n] != right[..n] {
            return Ok(false);
        }
    }
}

/// Removes `candidate` when it is a save an earlier install of this same
/// world left unfinished (GDLauncher quit or crashed during it, or a failure
/// could not clean up): a marker of no install that still runs, and besides
/// it only folders and files this world also has, each file whole or cut
/// short. Such a leftover holds nothing this install does not bring back, so
/// its name is used again. Anything else in it, such as Minecraft's
/// `session.lock` once someone opened it, keeps it as it is.
fn reclaim_leftover(candidate: &Path, world: &WorldSource) -> bool {
    if !std::fs::symlink_metadata(candidate).is_ok_and(|meta| meta.is_dir()) {
        return false;
    }
    let marker = candidate.join(WORLD_INCOMPLETE);
    let Some(token) = marker_token(&marker) else {
        return false;
    };
    if live_reservations().contains(&token) {
        return false;
    }
    let mut entries = Vec::new();
    for entry in walkdir::WalkDir::new(candidate).min_depth(1) {
        let Ok(entry) = entry else {
            return false;
        };
        if entry.path() == marker {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(candidate) else {
            return false;
        };
        let source = world.source.join(relative);
        let kind = entry.file_type();
        let fits = if kind.is_dir() {
            std::fs::symlink_metadata(&source).is_ok_and(|meta| meta.is_dir())
        } else {
            kind.is_file() && prefix_of(entry.path(), &source).unwrap_or(false)
        };
        if !fits {
            return false;
        }
        entries.push((entry.path().to_owned(), kind.is_dir()));
    }
    // Each folder comes before what it holds, so in reverse it is empty.
    let removed = entries
        .iter()
        .rev()
        .try_for_each(|(path, is_dir)| {
            if *is_dir {
                std::fs::remove_dir(path)
            } else {
                std::fs::remove_file(path)
            }
        })
        .and_then(|()| std::fs::remove_file(&marker))
        .and_then(|()| std::fs::remove_dir(candidate));
    match removed {
        Ok(()) => {
            tracing::info!(
                "Removed {}, left unfinished by an earlier install of the same world",
                candidate.display()
            );
            true
        }
        Err(error) => {
            tracing::warn!(
                "Could not remove the unfinished install {}: {error}",
                candidate.display()
            );
            false
        }
    }
}

/// How far a save name is numbered before GDLauncher gives up.
const COPY_LIMIT: usize = 100;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Naming {
    /// The world's own name, or "Name (copy N)" when a save already has it,
    /// the way Prism and Minecraft number a folder name that is taken.
    Own,
    /// Always "Name (copy N)": another version installed beside a world.
    Copy,
}

/// A save folder for `world`, created empty but for an install marker that
/// keeps it out of the world list until the world is complete.
fn reserve_tree(saves: &Path, world: WorldSource, naming: Naming) -> anyhow::Result<ReservedTree> {
    use std::io::Write;
    let first = if naming == Naming::Own { 0 } else { 1 };
    let mut reserved = None;
    for n in first..=COPY_LIMIT {
        let name = if n == 0 {
            world.name.clone()
        } else {
            format!("{} (copy {n})", world.name)
        };
        let candidate = saves.join(&name);
        match std::fs::create_dir(&candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if !reclaim_leftover(&candidate, &world) || std::fs::create_dir(&candidate).is_err()
                {
                    continue;
                }
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Could not create the save folder {}", candidate.display())
                });
            }
        }
        reserved = Some((candidate, name));
        break;
    }
    let Some((destination, name)) = reserved else {
        let first = if naming == Naming::Own {
            format!("\"{}\" and ", world.name)
        } else {
            String::new()
        };
        bail!(
            "{} already has saves named {first}\"{name} (copy 1)\" to \"{name} (copy {COPY_LIMIT})\", \
             so the world was not installed. Delete or rename some of them, then try again.",
            saves.display(),
            name = world.name,
        );
    };
    let marker = destination.join(WORLD_INCOMPLETE);
    let token = format!("{MARKER_PREFIX}{}", uuid::Uuid::new_v4());
    let live = LiveReservation::new(&token);
    let mut created = false;
    let written = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)?;
        created = true;
        file.write_all(token.as_bytes())?;
        // Through the helper: a data folder on a macOS network share
        // refuses std's F_FULLFSYNC, and 2.0.40 installed worlds there.
        crate::util::durable::sync_file(&file)
    })();
    if let Err(error) = written {
        // Only what this reservation made: never later content.
        if created {
            let _ = std::fs::remove_file(&marker);
        }
        let _ = std::fs::remove_dir(&destination);
        return Err(error).with_context(|| {
            format!(
                "Could not prepare the save folder {}",
                destination.display()
            )
        });
    }
    Ok(ReservedTree {
        world,
        name,
        destination,
        marker,
        token,
        _live: live,
    })
}

/// Gives up a reservation nothing was published into yet.
fn release(tree: &ReservedTree) {
    if marker_token(&tree.marker).as_deref() == Some(tree.token.as_str()) {
        let _ = std::fs::remove_file(&tree.marker);
    }
    let _ = std::fs::remove_dir(&tree.destination);
}

impl PreparedWorld {
    /// Reserves a save for each world under its own name, or "Name (copy N)"
    /// when a save already has it: an existing save is never written into.
    pub(super) async fn reserve(self, saves: &Path) -> anyhow::Result<ReservedWorld> {
        self.reserve_with(saves, Naming::Own).await
    }

    pub(super) async fn reserve_copy(self, saves: &Path) -> anyhow::Result<ReservedWorld> {
        self.reserve_with(saves, Naming::Copy).await
    }

    async fn reserve_with(self, saves: &Path, naming: Naming) -> anyhow::Result<ReservedWorld> {
        tokio::fs::create_dir_all(saves).await?;
        let saves = saves.to_owned();
        tokio::task::spawn_blocking(move || {
            let Self {
                _extraction,
                worlds,
            } = self;
            let mut trees: Vec<ReservedTree> = Vec::with_capacity(worlds.len());
            for world in worlds {
                match reserve_tree(&saves, world, naming) {
                    Ok(tree) => trees.push(tree),
                    Err(error) => {
                        trees.iter().for_each(release);
                        return Err(error);
                    }
                }
            }
            Ok(ReservedWorld { _extraction, trees })
        })
        .await
        .context("Reserving the save folder stopped unexpectedly")?
    }
}

/// Saves reserved for an archive's worlds, filled by [`ReservedWorld::finish`].
pub(super) struct ReservedWorld {
    _extraction: tempfile::TempDir,
    trees: Vec<ReservedTree>,
}

struct ReservedTree {
    world: WorldSource,
    /// The save folder's name: the world's own, or with a "(copy N)" suffix.
    name: String,
    destination: PathBuf,
    marker: PathBuf,
    token: String,
    _live: LiveReservation,
}

/// What a publication placed in one save so far, for undoing it.
#[derive(Default)]
struct PlacedTree {
    /// Folders made inside the save, each before what it holds.
    directories: Vec<PathBuf>,
    /// Files placed, with their stamp once complete. A file without one was
    /// created by this publication and left unfinished.
    files: Vec<(PathBuf, Option<FileStamp>)>,
}

impl PlacedTree {
    fn stamped(&self, destination: &Path) -> anyhow::Result<WorldTree> {
        let mut directories = vec![(destination.to_owned(), directory_stamp(destination)?)];
        for path in &self.directories {
            directories.push((path.clone(), directory_stamp(path)?));
        }
        let files = self
            .files
            .iter()
            .map(|(path, stamp)| {
                stamp
                    .clone()
                    .map(|stamp| (path.clone(), stamp))
                    .context("World file was not completed")
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(WorldTree {
            destination: destination.to_owned(),
            directories,
            files,
        })
    }

    /// Removes what this publication placed, newest first, and only a file
    /// that is as it was placed. Whether anything of it is still there.
    fn remove(&self) -> bool {
        let mut left = false;
        for (path, stamp) in self.files.iter().rev() {
            match std::fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => {
                    left = true;
                    continue;
                }
                Ok(_) => {}
            }
            let unchanged = stamp
                .as_ref()
                .is_none_or(|stamp| file_stamp(path).ok().as_ref() == Some(stamp));
            if !unchanged {
                left = true;
                continue;
            }
            if let Err(error) = std::fs::remove_file(path) {
                left |= error.kind() != std::io::ErrorKind::NotFound;
            }
        }
        for path in self.directories.iter().rev() {
            if let Err(error) = std::fs::remove_dir(path) {
                left |= error.kind() != std::io::ErrorKind::NotFound;
            }
        }
        left
    }
}

/// Test-only failures of a world file's copy, keyed by its destination, in
/// the style of `util::publish::fault`: a write that fails half way, as on
/// a full disk.
#[cfg(test)]
pub(crate) mod world_fault {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::{LazyLock, Mutex};

    static FAULTS: LazyLock<Mutex<HashMap<PathBuf, i32>>> = LazyLock::new(Default::default);

    /// The next copy of a world file to `target` fails with OS error `code`
    /// once its destination was created.
    pub(crate) fn inject(target: &Path, code: i32) {
        FAULTS.lock().unwrap().insert(target.to_owned(), code);
    }

    pub(super) fn take(target: &Path) -> std::io::Result<()> {
        match FAULTS.lock().unwrap().remove(target) {
            Some(code) => Err(std::io::Error::from_raw_os_error(code)),
            None => Ok(()),
        }
    }
}

/// Copies a world file the way a move to another volume would, but without
/// a flush per file, which made large maps slow (2.0.40 did not flush them
/// either). The destination is created new, so nothing is ever replaced.
fn copy_world_file(
    source: &Path,
    target: &Path,
    files: &mut Vec<(PathBuf, Option<FileStamp>)>,
) -> anyhow::Result<()> {
    let meta = std::fs::symlink_metadata(source)?;
    let mut from = std::fs::File::open(source)?;
    let mut to = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    files.push((target.to_owned(), None));
    #[cfg(test)]
    world_fault::take(target)?;
    let copied = std::io::copy(&mut from, &mut to)?;
    if copied != meta.len() {
        bail!("{} changed while it was copied", source.display());
    }
    if let Ok(modified) = meta.modified() {
        // A move keeps the modification time; the copy follows suit.
        let _ = to.set_modified(modified);
    }
    // Stamped once the handle is closed: FAT, exFAT, ReFS, and NTFS before
    // Windows 10 1809, set the modified time when the handle closes
    // (MS-FSA 2.1.4.17), so a stamp taken through the open handle would not
    // match the file afterwards and the world would look edited.
    drop(to);
    files.last_mut().expect("just pushed").1 = Some(file_stamp(target)?);
    Ok(())
}

/// Moves a world file into its save without replacing anything, or copies
/// it when the save is on another volume (`copying` then stays set for the
/// rest) or the move is refused, as when a virus scanner on Windows still
/// holds a file that was just extracted.
fn place_world_file(
    source: &Path,
    target: &Path,
    copying: &mut bool,
    files: &mut Vec<(PathBuf, Option<FileStamp>)>,
) -> anyhow::Result<()> {
    if !*copying {
        match crate::util::publish::rename_noclobber(source, target) {
            Ok(()) => {
                files.push((target.to_owned(), None));
                files.last_mut().expect("just pushed").1 = Some(file_stamp(target)?);
                return Ok(());
            }
            // Something else is there: never replaced, and not this install's.
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(error.into());
            }
            Err(error) if crate::util::publish::crosses_devices(&error) => *copying = true,
            Err(error) => {
                tracing::debug!("Copying {} instead of moving it: {error}", source.display())
            }
        }
    }
    copy_world_file(source, target, files)
}

/// Places every reserved world's files into its save, then lifts the
/// markers. `placed` grows as it goes, for [`undo_trees`].
fn place_trees(trees: &[ReservedTree], placed: &mut Vec<PlacedTree>) -> anyhow::Result<()> {
    let mut copying = false;
    for tree in trees {
        placed.push(PlacedTree::default());
        let record = placed.last_mut().expect("just pushed");
        for (relative, is_dir) in &tree.world.entries {
            let source = tree.world.source.join(relative);
            let target = tree.destination.join(relative);
            if *is_dir {
                std::fs::create_dir(&target)
                    .with_context(|| format!("Could not create {}", target.display()))?;
                record.directories.push(target);
            } else {
                place_world_file(&source, &target, &mut copying, &mut record.files)
                    .with_context(|| format!("Could not write {}", target.display()))?;
            }
        }
    }
    Ok(())
}

fn publish_trees(trees: &[ReservedTree]) -> anyhow::Result<Vec<WorldTree>> {
    let mut placed = Vec::with_capacity(trees.len());
    let published = place_trees(trees, &mut placed).and_then(|()| {
        let published = trees
            .iter()
            .zip(&placed)
            .map(|(tree, record)| record.stamped(&tree.destination))
            .collect::<anyhow::Result<Vec<_>>>()?;
        for tree in trees {
            if marker_token(&tree.marker).as_deref() != Some(tree.token.as_str()) {
                bail!(
                    "The install marker in {} was changed by something else during the install",
                    tree.destination.display()
                );
            }
            std::fs::remove_file(&tree.marker)?;
        }
        Ok(published)
    });
    published.map_err(|error| undo_trees(trees, &placed, error))
}

/// Undoes a publication that failed (a full disk, a file another program
/// holds, something put into the save meanwhile): what it placed goes, then
/// the marker and the save folder, so the save neither lingers hidden nor
/// blocks a retry. Something it cannot remove keeps the folder, and the
/// error names it; while some of this install is still in there the marker
/// stays, so a retry recognizes the leftover ([`reclaim_leftover`]).
fn undo_trees(
    trees: &[ReservedTree],
    placed: &[PlacedTree],
    error: anyhow::Error,
) -> anyhow::Error {
    let mut kept = Vec::new();
    for (index, tree) in trees.iter().enumerate().rev() {
        let ours_left = placed.get(index).is_some_and(PlacedTree::remove);
        if !ours_left && marker_token(&tree.marker).as_deref() == Some(tree.token.as_str()) {
            let _ = std::fs::remove_file(&tree.marker);
        }
        match std::fs::remove_dir(&tree.destination) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => kept.push(tree.destination.display().to_string()),
        }
    }
    let names = trees
        .iter()
        .map(|tree| format!("\"{}\"", tree.world.name))
        .collect::<Vec<_>>()
        .join(", ");
    if kept.is_empty() {
        return error.context(format!("Could not install the world {names}"));
    }
    anyhow!(
        "Could not install the world {names}: {error:#}. {} was kept because not everything in \
         it could be removed: check it in your file manager, then install the world again.",
        kept.join(", ")
    )
}

impl ReservedWorld {
    /// The reserved save folders.
    pub(super) fn destinations(&self) -> Vec<PathBuf> {
        self.trees
            .iter()
            .map(|tree| tree.destination.clone())
            .collect()
    }

    /// The reserved saves' names.
    pub(super) fn names(&self) -> Vec<String> {
        self.trees.iter().map(|tree| tree.name.clone()).collect()
    }

    /// For a [`PreparedWorld::reserve`]: each world saved under another name
    /// because a save already had its own, as (its name, the save's name).
    pub(super) fn renamed(&self) -> Vec<(String, String)> {
        self.trees
            .iter()
            .filter(|tree| tree.name != tree.world.name)
            .map(|tree| (tree.world.name.clone(), tree.name.clone()))
            .collect()
    }

    pub(super) async fn finish(self) -> anyhow::Result<()> {
        self.finish_recorded(None).await
    }

    pub(super) async fn finish_owned(
        self,
        root: PathBuf,
        receipt: &ReceiptSlot,
    ) -> anyhow::Result<()> {
        self.finish_recorded(Some((root, receipt))).await
    }

    /// Moves the worlds into their saves (a copy across volumes) and lifts
    /// the markers; on failure undoes it ([`undo_trees`]). The receipt is
    /// recorded only for a complete publication.
    async fn finish_recorded(self, receipt: Option<(PathBuf, &ReceiptSlot)>) -> anyhow::Result<()> {
        let Self { _extraction, trees } = self;
        let published = tokio::task::spawn_blocking(move || {
            let published = publish_trees(&trees);
            // What was not moved goes with the extraction, off the runtime.
            drop(_extraction);
            published
        })
        .await
        .context("Installing the world stopped unexpectedly")??;
        if let Some((root, slot)) = receipt {
            *slot.lock().await = Some(PublicationReceipt::World(WorldReceipt {
                root,
                trees: published,
            }));
        }
        Ok(())
    }
}

/// Called only while the instance gate is held and the exact requested artifact
/// has been verified. No-clobber publication refuses a late enabled occupant.
pub(super) async fn enable_satisfied(
    resource: &dyn ResourceInstaller,
    app: &Arc<AppInner>,
    ownership: &crate::managers::instance::InstanceOperationGuard,
) -> anyhow::Result<()> {
    let id = ownership.instance_id();
    let path = current_path(app, id).await?;
    let folder = checked_folder(&path, resource.addon_type()).await?;
    let name = filename(resource, &path)?;
    let (physical, enabled) = selected_file(&folder, &name)
        .await?
        .context("shader loader disappeared")?;
    if enabled {
        return Ok(());
    }
    let destination = folder.join(&name);
    move_noclobber(&physical, &destination).await?;
    app.meta_cache_manager()
        .refresh_published_addon(
            ownership,
            &folder,
            name,
            true,
            resource.addon_type().to_db_string().to_owned(),
        )
        .await?;
    Ok(())
}

/// Authority to undo one actual publication, never a project/filename guess.
/// The inode/metadata stamp also distinguishes a later equal-byte occupant.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
    created: Option<std::time::SystemTime>,
    #[cfg(unix)]
    inode: (u64, u64),
}
fn file_stamp(path: &Path) -> anyhow::Result<FileStamp> {
    let meta = std::fs::symlink_metadata(path)?;
    stamp_metadata(&meta)
}
fn stamp_metadata(meta: &std::fs::Metadata) -> anyhow::Result<FileStamp> {
    if !meta.is_file() || meta.file_type().is_symlink() {
        bail!("Publication is no longer a regular file");
    }
    Ok(FileStamp {
        len: meta.len(),
        modified: meta.modified().ok(),
        created: meta.created().ok(),
        #[cfg(unix)]
        inode: {
            use std::os::unix::fs::MetadataExt;
            (meta.dev(), meta.ino())
        },
    })
}
pub(super) struct FileReceipt {
    root: PathBuf,
    destination: PathBuf,
    kind: AddonType,
    name: String,
    enabled: bool,
    digest: String,
    stamp: FileStamp,
    previous: Option<(PathBuf, PathBuf, String, FileStamp, mfcdb::ModFileCacheRow)>,
}
pub(super) enum PublicationReceipt {
    File(FileReceipt),
    World(WorldReceipt),
}
impl PublicationReceipt {
    pub async fn validate_commit(&self) -> anyhow::Result<()> {
        match self {
            Self::File(receipt) => receipt.validate_commit().await,
            Self::World(receipt) => receipt.validate_commit().await,
        }
    }
    pub async fn commit(&self) -> anyhow::Result<()> {
        match self {
            Self::File(receipt) => receipt.commit().await,
            Self::World(_) => Ok(()),
        }
    }
}
pub(super) type ReceiptSlot = Arc<Mutex<Option<PublicationReceipt>>>;

impl FileReceipt {
    async fn still_owned(&self) -> anyhow::Result<bool> {
        Ok(
            file_stamp(&self.destination).ok().as_ref() == Some(&self.stamp)
                && file_digest(&self.destination).await? == self.digest,
        )
    }
    pub async fn validate_commit(&self) -> anyhow::Result<()> {
        if !self.still_owned().await? {
            bail!("Published addon changed before dependency graph completion; recovery retained");
        }
        if let Some((backup, _, digest, stamp, _)) = &self.previous {
            if file_stamp(backup).ok().as_ref() != Some(stamp)
                || file_digest(backup).await? != *digest
            {
                bail!("Replacement recovery changed; retained");
            }
        }
        Ok(())
    }
    pub async fn commit(&self) -> anyhow::Result<()> {
        if let Some((backup, _, digest, stamp, _)) = &self.previous {
            if file_stamp(backup).ok().as_ref() != Some(stamp)
                || file_digest(backup).await? != *digest
            {
                bail!(
                    "Replacement recovery changed; retained at {}",
                    backup.display()
                );
            }
            tokio::fs::remove_file(backup).await?;
            // The update is committed: its attempt's folder is empty now.
            remove_empty_staging(backup.parent()).await;
        }
        Ok(())
    }
}
/// Leaves a dependency this attempt published installed instead of removing
/// it, as 2.0.40 always did: another install that ran on the instance at the
/// same time may rely on it (see [`super::LiveGraphs`]). Only a new file: a
/// replaced version still goes back. Returns whether it kept one.
pub(super) async fn keep_new_dependency(slot: &ReceiptSlot) -> bool {
    let mut receipt = slot.lock().await;
    match receipt.as_ref() {
        Some(PublicationReceipt::File(proof)) if proof.previous.is_none() => {
            tracing::info!(
                file = %proof.destination.display(),
                "Keeping a dependency another install on this instance may rely on"
            );
            *receipt = None;
            true
        }
        _ => false,
    }
}

pub(super) async fn rollback_receipt(
    app: &Arc<AppInner>,
    id: InstanceId,
    slot: &ReceiptSlot,
) -> anyhow::Result<()> {
    let mut receipt = slot.lock().await;
    let Some(proof) = receipt.as_ref() else {
        return Ok(());
    };
    let proof = match proof {
        PublicationReceipt::File(proof) => proof,
        PublicationReceipt::World(proof) => {
            proof.rollback(app, id).await?;
            *receipt = None;
            return Ok(());
        }
    };
    let manager = app.instance_manager();
    let operation = manager.lock_instance(id).await;
    let current = current_path(app, id).await?;
    if current.get_root() != proof.root || !proof.still_owned().await? {
        bail!("Published addon changed ownership; retained new bytes and replacement recovery");
    }
    let folder = checked_folder(&current, proof.kind).await?;
    let Some((physical, enabled)) = selected_file(&folder, &proof.name).await? else {
        bail!("Published addon disappeared; recovery retained");
    };
    if physical != proof.destination || enabled != proof.enabled {
        bail!("Published addon representation changed; recovery retained");
    }
    if let Some((backup, original, digest, stamp, _)) = &proof.previous {
        if file_stamp(backup).ok().as_ref() != Some(stamp) || file_digest(backup).await? != *digest
        {
            bail!("Replacement backup changed; recovery retained");
        }
        if original != &proof.destination && addon_metadata(original).await?.is_some() {
            bail!("Original addon location has a later occupant; recovery retained");
        }
    }
    // Gate-held checks and exact receipt are the deletion authority. A
    // replaced version with another name goes back first, so a failure between
    // the two steps never leaves the instance with neither version; under the
    // same name the new file has to make room for it first.
    let restore_first = proof
        .previous
        .as_ref()
        .is_some_and(|(_, original, ..)| *original != proof.destination);
    if let Some((backup, original, ..)) = proof.previous.as_ref().filter(|_| restore_first) {
        move_noclobber(backup, original)
            .await
            .with_context(|| format!("Original addon recovery retained at {}", backup.display()))?;
    }
    tokio::fs::remove_file(&proof.destination)
        .await
        .with_context(|| match &proof.previous {
            Some((_, original, ..)) if restore_first => format!(
                "The earlier version is back at {}, but the new file {} could not be removed. \
                 Delete the new file in your file manager.",
                original.display(),
                proof.destination.display()
            ),
            Some((backup, ..)) => format!(
                "The new file {} could not be removed; the earlier version is kept at {}",
                proof.destination.display(),
                backup.display()
            ),
            None => format!(
                "The new file {} could not be removed",
                proof.destination.display()
            ),
        })?;
    if let Some((backup, original, _, _, row)) = &proof.previous {
        if !restore_first {
            move_noclobber(backup, original).await.with_context(|| {
                format!("Original addon recovery retained at {}", backup.display())
            })?;
        }
        // Put back: the attempt's folder is empty now.
        remove_empty_staging(backup.parent()).await;
        app.meta_cache_manager()
            .refresh_published_addon(
                &operation,
                &folder,
                row.filename.clone(),
                proof.enabled,
                row.addon_type.clone(),
            )
            .await?;
        let restored = mfcdb::get_mod_file_cache_by_instance_filename(&app.db, *id, &row.filename)
            .await?
            .context("Restored addon cache row missing")?;
        if restored.id != row.id
            && mfcdb::restore_mod_file_cache_identity(
                &app.db,
                &restored.id,
                &row.id,
                *id,
                &row.filename,
                &row.addon_type,
                carbon_repos::dbtypes::DbDateTime(chrono::Utc::now().fixed_offset()),
            )
            .await?
                != 1
        {
            bail!("Original cache identity is occupied; restored bytes retained");
        }
    }
    if proof
        .previous
        .as_ref()
        .is_none_or(|(_, _, _, _, row)| row.filename != proof.name)
    {
        if let Some(row) =
            mfcdb::get_mod_file_cache_by_instance_filename(&app.db, *id, &proof.name).await?
        {
            if row.addon_type == proof.kind.to_db_string() {
                mfcdb::delete_mod_file_cache_by_id(&app.db, &row.id).await?;
            }
        }
    }
    *receipt = None;
    app.invalidate(INSTANCE_MODS, Some(id.0.into()));
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DirectoryStamp {
    created: Option<std::time::SystemTime>,
    #[cfg(unix)]
    inode: (u64, u64),
    /// Volume and file index. A folder made again at the same path can carry
    /// the old one's creation time (same clock tick, or NTFS name tunneling),
    /// so the time alone cannot tell it is another folder.
    #[cfg(windows)]
    file_id: u128,
}
fn directory_stamp(path: &Path) -> anyhow::Result<DirectoryStamp> {
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        bail!("World directory ownership changed");
    }
    Ok(DirectoryStamp {
        created: meta.created().ok(),
        #[cfg(unix)]
        inode: {
            use std::os::unix::fs::MetadataExt;
            (meta.dev(), meta.ino())
        },
        #[cfg(windows)]
        file_id: super::super::addon_retry::link_identity(path, &meta)?,
    })
}
/// One save a world publication made: every folder and file it placed, each
/// stamped once complete. Recorded as they were placed, never inferred by
/// scanning a save after publication.
#[derive(Clone)]
struct WorldTree {
    destination: PathBuf,
    /// The save folder first, then each folder before what it holds.
    directories: Vec<(PathBuf, DirectoryStamp)>,
    files: Vec<(PathBuf, FileStamp)>,
}

impl WorldTree {
    /// Whether the save is still exactly what was published. Stamps (size,
    /// modified and created time, device and inode on Unix, and each folder's
    /// volume and file index on Windows) show any edit, such as Minecraft
    /// opening the world, without reading a large map again.
    fn validate(&self) -> anyhow::Result<()> {
        let edited = || {
            anyhow!(
                "The new save {} was opened or changed after it was installed, so it was kept",
                self.destination.display()
            )
        };
        for (path, stamp) in &self.directories {
            if directory_stamp(path).ok().as_ref() != Some(stamp) {
                return Err(edited());
            }
        }
        for (path, stamp) in &self.files {
            if file_stamp(path).ok().as_ref() != Some(stamp) {
                return Err(edited());
            }
        }
        let known = self
            .directories
            .iter()
            .map(|(path, _)| path.as_path())
            .chain(self.files.iter().map(|(path, _)| path.as_path()))
            .collect::<std::collections::HashSet<_>>();
        let mut observed = 0;
        for entry in walkdir::WalkDir::new(&self.destination) {
            let entry = entry?;
            observed += 1;
            if !known.contains(entry.path()) {
                return Err(edited());
            }
        }
        if observed != known.len() {
            return Err(edited());
        }
        Ok(())
    }
}

/// Authority to undo one complete world publication: the saves it made.
pub(super) struct WorldReceipt {
    root: PathBuf,
    trees: Vec<WorldTree>,
}
impl WorldReceipt {
    async fn validate_commit(&self) -> anyhow::Result<()> {
        let trees = self.trees.clone();
        tokio::task::spawn_blocking(move || trees.iter().try_for_each(WorldTree::validate))
            .await
            .context("Checking the new save stopped unexpectedly")?
    }
    async fn rollback(&self, app: &Arc<AppInner>, id: InstanceId) -> anyhow::Result<()> {
        let manager = app.instance_manager();
        let _operation = manager.lock_instance(id).await;
        let path = current_path(app, id).await?;
        if path.get_root() != self.root {
            bail!("World instance root changed; save retained");
        }
        let saves = checked_folder(&path, AddonType::Worlds).await?;
        if self
            .trees
            .iter()
            .any(|tree| tree.destination.parent() != Some(saves.as_path()))
        {
            bail!("World receipt has another destination");
        }
        self.validate_commit().await?;
        for (path, _, _) in &self.files {
            tokio::fs::remove_file(path).await?;
        }
        for (path, _) in self.directories.iter().rev() {
            tokio::fs::remove_dir(path).await?;
        }
        let name = self.destination.file_name().unwrap().to_string_lossy();
        if let Some(row) =
            mfcdb::get_mod_file_cache_by_instance_filename(&app.db, *id, &name).await?
        {
            if row.addon_type == AddonType::Worlds.to_db_string() {
                mfcdb::delete_mod_file_cache_by_id(&app.db, &row.id).await?;
            }
        }
        app.invalidate(INSTANCE_MODS, Some(id.0.into()));
        Ok(())
    }
}
