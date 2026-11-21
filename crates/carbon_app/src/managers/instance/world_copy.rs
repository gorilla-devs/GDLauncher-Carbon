//! The file work behind duplicating a world: survey a world folder, copy it
//! into a staging folder with progress and cancel, notice that the original
//! changed meanwhile, and read how much room a drive has. Nothing here writes
//! to the original world.
use anyhow::Context;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio_util::sync::CancellationToken;

use super::world_refusal::WorldRefusal;

/// Files are copied in pieces this big, so progress and cancel are seen often.
const CHUNK: usize = 1024 * 1024;

/// The game's lock on a world; it is rewritten whenever the world is open, so
/// it is neither copied nor counted as a change.
const SESSION_LOCK: &str = "session.lock";

/// What a world folder holds: its files and folders as paths relative to it,
/// each folder before its content.
pub(super) struct Survey {
    pub bytes: u64,
    pub entries: Vec<Entry>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub(super) struct Entry {
    pub path: PathBuf,
    pub folder: bool,
    pub stamp: Stamp,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub(super) struct Stamp {
    pub len: u64,
    pub modified: Option<SystemTime>,
}

/// The copy was stopped on request.
#[derive(Debug)]
pub(super) struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The copy was cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// Lists everything in `world` without following links. A link, or anything
/// else that is neither a regular file nor a folder, refuses the world: a copy
/// could not say what it should hold.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn survey(world: &Path) -> anyhow::Result<Survey> {
    let mut survey = Survey {
        bytes: 0,
        entries: Vec::new(),
    };
    walk(world, Path::new(""), &mut survey)?;
    Ok(survey)
}

fn walk(world: &Path, relative: &Path, survey: &mut Survey) -> anyhow::Result<()> {
    let folder = world.join(relative);
    let mut names = std::fs::read_dir(&folder)
        .with_context(|| format!("reading {}", folder.display()))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("reading {}", folder.display()))?;
    // A fixed order, so two surveys of an untouched world are equal.
    names.sort();

    for name in names {
        if relative.as_os_str().is_empty() && name == SESSION_LOCK {
            continue;
        }
        let path = relative.join(&name);
        let full = world.join(&path);
        let meta = std::fs::symlink_metadata(&full)
            .with_context(|| format!("reading {}", full.display()))?;
        let kind = meta.file_type();
        // A link of any kind, or a device, pipe or socket.
        if kind.is_symlink() || !(kind.is_file() || kind.is_dir()) {
            return Err(WorldRefusal::HoldsLink.into());
        }
        let modified = meta.modified().ok();
        if kind.is_dir() {
            survey.entries.push(Entry {
                path: path.clone(),
                folder: true,
                stamp: Stamp { len: 0, modified },
            });
            walk(world, &path, survey)?;
        } else {
            survey.bytes += meta.len();
            survey.entries.push(Entry {
                path,
                folder: false,
                stamp: Stamp {
                    len: meta.len(),
                    modified,
                },
            });
        }
    }
    Ok(())
}

/// Copies what `survey` lists into the new folder `staging`, calling `copied`
/// with the bytes copied so far after each chunk. The caller removes `staging`
/// when this fails; it is never removed here.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn copy_tree(
    world: &Path,
    staging: &Path,
    survey: &Survey,
    copied: &(dyn Fn(u64) + Sync),
    cancel: &CancellationToken,
) -> anyhow::Result<()> {
    use std::io::{Read, Write};

    std::fs::create_dir(staging).with_context(|| format!("creating {}", staging.display()))?;

    let mut total = 0u64;
    let mut buffer = vec![0u8; CHUNK];
    for entry in &survey.entries {
        let target = staging.join(&entry.path);
        if entry.folder {
            std::fs::create_dir(&target)
                .with_context(|| format!("creating {}", target.display()))?;
            continue;
        }

        let source = world.join(&entry.path);
        let mut from =
            open_source(&source).with_context(|| format!("opening {}", source.display()))?;
        let mut to = std::fs::File::create_new(&target)
            .with_context(|| format!("creating {}", target.display()))?;
        let mut length = 0u64;
        loop {
            if cancel.is_cancelled() {
                return Err(Cancelled.into());
            }
            let read = from
                .read(&mut buffer)
                .with_context(|| format!("reading {}", source.display()))?;
            if read == 0 {
                break;
            }
            length += read as u64;
            // Stop at once on a file that keeps growing.
            anyhow::ensure!(
                length <= entry.stamp.len,
                "{} changed while it was copied",
                source.display()
            );
            to.write_all(&buffer[..read])
                .with_context(|| format!("writing {}", target.display()))?;
            total += read as u64;
            copied(total);
        }
        anyhow::ensure!(
            length == entry.stamp.len,
            "{} changed while it was copied",
            source.display()
        );
        if let Some(modified) = entry.stamp.modified {
            to.set_modified(modified)
                .with_context(|| format!("setting the time of {}", target.display()))?;
        }
        // So a copy published right after this returns cannot hold empty
        // files after a power cut.
        to.sync_all()
            .with_context(|| format!("syncing {}", target.display()))?;
    }

    // A folder's time moves whenever something is put in it, so the folders
    // get theirs last, innermost first. A time a platform won't set is no
    // reason to fail a copy that holds every byte.
    for entry in survey.entries.iter().rev().filter(|entry| entry.folder) {
        if let Some(modified) = entry.stamp.modified {
            let _ = set_folder_modified(&staging.join(&entry.path), modified);
        }
    }
    Ok(())
}

/// Opens a surveyed file. On unix a link that replaced it since the survey is
/// an error instead of a read from outside the world.
fn open_source(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path)
}

fn set_folder_modified(folder: &Path, modified: SystemTime) -> std::io::Result<()> {
    #[cfg(windows)]
    let handle = {
        use std::os::windows::fs::OpenOptionsExt;
        // Needed to open a folder at all.
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(folder)?
    };
    #[cfg(not(windows))]
    let handle = std::fs::File::open(folder)?;
    handle.set_modified(modified)
}

/// Whether `world` still holds what `survey` saw: the same paths, kinds,
/// lengths and modified times.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn unchanged_since(world: &Path, survey: &Survey) -> anyhow::Result<bool> {
    Ok(same_entries(&survey.entries, &self::survey(world)?.entries))
}

/// Equal lists count only when every entry has a modified time: without one a
/// rewrite that keeps the length goes unseen, so the copy cannot be proven
/// untorn.
fn same_entries(before: &[Entry], after: &[Entry]) -> bool {
    before == after
        && before
            .iter()
            .chain(after)
            .all(|entry| entry.stamp.modified.is_some())
}

/// The room left on the drive that holds `folder`, or `None` when no listed
/// drive does.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn free_space(folder: &Path) -> Option<u64> {
    // Not `canonicalize`: on Windows that gives a `\\?\C:\` path, which never
    // starts with the `C:\` mount point sysinfo reports.
    let folder = dunce::canonicalize(folder).ok()?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mounts = disks
        .list()
        .iter()
        .map(|disk| (disk.mount_point().to_path_buf(), disk.available_space()))
        .collect::<Vec<_>>();
    free_space_among(&folder, &mounts, &device_of)
}

/// Picks the mount point that is the longest prefix of `folder` and returns
/// its room. sysinfo leaves some mounts out (tmpfs, network drives), which
/// would make the search settle on a shorter mount, a different drive; so on
/// unix the answer is given only when `device` says the folder and the chosen
/// mount point are on the same device.
fn free_space_among(
    folder: &Path,
    mounts: &[(PathBuf, u64)],
    device: &dyn Fn(&Path) -> Option<u64>,
) -> Option<u64> {
    let (mount, available) = mounts
        .iter()
        .filter(|(mount, _)| folder.starts_with(mount))
        .max_by_key(|(mount, _)| mount.components().count())?;
    #[cfg(unix)]
    {
        let on_mount = device(mount)?;
        if device(folder)? != on_mount {
            return None;
        }
    }
    #[cfg(not(unix))]
    let _ = (mount, device);
    Some(*available)
}

#[cfg(unix)]
fn device_of(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|meta| meta.dev())
}

#[cfg(not(unix))]
fn device_of(_: &Path) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::coded_cause;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    fn write(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn set_time(path: &Path, modified: SystemTime) {
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(modified)
            .unwrap();
    }

    fn the_world(root: &Path) -> PathBuf {
        let world = root.join("World");
        write(&world.join("level.dat"), b"level");
        write(&world.join("region/r.0.0.mca"), &vec![7u8; 3 * 1024 * 1024]);
        write(&world.join("日本.dat"), "日本".as_bytes());
        write(&world.join("empty.txt"), b"");
        std::fs::create_dir_all(world.join("data/none")).unwrap();
        write(&world.join("session.lock"), b"lock");
        world
    }

    fn nothing(_: u64) {}

    /// Every path under `root` with its kind, bytes and modified time.
    fn tree(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>, SystemTime)> {
        let mut all = Vec::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(folder) = pending.pop() {
            for entry in std::fs::read_dir(&folder).unwrap() {
                let path = entry.unwrap().path();
                let meta = std::fs::metadata(&path).unwrap();
                let content = meta.is_file().then(|| std::fs::read(&path).unwrap());
                if meta.is_dir() {
                    pending.push(path.clone());
                }
                all.push((
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    content,
                    meta.modified().unwrap(),
                ));
            }
        }
        all.sort();
        all
    }

    #[test]
    fn a_copy_holds_every_file_and_folder_with_its_time() {
        let dir = tempfile::tempdir().unwrap();
        let world = the_world(dir.path());
        set_time(
            &world.join("level.dat"),
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000),
        );
        let staging = dir.path().join("staging");

        let found = survey(&world).unwrap();
        let last = AtomicU64::new(0);
        copy_tree(
            &world,
            &staging,
            &found,
            &|total| last.store(total, Ordering::SeqCst),
            &CancellationToken::new(),
        )
        .unwrap();

        let expected = 5 + 3 * 1024 * 1024 + "日本".len() as u64;
        assert_eq!(found.bytes, expected);
        assert_eq!(last.load(Ordering::SeqCst), expected);
        assert!(!staging.join("session.lock").exists());
        let mut original = tree(&world);
        original.retain(|(path, _, _)| path != Path::new("session.lock"));
        assert_eq!(tree(&staging), original);
        assert!(staging.join("data/none").is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_inside_the_world_is_refused_before_anything_is_copied() {
        let links: [(&str, fn(&Path, &Path)); 3] = [
            ("file", |target, link| {
                std::os::unix::fs::symlink(target, link).unwrap()
            }),
            ("folder", |_, link| {
                std::os::unix::fs::symlink(link.parent().unwrap(), link).unwrap()
            }),
            ("dangling", |_, link| {
                std::os::unix::fs::symlink("/no/such/place", link).unwrap()
            }),
        ];
        for (name, make) in links {
            let dir = tempfile::tempdir().unwrap();
            let world = the_world(dir.path());
            make(&world.join("level.dat"), &world.join("region/link"));
            let staging = dir.path().join("staging");

            // The survey fails, so there is nothing to copy.
            let error = match survey(&world) {
                Err(error) => error,
                Ok(_) => panic!("a {name} link was accepted"),
            };
            assert_eq!(coded_cause(&error).unwrap().0, "WORLD_HOLDS_LINK", "{name}");
            assert!(!staging.exists(), "{name}");
        }
    }

    #[test]
    fn cancelling_stops_within_one_chunk() {
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path().join("World");
        write(&world.join("big.bin"), &vec![1u8; 5 * 1024 * 1024]);
        let staging = dir.path().join("staging");
        let found = survey(&world).unwrap();
        let cancel = CancellationToken::new();

        let error = copy_tree(&world, &staging, &found, &|_| cancel.cancel(), &cancel)
            .expect_err("a cancelled copy must fail");

        assert!(error.downcast_ref::<Cancelled>().is_some());
        let written = std::fs::metadata(staging.join("big.bin")).unwrap().len();
        assert!(written < 3 * 1024 * 1024, "{written} bytes were written");
        assert!(staging.exists());
    }

    #[test]
    fn a_file_that_grows_while_it_is_copied_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path().join("World");
        write(&world.join("log.txt"), b"short");
        let found = survey(&world).unwrap();
        write(&world.join("log.txt"), b"a lot longer than before");

        let error = copy_tree(
            &world,
            &dir.path().join("staging"),
            &found,
            &nothing,
            &CancellationToken::new(),
        )
        .expect_err("a changed length must fail");
        assert!(format!("{error:#}").contains("changed while it was copied"));
    }

    #[test]
    fn a_world_played_meanwhile_is_seen() {
        let dir = tempfile::tempdir().unwrap();
        let world = the_world(dir.path());
        let found = survey(&world).unwrap();
        assert!(unchanged_since(&world, &found).unwrap());

        write(&world.join("session.lock"), b"another lock");
        assert!(unchanged_since(&world, &found).unwrap());

        let changes: [(&str, fn(&Path)); 4] = [
            ("append", |world| {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(world.join("level.dat"))
                    .unwrap();
                file.write_all(b"!").unwrap();
            }),
            ("add", |world| write(&world.join("new.dat"), b"new")),
            ("remove", |world| {
                std::fs::remove_file(world.join("level.dat")).unwrap()
            }),
            ("touch", |world| {
                set_time(
                    &world.join("日本.dat"),
                    SystemTime::UNIX_EPOCH + Duration::from_secs(5),
                )
            }),
        ];
        for (name, change) in changes {
            let dir = tempfile::tempdir().unwrap();
            let world = the_world(dir.path());
            let found = survey(&world).unwrap();
            change(&world);
            assert!(!unchanged_since(&world, &found).unwrap(), "{name}");
        }
    }

    #[test]
    fn a_missing_modified_time_counts_as_changed() {
        let dir = tempfile::tempdir().unwrap();
        let world = the_world(dir.path());
        let found = survey(&world).unwrap();
        assert!(same_entries(&found.entries, &found.entries));
        let mut without = found.entries.clone();
        without[0].stamp.modified = None;
        assert!(!same_entries(&without, &without));
    }

    #[cfg(unix)]
    #[test]
    fn a_file_swapped_for_a_link_after_the_survey_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path().join("World");
        write(&world.join("level.dat"), b"inside");
        let outside = dir.path().join("outside.txt");
        write(&outside, b"secret");
        let found = survey(&world).unwrap();
        std::fs::remove_file(world.join("level.dat")).unwrap();
        std::os::unix::fs::symlink(&outside, world.join("level.dat")).unwrap();
        let staging = dir.path().join("staging");

        copy_tree(
            &world,
            &staging,
            &found,
            &nothing,
            &CancellationToken::new(),
        )
        .expect_err("a link must not be followed");

        let leaked = std::fs::read_dir(&staging)
            .unwrap()
            .any(|entry| std::fs::read(entry.unwrap().path()).is_ok_and(|b| b == b"secret"));
        assert!(!leaked);
    }

    #[cfg(unix)]
    #[test]
    fn odd_names_and_deep_folders_are_copied() {
        use std::os::unix::ffi::OsStrExt;
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path().join("World");
        write(
            &world.join(std::ffi::OsStr::from_bytes(b"\xff.dat")),
            b"odd",
        );
        let mut deep = world.clone();
        for level in 0..10 {
            deep = deep.join(format!("d{level}"));
        }
        write(&deep.join("leaf.dat"), b"leaf");
        let staging = dir.path().join("staging");

        let found = survey(&world).unwrap();
        copy_tree(
            &world,
            &staging,
            &found,
            &nothing,
            &CancellationToken::new(),
        )
        .unwrap();

        assert_eq!(tree(&staging), tree(&world));
        assert!(unchanged_since(&world, &found).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn the_drive_rule_picks_the_longest_mount_on_the_same_device() {
        let mounts = vec![
            (PathBuf::from("/"), 1u64),
            (PathBuf::from("/data"), 2),
            (PathBuf::from("/data/games"), 3),
        ];
        let device = |path: &Path| -> Option<u64> {
            Some(if path.starts_with("/data/games") {
                30
            } else if path.starts_with("/data") {
                20
            } else {
                10
            })
        };
        assert_eq!(
            free_space_among(Path::new("/data/games/saves"), &mounts, &device),
            Some(3)
        );
        assert_eq!(
            free_space_among(Path::new("/data/x"), &mounts, &device),
            Some(2)
        );
        assert_eq!(
            free_space_among(Path::new("/home/me"), &mounts, &device),
            Some(1)
        );
        // A mount sysinfo leaves out: the folder is on another device than "/".
        let other = |path: &Path| -> Option<u64> {
            Some(if path == Path::new("/run/x") { 99 } else { 10 })
        };
        assert_eq!(free_space_among(Path::new("/run/x"), &mounts, &other), None);
        assert_eq!(free_space_among(Path::new("/x"), &[], &device), None);
    }

    #[test]
    fn free_space_is_known_for_the_temp_folder() {
        let dir = tempfile::tempdir().unwrap();
        match free_space(dir.path()) {
            Some(free) => assert!(free > 0),
            // A platform where sysinfo lists no disks.
            None => assert!(sysinfo::Disks::new_with_refreshed_list().list().is_empty()),
        }
    }
}
