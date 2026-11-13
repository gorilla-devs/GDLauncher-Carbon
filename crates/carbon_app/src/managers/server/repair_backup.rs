//! Keeps what Repair replaces until the reinstall has succeeded, so a Repair
//! that fails (offline, the platform down, the pack withdrawn, the launcher
//! closed) leaves the server as it was instead of without mods and jars.
//!
//! The entries are moved into a folder inside the server's data folder, which
//! is the same volume, so nothing is copied. That folder existing means a
//! Repair did not finish; [`restore`] puts it back.

use anyhow::Context;
use std::path::Path;
use tracing::{info, warn};

const FOLDERS: [&str; 3] = ["mods", "libraries", "defaultconfigs"];
const FILES: [&str; 1] = ["launch_config.json"];
const BACKUP: &str = ".gdl-repair-backup";
/// A backup whose Repair succeeded, on its way to being deleted.
const DISCARDED: &str = ".gdl-repair-backup.done";
/// Written inside the backup once every entry is in it. Until then nothing
/// of the new install exists and the entries left in place are the old ones.
const COMPLETE: &str = ".complete";

/// The fixed names Repair replaces (it also replaces every jar at the root).
pub(super) fn replaced_names() -> impl Iterator<Item = &'static str> {
    FOLDERS.into_iter().chain(FILES)
}

fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Removes a folder or file. A link, or a junction on Windows, is removed
/// itself and what it points to is kept.
fn remove_entry(path: &Path) -> std::io::Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if meta.file_type().is_symlink() {
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            if meta.file_type().is_symlink_dir() {
                return std::fs::remove_dir(path);
            }
        }
        return std::fs::remove_file(path);
    }
    if meta.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

/// What Repair replaces of the entries now in `data_path`.
fn replaced_entries(data_path: &Path) -> anyhow::Result<Vec<String>> {
    let mut names: Vec<String> = replaced_names()
        .filter(|name| exists(&data_path.join(name)))
        .map(str::to_string)
        .collect();
    for entry in std::fs::read_dir(data_path).context("Failed to read server data dir")? {
        let path = entry?.path();
        let jar = path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"));
        if let (true, Some(name)) = (jar, path.file_name().and_then(|name| name.to_str())) {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

/// Moves what Repair replaces out of the way. Call [`restore`] first: a
/// backup from an earlier Repair must not be there.
pub(super) fn set_aside(data_path: &Path) -> anyhow::Result<()> {
    let backup = data_path.join(BACKUP);
    std::fs::create_dir(&backup).context("Failed to create the Repair backup folder")?;
    for name in replaced_entries(data_path)? {
        let live = data_path.join(&name);
        if let Err(error) = std::fs::rename(&live, backup.join(&name)) {
            // Somewhere a move cannot reach, such as another volume mounted
            // at this name. Repair still goes ahead, as it did before.
            warn!("Repair could not keep {}: {error}", live.display());
            remove_entry(&live).with_context(|| format!("Failed to wipe {}", live.display()))?;
        }
    }
    std::fs::write(backup.join(COMPLETE), b"")
        .context("Failed to complete the Repair backup folder")?;
    Ok(())
}

fn delete_discarded(data_path: &Path) -> anyhow::Result<()> {
    let discarded = data_path.join(DISCARDED);
    if !exists(&discarded) {
        return Ok(());
    }
    for entry in std::fs::read_dir(&discarded)? {
        remove_entry(&entry?.path())?;
    }
    Ok(std::fs::remove_dir(&discarded)?)
}

/// Deletes the backup of a Repair that succeeded.
pub(super) fn discard(data_path: &Path) -> anyhow::Result<()> {
    delete_discarded(data_path)?;
    let backup = data_path.join(BACKUP);
    if exists(&backup) {
        // One step, so a launcher closed during the deletion below never
        // leaves half a backup that would be put back over the new install.
        std::fs::rename(&backup, data_path.join(DISCARDED))?;
    }
    delete_discarded(data_path)
}

/// Puts back what a Repair that did not finish had set aside, removing what
/// that Repair installed in its place. Returns whether there was anything to
/// put back. Safe to run again after being interrupted at any point.
pub(super) fn restore(data_path: &Path) -> anyhow::Result<bool> {
    delete_discarded(data_path)?;
    let backup = data_path.join(BACKUP);
    if !exists(&backup) {
        return Ok(false);
    }
    let complete = backup.join(COMPLETE);
    if exists(&complete) {
        for name in replaced_entries(data_path)? {
            remove_entry(&data_path.join(name))?;
        }
        std::fs::remove_file(&complete)?;
    }
    for entry in std::fs::read_dir(&backup)? {
        let entry = entry?;
        let live = data_path.join(entry.file_name());
        if exists(&live) {
            // Only where the entry was never moved: the one in place is the
            // server's own.
            warn!("Repair backup holds a second {}", live.display());
            remove_entry(&entry.path())?;
        } else {
            std::fs::rename(entry.path(), &live)?;
        }
    }
    std::fs::remove_dir(&backup)?;
    info!(
        "Put back what an unfinished Repair replaced in {}",
        data_path.display()
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn write(path: std::path::PathBuf, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    /// Every file under `root` with its contents, links by their target.
    fn tree(root: &Path) -> BTreeMap<String, String> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let meta = std::fs::symlink_metadata(&path).unwrap();
                if meta.file_type().is_symlink() {
                    out.insert(
                        name,
                        format!("-> {}", std::fs::read_link(&path).unwrap().display()),
                    );
                } else if meta.is_dir() {
                    out.insert(format!("{name}/"), String::new());
                    walk(root, &path, out);
                } else {
                    out.insert(
                        name,
                        String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into(),
                    );
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    fn server() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        write(data.join("mods/pack.jar"), b"pack mod");
        write(data.join("mods/added-by-hand.jar"), b"own mod");
        write(data.join("libraries/net/loader/loader.jar"), b"loader");
        write(data.join("defaultconfigs/a.toml"), b"defaults");
        write(data.join("launch_config.json"), b"{}");
        write(data.join("server.jar"), b"vanilla");
        write(data.join("forge-1.20.1-47.2.0-shim.JAR"), b"shim");
        write(data.join("world/level.dat"), b"save");
        write(data.join("config/own.toml"), b"config");
        write(data.join("server.properties"), b"level-name=world\n");
        write(data.join("notes.jar.txt"), b"not a jar");
        dir
    }

    /// What a Repair writes before it fails or succeeds.
    fn install_new(data: &Path) {
        write(data.join("mods/new.jar"), b"new mod");
        write(data.join("libraries/new/new.jar"), b"new loader");
        write(data.join("server.jar"), b"new vanilla");
        write(data.join("neoforge-installer.jar"), b"installer");
        write(data.join("launch_config.json"), b"{\"new\":true}");
    }

    const KEPT: [&str; 4] = [
        "world/level.dat",
        "config/own.toml",
        "server.properties",
        "notes.jar.txt",
    ];

    #[test]
    fn sets_aside_exactly_what_repair_replaces() {
        let dir = server();
        set_aside(dir.path()).unwrap();
        let mut left: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                BACKUP,
                "config",
                "notes.jar.txt",
                "server.properties",
                "world"
            ]
        );
    }

    #[test]
    fn a_failed_repair_leaves_the_server_as_it_was() {
        let dir = server();
        let before = tree(dir.path());
        set_aside(dir.path()).unwrap();
        install_new(dir.path());
        assert!(restore(dir.path()).unwrap());
        assert_eq!(tree(dir.path()), before);
        assert!(!restore(dir.path()).unwrap(), "nothing is left to put back");
    }

    #[test]
    fn a_repair_that_fails_before_writing_anything_is_undone_too() {
        let dir = server();
        let before = tree(dir.path());
        set_aside(dir.path()).unwrap();
        assert!(restore(dir.path()).unwrap());
        assert_eq!(tree(dir.path()), before);
    }

    #[test]
    fn a_successful_repair_keeps_the_new_install_and_drops_the_old() {
        let dir = server();
        set_aside(dir.path()).unwrap();
        install_new(dir.path());
        let installed = tree(dir.path());
        discard(dir.path()).unwrap();
        let after = tree(dir.path());
        assert!(
            after.keys().all(|name| !name.starts_with(".gdl-repair")),
            "{after:?}"
        );
        assert!(!after.contains_key("mods/added-by-hand.jar"));
        assert_eq!(after["mods/new.jar"], "new mod");
        for (name, content) in &after {
            assert_eq!(&installed[name], content, "{name}");
        }
        assert!(!restore(dir.path()).unwrap());
        assert_eq!(tree(dir.path()), after);
    }

    #[test]
    fn a_launcher_closed_while_setting_aside_loses_nothing() {
        let dir = server();
        let before = tree(dir.path());
        // Two entries moved, the rest still in place, no completion mark.
        let backup = dir.path().join(BACKUP);
        std::fs::create_dir(&backup).unwrap();
        for name in ["mods", "server.jar"] {
            std::fs::rename(dir.path().join(name), backup.join(name)).unwrap();
        }
        assert!(restore(dir.path()).unwrap());
        assert_eq!(tree(dir.path()), before);
    }

    #[test]
    fn a_launcher_closed_while_putting_back_loses_nothing() {
        let dir = server();
        let before = tree(dir.path());
        set_aside(dir.path()).unwrap();
        install_new(dir.path());
        // The new install is cleared and one entry is already back.
        let backup = dir.path().join(BACKUP);
        for name in replaced_entries(dir.path()).unwrap() {
            remove_entry(&dir.path().join(name)).unwrap();
        }
        std::fs::remove_file(backup.join(COMPLETE)).unwrap();
        std::fs::rename(backup.join("mods"), dir.path().join("mods")).unwrap();
        assert!(restore(dir.path()).unwrap());
        assert_eq!(tree(dir.path()), before);
    }

    #[test]
    fn a_launcher_closed_while_dropping_the_old_install_keeps_the_new_one() {
        let dir = server();
        set_aside(dir.path()).unwrap();
        install_new(dir.path());
        // Renamed for deletion and half deleted.
        let discarded = dir.path().join(DISCARDED);
        std::fs::rename(dir.path().join(BACKUP), &discarded).unwrap();
        std::fs::remove_dir_all(discarded.join("mods")).unwrap();
        assert!(!restore(dir.path()).unwrap());
        let after = tree(dir.path());
        assert_eq!(after["mods/new.jar"], "new mod");
        assert_eq!(after["server.jar"], "new vanilla");
        assert!(
            after.keys().all(|name| !name.starts_with(".gdl-repair")),
            "{after:?}"
        );
    }

    #[test]
    fn nothing_else_in_the_server_is_touched_at_any_point() {
        let dir = server();
        let kept = |data: &Path| KEPT.map(|name| std::fs::read(data.join(name)).unwrap());
        let before = kept(dir.path());
        set_aside(dir.path()).unwrap();
        assert_eq!(kept(dir.path()), before);
        install_new(dir.path());
        restore(dir.path()).unwrap();
        assert_eq!(kept(dir.path()), before);
        set_aside(dir.path()).unwrap();
        discard(dir.path()).unwrap();
        assert_eq!(kept(dir.path()), before);
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_mods_folder_comes_back_as_the_same_link_and_its_target_is_never_emptied() {
        let outside = tempfile::tempdir().unwrap();
        write(outside.path().join("shared/shared.jar"), b"shared");
        let dir = server();
        std::fs::remove_dir_all(dir.path().join("mods")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("shared"), dir.path().join("mods")).unwrap();
        let before = tree(dir.path());

        // Failure: the link is back.
        set_aside(dir.path()).unwrap();
        assert!(!exists(&dir.path().join("mods")));
        install_new(dir.path());
        restore(dir.path()).unwrap();
        assert_eq!(tree(dir.path()), before);

        // Success: the link is gone, what it pointed to is kept.
        set_aside(dir.path()).unwrap();
        install_new(dir.path());
        discard(dir.path()).unwrap();
        assert_eq!(
            std::fs::read(outside.path().join("shared/shared.jar")).unwrap(),
            b"shared"
        );
        assert!(
            !std::fs::symlink_metadata(dir.path().join("mods"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn a_server_with_none_of_the_replaced_entries_repairs_and_restores() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path().join("server.properties"), b"x");
        set_aside(dir.path()).unwrap();
        install_new(dir.path());
        assert!(restore(dir.path()).unwrap());
        assert_eq!(
            tree(dir.path()),
            BTreeMap::from([("server.properties".to_string(), "x".to_string())])
        );
    }
}
