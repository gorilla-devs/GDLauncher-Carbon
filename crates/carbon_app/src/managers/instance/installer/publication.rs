//! Local addon publication. Provider acquisition stays outside the instance gate.
use super::super::mods::{addon_metadata, twin_copies_refusal, validate_addon_basename};
use super::*;
use crate::domain::instance::AddonType;
use anyhow::anyhow;
use sha2::{Digest, Sha512};
use std::path::Path;

pub(crate) const WORLD_INCOMPLETE: &str = ".gdl-world-install-incomplete";

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
        .strip_prefix("gdl-world-install-v1:")
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
        if let Some(meta) = addon_metadata(&entry).await? {
            if !meta.is_dir() || meta.file_type().is_symlink() {
                bail!(
                    "addon directory is not an owned directory: {}",
                    entry.display()
                );
            }
        }
    }
    Ok(folder)
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
        bail!("ambiguous enabled and disabled addon copies");
    }
    for (path, meta, enabled) in [(enabled, a, true), (disabled, b, false)] {
        if let Some(meta) = meta {
            if !meta.is_file() || meta.file_type().is_symlink() {
                bail!("addon is not a regular file");
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
    let mut bytes = [0u8; 65536];
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

async fn file_digest(path: &Path) -> anyhow::Result<String> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut digest = Sha512::new();
    let mut buffer = [0u8; 65536];
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
            return Ok(Self { row, original });
        }
        let (physical, _) = selected_file(&folder, &row.filename)
            .await?
            .context("replacement file is missing")?;
        Ok(Self {
            row,
            original: ReplacementOriginal::FileDigest(file_digest(&physical).await?),
        })
    }
}

/// Use the existing tempfile platform primitive: rename-without-replacement
/// where supported, with its no-clobber fallback. Failure retains the source.
fn move_noclobber(source: &Path, destination: &Path) -> std::io::Result<()> {
    let mut path = tempfile::TempPath::from_path(source.to_owned());
    path.disable_cleanup(true);
    path.persist_noclobber(destination)
        .map_err(|error| error.error)
}

pub(super) async fn publish_file(
    resource: &dyn ResourceInstaller,
    app: &Arc<AppInner>,
    id: InstanceId,
    staged: &Path,
    replacement: Option<&ReplacementPlan>,
    required: Option<&super::super::shader_loader::ShaderLoaderVerification>,
) -> anyhow::Result<()> {
    let manager = app.instance_manager();
    let _operation = manager.lock_instance(id).await;
    manager.ensure_modpack_not_locked(id).await?;
    if let Some(target) = required {
        target.validate_target(app, id).await?;
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
    // Same filesystem, no-clobber publication. Keep the original recoverable until
    // local cache finalization succeeds; failure never deletes new public bytes.
    let backup = staged
        .parent()
        .context("missing staging directory")?
        .join("previous-addon");
    if let Some((physical, _)) = &old {
        let ReplacementOriginal::FileDigest(digest) = &replacement
            .context("missing file replacement plan")?
            .original
        else {
            bail!("world copy plan cannot replace a file addon");
        };
        move_noclobber(physical, &backup)?;
        if file_digest(&backup).await? != *digest {
            // Keep the moved bytes recoverable, without overwriting a later arrival.
            let _ = move_noclobber(&backup, physical);
            bail!(
                "old addon changed at publication; recovery: {}",
                backup.display()
            );
        }
    }
    #[cfg(test)]
    manager.test_hooks.pause("addon_before_publish", id).await;
    let publish = move_noclobber(staged, &destination);
    if let Err(error) = publish {
        if let Some((physical, _)) = &old {
            let _ = move_noclobber(&backup, physical);
        }
        return Err(error).with_context(|| {
            format!(
                "publication failed; recovery (if needed): {}",
                backup.display()
            )
        });
    }
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
        tokio::fs::remove_file(&backup).await?;
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

/// Explicit world versions are new saves. The guard binds the current instance;
/// this plan never grants deletion authority over the mutable original world.
pub(super) async fn publish_world_version(
    resource: &dyn ResourceInstaller,
    app: &Arc<AppInner>,
    ownership: &crate::managers::instance::InstanceOperationGuard,
    staged: &Path,
    plan: &ReplacementPlan,
) -> anyhow::Result<PathBuf> {
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
    let name = filename(resource, &current)?;
    let default_name = Path::new(&name)
        .file_stem()
        .and_then(|name| name.to_str())
        .context("invalid world archive name")?;
    let download = resource
        .downloadable(&current)
        .await
        .context("world has no artifact")?;
    verify_acquired(staged, &download).await?;
    let prepared = prepare_world(staged, default_name).await?;
    #[cfg(test)]
    app.instance_manager()
        .test_hooks
        .pause("world_copy_before_reserve", id)
        .await;
    let reserved = prepared.reserve_copy(&saves).await?;
    let destination = reserved.destination.clone();
    reserved.finish().await?;
    Ok(destination)
}

pub(super) struct PreparedWorld {
    _extraction: tempfile::TempDir,
    source: PathBuf,
    name: String,
}

pub(super) struct ReservedWorld {
    prepared: PreparedWorld,
    pub(super) destination: PathBuf,
    marker: PathBuf,
    token: String,
}

pub(super) async fn prepare_world(
    archive: &Path,
    default_name: &str,
) -> anyhow::Result<PreparedWorld> {
    let extraction = tempfile::tempdir_in(archive.parent().context("archive has no parent")?)?;
    carbon_compression::decompress(archive, extraction.path()).await?;
    let root = extraction.path();
    let (source, name) = if root.join("level.dat").is_file() {
        validate_addon_basename(default_name)?;
        (root.to_owned(), default_name.to_owned())
    } else {
        let mut roots = Vec::new();
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.path().join("level.dat").is_file() {
                roots.push(entry.path());
            } else {
                bail!("world archive must contain exactly one save root");
            }
        }
        if roots.len() != 1 {
            bail!("world archive is empty or has ambiguous save roots");
        }
        let source = roots.remove(0);
        let name = source
            .file_name()
            .and_then(|s| s.to_str())
            .context("invalid world name")?
            .to_owned();
        validate_addon_basename(&name)?;
        (source, name)
    };
    for entry in walkdir::WalkDir::new(&source).min_depth(1) {
        let entry = entry?;
        if entry.file_type().is_symlink()
            || (!entry.file_type().is_dir() && !entry.file_type().is_file())
        {
            bail!("unsupported world entry");
        }
        if entry.path() != source.join("level.dat") && entry.file_name() == "level.dat" {
            bail!("ambiguous nested save root");
        }
        if entry.file_name() == WORLD_INCOMPLETE {
            bail!("reserved world installation marker");
        }
    }
    Ok(PreparedWorld {
        _extraction: extraction,
        source,
        name,
    })
}

impl PreparedWorld {
    pub(super) async fn reserve(self, saves: &Path) -> anyhow::Result<ReservedWorld> {
        self.reserve_with_policy(saves, false).await
    }

    pub(super) async fn reserve_copy(self, saves: &Path) -> anyhow::Result<ReservedWorld> {
        self.reserve_with_policy(saves, true).await
    }

    async fn reserve_with_policy(self, saves: &Path, copy: bool) -> anyhow::Result<ReservedWorld> {
        use tokio::io::AsyncWriteExt;
        tokio::fs::create_dir_all(saves).await?;
        let mut reserved = None;
        for suffix in 1..=100 {
            let name = if copy {
                format!("{} (copy {suffix})", self.name)
            } else {
                self.name.clone()
            };
            let destination = saves.join(name);
            match tokio::fs::create_dir(&destination).await {
                Ok(()) => {
                    reserved = Some(destination);
                    break;
                }
                Err(error) if copy && error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "world destination already exists or cannot be reserved: {}",
                            destination.display()
                        )
                    });
                }
            }
        }
        let destination = reserved.context(
            "no free world copy destination among 100 candidates; existing saves retained",
        )?;
        let marker = destination.join(WORLD_INCOMPLETE);
        let token = format!("gdl-world-install-v1:{}", uuid::Uuid::new_v4());
        let result = async {
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&marker)
                .await?;
            file.write_all(token.as_bytes()).await?;
            file.sync_all().await?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if let Err(error) = result {
            // Remove only an empty reservation; never recursively delete later content.
            let _ = tokio::fs::remove_dir(&destination).await;
            return Err(error)
                .with_context(|| format!("world reservation failed at {}", destination.display()));
        }
        Ok(ReservedWorld {
            prepared: self,
            destination,
            marker,
            token,
        })
    }
}

impl ReservedWorld {
    pub(super) async fn finish(self) -> anyhow::Result<()> {
        let Self {
            prepared,
            destination,
            marker,
            token,
        } = self;
        let source = &prepared.source;
        // No Drop cleanup: interruption/error retains the marked public tree.
        let result = async {
            for entry in walkdir::WalkDir::new(&source).min_depth(1) {
                let entry = entry?;
                let target = destination.join(entry.path().strip_prefix(source)?);
                if entry.file_type().is_dir() {
                    tokio::fs::create_dir(&target).await?;
                } else {
                    let mut from = tokio::fs::File::open(entry.path()).await?;
                    let mut to = tokio::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(&target)
                        .await?;
                    tokio::io::copy(&mut from, &mut to).await?;
                    to.sync_all().await?;
                }
            }
            if tokio::fs::read_to_string(&marker).await? != token {
                bail!("world marker ownership changed");
            }
            tokio::fs::remove_file(&marker).await?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        result.with_context(|| {
            format!(
                "incomplete world retained at {}; inspect it before retrying",
                destination.display()
            )
        })
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
    move_noclobber(&physical, &destination)?;
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
