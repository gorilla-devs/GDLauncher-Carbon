//! Finished pieces of world maps, kept on disk across restarts so a world is painted once
//! until it is played again. A piece is given back only for the revision it was painted
//! for, and the store stays under a size limit by dropping the least recently used pieces.
//!
//! Everything here is blocking file work, for `spawn_blocking`. A cache problem never fails
//! the caller: a piece that cannot be read is not there, and what cannot be forgotten is
//! logged.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::SystemTime;

use sha2::{Digest, Sha256};
use tracing::debug;

use crate::domain::instance::InstanceId;

/// The most the store holds before the least recently used pieces go.
pub(crate) const STORE_LIMIT: u64 = 512 * 1024 * 1024;

/// The longest name a folder of the store spells out in hex; a longer one goes by its
/// digest, so a path stays within what Windows allows.
const LONGEST_PART: usize = 200;

/// What a piece is kept by.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct PieceKey<'a> {
    pub instance: InstanceId,
    /// The world's folder in `saves`.
    pub world: &'a str,
    pub dimension: &'a str,
    pub scale: u8,
    pub x: i32,
    pub z: i32,
    pub revision: &'a str,
}

/// Kept as `<root>/<instance>/<world>/<dimension>/<scale>/<x>.<z>.<revision>.png`, the
/// world and the dimension in hex so no name can reach outside its folder.
#[derive(Debug)]
pub(crate) struct Store {
    root: PathBuf,
    limit: u64,
    /// The bytes the store holds: counted by a walk on first use, then kept up to date by
    /// what the store itself writes and removes. `None` until counted, and again after the
    /// folder was emptied from outside. Held while the store changes.
    total: Mutex<Option<u64>>,
}

impl Store {
    /// A store kept in `root`, which is made by the first `put`.
    pub(crate) fn new(root: PathBuf, limit: u64) -> Store {
        Store {
            root,
            limit,
            total: Mutex::new(None),
        }
    }

    /// The piece kept for `key`, only for its revision. It is then the most recently used.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn get(&self, key: &PieceKey) -> Option<Vec<u8>> {
        let (folder, name) = self.place(key)?;
        let path = folder.join(name);
        let png = fs::read(&path).ok()?;
        if let Ok(file) = fs::File::options().write(true).open(&path) {
            let _ = file.set_modified(SystemTime::now());
        }
        Some(png)
    }

    /// Keeps `png` for `key`, in place of any other revision of the same piece, then drops
    /// the least recently used pieces if the store holds more than its limit.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn put(&self, key: &PieceKey, png: &[u8]) -> io::Result<()> {
        let (folder, name) = self.place(key).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "not a revision of a map piece")
        })?;
        let mut total = self.lock();
        // Counted before the piece is written, so it is not counted twice.
        let mut bytes = self.counted(&mut total);
        fs::create_dir_all(&folder)?;
        let path = folder.join(&name);
        let replaced = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        write_new(&folder, &name, png)?;
        bytes = bytes.saturating_sub(replaced) + png.len() as u64;

        let same_piece = format!("{}.{}.", key.x, key.z);
        if let Ok(entries) = fs::read_dir(&folder) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let Some(other) = file_name.to_str() else {
                    continue;
                };
                if other == name || !other.starts_with(&same_piece) || !other.ends_with(".png") {
                    continue;
                }
                let len = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
                match fs::remove_file(entry.path()) {
                    Ok(()) => bytes = bytes.saturating_sub(len),
                    Err(error) => debug!(?error, "could not remove the map piece {other}"),
                }
            }
        }
        if bytes > self.limit {
            bytes = self.trim(&path);
        }
        *total = Some(bytes);
        Ok(())
    }

    /// Drops every piece kept for `world` of `instance`.
    pub(crate) fn forget_world(&self, instance: InstanceId, world: &str) {
        let mut total = self.lock();
        let folder = self.instance_folder(instance).join(part(world));
        self.forget_folder(&mut total, &folder);
    }

    /// Drops the pieces of every world of `instance` whose folder name is not in `existing`.
    pub(crate) fn forget_missing_worlds(&self, instance: InstanceId, existing: &[String]) {
        let mut total = self.lock();
        let folder = self.instance_folder(instance);
        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return,
            Err(error) => {
                debug!(
                    ?error,
                    "could not list the kept maps of instance {instance}"
                );
                return;
            }
        };
        let kept: HashSet<String> = existing.iter().map(|world| part(world)).collect();
        for entry in entries.flatten() {
            let missing = entry
                .file_name()
                .to_str()
                .is_none_or(|name| !kept.contains(name));
            if missing {
                self.forget_folder(&mut total, &entry.path());
            }
        }
    }

    /// Counts the store again on next use, for after its folder was emptied from outside.
    pub(crate) fn forget_all(&self) {
        *self.lock() = None;
    }

    /// The bytes the store has counted, `None` when it will count again on next use.
    #[cfg(test)]
    pub(crate) fn counted_bytes(&self) -> Option<u64> {
        *self.lock()
    }

    fn lock(&self) -> MutexGuard<'_, Option<u64>> {
        self.total
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn counted(&self, total: &mut Option<u64>) -> u64 {
        *total.get_or_insert_with(|| pieces_below(&self.root).iter().map(|piece| piece.1).sum())
    }

    fn instance_folder(&self, instance: InstanceId) -> PathBuf {
        self.root.join(instance.0.to_string())
    }

    /// The folder and file name of `key`; `None` for a revision that is not one.
    fn place(&self, key: &PieceKey) -> Option<(PathBuf, String)> {
        let revision = key.revision;
        let is_revision = (1..=64).contains(&revision.len())
            && revision
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
        if !is_revision {
            return None;
        }
        let folder = self
            .instance_folder(key.instance)
            .join(part(key.world))
            .join(part(key.dimension))
            .join(key.scale.to_string());
        Some((folder, format!("{}.{}.{revision}.png", key.x, key.z)))
    }

    fn forget_folder(&self, total: &mut Option<u64>, folder: &Path) {
        // Not a folder of the store's own (a stray file, a link): removed as what it is,
        // never followed, and not counted.
        match fs::symlink_metadata(folder) {
            Ok(meta) if !meta.is_dir() => {
                if let Err(error) = fs::remove_file(folder).or_else(|_| fs::remove_dir(folder)) {
                    debug!(?error, "could not remove {}", folder.display());
                }
                return;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return,
            _ => {}
        }
        let bytes = total.is_some().then(|| size_below(folder));
        match fs::remove_dir_all(folder) {
            Ok(()) => {
                if let (Some(total), Some(bytes)) = (total.as_mut(), bytes) {
                    *total = total.saturating_sub(bytes);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                debug!(?error, "could not forget the kept map {}", folder.display());
                // Some of it may be gone: count again on next use.
                *total = None;
            }
        }
    }

    /// Drops the least recently used pieces until the store holds at most nine tenths of
    /// its limit, so the next few pieces do not each start a walk. Returns what is left.
    /// `keep`, the piece just put, is never dropped, whatever its time says.
    fn trim(&self, keep: &Path) -> u64 {
        let mut pieces = pieces_below(&self.root);
        pieces.sort_by_key(|piece| piece.2);
        let mut bytes: u64 = pieces.iter().map(|piece| piece.1).sum();
        let goal = self.limit / 10 * 9;
        for (path, len, _) in pieces {
            if bytes <= goal {
                break;
            }
            if path == keep {
                continue;
            }
            match fs::remove_file(&path) {
                Ok(()) => bytes -= len,
                Err(error) => debug!(?error, "could not drop the map piece {}", path.display()),
            }
        }
        bytes
    }
}

/// The folder name for `name`: its hex, or the digest of a name too long for that.
fn part(name: &str) -> String {
    let spelled = hex::encode(name);
    if spelled.is_empty() || spelled.len() > LONGEST_PART {
        format!("h-{}", &hex::encode(Sha256::digest(name))[..64])
    } else {
        spelled
    }
}

/// Writes `png` as `folder/name` through a file of its own, so a reader never sees half a
/// piece. Marked used now, to the nanosecond, so pieces made in a row keep their order.
fn write_new(folder: &Path, name: &str, png: &[u8]) -> io::Result<()> {
    static WRITES: AtomicU64 = AtomicU64::new(0);
    let temporary = folder.join(format!(
        ".{name}.{}-{}.tmp",
        std::process::id(),
        WRITES.fetch_add(1, Ordering::Relaxed)
    ));
    let written = (|| {
        let mut file = fs::File::create_new(&temporary)?;
        file.write_all(png)?;
        let _ = file.set_modified(SystemTime::now());
        drop(file);
        fs::rename(&temporary, folder.join(name))
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    written
}

/// Every file below `folder` with its size and when it was last used. Links are not
/// followed.
fn pieces_below(folder: &Path) -> Vec<(PathBuf, u64, SystemTime)> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                folders.push(entry.path());
            } else if kind.is_file()
                && let Ok(meta) = entry.metadata()
            {
                let used = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                found.push((entry.path(), meta.len(), used));
            }
        }
    }
    found
}

fn size_below(folder: &Path) -> u64 {
    pieces_below(folder).iter().map(|piece| piece.1).sum()
}

/// A piece of `world` in the overworld, for tests of what forgets maps.
#[cfg(test)]
pub(crate) fn test_piece(instance: InstanceId, world: &str) -> PieceKey<'_> {
    PieceKey {
        instance,
        world,
        dimension: "minecraft:overworld",
        scale: 0,
        x: 0,
        z: 0,
        revision: "aa11",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const PIECE: usize = 1000;

    fn store_in(name: &str, limit: u64) -> (tempfile::TempDir, Store) {
        let folder = tempfile::Builder::new()
            .prefix(&format!("world-map-store-{name}-"))
            .tempdir()
            .unwrap();
        let store = Store::new(folder.path().join("world_maps"), limit);
        (folder, store)
    }

    fn key<'a>(world: &'a str, x: i32, revision: &'a str) -> PieceKey<'a> {
        PieceKey {
            instance: InstanceId(1),
            world,
            dimension: "minecraft:overworld",
            scale: 0,
            x,
            z: 0,
            revision,
        }
    }

    fn piece(byte: u8) -> Vec<u8> {
        vec![byte; PIECE]
    }

    /// Every file below `folder`, as paths relative to it.
    fn files(folder: &Path) -> Vec<String> {
        let mut found = Vec::new();
        if folder.exists() {
            for entry in walkdir::WalkDir::new(folder) {
                let entry = entry.unwrap();
                if entry.file_type().is_file() {
                    let relative = entry.path().strip_prefix(folder).unwrap();
                    found.push(relative.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        found.sort();
        found
    }

    fn size_of(folder: &Path) -> u64 {
        walkdir::WalkDir::new(folder)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.metadata().unwrap().len())
            .sum()
    }

    #[test]
    fn a_kept_piece_comes_back_only_for_its_revision() {
        let (_folder, store) = store_in("revision", STORE_LIMIT);
        store.put(&key("World", 0, "aa11"), &piece(1)).unwrap();

        assert_eq!(store.get(&key("World", 0, "aa11")), Some(piece(1)));
        assert_eq!(store.get(&key("World", 0, "bb22")), None);
        assert_eq!(store.get(&key("World", 1, "aa11")), None);
        assert_eq!(store.get(&key("Other", 0, "aa11")), None);
    }

    #[test]
    fn a_new_revision_replaces_the_old_file() {
        let (_folder, store) = store_in("replace", STORE_LIMIT);
        store.put(&key("World", 0, "aa11"), &piece(1)).unwrap();
        store.put(&key("World", 1, "aa11"), &piece(3)).unwrap();
        store.put(&key("World", 0, "bb22"), &piece(2)).unwrap();

        assert_eq!(store.get(&key("World", 0, "aa11")), None);
        assert_eq!(store.get(&key("World", 0, "bb22")), Some(piece(2)));
        assert_eq!(store.get(&key("World", 1, "aa11")), Some(piece(3)));
        let world = hex::encode("World");
        let dimension = hex::encode("minecraft:overworld");
        assert_eq!(
            files(&store.root),
            [
                format!("1/{world}/{dimension}/0/0.0.bb22.png"),
                format!("1/{world}/{dimension}/0/1.0.aa11.png"),
            ]
        );
        assert_eq!(store.counted_bytes(), Some(2 * PIECE as u64));
    }

    #[test]
    fn the_store_stays_under_its_limit_dropping_the_least_recently_used() {
        let (_folder, store) = store_in("limit", 3 * PIECE as u64);
        for x in 0..3 {
            store
                .put(&key("World", x, "aa11"), &piece(x as u8))
                .unwrap();
        }
        // Used again: now the most recently used.
        assert_eq!(store.get(&key("World", 0, "aa11")), Some(piece(0)));
        for x in 3..5 {
            store
                .put(&key("World", x, "aa11"), &piece(x as u8))
                .unwrap();
            assert!(size_of(&store.root) <= 3 * PIECE as u64);
        }

        assert_eq!(store.get(&key("World", 0, "aa11")), Some(piece(0)));
        assert_eq!(store.get(&key("World", 1, "aa11")), None);
        assert_eq!(store.get(&key("World", 4, "aa11")), Some(piece(4)));
        assert_eq!(store.counted_bytes(), Some(size_of(&store.root)));
    }

    #[test]
    fn what_was_kept_before_a_restart_counts_toward_the_limit() {
        let (folder, store) = store_in("restart", 3 * PIECE as u64);
        for x in 0..3 {
            store
                .put(&key("World", x, "aa11"), &piece(x as u8))
                .unwrap();
        }
        drop(store);
        let store = Store::new(folder.path().join("world_maps"), 3 * PIECE as u64);

        store.put(&key("World", 3, "aa11"), &piece(3)).unwrap();

        assert!(size_of(&store.root) <= 3 * PIECE as u64);
        assert_eq!(store.get(&key("World", 0, "aa11")), None);
        assert_eq!(store.get(&key("World", 3, "aa11")), Some(piece(3)));
    }

    #[test]
    fn an_emptied_store_counts_again_from_what_is_there() {
        let (_folder, store) = store_in("emptied", 3 * PIECE as u64);
        for x in 0..3 {
            store
                .put(&key("World", x, "aa11"), &piece(x as u8))
                .unwrap();
        }
        std::fs::remove_dir_all(&store.root).unwrap();

        store.forget_all();
        assert_eq!(store.counted_bytes(), None);
        for x in 3..6 {
            store
                .put(&key("World", x, "aa11"), &piece(x as u8))
                .unwrap();
        }

        for x in 3..6 {
            assert_eq!(store.get(&key("World", x, "aa11")), Some(piece(x as u8)));
        }
    }

    #[test]
    fn a_world_is_forgotten() {
        let (_folder, store) = store_in("forget", STORE_LIMIT);
        store.put(&key("World", 0, "aa11"), &piece(1)).unwrap();
        store.put(&key("World", 1, "aa11"), &piece(1)).unwrap();
        store.put(&key("Other", 0, "aa11"), &piece(2)).unwrap();
        let elsewhere = PieceKey {
            instance: InstanceId(2),
            ..key("World", 0, "aa11")
        };
        store.put(&elsewhere, &piece(3)).unwrap();

        store.forget_world(InstanceId(1), "World");

        assert_eq!(store.get(&key("World", 0, "aa11")), None);
        assert_eq!(store.get(&key("World", 1, "aa11")), None);
        assert_eq!(store.get(&key("Other", 0, "aa11")), Some(piece(2)));
        assert_eq!(store.get(&elsewhere), Some(piece(3)));
        assert_eq!(store.counted_bytes(), Some(2 * PIECE as u64));
        // A world never kept is nothing to forget.
        store.forget_world(InstanceId(7), "Never");
    }

    #[test]
    fn missing_worlds_are_forgotten_and_others_kept() {
        let (_folder, store) = store_in("missing", STORE_LIMIT);
        for world in ["Kept", "Gone", "Also gone"] {
            store.put(&key(world, 0, "aa11"), &piece(1)).unwrap();
        }
        let elsewhere = PieceKey {
            instance: InstanceId(2),
            ..key("Gone", 0, "aa11")
        };
        store.put(&elsewhere, &piece(3)).unwrap();

        store.forget_missing_worlds(InstanceId(1), &["Kept".to_owned(), "New".to_owned()]);

        assert_eq!(store.get(&key("Kept", 0, "aa11")), Some(piece(1)));
        assert_eq!(store.get(&key("Gone", 0, "aa11")), None);
        assert_eq!(store.get(&key("Also gone", 0, "aa11")), None);
        assert_eq!(store.get(&elsewhere), Some(piece(3)));
        assert_eq!(store.counted_bytes(), Some(2 * PIECE as u64));
        // An instance with nothing kept is nothing to forget.
        store.forget_missing_worlds(InstanceId(7), &[]);
    }

    #[test]
    fn odd_names_are_safe() {
        let (folder, store) = store_in("odd", STORE_LIMIT);
        let odd = PieceKey {
            dimension: "../mod:sky/../../x",
            ..key("..", -3, "aa11")
        };
        store.put(&odd, &piece(1)).unwrap();

        assert_eq!(store.get(&odd), Some(piece(1)));
        assert_eq!(
            files(&store.root),
            [format!(
                "1/{}/{}/0/-3.0.aa11.png",
                hex::encode(".."),
                hex::encode("../mod:sky/../../x")
            )]
        );
        assert_eq!(files(folder.path()).len(), 1, "nothing outside the store");

        // A revision is a word of letters, digits and dashes; anything else is no piece.
        for revision in ["../x", "a/b", "a.b", ""] {
            let bad = key("World", 0, revision);
            assert!(store.put(&bad, &piece(1)).is_err(), "{revision:?}");
            assert_eq!(store.get(&bad), None);
        }
        assert_eq!(files(folder.path()).len(), 1);
    }

    #[test]
    fn long_names_are_kept_under_a_digest() {
        let (_folder, store) = store_in("long", STORE_LIMIT);
        let digest = |name: &str| {
            use sha2::{Digest, Sha256};
            format!("h-{}", &hex::encode(Sha256::digest(name.as_bytes()))[..64])
        };
        // 100 bytes spell out in 200 hex characters, the most a part may have.
        let longest = "w".repeat(100);
        let longer = "w".repeat(101);
        let dimension = format!("mod:{}", "d".repeat(150));
        let spelled = PieceKey {
            dimension: &dimension,
            ..key(&longest, 0, "aa11")
        };
        let digested = key(&longer, 0, "aa11");
        store.put(&spelled, &piece(1)).unwrap();
        store.put(&digested, &piece(2)).unwrap();

        assert_eq!(store.get(&spelled), Some(piece(1)));
        assert_eq!(store.get(&digested), Some(piece(2)));
        assert_eq!(
            files(&store.root),
            [
                format!(
                    "1/{}/{}/0/0.0.aa11.png",
                    hex::encode(&longest),
                    digest(&dimension)
                ),
                format!(
                    "1/{}/{}/0/0.0.aa11.png",
                    digest(&longer),
                    hex::encode("minecraft:overworld")
                ),
            ]
        );

        store.forget_world(InstanceId(1), &longer);
        assert_eq!(store.get(&digested), None);
        assert_eq!(store.get(&spelled), Some(piece(1)));
    }

    #[test]
    fn the_piece_just_put_survives_a_trim_when_every_time_ties() {
        let (_folder, store) = store_in("ties", 3 * PIECE as u64);
        for x in 0..3 {
            store
                .put(&key("World", x, "aa11"), &piece(x as u8))
                .unwrap();
        }
        // Everything older is "newer" than the piece about to be put, as on a file system
        // whose times are too coarse to tell pieces put in a row apart.
        let later = SystemTime::now() + std::time::Duration::from_secs(86_400);
        for (path, _, _) in pieces_below(&store.root) {
            fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_modified(later)
                .unwrap();
        }

        store.put(&key("World", 3, "aa11"), &piece(3)).unwrap();

        assert_eq!(store.get(&key("World", 3, "aa11")), Some(piece(3)));
        assert!(size_of(&store.root) <= 3 * PIECE as u64);
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_world_folder_is_removed_without_following_it() {
        let (folder, store) = store_in("link", STORE_LIMIT);
        store.put(&key("Other", 0, "aa11"), &piece(2)).unwrap();
        let outside = folder.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("big"), vec![0u8; 5 * PIECE]).unwrap();
        let link = store.instance_folder(InstanceId(1)).join(part("Linked"));
        std::os::unix::fs::symlink(&outside, &link).unwrap();

        store.forget_world(InstanceId(1), "Linked");

        assert!(fs::symlink_metadata(&link).is_err());
        assert!(outside.join("big").exists());
        assert_eq!(store.counted_bytes(), Some(PIECE as u64));
    }

    #[test]
    fn a_stray_file_does_not_make_the_store_count_again() {
        let (_folder, store) = store_in("stray", STORE_LIMIT);
        store.put(&key("Kept", 0, "aa11"), &piece(1)).unwrap();
        let stray = store.instance_folder(InstanceId(1)).join(".DS_Store");
        fs::write(&stray, b"x").unwrap();

        store.forget_missing_worlds(InstanceId(1), &["Kept".to_owned()]);

        assert!(!stray.exists());
        assert_eq!(store.get(&key("Kept", 0, "aa11")), Some(piece(1)));
        assert_eq!(store.counted_bytes(), Some(PIECE as u64));
    }
}
