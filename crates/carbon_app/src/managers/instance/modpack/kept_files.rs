//! Pack files a version change kept as the player has them although the new
//! version ships them changed, and the way to take the pack's instead.
//!
//! A version change never overwrites a pack file whose bytes changed since it
//! was installed (see `apply_plan::decide_version_change`). The launcher
//! cannot tell the player's edit from a config the game or a mod saved again,
//! so the new version's copy of such a file would otherwise be thrown away
//! with the rest of the setup, and only a full Repair could bring it back.
//!
//! Instead the apply sets the pack's copy of each such file aside
//! ([`set_aside`]) in the audit folder, which every apply replaces, and tells
//! the player which files those are. [`use_pack_versions`] then puts the
//! copies the player picks in place: no download, and nothing else in the
//! instance changes. The installed record already holds the pack's hashes for
//! these paths, so it needs no change either way.

use super::{
    apply_plan::{PlanAction, PlanEntry, PlanReason},
    origin_check::hash_file_sha512,
    packinfo::{self, PackInfo},
};
use crate::{
    domain::instance::InstanceId,
    managers::{
        ManagerRef,
        instance::{InstanceManager, InvalidInstanceIdError},
        minecraft::modrinth::secure_path_join,
    },
    util::recycle_bin::{self, Removal},
};
use anyhow::{Context, anyhow, bail};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Where the pack's copies wait, under the instance folder.
const SET_ASIDE: &str = ".install_audit/kept";

/// Files that are the player's own by nature (key binds, video settings, the
/// server list), which the game writes again on every launch. A pack ships
/// them only as a starting point, so they are never offered: taking the
/// pack's would reset the player's settings.
const PERSONAL: [&str; 2] = ["/options.txt", "/servers.dat"];

/// The record keys of the files `entries` kept as the player has them
/// although `target` ships them changed.
fn kept_by_update<'a>(entries: &'a [PlanEntry], target: &PackInfo) -> Vec<&'a str> {
    entries
        .iter()
        .filter(|entry| entry.action == PlanAction::Keep)
        .filter(|entry| match &entry.reason {
            // The pack's own change, not only the player's: a file the new
            // version ships as the old one did has nothing to offer.
            PlanReason::ModifiedByUser { original, .. } => target
                .files
                .get(&entry.path)
                .is_some_and(|hashes| hashes.md5 != *original),
            _ => false,
        })
        .map(|entry| entry.path.as_str())
        .filter(|key| !PERSONAL.iter().any(|own| key.eq_ignore_ascii_case(own)))
        .collect()
}

fn is_plain_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// Copies the staged pack file of every path `entries` kept (see
/// [`kept_by_update`]) into the audit folder of `root`, and returns those
/// paths, sorted, as `/`-separated paths inside the game folder.
///
/// Only a staged file that still is what `target` records is set aside. One
/// that isn't, or can't be copied, is left out: the update goes on, and the
/// player is told of no file the launcher can't deliver.
pub async fn set_aside(
    root: &Path,
    staged: &Path,
    entries: &[PlanEntry],
    target: &PackInfo,
) -> Vec<String> {
    let mut kept = Vec::new();
    for key in kept_by_update(entries, target) {
        let relative = key.trim_start_matches('/');
        let copied: anyhow::Result<()> = async {
            let source = secure_path_join(staged, relative)?;
            if !is_plain_file(&source) {
                bail!("the pack's copy is not staged under its own name");
            }
            if hash_file_sha512(&source).await? != target.files[key].sha512 {
                bail!("the staged copy is not what the pack ships");
            }
            let destination = secure_path_join(root.join(SET_ASIDE), relative)?;
            let parent = destination.parent().context("no parent")?;
            tokio::fs::create_dir_all(parent).await?;
            tokio::fs::copy(&source, &destination).await?;
            Ok(())
        }
        .await;
        match copied {
            Ok(()) => kept.push(relative.to_owned()),
            Err(error) => tracing::warn!(?error, path = key, "kept file not offered"),
        }
    }
    kept.sort();
    kept
}

/// Every folder from `data` down to the one holding `file` is a real folder,
/// not a link: the launcher does not write through links inside the game
/// folder.
fn plain_folders(data: &Path, file: &Path) -> anyhow::Result<()> {
    let mut folder = file.parent().context("no parent")?;
    while folder != data {
        match std::fs::symlink_metadata(folder) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            _ => bail!("its folder is a link or is no longer a folder"),
        }
        folder = folder.parent().context("outside the game folder")?;
    }
    Ok(())
}

/// One file the player asked the pack's copy of.
struct Wanted {
    path: String,
    set_aside: PathBuf,
    live: PathBuf,
}

/// Checks that `path` is a file the last update kept and whose pack copy is
/// still the one the installed `record` names.
async fn wanted(root: &Path, data: &Path, record: &PackInfo, path: &str) -> anyhow::Result<Wanted> {
    let hashes = record
        .files
        .get(&format!("/{path}"))
        .context("it is not one of the modpack's files")?;
    let set_aside = secure_path_join(root.join(SET_ASIDE), path)?;
    if !is_plain_file(&set_aside) {
        bail!("the modpack's version of it is no longer kept; use Repair to get it");
    }
    if hash_file_sha512(&set_aside).await? != hashes.sha512 {
        bail!("the kept copy is not the installed version's; use Repair to get it");
    }
    let live = secure_path_join(data, path)?;
    plain_folders(data, &live)?;
    match std::fs::symlink_metadata(&live) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => bail!("it is no longer a file"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut disabled = live.clone().into_os_string();
            disabled.push(".disabled");
            if std::fs::symlink_metadata(Path::new(&disabled)).is_ok() {
                bail!("it is switched off now; switch it on first");
            }
        }
        Err(error) => return Err(error.into()),
    }
    Ok(Wanted {
        path: path.to_owned(),
        set_aside,
        live,
    })
}

/// Puts the pack's copy of `file` in place of the player's, which `removal`
/// has already taken to the Recycle Bin or which is overwritten here.
async fn publish(file: &Wanted) -> anyhow::Result<()> {
    let source = file.set_aside.clone();
    let live = file.live.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let parent = live.parent().context("no parent")?;
        std::fs::create_dir_all(parent)?;
        let mut temporary = crate::util::publish::temp_in(parent)?;
        std::io::copy(&mut std::fs::File::open(&source)?, temporary.as_file_mut())?;
        crate::util::durable::sync_file(temporary.as_file())?;
        crate::util::publish::persist(temporary, &live)?;
        Ok(())
    })
    .await??;
    // Not offered twice. A copy that stays is replaced by the next apply.
    if let Err(error) = tokio::fs::remove_file(&file.set_aside).await {
        tracing::debug!(?error, "used pack copy kept");
    }
    Ok(())
}

impl ManagerRef<'_, InstanceManager> {
    /// The game folder and instance folder of an instance that is neither
    /// running nor being set up, for a change to its kept files. Requires the
    /// instance gate.
    async fn kept_files_roots(self, instance_id: InstanceId) -> anyhow::Result<(PathBuf, PathBuf)> {
        let (shortpath, _) = self.current_config(instance_id).await?;
        let inactive = matches!(
            self.instances
                .read()
                .await
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?
                .data()?
                .state,
            super::super::run::LaunchState::Inactive { .. }
        );
        if !inactive {
            bail!(
                "This instance is running or being installed. Close the game or wait for it to finish, then try again."
            );
        }
        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath);
        let root = path.get_root();
        if root.join(".setup").try_exists()? {
            bail!(
                "This instance has an install or update that didn't finish. Press Play to finish it, then try again."
            );
        }
        Ok((root, path.get_data_path()))
    }

    /// Replaces the player's copy of each of `paths` (as [`set_aside`]
    /// returned them) with the installed pack version's, which the last
    /// update set aside. The player's copy goes to the Recycle Bin when the
    /// "Deletion through Recycle Bin" setting is on, and is overwritten
    /// otherwise. Each path has its own result, in the order asked, so one
    /// that can't be replaced does not hide what happened to the others.
    pub async fn use_pack_versions(
        self,
        instance_id: InstanceId,
        paths: Vec<String>,
    ) -> anyhow::Result<Vec<(String, anyhow::Result<()>)>> {
        let _activity = self
            .app
            .runtime_lifecycle
            .admit(format!("instance {instance_id} kept files"))?;
        let _operation = self.lock_instance(instance_id).await;
        let (root, data) = self.kept_files_roots(instance_id).await?;
        let record = packinfo::Installed::read(&root)
            .await?
            .usable()
            .context("This instance has no record of its modpack's files. Use Repair instead.")?;
        let removal = Removal::from_setting(
            self.app
                .settings_manager()
                .get_settings()
                .await?
                .deletion_through_recycle_bin,
        );

        let asked: Vec<String> = paths
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut results = Vec::with_capacity(asked.len());
        let mut accepted = Vec::new();
        for path in asked {
            match wanted(&root, &data, &record, &path).await {
                Ok(file) => accepted.push(file),
                Err(error) => results.push((path, Err(error))),
            }
        }
        // The whole selection goes to the Recycle Bin in one request. A file
        // it refuses stays the player's.
        let binned = match removal {
            Removal::RecycleBin => {
                recycle_bin::move_all_to_recycle_bin(
                    accepted.iter().map(|file| file.live.clone()).collect(),
                )
                .await
            }
            Removal::Permanent => accepted.iter().map(|_| Ok(())).collect(),
        };
        for (file, binned) in accepted.into_iter().zip(binned) {
            let replaced = match binned {
                Ok(()) => publish(&file).await.map_err(|error| {
                    anyhow!("the modpack's version could not be put in place: {error:#}")
                }),
                Err(error) => Err(error),
            };
            results.push((file.path, replaced));
        }
        results.sort_by(|a, b| a.0.cmp(&b.0));

        // A replaced file may be a mod the library lists.
        self.app
            .meta_cache_manager()
            .queue_caching(
                crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
                false,
            )
            .await;
        self.app.invalidate(
            crate::api::keys::instance::INSTANCE_MODS,
            Some(instance_id.0.into()),
        );
        Ok(results)
    }

    /// Where the player's copy of the kept file `path` is, to show it in the
    /// file manager.
    pub async fn kept_file_location(
        self,
        instance_id: InstanceId,
        path: String,
    ) -> anyhow::Result<PathBuf> {
        let (shortpath, _) = self.current_config(instance_id).await?;
        let instance = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath);
        let record = packinfo::Installed::read(&instance.get_root())
            .await?
            .usable()
            .context("This instance has no record of its modpack's files.")?;
        if !record.files.contains_key(&format!("/{path}")) {
            bail!("{path} is not one of the modpack's files");
        }
        Ok(secure_path_join(instance.get_data_path(), &path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::instance::modpack::packinfo::FileHashes;
    use sha2::Digest;

    fn hashes(bytes: &[u8]) -> FileHashes {
        FileHashes {
            sha512: sha2::Sha512::digest(bytes).into(),
            md5: md5::Md5::digest(bytes).into(),
        }
    }

    fn md5_of(bytes: &[u8]) -> [u8; 16] {
        md5::Md5::digest(bytes).into()
    }

    fn kept(path: &str, original: &[u8]) -> PlanEntry {
        PlanEntry {
            path: path.into(),
            action: PlanAction::Keep,
            reason: PlanReason::ModifiedByUser {
                original: md5_of(original),
                current: md5_of(b"the player's"),
            },
        }
    }

    fn target(files: &[(&str, &[u8])]) -> PackInfo {
        PackInfo {
            files: files
                .iter()
                .map(|(path, bytes)| ((*path).to_owned(), hashes(bytes)))
                .collect(),
        }
    }

    /// Only a file kept because its bytes changed, and which the new version
    /// changed as well, is one the pack has something to offer for. The
    /// game's options and the server list are the player's own.
    #[test]
    fn only_kept_files_the_new_version_changed_are_offered() {
        let target = target(&[
            ("/config/changed.cfg", b"v2"),
            ("/config/same.cfg", b"v1"),
            ("/options.txt", b"v2"),
            ("/Servers.dat", b"v2"),
            ("/config/deleted.cfg", b"v2"),
            ("/config/replaced.cfg", b"v2"),
        ]);
        let entries = vec![
            kept("/config/changed.cfg", b"v1"),
            kept("/config/same.cfg", b"v1"),
            kept("/options.txt", b"v1"),
            kept("/Servers.dat", b"v1"),
            // Dropped by the new version: nothing to offer.
            PlanEntry {
                path: "/config/dropped.cfg".into(),
                action: PlanAction::Keep,
                reason: PlanReason::DroppedButModified {
                    original: md5_of(b"v1"),
                    current: md5_of(b"the player's"),
                },
            },
            PlanEntry {
                path: "/config/deleted.cfg".into(),
                action: PlanAction::Keep,
                reason: PlanReason::DeletedByUser,
            },
            PlanEntry {
                path: "/config/replaced.cfg".into(),
                action: PlanAction::Replace,
                reason: PlanReason::PackUpdate,
            },
        ];
        assert_eq!(kept_by_update(&entries, &target), ["/config/changed.cfg"]);
    }

    /// A staged copy that is not the bytes the pack ships, or that is staged
    /// switched off, is not set aside and not named.
    #[tokio::test]
    async fn only_a_staged_copy_the_pack_ships_is_set_aside() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let staged = dir.path().join("staged");
        std::fs::create_dir_all(staged.join("config/deep")).unwrap();
        std::fs::create_dir_all(staged.join("mods")).unwrap();
        std::fs::write(staged.join("config/deep/b.cfg"), b"v2").unwrap();
        std::fs::write(staged.join("config/a.cfg"), b"v2").unwrap();
        std::fs::write(staged.join("config/altered.cfg"), b"not v2").unwrap();
        std::fs::write(staged.join("mods/off.jar.disabled"), b"v2").unwrap();
        // A link in the staged pack, even to the right bytes, is not a copy
        // of the pack's.
        #[cfg(unix)]
        std::os::unix::fs::symlink(staged.join("config/a.cfg"), staged.join("config/link.cfg"))
            .unwrap();
        let target = target(&[
            ("/config/deep/b.cfg", b"v2"),
            ("/config/a.cfg", b"v2"),
            ("/config/altered.cfg", b"v2"),
            ("/config/unstaged.cfg", b"v2"),
            ("/config/link.cfg", b"v2"),
            ("/mods/off.jar", b"v2"),
        ]);
        let entries: Vec<_> = target.files.keys().map(|path| kept(path, b"v1")).collect();

        let named = set_aside(&root, &staged, &entries, &target).await;

        assert_eq!(named, ["config/a.cfg", "config/deep/b.cfg"]);
        let aside = root.join(SET_ASIDE);
        assert_eq!(std::fs::read(aside.join("config/a.cfg")).unwrap(), b"v2");
        assert_eq!(
            std::fs::read(aside.join("config/deep/b.cfg")).unwrap(),
            b"v2"
        );
        assert!(!aside.join("config/altered.cfg").exists());
        assert!(!aside.join("mods").exists());
        // The staged files stay for the rest of the setup to read.
        assert!(staged.join("config/a.cfg").is_file());
    }
}
