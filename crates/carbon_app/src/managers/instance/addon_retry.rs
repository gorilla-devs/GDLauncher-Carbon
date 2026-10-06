//! Owned file-operation boundary. Caller holds the runtime admission and instance gate.
use super::mods::{
    InvalidInstanceModIdError, addon_metadata, ensure_toggleable, remove_addon_from_disk,
    validate_addon_basename,
};
use crate::domain::instance::{AddonType, InstanceId};
use anyhow::{anyhow, bail};
use carbon_repos::{db_exec::Db, dbtypes::DbDateTime, repos::mod_file_cache as repo};
use carbon_rt_path::InstancePath;
use chrono::Utc;
use std::future::Future;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Disable,
    Delete,
}

impl Action {
    /// What the action does to one copy, for [`twin_copies_refusal`].
    fn verb(self) -> &'static str {
        match self {
            Action::Disable => "disable",
            Action::Delete => "remove",
        }
    }
}

/// What a plain [`execute`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Done,
    /// Delete removed the copy the row showed and kept its twin (`name`
    /// beside `name.disabled`), which the row now shows.
    KeptTwin(KeptTwin),
}

/// The copy a Delete of two copies left in place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct KeptTwin {
    pub folder: std::path::PathBuf,
    /// The row's name, without `.disabled`.
    pub filename: String,
    pub addon_type: String,
    /// Whether the copy left is the enabled spelling.
    pub enabled: bool,
}

impl KeptTwin {
    /// The file names of the copy removed and the copy left.
    pub(super) fn names(&self) -> (String, String) {
        let disabled = format!("{}.disabled", self.filename);
        if self.enabled {
            (disabled, self.filename.clone())
        } else {
            (self.filename.clone(), disabled)
        }
    }
}
pub(super) async fn execute<F: Future<Output = ()>>(
    db: &Db,
    instance: InstanceId,
    id: &str,
    path: &InstancePath,
    action: Action,
    after_effect: impl FnOnce() -> F,
) -> anyhow::Result<()> {
    let row = repo::get_mod_file_cache_by_id(db, id)
        .await?
        .filter(|row| row.instance_id == instance.0)
        .ok_or_else(|| InvalidInstanceModIdError(instance, id.to_owned()))?;
    let kind =
        AddonType::from_db_string(&row.addon_type).ok_or_else(|| anyhow!("unknown addon kind"))?;
    validate_addon_basename(&row.filename)?;
    if action == Action::Disable {
        ensure_toggleable(kind)?;
    }
    let folder = super::installer::publication::checked_folder(path, kind).await?;
    let enabled = folder.join(&row.filename);
    let disabled = folder.join(format!("{}.disabled", row.filename));
    match action {
        Action::Disable => {
            if disabled.exists() {
                bail!("mod is already disabled")
            }
            if !enabled.exists() {
                bail!("mod does not exist on disk")
            }
            tokio::fs::rename(&enabled, &disabled).await?;
            after_effect().await;
            repo::update_mod_file_enabled(db, id, false, DbDateTime(Utc::now().fixed_offset()))
                .await?;
        }
        Action::Delete => {
            remove_addon_from_disk(&enabled, &disabled, kind == AddonType::Worlds).await?;
            after_effect().await;
            for p in [&enabled, &disabled]
                .into_iter()
                .take(if kind == AddonType::Worlds { 1 } else { 2 })
            {
                if addon_metadata(p).await?.is_some() {
                    bail!(
                        "refusing to drop addon cache row: content remains at {}",
                        p.display()
                    )
                }
            }
            repo::delete_mod_file_cache_by_id(db, id).await?;
        }
    }
    Ok(())
}

use sha2::{Digest, Sha512};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant, SystemTime},
};
use tokio::io::AsyncReadExt;
const TTL: Duration = Duration::from_secs(600);
const CAPACITY: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Physical {
    identity: u128,
    size: u64,
    modified: SystemTime,
    hash: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct DirectoryStamp {
    identity: u128,
    modified: SystemTime,
    changed: (i64, i64),
}
#[derive(Clone, Debug)]
struct Receipt {
    created: Instant,
    action: Action,
    row: repo::ModFileCacheRow,
    root: PathBuf,
    root_identity: u128,
    folder: PathBuf,
    folder_identity: u128,
    before: Physical,
    effect: Option<Effect>,
}
#[derive(Clone, Debug)]
enum Effect {
    Disabled(Physical),
    Removed(DirectoryStamp),
}
#[derive(Debug, Default)]
pub(super) struct Receipts(Mutex<HashMap<(i32, String), Receipt>>);
impl Receipts {
    fn get(&self, instance: InstanceId, token: &str) -> anyhow::Result<Option<Receipt>> {
        let mut map = self
            .0
            .lock()
            .map_err(|_| anyhow!("retry receipt lock poisoned"))?;
        map.retain(|_, v| v.created.elapsed() < TTL);
        Ok(map.get(&(instance.0, token.to_owned())).cloned())
    }
    fn put(&self, instance: InstanceId, token: &str, receipt: Receipt) -> anyhow::Result<()> {
        let mut map = self
            .0
            .lock()
            .map_err(|_| anyhow!("retry receipt lock poisoned"))?;
        map.retain(|_, v| v.created.elapsed() < TTL);
        let key = (instance.0, token.to_owned());
        if !map.contains_key(&key) && map.len() >= CAPACITY {
            if let Some(oldest) = map
                .iter()
                .min_by_key(|(_, v)| v.created)
                .map(|(k, _)| k.clone())
            {
                map.remove(&oldest);
            }
        }
        map.insert(key, receipt);
        Ok(())
    }
}

fn open_owned(path: &std::path::Path) -> anyhow::Result<std::fs::File> {
    let meta = std::fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || (!meta.is_file() && !meta.is_dir()) {
        bail!("retry refuses linked or special artifact");
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x02000000 | 0x00200000);
    }
    let file = options.open(path)?;
    let current = std::fs::symlink_metadata(path)?;
    if current.file_type().is_symlink() || current.is_file() != file.metadata()?.is_file() {
        bail!("artifact changed while opening");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let opened = file.metadata()?;
        if (current.dev(), current.ino()) != (opened.dev(), opened.ino()) {
            bail!("artifact changed while opening");
        }
    }
    Ok(file)
}
fn file_identity(file: &std::fs::File) -> anyhow::Result<u128> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let m = file.metadata()?;
        Ok(((m.dev() as u128) << 64) | m.ino() as u128)
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        #[repr(C)]
        struct Info {
            attributes: u32,
            created: [u32; 2],
            accessed: [u32; 2],
            written: [u32; 2],
            volume: u32,
            size_high: u32,
            size_low: u32,
            links: u32,
            index_high: u32,
            index_low: u32,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetFileInformationByHandle(handle: *mut std::ffi::c_void, info: *mut Info) -> i32;
        }
        let mut info = std::mem::MaybeUninit::<Info>::uninit();
        // The live owned handle and correctly sized Windows BY_HANDLE_FILE_INFORMATION
        // buffer remain valid for the entire call; no path is trusted as identity.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let info = unsafe { info.assume_init() };
        Ok(((info.volume as u128) << 64)
            | ((info.index_high as u128) << 32)
            | info.index_low as u128)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = file;
        bail!("retry file identity is unsupported on this platform")
    }
}
fn directory_stamp(path: &std::path::Path) -> anyhow::Result<DirectoryStamp> {
    let file = open_owned(path)?;
    let meta = file.metadata()?;
    if !meta.is_dir() {
        bail!("retry folder is not a directory");
    }
    #[cfg(unix)]
    let changed = {
        use std::os::unix::fs::MetadataExt;
        (meta.ctime(), meta.ctime_nsec())
    };
    #[cfg(not(unix))]
    let changed = (0, 0);
    Ok(DirectoryStamp {
        identity: file_identity(&file)?,
        modified: meta.modified()?,
        changed,
    })
}
async fn physical(path: &std::path::Path) -> anyhow::Result<Physical> {
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return link_physical(path);
    }
    let file = open_owned(path)?;
    let before = file.metadata()?;
    if !before.is_file() {
        bail!("retry only supports regular file addons");
    }
    let identity = file_identity(&file)?;
    let mut file = tokio::fs::File::from_std(file);
    let mut hash = Sha512::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    let after = file.metadata().await?;
    let current = open_owned(path)?;
    if file_identity(&current)? != identity
        || before.len() != after.len()
        || before.modified()? != after.modified()?
        || current.metadata()?.modified()? != after.modified()?
    {
        bail!("artifact changed while fingerprinting");
    }
    Ok(Physical {
        identity,
        size: after.len(),
        modified: after.modified()?,
        hash: hash.finalize().to_vec(),
    })
}
/// A file addon that is a link to a jar or zip kept elsewhere (shared between
/// instances) is the link entry itself: its identity, stamp and target text.
/// Disable renames the link and Delete removes it, never what it points to,
/// so the proof never reads the target. A link must still point to a file,
/// as for an addon the scanner could read.
fn link_physical(path: &std::path::Path) -> anyhow::Result<Physical> {
    let before = std::fs::symlink_metadata(path)?;
    let target = std::fs::read_link(path)?;
    if !before.file_type().is_symlink() || !std::fs::metadata(path).is_ok_and(|m| m.is_file()) {
        bail!("retry refuses a link that does not point to a file");
    }
    let identity = link_identity(path, &before)?;
    let after = std::fs::symlink_metadata(path)?;
    if !after.file_type().is_symlink()
        || link_identity(path, &after)? != identity
        || std::fs::read_link(path)? != target
        || before.modified()? != after.modified()?
    {
        bail!("artifact changed while fingerprinting");
    }
    let mut hash = Sha512::new();
    hash.update(b"link\0");
    hash.update(target.as_os_str().as_encoded_bytes());
    Ok(Physical {
        identity,
        size: after.len(),
        modified: after.modified()?,
        hash: hash.finalize().to_vec(),
    })
}
/// The identity of a link entry, not of what it points to.
pub(super) fn link_identity(
    path: &std::path::Path,
    meta: &std::fs::Metadata,
) -> anyhow::Result<u128> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let _ = path;
        Ok(((meta.dev() as u128) << 64) | meta.ino() as u128)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let _ = meta;
        // Attribute access with FILE_FLAG_BACKUP_SEMANTICS and
        // FILE_FLAG_OPEN_REPARSE_POINT opens the link itself, as lstat does.
        let mut options = std::fs::OpenOptions::new();
        options.access_mode(0);
        options.custom_flags(0x02000000 | 0x00200000);
        file_identity(&options.open(path)?)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, meta);
        bail!("retry file identity is unsupported on this platform")
    }
}
fn same_row(a: &repo::ModFileCacheRow, b: &repo::ModFileCacheRow) -> bool {
    a.id == b.id
        && a.instance_id == b.instance_id
        && a.filename == b.filename
        && a.addon_type == b.addon_type
        && a.metadata_id == b.metadata_id
        && a.filesize == b.filesize
}
async fn verify_bookkeeping(
    db: &Db,
    instance: InstanceId,
    receipt: &Receipt,
) -> anyhow::Result<()> {
    let current = repo::get_mod_file_cache_by_id(db, &receipt.row.id).await?;
    if current
        .as_ref()
        .is_some_and(|row| !same_row(row, &receipt.row))
    {
        bail!("retry cache row ownership changed");
    }
    if current.is_none() && receipt.action == Action::Disable {
        bail!("retry disable cache row vanished");
    }
    for row in repo::get_mod_files_by_instance(db, instance.0).await? {
        if row.addon_type == receipt.row.addon_type
            && (row.filename == receipt.row.filename
                || row.filename == format!("{}.disabled", receipt.row.filename))
            && row.id != receipt.row.id
        {
            bail!("retry path has another cache owner");
        }
    }
    Ok(())
}
async fn verify_effect(receipt: &Receipt) -> anyhow::Result<()> {
    let a = receipt.folder.join(&receipt.row.filename);
    let b = receipt
        .folder
        .join(format!("{}.disabled", receipt.row.filename));
    match receipt
        .effect
        .as_ref()
        .ok_or_else(|| anyhow!("retry has no positive effect receipt"))?
    {
        Effect::Disabled(expected) => {
            if addon_metadata(&a).await?.is_some() {
                bail!("retry enabled path reappeared");
            }
            if physical(&b).await? != *expected {
                bail!("retry disabled artifact was replaced or changed");
            }
        }
        Effect::Removed(stamp) => {
            if addon_metadata(&a).await?.is_some() || addon_metadata(&b).await?.is_some() {
                bail!("retry removed artifact path was replaced");
            }
            // The folder's stamp proves the absence was continuous only to the
            // filesystem's timestamp tick (about 4 ms on Linux, 15.6 ms on
            // Windows): a file created and deleted again inside the same tick
            // as the removal leaves the stamp unchanged. Listing the folder
            // again would not see that either, since the file is gone by then.
            if directory_stamp(&receipt.folder)? != *stamp {
                bail!("retry removal absence is no longer continuous");
            }
        }
    }
    Ok(())
}

/// Optional bounded proof. Missing disk/cache state alone never authorizes this route.
pub(super) async fn execute_with_token<F: Future<Output = ()>>(
    db: &Db,
    receipts: &Receipts,
    instance: InstanceId,
    id: &str,
    path: &InstancePath,
    action: Action,
    token: &str,
    after_effect: impl FnOnce() -> F,
) -> anyhow::Result<()> {
    if token.len() != 36 || uuid::Uuid::parse_str(token).is_err() {
        bail!("invalid addon operation token");
    }
    let mut receipt = if let Some(receipt) = receipts.get(instance, token)? {
        if receipt.action != action || receipt.row.id != id {
            bail!("operation token belongs to another artifact/action");
        }
        receipt
    } else {
        let row = repo::get_mod_file_cache_by_id(db, id)
            .await?
            .filter(|r| r.instance_id == instance.0)
            .ok_or_else(|| InvalidInstanceModIdError(instance, id.to_owned()))?;
        let kind = AddonType::from_db_string(&row.addon_type)
            .ok_or_else(|| anyhow!("unknown addon kind"))?;
        validate_addon_basename(&row.filename)?;
        if kind == AddonType::Worlds {
            bail!("retry receipts do not support world directories");
        }
        if action == Action::Disable {
            ensure_toggleable(kind)?;
        }
        let folder = super::installer::publication::checked_folder(path, kind).await?;
        let a = folder.join(&row.filename);
        let b = folder.join(format!("{}.disabled", row.filename));
        let ma = addon_metadata(&a).await?;
        let mb = addon_metadata(&b).await?;
        if ma.is_some() && mb.is_some() {
            // A proof names one artifact, so this path never picks one.
            return Err(twin_copies_refusal(&folder, &row.filename, action.verb()));
        }
        let selected = if ma.is_some() {
            &a
        } else if mb.is_some() && action == Action::Delete {
            &b
        } else {
            bail!("no owned first-execution artifact; no positive retry receipt");
        };
        // The instance or its addon folder may be linked elsewhere
        // (`publication::followed_folder`): the proof binds the folders the
        // links resolve to.
        let root = tokio::fs::canonicalize(path.get_root()).await?;
        let resolved = tokio::fs::canonicalize(&folder).await?;
        let receipt = Receipt {
            created: Instant::now(),
            action,
            row,
            root_identity: directory_stamp(&root)?.identity,
            root,
            folder_identity: directory_stamp(&resolved)?.identity,
            folder: resolved,
            before: physical(selected).await?,
            effect: None,
        };
        receipts.put(instance, token, receipt.clone())?;
        receipt
    };
    let kind = AddonType::from_db_string(&receipt.row.addon_type)
        .ok_or_else(|| anyhow!("unknown addon kind"))?;
    let current_root = tokio::fs::canonicalize(path.get_root()).await?;
    let current_folder =
        tokio::fs::canonicalize(super::installer::publication::checked_folder(path, kind).await?)
            .await?;
    if current_root != receipt.root
        || directory_stamp(&current_root)?.identity != receipt.root_identity
        || current_folder != receipt.folder
        || directory_stamp(&current_folder)?.identity != receipt.folder_identity
    {
        bail!("retry instance root/folder ownership changed");
    }
    verify_bookkeeping(db, instance, &receipt).await?;
    if receipt.effect.is_none() {
        let a = receipt.folder.join(&receipt.row.filename);
        let b = receipt
            .folder
            .join(format!("{}.disabled", receipt.row.filename));
        let ma = addon_metadata(&a).await?;
        let mb = addon_metadata(&b).await?;
        if ma.is_some() && mb.is_some() {
            return Err(twin_copies_refusal(
                &kind.get_folder_path(path),
                &receipt.row.filename,
                action.verb(),
            ));
        }
        let selected = if ma.is_some() {
            &a
        } else if mb.is_some() && action == Action::Delete {
            &b
        } else {
            bail!("first execution artifact changed");
        };
        if physical(selected).await? != receipt.before {
            bail!("first execution artifact changed");
        }
        receipt.effect = Some(match action {
            Action::Disable => {
                tokio::fs::rename(&a, &b).await?;
                let observed = physical(&b).await?;
                if observed != receipt.before {
                    bail!("disabled effect is not the captured artifact");
                }
                Effect::Disabled(observed)
            }
            Action::Delete => {
                if !remove_addon_from_disk(&a, &b, false).await? {
                    bail!("no positively completed removal");
                }
                if addon_metadata(&a).await?.is_some() || addon_metadata(&b).await?.is_some() {
                    bail!("content remains after removal");
                }
                Effect::Removed(directory_stamp(&receipt.folder)?)
            }
        });
        receipt.created = Instant::now();
        receipts.put(instance, token, receipt.clone())?;
        after_effect().await;
    }
    verify_effect(&receipt).await?;
    verify_bookkeeping(db, instance, &receipt).await?;
    match action {
        Action::Disable => {
            repo::update_mod_file_enabled(db, id, false, DbDateTime(Utc::now().fixed_offset()))
                .await?;
        }
        Action::Delete => {
            repo::delete_mod_file_cache_by_id(db, id).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod task32_tests {
    use super::*;
    use carbon_repos::db_exec::WriteAccess;
    async fn fixture() -> (tempfile::TempDir, Db, InstancePath) {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("db.sqlite"), 1, false).unwrap();
        db.write(|c| { c.execute_batch("CREATE TABLE ModFileCache(fileStamp TEXT,id TEXT PRIMARY KEY,lastUpdatedAt DATETIME,instanceId INTEGER,filename TEXT,filesize INTEGER,enabled INTEGER,addonType TEXT,metadataId TEXT); INSERT INTO ModFileCache VALUES(NULL,'owned','2026-10-06T00:00:00+00:00',7,'pack.jar',4,1,'mods','metadata');")?; Ok(()) }).await.unwrap();
        let path = InstancePath::new(dir.path().join("owned-root"));
        tokio::fs::create_dir_all(path.get_mods_path())
            .await
            .unwrap();
        tokio::fs::write(path.get_mods_path().join("pack.jar"), b"safe")
            .await
            .unwrap();
        (dir, db, path)
    }
    #[tokio::test]
    async fn late_disable_db_fault_retries_owned_effect() {
        let (_dir, db, path) = fixture().await;
        let receipts = Receipts::default();
        let token = uuid::Uuid::new_v4().to_string();
        db.write(|c| { c.execute_batch("CREATE TRIGGER fail BEFORE UPDATE ON ModFileCache BEGIN SELECT RAISE(ABORT,'injected bookkeeping fault'); END;")?; Ok(()) }).await.unwrap();
        assert!(
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Disable,
                &token,
                || async {}
            )
            .await
            .is_err()
        );
        assert_eq!(
            tokio::fs::read(path.get_mods_path().join("pack.jar.disabled"))
                .await
                .unwrap(),
            b"safe"
        );
        db.write(|c| {
            c.execute_batch("DROP TRIGGER fail;")?;
            Ok(())
        })
        .await
        .unwrap();
        assert!(
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Disable,
                &token,
                || async {}
            )
            .await
            .is_ok(),
            "retry must reconcile its proven earlier effect instead of rejecting already disabled"
        );
        assert!(
            !repo::get_mod_file_cache_by_id(&db, "owned")
                .await
                .unwrap()
                .unwrap()
                .enabled
        );
    }
    #[tokio::test]
    async fn deletion_lost_ack_retries_positive_effect() {
        let (_dir, db, path) = fixture().await;
        let receipts = Receipts::default();
        let token = uuid::Uuid::new_v4().to_string();
        execute_with_token(
            &db,
            &receipts,
            InstanceId(7),
            "owned",
            &path,
            Action::Delete,
            &token,
            || async {},
        )
        .await
        .unwrap();
        assert!(
            repo::get_mod_file_cache_by_id(&db, "owned")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Delete,
                &token,
                || async {}
            )
            .await
            .is_ok(),
            "replay requires positive removal ownership rather than fresh missing-row failure"
        );
    }
    #[tokio::test]
    async fn normal_owned_operations_and_foreign_row_control() {
        let (_dir, db, path) = fixture().await;
        assert!(
            execute(
                &db,
                InstanceId(8),
                "owned",
                &path,
                Action::Delete,
                || async {}
            )
            .await
            .is_err()
        );
        assert_eq!(
            tokio::fs::read(path.get_mods_path().join("pack.jar"))
                .await
                .unwrap(),
            b"safe"
        );
        execute(
            &db,
            InstanceId(7),
            "owned",
            &path,
            Action::Disable,
            || async {},
        )
        .await
        .unwrap();
        execute(
            &db,
            InstanceId(7),
            "owned",
            &path,
            Action::Delete,
            || async {},
        )
        .await
        .unwrap();
        assert!(
            repo::get_mod_file_cache_by_id(&db, "owned")
                .await
                .unwrap()
                .is_none()
        );
    }
    #[tokio::test]
    async fn late_delete_db_fault_and_disable_lost_ack_are_reconciled() {
        for action in [Action::Delete, Action::Disable] {
            let (_dir, db, path) = fixture().await;
            let receipts = Receipts::default();
            let token = uuid::Uuid::new_v4().to_string();
            if action == Action::Delete {
                db.write(|c|{c.execute_batch("CREATE TRIGGER fail BEFORE DELETE ON ModFileCache BEGIN SELECT RAISE(ABORT,'delete fault'); END;")?;Ok(())}).await.unwrap();
            }
            let first = execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                action,
                &token,
                || async {},
            )
            .await;
            if action == Action::Delete {
                assert!(first.is_err());
                db.write(|c| {
                    c.execute_batch("DROP TRIGGER fail;")?;
                    Ok(())
                })
                .await
                .unwrap();
            } else {
                first.unwrap();
            }
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                action,
                &token,
                || async {},
            )
            .await
            .unwrap();
        }
    }
    #[tokio::test]
    async fn replacement_bytes_and_same_bytes_new_inode_are_rejected() {
        for bytes in [b"safe".as_slice(), b"foreign".as_slice()] {
            let (_dir, db, path) = fixture().await;
            let receipts = Receipts::default();
            let token = uuid::Uuid::new_v4().to_string();
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Disable,
                &token,
                || async {},
            )
            .await
            .unwrap();
            let target = path.get_mods_path().join("pack.jar.disabled");
            let replacement = path.get_mods_path().join("new");
            tokio::fs::write(&replacement, bytes).await.unwrap();
            tokio::fs::rename(&replacement, &target).await.unwrap();
            assert!(
                execute_with_token(
                    &db,
                    &receipts,
                    InstanceId(7),
                    "owned",
                    &path,
                    Action::Disable,
                    &token,
                    || async {}
                )
                .await
                .is_err()
            );
            assert_eq!(tokio::fs::read(target).await.unwrap(), bytes);
        }
    }
    async fn with_token(db: &Db, receipts: &Receipts, path: &InstancePath, action: Action) {
        let token = uuid::Uuid::new_v4().to_string();
        for _ in 0..2 {
            // The second call is a retry of an answer that got lost.
            execute_with_token(
                db,
                receipts,
                InstanceId(7),
                "owned",
                path,
                action,
                &token,
                || async {},
            )
            .await
            .unwrap();
        }
    }
    /// Duplicate resolution (the only caller with a token) works when the
    /// instance or its mods folder is linked elsewhere, as people do to move
    /// an instance to another drive or share mods; the proof binds the
    /// folders the links resolve to.
    async fn assert_token_actions_work_through_linked_folders(
        link_dir: fn(&std::path::Path, &std::path::Path),
    ) {
        for linked in ["root", "mods"] {
            let (dir, db, path) = fixture().await;
            let receipts = Receipts::default();
            let link = if linked == "root" {
                path.get_root()
            } else {
                path.get_mods_path()
            };
            let target = dir.path().join("elsewhere");
            tokio::fs::rename(&link, &target).await.unwrap();
            link_dir(&link, &target);
            let mods = if linked == "root" {
                target.join("instance/mods")
            } else {
                target.clone()
            };
            with_token(&db, &receipts, &path, Action::Disable).await;
            assert_eq!(
                tokio::fs::read(mods.join("pack.jar.disabled"))
                    .await
                    .unwrap(),
                b"safe"
            );
            with_token(&db, &receipts, &path, Action::Delete).await;
            assert!(!mods.join("pack.jar.disabled").exists());
            assert!(
                repo::get_mod_file_cache_by_id(&db, "owned")
                    .await
                    .unwrap()
                    .is_none()
            );
            assert!(
                std::fs::symlink_metadata(&link)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        }
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn linked_instance_and_mods_folders_disable_and_delete_with_a_token() {
        assert_token_actions_work_through_linked_folders(|link, target| {
            std::os::unix::fs::symlink(target, link).unwrap()
        })
        .await;
    }
    /// A junction (`mklink /J`), which needs no extra rights on Windows.
    #[cfg(windows)]
    fn junction(link: &std::path::Path, target: &std::path::Path) {
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed for {}", link.display());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn junctioned_instance_and_mods_folders_disable_and_delete_with_a_token() {
        assert_token_actions_work_through_linked_folders(junction).await;
    }
    /// A mod that is a link to a jar kept elsewhere is disabled by renaming
    /// the link and removed by removing the link; the jar stays.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_linked_mod_disables_and_deletes_only_the_link_with_a_token() {
        let (dir, db, path) = fixture().await;
        let receipts = Receipts::default();
        let shared = dir.path().join("shared.jar");
        tokio::fs::write(&shared, b"shared").await.unwrap();
        let enabled = path.get_mods_path().join("pack.jar");
        let disabled = path.get_mods_path().join("pack.jar.disabled");
        tokio::fs::remove_file(&enabled).await.unwrap();
        std::os::unix::fs::symlink(&shared, &enabled).unwrap();
        with_token(&db, &receipts, &path, Action::Disable).await;
        assert!(
            std::fs::symlink_metadata(&disabled)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(
            !repo::get_mod_file_cache_by_id(&db, "owned")
                .await
                .unwrap()
                .unwrap()
                .enabled
        );
        with_token(&db, &receipts, &path, Action::Delete).await;
        assert!(std::fs::symlink_metadata(&disabled).is_err());
        assert_eq!(tokio::fs::read(&shared).await.unwrap(), b"shared");
        assert!(
            repo::get_mod_file_cache_by_id(&db, "owned")
                .await
                .unwrap()
                .is_none()
        );
    }
    /// A link swapped in for the jar after the proof was taken is not the
    /// artifact the proof names.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_link_swapped_in_after_the_first_attempt_is_refused() {
        let (dir, db, path) = fixture().await;
        let receipts = Receipts::default();
        let token = uuid::Uuid::new_v4().to_string();
        db.write(|c| { c.execute_batch("CREATE TRIGGER fail BEFORE UPDATE ON ModFileCache BEGIN SELECT RAISE(ABORT,'injected bookkeeping fault'); END;")?; Ok(()) }).await.unwrap();
        let run = || {
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Disable,
                &token,
                || async {},
            )
        };
        assert!(run().await.is_err());
        db.write(|c| {
            c.execute_batch("DROP TRIGGER fail;")?;
            Ok(())
        })
        .await
        .unwrap();
        let disabled = path.get_mods_path().join("pack.jar.disabled");
        let moved = dir.path().join("moved.jar");
        tokio::fs::rename(&disabled, &moved).await.unwrap();
        std::os::unix::fs::symlink(&moved, &disabled).unwrap();
        assert!(run().await.is_err());
        assert_eq!(tokio::fs::read(&moved).await.unwrap(), b"safe");
        assert!(
            repo::get_mod_file_cache_by_id(&db, "owned")
                .await
                .unwrap()
                .unwrap()
                .enabled
        );
    }
    #[tokio::test]
    async fn both_spellings_links_and_io_ambiguity_refuse_without_bookkeeping() {
        for scenario in [0, 1, 2] {
            let (_dir, db, path) = fixture().await;
            let receipts = Receipts::default();
            let token = uuid::Uuid::new_v4().to_string();
            if scenario == 0 {
                tokio::fs::write(path.get_mods_path().join("pack.jar.disabled"), b"foreign")
                    .await
                    .unwrap();
            }
            if scenario == 1 {
                tokio::fs::remove_file(path.get_mods_path().join("pack.jar"))
                    .await
                    .unwrap();
                #[cfg(unix)]
                std::os::unix::fs::symlink("absent", path.get_mods_path().join("pack.jar"))
                    .unwrap();
                #[cfg(not(unix))]
                tokio::fs::create_dir(path.get_mods_path().join("pack.jar"))
                    .await
                    .unwrap();
            }
            if scenario == 2 {
                tokio::fs::remove_dir_all(path.get_mods_path())
                    .await
                    .unwrap();
                tokio::fs::write(path.get_mods_path(), b"not a directory")
                    .await
                    .unwrap();
            }
            let error = execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Delete,
                &token,
                || async {},
            )
            .await
            .unwrap_err();
            if scenario == 0 {
                // Both copies: the proof cannot name one, so this refuses
                // and says what to do.
                let disable = execute_with_token(
                    &db,
                    &receipts,
                    InstanceId(7),
                    "owned",
                    &path,
                    Action::Disable,
                    &uuid::Uuid::new_v4().to_string(),
                    || async {},
                )
                .await
                .unwrap_err();
                for (error, verb) in [(&error, "remove"), (&disable, "disable")] {
                    let text = format!("{error:#}");
                    assert!(
                        text.contains("Both pack.jar and pack.jar.disabled are in")
                            && text.contains(&format!("which copy to {verb}"))
                            && text.contains("Delete in the Addons tab removes one copy"),
                        "{text}"
                    );
                }
                for name in ["pack.jar", "pack.jar.disabled"] {
                    assert!(path.get_mods_path().join(name).is_file());
                }
            }
            assert!(
                repo::get_mod_file_cache_by_id(&db, "owned")
                    .await
                    .unwrap()
                    .is_some()
            );
        }
    }
    #[tokio::test]
    async fn delete_replacement_and_interrupted_absence_cannot_certify_success() {
        for remove_again in [false, true] {
            let (_dir, db, path) = fixture().await;
            let receipts = Receipts::default();
            let token = uuid::Uuid::new_v4().to_string();
            let file = path.get_mods_path().join("pack.jar");
            assert!(
                execute_with_token(
                    &db,
                    &receipts,
                    InstanceId(7),
                    "owned",
                    &path,
                    Action::Delete,
                    &token,
                    || async {
                        // A real external process cannot create and delete the
                        // jar inside the timestamp tick of the launcher's own
                        // removal; 50 ms is longer than the Linux (4 ms) and
                        // Windows (15.6 ms) ticks, so the folder stamp changes.
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        tokio::fs::write(&file, b"replacement").await.unwrap();
                        if remove_again {
                            tokio::fs::remove_file(&file).await.unwrap();
                        }
                    }
                )
                .await
                .is_err()
            );
            assert!(
                repo::get_mod_file_cache_by_id(&db, "owned")
                    .await
                    .unwrap()
                    .is_some()
            );
        }
    }
    #[tokio::test]
    async fn foreign_cache_owner_changed_row_and_changed_root_are_rejected() {
        for scenario in [0, 1, 2] {
            let (_dir, db, path) = fixture().await;
            let receipts = Receipts::default();
            let token = uuid::Uuid::new_v4().to_string();
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Delete,
                &token,
                || async {},
            )
            .await
            .unwrap();
            if scenario == 0 {
                db.write(|c|{c.execute_batch("INSERT INTO ModFileCache VALUES(NULL,'foreign',0,7,'pack.jar',4,1,'mods','metadata');")?;Ok(())}).await.unwrap();
            }
            if scenario == 1 {
                db.write(|c|{c.execute_batch("INSERT INTO ModFileCache VALUES(NULL,'owned',0,8,'pack.jar',4,1,'mods','other');")?;Ok(())}).await.unwrap();
            }
            if scenario == 2 {
                let old = path.get_root().with_extension("old");
                tokio::fs::rename(path.get_root(), &old).await.unwrap();
                tokio::fs::create_dir_all(path.get_mods_path())
                    .await
                    .unwrap();
            }
            assert!(
                execute_with_token(
                    &db,
                    &receipts,
                    InstanceId(7),
                    "owned",
                    &path,
                    Action::Delete,
                    &token,
                    || async {}
                )
                .await
                .is_err()
            );
        }
    }
    #[tokio::test]
    async fn tokens_bind_action_artifact_instance_and_refuse_invalid_input() {
        let (_dir, db, path) = fixture().await;
        let receipts = Receipts::default();
        let token = uuid::Uuid::new_v4().to_string();
        execute_with_token(
            &db,
            &receipts,
            InstanceId(7),
            "owned",
            &path,
            Action::Disable,
            &token,
            || async {},
        )
        .await
        .unwrap();
        assert!(
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Delete,
                &token,
                || async {}
            )
            .await
            .is_err()
        );
        assert!(
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "other",
                &path,
                Action::Disable,
                &token,
                || async {}
            )
            .await
            .is_err()
        );
        assert!(
            execute_with_token(
                &db,
                &receipts,
                InstanceId(8),
                "owned",
                &path,
                Action::Disable,
                &token,
                || async {}
            )
            .await
            .is_err()
        );
        assert!(
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Disable,
                &"x".repeat(4096),
                || async {}
            )
            .await
            .is_err()
        );
    }
    #[tokio::test]
    async fn missing_expired_and_evicted_proofs_never_use_absence_as_success() {
        let (_dir, db, path) = fixture().await;
        let receipts = Receipts::default();
        let token = uuid::Uuid::new_v4().to_string();
        execute_with_token(
            &db,
            &receipts,
            InstanceId(7),
            "owned",
            &path,
            Action::Delete,
            &token,
            || async {},
        )
        .await
        .unwrap();
        let receipt = receipts.get(InstanceId(7), &token).unwrap().unwrap();
        receipts
            .0
            .lock()
            .unwrap()
            .get_mut(&(7, token.clone()))
            .unwrap()
            .created = Instant::now() - TTL;
        assert!(
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Delete,
                &token,
                || async {}
            )
            .await
            .is_err()
        );
        receipts
            .put(InstanceId(7), &token, receipt.clone())
            .unwrap();
        for _ in 0..CAPACITY {
            let mut newer = receipt.clone();
            newer.created = Instant::now();
            receipts
                .put(InstanceId(7), &uuid::Uuid::new_v4().to_string(), newer)
                .unwrap();
        }
        assert_eq!(receipts.0.lock().unwrap().len(), CAPACITY);
        assert!(receipts.get(InstanceId(7), &token).unwrap().is_none());
        assert!(
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "owned",
                &path,
                Action::Delete,
                &token,
                || async {}
            )
            .await
            .is_err()
        );
    }
    #[tokio::test]
    async fn no_token_missing_file_and_literal_disabled_world_preserve_contracts() {
        let (_dir, db, path) = fixture().await;
        tokio::fs::remove_file(path.get_mods_path().join("pack.jar"))
            .await
            .unwrap();
        execute(
            &db,
            InstanceId(7),
            "owned",
            &path,
            Action::Delete,
            || async {},
        )
        .await
        .unwrap();
        db.write(|c|{c.execute_batch("INSERT INTO ModFileCache VALUES(NULL,'world',0,7,'World',0,1,'worlds','metadata');")?;Ok(())}).await.unwrap();
        tokio::fs::create_dir_all(path.get_saves_path().join("World"))
            .await
            .unwrap();
        tokio::fs::create_dir_all(path.get_saves_path().join("World.disabled"))
            .await
            .unwrap();
        tokio::fs::write(
            path.get_saves_path().join("World.disabled/valuable"),
            b"retained",
        )
        .await
        .unwrap();
        let receipts = Receipts::default();
        assert!(
            execute_with_token(
                &db,
                &receipts,
                InstanceId(7),
                "world",
                &path,
                Action::Delete,
                &uuid::Uuid::new_v4().to_string(),
                || async {}
            )
            .await
            .is_err()
        );
        assert!(
            execute(
                &db,
                InstanceId(7),
                "world",
                &path,
                Action::Disable,
                || async {}
            )
            .await
            .is_err()
        );
        execute(
            &db,
            InstanceId(7),
            "world",
            &path,
            Action::Delete,
            || async {},
        )
        .await
        .unwrap();
        assert_eq!(
            tokio::fs::read(path.get_saves_path().join("World.disabled/valuable"))
                .await
                .unwrap(),
            b"retained"
        );
    }
}
