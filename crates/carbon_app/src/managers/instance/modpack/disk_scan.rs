//! Produces [`apply_plan::DiskState`] from the real filesystem for a given
//! universe of packinfo-style paths, and takes a full disk snapshot of an
//! instance's data directory as a [`PackInfo`] for the repair skip-oracle.
//!
//! Neither function consults any pack metadata — they only look at what is
//! actually on disk right now. Callers combine their output with `old`/
//! `target` packinfos in [`apply_plan::plan`](super::apply_plan::plan).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use md5::Md5;
use sha2::{Digest, Sha512};

use crate::util::NormalizedWalkdir;

use super::apply_plan;
use super::packinfo::{FileHashes, PackInfo};

/// Disk state for every path in `universe` (packinfo-style keys with leading '/').
/// Probes `<data>/<path>` then `<data>/<path>.disabled`; hashes md5.
pub async fn scan_disk_state(
    data_path: &Path,
    universe: &BTreeSet<String>,
) -> anyhow::Result<HashMap<String, apply_plan::DiskState>> {
    let mut futures = Vec::with_capacity(universe.len());

    for path in universe {
        // `strip_prefix` rather than `[1..]`: char-boundary-safe (an empty
        // key or one starting with a multibyte character would panic on a
        // byte-index slice), and it gives `None` instead of a garbage
        // substring for a key that doesn't actually start with '/'.
        let rel = path
            .strip_prefix('/')
            .ok_or_else(|| anyhow::anyhow!("packinfo key '{path}' must start with '/'"))?;
        // Re-checked here rather than trusted from `parse_packinfo`: a `..`
        // segment survives `Path::join` as a literal component, and
        // `Path::starts_with` below compares components lexically without
        // ever resolving `..` — so `data_path.join("mods/../../evil")`
        // would otherwise pass the containment check below even though the
        // real, OS-resolved path escapes `data_path` entirely.
        if super::packinfo::has_dotdot_segment(path) {
            anyhow::bail!("packinfo key '{path}' contains a '..' path segment");
        }
        let disk_path = data_path.join(rel);
        // Belt-and-braces even though `parse_packinfo` already rejects a
        // doubled leading '/': `Path::join` with an absolute argument
        // REPLACES the base entirely, so a key like "//tmp/esc" (whose tail
        // after stripping one '/' is itself still absolute) would otherwise
        // resolve to a path outside `data_path` — never trust a join alone.
        if !disk_path.starts_with(data_path) {
            anyhow::bail!("packinfo key '{path}' escapes the instance data dir");
        }
        let key = path.clone();

        futures.push(async move {
            let state = match probe_md5(&disk_path).await? {
                Some(md5) => apply_plan::DiskState::Present { md5 },
                None => match probe_md5(&disabled_sibling(&disk_path)).await? {
                    Some(md5) => apply_plan::DiskState::Disabled { md5 },
                    None => apply_plan::DiskState::Missing,
                },
            };

            Ok::<_, anyhow::Error>((key, state))
        });
    }

    futures::future::join_all(futures)
        .await
        .into_iter()
        .collect::<Result<HashMap<_, _>, anyhow::Error>>()
}

/// `<name>` -> `<name>.disabled`, the same sibling-probe idiom used by the
/// packinfo-replacement pass in `run/modpack.rs`.
fn disabled_sibling(path: &Path) -> PathBuf {
    let mut disabled = path.to_path_buf();
    let mut name = disabled
        .file_name()
        .expect("scan paths are always joined from a non-empty file path")
        .to_owned();
    name.push(".disabled");
    disabled.set_file_name(name);
    disabled
}

/// Opens `path` and md5-hashes it, or `Ok(None)` if it doesn't exist —
/// including when an intermediate directory component is missing.
async fn probe_md5(path: &Path) -> anyhow::Result<Option<[u8; 16]>> {
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let mut md5 = Md5::new();
    carbon_scheduler::buffered_digest(&mut file, |chunk| {
        md5.update(&chunk);
    })
    .await?;

    Ok(Some(md5.finalize().into()))
}

/// Repair skip-oracle: full scan of the instance data dir as a PackInfo
/// (md5+sha512), keyed under enabled names (`.disabled` stripped), skipping
/// `/saves` and `.install_audit`. When both `X` and `X.disabled` exist the
/// ENABLED file's hashes win (deterministic, unlike scan_dir's racy insert).
pub async fn scan_instance_as_packinfo(data_path: &Path) -> anyhow::Result<PackInfo> {
    let mut futures = Vec::new();

    let mut walker = NormalizedWalkdir::new(data_path)?;
    while let Some(entry) = walker.next()? {
        if entry.is_dir {
            continue;
        }

        let relative_path = entry.relative_path;
        if relative_path.starts_with("/saves") || relative_path.starts_with("/.install_audit") {
            continue;
        }

        let path = entry.entry.path();
        let mut key = relative_path.to_string();

        futures.push(async move {
            let from_disabled = key.ends_with(".disabled");
            if from_disabled {
                key.truncate(key.len() - ".disabled".len());
            }

            let mut file = tokio::fs::File::open(path).await?;
            let mut sha512 = Sha512::new();
            let mut md5 = Md5::new();

            carbon_scheduler::buffered_digest(&mut file, |chunk| {
                sha512.update(&chunk);
                md5.update(&chunk);
            })
            .await?;

            let sha512 = sha512.finalize().into();
            let md5 = md5.finalize().into();

            Ok::<_, anyhow::Error>((key, FileHashes { sha512, md5 }, from_disabled))
        });
    }

    let results = futures::future::join_all(futures)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    // Enabled-wins, order-independent: a `.disabled`-sourced insertion never
    // overwrites an already-present entry unless that entry itself came from
    // a `.disabled` twin (tracked in `disabled_backed`); an enabled-sourced
    // insertion always overwrites, regardless of what (if anything) preceded
    // it. This is deterministic regardless of the walker's/readdir's
    // discovery order, unlike `scan_dir`'s plain `collect()`.
    let mut files: HashMap<String, FileHashes> = HashMap::new();
    let mut disabled_backed: HashSet<String> = HashSet::new();

    for (key, hashes, from_disabled) in results {
        if from_disabled {
            if !files.contains_key(&key) || disabled_backed.contains(&key) {
                files.insert(key.clone(), hashes);
                disabled_backed.insert(key);
            }
        } else {
            files.insert(key.clone(), hashes);
            disabled_backed.remove(&key);
        }
    }

    Ok(PackInfo { files })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn universe(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|s| s.to_string()).collect()
    }

    fn md5_of(bytes: &[u8]) -> [u8; 16] {
        Md5::digest(bytes).into()
    }

    // --- scan_disk_state --------------------------------------------------

    #[tokio::test]
    async fn enabled_file_is_present_with_correct_md5() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods")).unwrap();
        std::fs::write(dir.path().join("mods/a.jar"), b"hello").unwrap();

        let result = scan_disk_state(dir.path(), &universe(&["/mods/a.jar"]))
            .await
            .unwrap();

        assert_eq!(
            result.get("/mods/a.jar"),
            Some(&apply_plan::DiskState::Present {
                md5: md5_of(b"hello")
            })
        );
    }

    #[tokio::test]
    async fn absent_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();

        let result = scan_disk_state(dir.path(), &universe(&["/mods/a.jar"]))
            .await
            .unwrap();

        assert_eq!(
            result.get("/mods/a.jar"),
            Some(&apply_plan::DiskState::Missing)
        );
    }

    #[tokio::test]
    async fn only_disabled_twin_reports_disabled_with_twins_md5() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods")).unwrap();
        std::fs::write(dir.path().join("mods/a.jar.disabled"), b"twin-bytes").unwrap();

        let result = scan_disk_state(dir.path(), &universe(&["/mods/a.jar"]))
            .await
            .unwrap();

        assert_eq!(
            result.get("/mods/a.jar"),
            Some(&apply_plan::DiskState::Disabled {
                md5: md5_of(b"twin-bytes")
            })
        );
    }

    #[tokio::test]
    async fn both_present_prefers_enabled_files_md5() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods")).unwrap();
        std::fs::write(dir.path().join("mods/a.jar"), b"enabled-bytes").unwrap();
        std::fs::write(dir.path().join("mods/a.jar.disabled"), b"disabled-bytes").unwrap();

        let result = scan_disk_state(dir.path(), &universe(&["/mods/a.jar"]))
            .await
            .unwrap();

        assert_eq!(
            result.get("/mods/a.jar"),
            Some(&apply_plan::DiskState::Present {
                md5: md5_of(b"enabled-bytes")
            })
        );
    }

    #[tokio::test]
    async fn saves_path_is_still_scanned() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("saves/world")).unwrap();
        std::fs::write(dir.path().join("saves/world/level.dat"), b"save-bytes").unwrap();

        let result = scan_disk_state(dir.path(), &universe(&["/saves/world/level.dat"]))
            .await
            .unwrap();

        assert_eq!(
            result.get("/saves/world/level.dat"),
            Some(&apply_plan::DiskState::Present {
                md5: md5_of(b"save-bytes")
            }),
            "the planner guards /saves, not the scanner — it must still be probed"
        );
    }

    #[tokio::test]
    async fn path_in_missing_directory_is_missing_without_error() {
        let dir = tempfile::tempdir().unwrap();
        // Note: `dir.path().join("nonexistent")` is deliberately never created.

        let result = scan_disk_state(dir.path(), &universe(&["/nonexistent/a.jar"]))
            .await
            .unwrap();

        assert_eq!(
            result.get("/nonexistent/a.jar"),
            Some(&apply_plan::DiskState::Missing)
        );
    }

    #[tokio::test]
    async fn scan_disk_state_never_escapes_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        let data_path = dir.path().join("instance");
        std::fs::create_dir_all(&data_path).unwrap();

        let result = scan_disk_state(&data_path, &universe(&["//tmp/esc"])).await;

        assert!(
            result.is_err(),
            "a key that leaves an absolute tail after stripping the leading '/' must error, \
             not silently probe outside the data dir"
        );
    }

    #[tokio::test]
    async fn scan_disk_state_rejects_a_dotdot_segment() {
        // `Path::starts_with` compares path components lexically and never
        // resolves `..`, so the plain containment check alone does not
        // catch this: `data_path.join("mods/../../escaped.jar")` still
        // lexically "starts with" data_path even though the real,
        // OS-resolved path is data_path's own parent. `mods` must
        // physically exist for the filesystem to walk through it while
        // resolving the `..`s below. This key never goes through
        // `parse_packinfo` at all, proving the guard holds on its own.
        let dir = tempfile::tempdir().unwrap();
        let data_path = dir.path().join("instance");
        std::fs::create_dir_all(data_path.join("mods")).unwrap();
        std::fs::write(dir.path().join("escaped.jar"), b"outside-bytes").unwrap();

        let result = scan_disk_state(&data_path, &universe(&["/mods/../../escaped.jar"])).await;

        assert!(
            result.is_err(),
            "a '..' segment must be rejected before ever probing outside data_path"
        );
    }

    // --- scan_instance_as_packinfo -----------------------------------------

    #[tokio::test]
    async fn keys_carry_a_leading_slash() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.jar"), b"hi").unwrap();

        let packinfo = scan_instance_as_packinfo(dir.path()).await.unwrap();

        assert!(
            packinfo.files.contains_key("/a.jar"),
            "got keys: {:?}",
            packinfo.files.keys().collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn disabled_suffix_is_stripped_from_the_key() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.jar.disabled"), b"hi").unwrap();

        let packinfo = scan_instance_as_packinfo(dir.path()).await.unwrap();

        assert!(packinfo.files.contains_key("/a.jar"));
        assert!(!packinfo.files.contains_key("/a.jar.disabled"));
    }

    #[tokio::test]
    async fn both_present_prefers_enabled_hashes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.jar"), b"enabled-bytes").unwrap();
        std::fs::write(dir.path().join("a.jar.disabled"), b"disabled-bytes").unwrap();

        let packinfo = scan_instance_as_packinfo(dir.path()).await.unwrap();

        let hashes = packinfo
            .files
            .get("/a.jar")
            .expect("enabled file must be present under its own key");
        assert_eq!(hashes.md5, md5_of(b"enabled-bytes"));
    }

    #[tokio::test]
    async fn saves_are_excluded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("saves/world")).unwrap();
        std::fs::write(dir.path().join("saves/world/level.dat"), b"save").unwrap();
        std::fs::write(dir.path().join("a.jar"), b"kept").unwrap();

        let packinfo = scan_instance_as_packinfo(dir.path()).await.unwrap();

        assert!(packinfo.files.contains_key("/a.jar"));
        assert!(
            !packinfo.files.keys().any(|k| k.starts_with("/saves")),
            "got keys: {:?}",
            packinfo.files.keys().collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn install_audit_is_excluded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".install_audit")).unwrap();
        std::fs::write(dir.path().join(".install_audit/log.json"), b"audit").unwrap();
        std::fs::write(dir.path().join("a.jar"), b"kept").unwrap();

        let packinfo = scan_instance_as_packinfo(dir.path()).await.unwrap();

        assert!(packinfo.files.contains_key("/a.jar"));
        assert!(
            !packinfo
                .files
                .keys()
                .any(|k| k.starts_with("/.install_audit")),
            "got keys: {:?}",
            packinfo.files.keys().collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn nested_directories_are_walked() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods/sub")).unwrap();
        std::fs::write(dir.path().join("mods/sub/nested.jar"), b"nested-bytes").unwrap();

        let packinfo = scan_instance_as_packinfo(dir.path()).await.unwrap();

        let hashes = packinfo
            .files
            .get("/mods/sub/nested.jar")
            .expect("nested file must be found by the walk");
        assert_eq!(hashes.md5, md5_of(b"nested-bytes"));
    }
}
