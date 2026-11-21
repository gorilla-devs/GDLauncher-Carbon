//! Produces [`apply_plan::DiskState`] from the real filesystem for a given
//! universe of packinfo-style paths, and reads the hashes of an installed
//! pack's files as a [`PackInfo`] for the repair skip-oracle.
//!
//! Both are given the paths to look at and report what is actually on disk
//! right now; neither reads anything else in the instance folder. Callers
//! combine their output with `old`/`target` packinfos in
//! [`apply_plan::plan`](super::apply_plan::plan).

use anyhow::Context;
use futures::{StreamExt, TryStreamExt};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::time::SystemTime;

use md5::Md5;
use sha2::{Digest, Sha512};

use super::apply_plan;
use super::packinfo::{FileHashes, PackInfo};

/// Fresh requested-path states and hashes for coexisting disabled twins.
/// The twin hash supports the existing legacy VersionChange retry rule.
/// New publication retains staged sources, so Repair retries use those
/// verified sources instead of inferring ownership from a matching twin.
pub struct DiskScan {
    pub states: HashMap<String, apply_plan::DiskState>,
    pub coexisting_disabled_twin_md5: HashMap<String, [u8; 16]>,
}

/// Disk state for every path in `universe` (packinfo-style keys with leading '/').
/// Probes `<data>/<path>` then `<data>/<path>.disabled`; hashes md5. When the
/// bare spelling is present, also probes the `.disabled` twin (see
/// [`DiskScan`]'s own doc for why) — this costs one extra probe only in that
/// branch; the twin-only and neither-present branches already probed it.
pub async fn scan_disk_state(
    data_path: &Path,
    universe: &BTreeSet<String>,
) -> anyhow::Result<DiskScan> {
    let mut futures = Vec::with_capacity(universe.len());

    for path in universe {
        let disk_path = tracked_path(data_path, path)?;
        let key = path.clone();

        futures.push(async move {
            let (state, coexisting_twin_md5) = match probe_md5(&disk_path).await? {
                Some(md5) => {
                    let twin_md5 = probe_md5(&disabled_sibling(&disk_path)).await?;
                    (apply_plan::DiskState::Present { md5 }, twin_md5)
                }
                None => match probe_md5(&disabled_sibling(&disk_path)).await? {
                    Some(md5) => (apply_plan::DiskState::Disabled { md5 }, None),
                    None => (apply_plan::DiskState::Missing, None),
                },
            };

            Ok::<_, anyhow::Error>((key, state, coexisting_twin_md5))
        });
    }

    let results = futures::stream::iter(futures)
        .buffer_unordered(8)
        .try_collect::<Vec<_>>()
        .await?;

    let mut states = HashMap::with_capacity(results.len());
    let mut coexisting_disabled_twin_md5 = HashMap::new();
    for (key, state, twin_md5) in results {
        if let Some(twin_md5) = twin_md5 {
            coexisting_disabled_twin_md5.insert(key.clone(), twin_md5);
        }
        states.insert(key, state);
    }

    Ok(DiskScan {
        states,
        coexisting_disabled_twin_md5,
    })
}

/// Where the file of a pack record key is in the instance, refusing a key
/// that leads outside it. The record is a file anyone can edit.
fn tracked_path(data_path: &Path, key: &str) -> anyhow::Result<PathBuf> {
    // `strip_prefix` rather than `[1..]`: char-boundary-safe (an empty
    // key or one starting with a multibyte character would panic on a
    // byte-index slice), and it gives `None` instead of a garbage
    // substring for a key that doesn't actually start with '/'.
    let rel = key
        .strip_prefix('/')
        .ok_or_else(|| anyhow::anyhow!("packinfo key '{key}' must start with '/'"))?;
    // Re-checked here rather than trusted from `parse_packinfo`: a `..`
    // segment survives `Path::join` as a literal component, and
    // `Path::starts_with` below compares components lexically without
    // ever resolving `..` — so `data_path.join("mods/../../evil")`
    // would otherwise pass the containment check below even though the
    // real, OS-resolved path escapes `data_path` entirely.
    if super::packinfo::has_dotdot_segment(key) {
        anyhow::bail!("packinfo key '{key}' contains a '..' path segment");
    }
    let disk_path = data_path.join(rel);
    // Belt-and-braces even though `parse_packinfo` already rejects a
    // doubled leading '/': `Path::join` with an absolute argument
    // REPLACES the base entirely, so a key like "//tmp/esc" (whose tail
    // after stripping one '/' is itself still absolute) would otherwise
    // resolve to a path outside `data_path` — never trust a join alone.
    if !disk_path.starts_with(data_path) {
        anyhow::bail!("packinfo key '{key}' escapes the instance data dir");
    }
    Ok(disk_path)
}

/// Probe the data directory itself (case policy can be per-directory).
/// Only this call's exclusively created file is removed; old prefix matches
/// provide no evidence of ownership and are never swept.
pub async fn probe_case_insensitive(dir: &Path) -> bool {
    let dir = dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let probe = tempfile::Builder::new()
            .prefix(".gdl-case-probe-")
            .tempfile_in(&dir)?;
        let name = probe
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_uppercase();
        match std::fs::metadata(dir.join(name)) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    })
    .await
    .ok()
    .and_then(Result::ok)
    .unwrap_or(cfg!(any(windows, target_os = "macos")))
}

/// Resolve existing aliases of the actual saves root without conflating
/// independent case-sensitive directories. Resolve ancestors too, so a new
/// file under an existing symlink into saves is protected consistently.
pub(crate) fn resolves_into_saves(data: &Path, path: &Path) -> bool {
    let Ok(saves) = std::fs::canonicalize(data.join("saves")) else {
        return false;
    };
    resolves_into(&saves, path)
}

/// [`resolves_into_saves`] against an already resolved saves root.
fn resolves_into(saves: &Path, path: &Path) -> bool {
    let mut candidate = Some(path);
    while let Some(current) = candidate {
        if let Ok(resolved) = std::fs::canonicalize(current) {
            return resolved.starts_with(saves);
        }
        candidate = current.parent();
    }
    false
}

/// Physical save aliases get the same preview/apply decision as /saves.
/// The executor repeats this check at publication to cover later aliases.
pub(crate) fn protect_resolved_saves(
    entries: &mut [apply_plan::PlanEntry],
    data: &Path,
    old: Option<&PackInfo>,
) {
    // One plan is one moment: the saves root is resolved once for all of it.
    let Ok(saves) = std::fs::canonicalize(data.join("saves")) else {
        return;
    };
    for entry in entries {
        let live = data.join(entry.path.trim_start_matches('/'));
        if resolves_into(&saves, &live)
            && (live.exists()
                || disabled_sibling(&live).exists()
                || old.is_some_and(|record| record.files.contains_key(&entry.path)))
        {
            entry.action = apply_plan::PlanAction::Keep;
            entry.reason = apply_plan::PlanReason::InSaveFolder;
        }
    }
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

/// Why a link on a path the pack tracks cannot be read, with the way out:
/// its target is usually on a drive that is not plugged in, or was moved.
const UNAVAILABLE_LINK: &str = "is a link to a file or folder that cannot be found. Reconnect \
     the drive or folder it points to, or remove the link, then try again.";

/// Opens the file at a tracked path, or `Ok(None)` if it doesn't exist —
/// including when an intermediate directory component is missing.
async fn open_tracked(path: &Path) -> anyhow::Result<Option<tokio::fs::File>> {
    // Asked before opening: opening a pipe waits for its other end.
    if let Ok(metadata) = tokio::fs::metadata(path).await {
        anyhow::ensure!(
            metadata.is_file(),
            "Could not read {}: it is not a regular file",
            path.display()
        );
    }
    match tokio::fs::File::open(path).await {
        Ok(file) => Ok(Some(file)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // A dangling entry (including an ancestor link) is not verified
            // absence. Keep required-path failures visible to the fresh scan.
            let mut candidate = Some(path);
            while let Some(entry) = candidate {
                match tokio::fs::symlink_metadata(entry).await {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        if let Err(error) = tokio::fs::metadata(entry).await {
                            return Err(anyhow::Error::new(error)
                                .context(format!("{} {UNAVAILABLE_LINK}", entry.display())));
                        }
                        break;
                    }
                    Ok(_) if entry == path => {
                        return Err(anyhow::Error::new(e)
                            .context(format!("Could not open {}", path.display())));
                    }
                    Ok(_) => break,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        candidate = entry.parent();
                    }
                    Err(error) => {
                        return Err(anyhow::Error::new(error)
                            .context(format!("Could not open {}", entry.display())));
                    }
                }
            }
            Ok(None)
        }
        Err(e) => Err(anyhow::Error::new(e).context(format!("Could not open {}", path.display()))),
    }
}

/// The md5 of the file at a tracked path, or `Ok(None)` if it doesn't exist.
pub(crate) async fn probe_md5(path: &Path) -> anyhow::Result<Option<[u8; 16]>> {
    let Some(mut file) = open_tracked(path).await? else {
        return Ok(None);
    };
    let mut md5 = Md5::new();
    carbon_scheduler::buffered_digest(&mut file, |chunk| {
        md5.update(&chunk);
    })
    .await
    .with_context(|| format!("Could not read {}", path.display()))?;

    Ok(Some(md5.finalize().into()))
}

/// [`probe_md5`] with both hashes a pack record keeps.
async fn probe_hashes(path: &Path) -> anyhow::Result<Option<FileHashes>> {
    let Some(mut file) = open_tracked(path).await? else {
        return Ok(None);
    };
    let mut sha512 = Sha512::new();
    let mut md5 = Md5::new();
    carbon_scheduler::buffered_digest(&mut file, |chunk| {
        sha512.update(&chunk);
        md5.update(&chunk);
    })
    .await
    .with_context(|| format!("Could not read {}", path.display()))?;

    Ok(Some(FileHashes {
        sha512: sha512.finalize().into(),
        md5: md5.finalize().into(),
    }))
}

/// What a Repair can skip downloading: the hashes, read now, of the files
/// the installed pack record lists (`recorded`, packinfo-style keys). A file
/// is keyed under its enabled name whether it is switched off (`.disabled`)
/// or not, and the enabled file's hashes win when both exist. A listed file
/// that is gone has no entry, so it is downloaded. One that is there but
/// cannot be read fails the Repair, as it fails the apply's own scan
/// ([`scan_disk_state`]).
///
/// Nothing outside the record is read. Only pack paths are ever looked up in
/// the result, and the rest of an instance folder (backups, map caches,
/// screenshots, linked folders) can be tens of GB. Saves are never hashed.
///
/// `checked` gets the number of files done so far and the number to do.
pub async fn scan_repair_oracle(
    data_path: &Path,
    recorded: &BTreeSet<String>,
    checked: impl Fn(u32, u32),
) -> anyhow::Result<PackInfo> {
    let saves = std::fs::canonicalize(data_path.join("saves")).ok();
    let mut paths = Vec::with_capacity(recorded.len());
    for key in recorded {
        let path = tracked_path(data_path, key)?;
        // Saves, under their own name or through a link, are never hashed.
        if key.starts_with("/.install_audit")
            || saves
                .as_deref()
                .is_some_and(|saves| resolves_into(saves, &path))
        {
            continue;
        }
        paths.push((key.clone(), path));
    }

    let total = paths.len() as u32;
    checked(0, total);
    let mut probes = futures::stream::iter(paths.into_iter().map(|(key, path)| async move {
        let hashes = match probe_hashes(&path).await? {
            Some(hashes) => Some(hashes),
            None => probe_hashes(&disabled_sibling(&path)).await?,
        };
        Ok::<_, anyhow::Error>((key, hashes))
    }))
    .buffer_unordered(8);

    let mut files = HashMap::new();
    let mut done = 0;
    while let Some((key, hashes)) = probes.try_next().await? {
        done += 1;
        checked(done, total);
        if let Some(hashes) = hashes {
            files.insert(key, hashes);
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

    #[cfg(unix)]
    #[tokio::test]
    async fn fresh_required_dangling_path_is_error_not_verified_missing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("mods")).unwrap();
        std::os::unix::fs::symlink("absent", dir.path().join("mods/required.jar")).unwrap();
        let error = scan_disk_state(dir.path(), &universe(&["/mods/required.jar"]))
            .await
            .err()
            .expect("a dangling tracked link is not verified absence");
        let link = dir.path().join("mods/required.jar");
        assert!(
            error.to_string().contains(&link.display().to_string()),
            "{error:#}"
        );
    }

    /// A tracked folder linked to a drive that is not plugged in fails an
    /// update, the repair preview and Repair with the link's name and the
    /// way out, not a bare "No such file or directory".
    #[cfg(unix)]
    #[tokio::test]
    async fn a_dangling_link_on_a_tracked_path_names_the_link_and_the_way_out() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("resourcepacks");
        std::os::unix::fs::symlink(dir.path().join("unplugged"), &link).unwrap();
        let tracked = universe(&["/resourcepacks/pack.zip"]);
        let error = scan_disk_state(dir.path(), &tracked)
            .await
            .err()
            .expect("a dangling tracked folder link is not verified absence");
        let message = error.to_string();
        assert!(message.contains(&link.display().to_string()), "{error:#}");
        assert!(message.contains("cannot be found"), "{error:#}");
        assert!(message.contains("Reconnect the drive"), "{error:#}");
        assert!(message.contains("remove the link"), "{error:#}");

        let message = scan_repair_oracle(dir.path(), &tracked, |_, _| {})
            .await
            .unwrap_err()
            .to_string();
        assert!(message.contains(&link.display().to_string()), "{message}");
        assert!(message.contains("Reconnect the drive"), "{message}");
        assert!(message.contains("remove the link"), "{message}");
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
            result.states.get("/mods/a.jar"),
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
            result.states.get("/mods/a.jar"),
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
            result.states.get("/mods/a.jar"),
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
            result.states.get("/mods/a.jar"),
            Some(&apply_plan::DiskState::Present {
                md5: md5_of(b"enabled-bytes")
            })
        );
        // The coexisting `.disabled` twin's own md5 is also surfaced
        // alongside `Present` — the shape `DiskState` alone can't express,
        // and what an interrupted disabled `PlanAction::Replace` leaves
        // behind (see `DiskScan`'s own doc).
        assert_eq!(
            result.coexisting_disabled_twin_md5.get("/mods/a.jar"),
            Some(&md5_of(b"disabled-bytes"))
        );
    }

    #[tokio::test]
    async fn bare_only_reports_no_coexisting_twin() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods")).unwrap();
        std::fs::write(dir.path().join("mods/a.jar"), b"enabled-bytes").unwrap();

        let result = scan_disk_state(dir.path(), &universe(&["/mods/a.jar"]))
            .await
            .unwrap();

        assert!(
            result
                .coexisting_disabled_twin_md5
                .get("/mods/a.jar")
                .is_none(),
            "no twin on disk must never synthesize a coexisting-twin entry"
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
            result.states.get("/saves/world/level.dat"),
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
            result.states.get("/nonexistent/a.jar"),
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

    // --- scan_repair_oracle -------------------------------------------------

    fn sha512_of(bytes: &[u8]) -> [u8; 64] {
        Sha512::digest(bytes).into()
    }

    async fn oracle(data: &Path, recorded: &[&str]) -> anyhow::Result<PackInfo> {
        scan_repair_oracle(data, &universe(recorded), |_, _| {}).await
    }

    /// A long-played instance holds tens of GB the pack never shipped. A
    /// Repair reads the pack's files and leaves the rest alone.
    #[tokio::test]
    async fn the_repair_check_reads_the_files_the_record_lists_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods")).unwrap();
        std::fs::create_dir_all(dir.path().join("backups")).unwrap();
        std::fs::write(dir.path().join("mods/pack.jar"), b"pack").unwrap();
        std::fs::write(dir.path().join("mods/own.jar"), b"own").unwrap();
        std::fs::write(dir.path().join("backups/world.zip"), b"backup").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("absent", dir.path().join("backups/unplugged")).unwrap();

        let info = oracle(dir.path(), &["/mods/pack.jar", "/mods/gone.jar"])
            .await
            .unwrap();

        assert_eq!(info.files.keys().collect::<Vec<_>>(), ["/mods/pack.jar"]);
        assert_eq!(info.files["/mods/pack.jar"].md5, md5_of(b"pack"));
        assert_eq!(info.files["/mods/pack.jar"].sha512, sha512_of(b"pack"));
    }

    #[tokio::test]
    async fn a_switched_off_file_counts_under_its_enabled_name_and_the_enabled_file_wins() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods/sub")).unwrap();
        std::fs::write(dir.path().join("mods/sub/off.jar.disabled"), b"off").unwrap();
        std::fs::write(dir.path().join("mods/both.jar"), b"enabled").unwrap();
        std::fs::write(dir.path().join("mods/both.jar.disabled"), b"disabled").unwrap();

        let info = oracle(dir.path(), &["/mods/sub/off.jar", "/mods/both.jar"])
            .await
            .unwrap();

        assert_eq!(info.files.len(), 2);
        assert_eq!(info.files["/mods/sub/off.jar"].md5, md5_of(b"off"));
        assert_eq!(info.files["/mods/both.jar"].md5, md5_of(b"enabled"));
    }

    #[tokio::test]
    async fn the_repair_check_never_hashes_saves_or_the_install_audit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("saves/world")).unwrap();
        std::fs::create_dir_all(dir.path().join(".install_audit")).unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(dir.path().join("saves/world/level.dat"), b"save").unwrap();
        std::fs::write(dir.path().join(".install_audit/log.json"), b"audit").unwrap();
        std::fs::write(dir.path().join("a.jar"), b"kept").unwrap();
        let mut recorded = vec![
            "/a.jar",
            "/saves/world/level.dat",
            "/.install_audit/log.json",
        ];
        // A folder that is a link into saves is saves too.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                dir.path().join("saves/world"),
                dir.path().join("config/linked"),
            )
            .unwrap();
            recorded.push("/config/linked/level.dat");
        }

        let counts = std::cell::RefCell::new(Vec::new());
        let info = scan_repair_oracle(dir.path(), &universe(&recorded), |done, total| {
            counts.borrow_mut().push((done, total))
        })
        .await
        .unwrap();

        assert_eq!(info.files.keys().collect::<Vec<_>>(), ["/a.jar"]);
        assert_eq!(counts.into_inner(), [(0, 1), (1, 1)]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_recorded_file_that_cannot_be_read_fails_the_repair_check() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods")).unwrap();
        std::fs::create_dir_all(dir.path().join("config/folder.toml")).unwrap();
        std::fs::write(dir.path().join("mods/good.jar"), b"good").unwrap();
        std::os::unix::fs::symlink("good.jar", dir.path().join("mods/alias.jar")).unwrap();
        std::os::unix::fs::symlink("absent", dir.path().join("mods/required.jar.disabled"))
            .unwrap();

        // A link to a file that is there is read through.
        let info = oracle(dir.path(), &["/mods/good.jar", "/mods/alias.jar"])
            .await
            .unwrap();
        assert_eq!(info.files["/mods/alias.jar"].md5, md5_of(b"good"));

        assert!(oracle(dir.path(), &["/mods/required.jar"]).await.is_err());
        let error = oracle(dir.path(), &["/config/folder.toml"])
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("not a regular file"),
            "{error:#}"
        );
    }

    #[tokio::test]
    async fn a_record_path_that_leads_outside_the_instance_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("instance");
        std::fs::create_dir_all(data.join("mods")).unwrap();
        std::fs::write(dir.path().join("outside.jar"), b"outside").unwrap();

        assert!(oracle(&data, &["/mods/../../outside.jar"]).await.is_err());
        assert!(oracle(&data, &["//tmp/esc"]).await.is_err());
        assert!(oracle(&data, &["mods/a.jar"]).await.is_err());
    }

    #[tokio::test]
    async fn the_repair_check_counts_every_recorded_file_it_has_done() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("mods")).unwrap();
        let recorded: Vec<String> = (0..512).map(|n| format!("/mods/{n}.jar")).collect();
        // The last one is gone: it is counted, and has no entry.
        for path in &recorded[..511] {
            std::fs::write(dir.path().join(&path[1..]), vec![42; 32768]).unwrap();
        }

        let counts = std::cell::RefCell::new(Vec::new());
        let info = scan_repair_oracle(
            dir.path(),
            &recorded.iter().cloned().collect(),
            |done, total| counts.borrow_mut().push((done, total)),
        )
        .await
        .unwrap();

        assert_eq!(info.files.len(), 511);
        let expected: Vec<(u32, u32)> = (0..=512).map(|done| (done, 512)).collect();
        assert_eq!(counts.into_inner(), expected);
    }

    // --- probe_case_insensitive ---------------------------------------

    #[tokio::test]
    async fn probe_case_insensitive_returns_a_bool_and_cleans_up_after_itself() {
        // CI filesystems vary (ext4 vs. overlay vs. whatever a container
        // host mounts), so this deliberately does not assert which bool
        // comes back — only that the probe completes and leaves no marker
        // file behind, under either outcome.
        let dir = tempfile::tempdir().unwrap();

        let _ = probe_case_insensitive(dir.path()).await;

        let mut leftovers = tokio::fs::read_dir(dir.path()).await.unwrap();
        assert!(
            leftovers.next_entry().await.unwrap().is_none(),
            "the probe file must be removed regardless of the outcome"
        );
    }

    /// Creates `path` with its mtime backdated to `mtime`, for exercising
    /// the age-gated stale-probe sweep without waiting a real minute.
    fn touch_with_mtime(path: &std::path::Path, mtime: std::time::SystemTime) {
        let file = std::fs::File::create(path).unwrap();
        file.set_modified(mtime).unwrap();
    }

    #[tokio::test]
    async fn probe_case_insensitive_preserves_unowned_old_files_of_either_casing() {
        let dir = tempfile::tempdir().unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);

        // Deliberately different literal names (not case-variants of each
        // other) so this stays meaningful even on a case-insensitive CI
        // host, where two spellings of the SAME name would alias one file.
        let stale_lower = dir.path().join(".gdl-case-probe-stale-lower");
        let stale_upper = dir.path().join(".GDL-CASE-PROBE-STALE-UPPER");
        touch_with_mtime(&stale_lower, old);
        touch_with_mtime(&stale_upper, old);

        let _ = probe_case_insensitive(dir.path()).await;

        assert!(
            stale_lower.exists(),
            "old prefix matches are not owned probes"
        );
        assert!(stale_upper.exists(), "case variants also remain unowned");
    }

    #[tokio::test]
    async fn probe_case_insensitive_does_not_remove_a_fresh_same_prefixed_file() {
        // Pins the race-avoidance mechanism directly, without needing actual
        // concurrency: a file matching the probe's own naming prefix but
        // freshly written (as a genuinely concurrent caller's in-flight
        // probe file would be) must survive another call's cleanup sweep —
        // unowned names are never removed by another probe.
        let dir = tempfile::tempdir().unwrap();
        let concurrent = dir.path().join(".gdl-case-probe-concurrent-simulated");
        std::fs::write(&concurrent, b"in-flight-from-another-call").unwrap();

        let _ = probe_case_insensitive(dir.path()).await;

        assert!(
            concurrent.exists(),
            "a fresh, still-in-flight-looking probe file must not be swept as stale"
        );
    }
    #[tokio::test]
    async fn case_probe_preserves_aged_user_notes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".gdl-case-probe-user-notes.txt");
        std::fs::write(&path, b"my notes").unwrap();
        // Windows sets file times only through a write-capable handle.
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH)
            .unwrap();
        probe_case_insensitive(dir.path()).await;
        assert_eq!(std::fs::read(path).unwrap(), b"my notes");
    }
}
