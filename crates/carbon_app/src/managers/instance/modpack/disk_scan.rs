//! Produces [`apply_plan::DiskState`] from the real filesystem for a given
//! universe of packinfo-style paths, and takes a full disk snapshot of an
//! instance's data directory as a [`PackInfo`] for the repair skip-oracle.
//!
//! Neither function consults any pack metadata — they only look at what is
//! actually on disk right now. Callers combine their output with `old`/
//! `target` packinfos in [`apply_plan::plan`](super::apply_plan::plan).

use anyhow::Context;
use futures::{StreamExt, TryStreamExt};
use std::collections::{BTreeSet, HashMap, HashSet};
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

/// Opens `path` and md5-hashes it, or `Ok(None)` if it doesn't exist —
/// including when an intermediate directory component is missing.
pub(crate) async fn probe_md5(path: &Path) -> anyhow::Result<Option<[u8; 16]>> {
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
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
            return Ok(None);
        }
        Err(e) => {
            return Err(anyhow::Error::new(e).context(format!("Could not open {}", path.display())));
        }
    };

    let mut md5 = Md5::new();
    carbon_scheduler::buffered_digest(&mut file, |chunk| {
        md5.update(&chunk);
    })
    .await
    .with_context(|| format!("Could not read {}", path.display()))?;

    Ok(Some(md5.finalize().into()))
}

/// Repair skip-oracle: full scan of the instance data dir, producing both a
/// [`PackInfo`] (md5+sha512, keyed under enabled names with `.disabled`
/// stripped, skipping `/saves` and `.install_audit` — same as
/// [`scan_dir`](super::packinfo::scan_dir)) for the skip-optimising prep
/// functions, and a [`DiskScan`] with Present/Disabled/twin semantics
/// identical to [`scan_disk_state`] over that same directory. Both are
/// derived from one walk so a repair's
/// [`super::super::run::modpack::process_modpack_staging`] can reuse the
/// `DiskScan` half instead of paying for a second full-tree hash of the
/// instance data dir. When both `X` and `X.disabled` exist the ENABLED
/// file's hashes win in both outputs (deterministic, unlike scan_dir's racy
/// insert).
///
/// `/saves` and `.install_audit` are never hashed and never appear in the
/// returned `PackInfo`, exactly as before — but a file under either that the
/// walk finds still needs to land in `DiskScan`: a modpack override can
/// legitimately ship a file under `/saves` (see [`apply_plan::plan`]'s own
/// doc), so its planner needs to know whether such a path exists on disk at
/// all. That branch of the planner only ever inspects the `DiskState`
/// variant for a `/saves` path (`Present`/`Disabled`/`Missing`), never the
/// md5 carried by the first two, so a placeholder stands in for the real
/// hash here — recording existence, which the walk already observed for
/// free, without opening and hashing a save that can be many gigabytes,
/// which would defeat the entire point of this function producing a
/// `DiskScan` a repair can reuse in the first place.
/// A failed optional oracle observation. UTF-8 paths remain attributable until
/// the full old/target universe is known; invalid names cannot match a manifest.
#[derive(Debug)]
pub struct OracleDiagnostic {
    pub path: Option<String>,
    pub message: String,
}

pub fn validate_oracle_diagnostics(
    diagnostics: &[OracleDiagnostic],
    universe: &BTreeSet<String>,
    insensitive: bool,
) -> anyhow::Result<()> {
    for diagnostic in diagnostics {
        let Some(path) = &diagnostic.path else {
            continue;
        };
        let normalize = |p: &str| {
            let p = if insensitive {
                p.to_lowercase()
            } else {
                p.to_owned()
            };
            p.strip_suffix(".disabled")
                .unwrap_or(&p)
                .trim_end_matches('/')
                .to_owned()
        };
        let path = normalize(path);
        if path.is_empty()
            || universe.iter().any(|key| {
                let key = normalize(key);
                key == path
                    || key.starts_with(&(path.clone() + "/"))
                    || path.starts_with(&(key + "/"))
            })
        {
            anyhow::bail!(
                "Could not read {} in the instance folder: {}",
                diagnostic.path.as_deref().unwrap_or("/"),
                diagnostic.message
            );
        }
    }
    Ok(())
}

pub async fn scan_instance_as_packinfo(data_path: &Path) -> anyhow::Result<(PackInfo, DiskScan)> {
    let (info, scan, _) = scan_repair_oracle(data_path).await?;
    Ok((info, scan))
}

/// A folder the repair walk is inside, to spot a folder link that leads back
/// into it.
#[derive(Clone)]
struct WalkedFolder {
    path: PathBuf,
    /// Where it really is. None on a volume that cannot report final paths
    /// (some Windows RAM disks and virtual drives); such a folder is compared
    /// by file identity instead.
    resolved: Option<PathBuf>,
}

impl WalkedFolder {
    fn new(path: &Path) -> Self {
        let resolved = match resolve_folder(path) {
            Ok(resolved) => Some(resolved),
            Err(error) => {
                tracing::debug!(path = %path.display(), "repair oracle: folder path not resolved: {error}");
                None
            }
        };
        Self {
            path: path.to_owned(),
            resolved,
        }
    }

    fn is(&self, other: &Self) -> bool {
        match (&self.resolved, &other.resolved) {
            (Some(resolved), Some(other)) => resolved == other,
            _ => same_file::is_same_file(&self.path, &other.path).unwrap_or(false),
        }
    }
}

fn resolve_folder(path: &Path) -> std::io::Result<PathBuf> {
    #[cfg(test)]
    if unresolvable::contains(path) {
        return Err(std::io::ErrorKind::Unsupported.into());
    }
    std::fs::canonicalize(path)
}

/// Folders whose final path the test volume "cannot report".
#[cfg(test)]
mod unresolvable {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::sync::{LazyLock, Mutex};

    static PATHS: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(Default::default);

    pub(super) fn add(path: &Path) {
        PATHS.lock().unwrap().insert(path.to_owned());
    }

    pub(super) fn contains(path: &Path) -> bool {
        PATHS.lock().unwrap().contains(path)
    }
}

pub async fn scan_repair_oracle(
    data_path: &Path,
) -> anyhow::Result<(PackInfo, DiskScan, Vec<OracleDiagnostic>)> {
    let mut diagnostics = Vec::new();
    let mut futures = Vec::new();
    let mut unhashed_presence: Vec<(String, bool)> = Vec::new();
    let mut pending = vec![(
        data_path.to_owned(),
        std::fs::read_dir(data_path)?,
        vec![WalkedFolder::new(data_path)],
    )];
    while let Some((directory, reader, ancestors)) = pending.pop() {
        for entry in reader {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    let relative = directory.strip_prefix(data_path)?;
                    diagnostics.push(OracleDiagnostic {
                        path: relative
                            .to_str()
                            .map(|p| format!("/{}", p.replace('\\', "/"))),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            let path = entry.path();
            let relative = path.strip_prefix(data_path)?;
            let Some(relative) = relative.to_str() else {
                diagnostics.push(OracleDiagnostic {
                    path: None,
                    message: format!("non UTF-8 path: {relative:?}"),
                });
                continue;
            };
            let relative_path = format!("/{}", relative.replace('\\', "/"));
            let metadata = match std::fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    let link = error.kind() == std::io::ErrorKind::NotFound
                        && std::fs::symlink_metadata(&path)
                            .is_ok_and(|metadata| metadata.file_type().is_symlink());
                    diagnostics.push(OracleDiagnostic {
                        path: Some(relative_path),
                        message: if link {
                            format!("it {UNAVAILABLE_LINK}")
                        } else {
                            error.to_string()
                        },
                    });
                    continue;
                }
            };
            if metadata.is_dir() {
                let folder = WalkedFolder::new(&path);
                let result = if ancestors.iter().any(|ancestor| ancestor.is(&folder)) {
                    Err(std::io::Error::other("directory link cycle"))
                } else {
                    std::fs::read_dir(&path).map(|reader| {
                        let mut next = ancestors.clone();
                        next.push(folder);
                        (path.clone(), reader, next)
                    })
                };
                match result {
                    Ok(next) => pending.push(next),
                    Err(error) => diagnostics.push(OracleDiagnostic {
                        path: Some(relative_path),
                        message: error.to_string(),
                    }),
                }
                continue;
            }
            if !metadata.is_file() {
                diagnostics.push(OracleDiagnostic {
                    path: Some(relative_path),
                    message: "not a regular file".into(),
                });
                continue;
            }
            let from_disabled = relative_path.ends_with(".disabled");
            let key = relative_path
                .strip_suffix(".disabled")
                .unwrap_or(&relative_path)
                .to_owned();
            if apply_plan::is_saves_path(&relative_path)
                || resolves_into_saves(data_path, &path)
                || relative_path.starts_with("/.install_audit")
            {
                unhashed_presence.push((key, from_disabled));
                continue;
            }
            futures.push(async move {
                let result = async {
                    let mut file = tokio::fs::File::open(path).await?;
                    let mut sha512 = Sha512::new();
                    let mut md5 = Md5::new();
                    carbon_scheduler::buffered_digest(&mut file, |chunk| {
                        sha512.update(&chunk);
                        md5.update(&chunk);
                    })
                    .await?;
                    Ok::<_, anyhow::Error>((
                        key,
                        FileHashes {
                            sha512: sha512.finalize().into(),
                            md5: md5.finalize().into(),
                        },
                        from_disabled,
                    ))
                }
                .await;
                (relative_path, result)
            });
        }
    }
    let mut results = Vec::new();
    for (path, result) in futures::stream::iter(futures)
        .buffer_unordered(8)
        .collect::<Vec<_>>()
        .await
    {
        match result {
            Ok(value) => results.push(value),
            Err(error) => diagnostics.push(OracleDiagnostic {
                path: Some(path),
                message: error.to_string(),
            }),
        }
    }
    for diagnostic in &diagnostics {
        tracing::warn!(path = ?diagnostic.path, "repair oracle: {}", diagnostic.message);
    }

    // Enabled-wins, order-independent: a `.disabled`-sourced insertion never
    // overwrites an already-present entry unless that entry itself came from
    // a `.disabled` twin (tracked in `disabled_backed`); an enabled-sourced
    // insertion always overwrites, regardless of what (if anything) preceded
    // it. This is deterministic regardless of the walker's/readdir's
    // discovery order, unlike `scan_dir`'s plain `collect()`.
    let mut files: HashMap<String, FileHashes> = HashMap::new();
    let mut disabled_backed: HashSet<String> = HashSet::new();

    for (key, hashes, from_disabled) in &results {
        if *from_disabled {
            if !files.contains_key(key) || disabled_backed.contains(key) {
                files.insert(key.clone(), hashes.clone());
                disabled_backed.insert(key.clone());
            }
        } else {
            files.insert(key.clone(), hashes.clone());
            disabled_backed.remove(key);
        }
    }

    // `DiskScan` companion to `files` above, built from the same walk.
    // Tracked via separate enabled/disabled md5 maps rather than reusing
    // `files`/`disabled_backed`: a coexisting `.disabled` twin's md5 is
    // exactly what `disabled_backed`'s bookkeeping discards once an enabled
    // copy wins the key above, but `DiskScan::coexisting_disabled_twin_md5`
    // needs that value regardless. Placeholder md5s from `unhashed_presence`
    // (see this function's own doc) are folded in via `or_insert` only,
    // never able to clobber a real hashed value — a path is either under
    // `/saves`/`.install_audit` or hashed, never both, so this can't happen
    // in practice, but `or_insert` keeps that an explicit ordering rather
    // than an assumed invariant.
    const UNHASHED_PRESENCE_PLACEHOLDER: [u8; 16] = [0xEE; 16];

    let mut enabled_md5: HashMap<String, [u8; 16]> = HashMap::new();
    let mut disabled_md5: HashMap<String, [u8; 16]> = HashMap::new();
    for (key, hashes, from_disabled) in &results {
        if *from_disabled {
            disabled_md5.entry(key.clone()).or_insert(hashes.md5);
        } else {
            enabled_md5.entry(key.clone()).or_insert(hashes.md5);
        }
    }
    for (key, from_disabled) in &unhashed_presence {
        if *from_disabled {
            disabled_md5
                .entry(key.clone())
                .or_insert(UNHASHED_PRESENCE_PLACEHOLDER);
        } else {
            enabled_md5
                .entry(key.clone())
                .or_insert(UNHASHED_PRESENCE_PLACEHOLDER);
        }
    }

    let mut states = HashMap::with_capacity(enabled_md5.len() + disabled_md5.len());
    let mut coexisting_disabled_twin_md5 = HashMap::new();
    for (key, &md5) in &enabled_md5 {
        states.insert(key.clone(), apply_plan::DiskState::Present { md5 });
        if let Some(&twin_md5) = disabled_md5.get(key) {
            coexisting_disabled_twin_md5.insert(key.clone(), twin_md5);
        }
    }
    for (key, &md5) in &disabled_md5 {
        if enabled_md5.contains_key(key) {
            // Already recorded as `Present` above; its own md5 is in
            // `coexisting_disabled_twin_md5`, not a second `states` entry.
            continue;
        }
        states.insert(key.clone(), apply_plan::DiskState::Disabled { md5 });
    }

    Ok((
        PackInfo { files },
        DiskScan {
            states,
            coexisting_disabled_twin_md5,
        },
        diagnostics,
    ))
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
    async fn task08_oracle_continues_after_unrelated_dangling_link() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("logs")).unwrap();
        std::fs::create_dir_all(dir.path().join("mods")).unwrap();
        std::fs::write(dir.path().join("mods/required.jar"), b"tracked").unwrap();
        std::os::unix::fs::symlink("absent", dir.path().join("logs/old-output")).unwrap();
        let (info, _) = scan_instance_as_packinfo(dir.path()).await.unwrap();
        assert_eq!(info.files["/mods/required.jar"].md5, md5_of(b"tracked"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn task08_oracle_continues_after_unrelated_non_utf8_name() {
        use std::os::unix::ffi::OsStringExt;
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("logs")).unwrap();
        std::fs::write(dir.path().join("tracked"), b"tracked").unwrap();
        match std::fs::write(
            dir.path()
                .join("logs")
                .join(std::ffi::OsString::from_vec(vec![255])),
            b"log",
        ) {
            Ok(()) => {}
            // APFS and HFS+ store names as UTF-8 and refuse other bytes with
            // EILSEQ, so such a name cannot exist there to be skipped.
            #[cfg(target_vendor = "apple")]
            Err(error) if error.raw_os_error() == Some(libc::EILSEQ) => return,
            Err(error) => panic!("creating the non-UTF-8 name failed: {error}"),
        }
        let (info, _) = scan_instance_as_packinfo(dir.path()).await.unwrap();
        assert_eq!(info.files["/tracked"].md5, md5_of(b"tracked"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn task08_required_oracle_faults_and_normal_links_remain_visible() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("mods")).unwrap();
        std::fs::create_dir(dir.path().join("logs")).unwrap();
        std::fs::write(dir.path().join("mods/good.jar"), b"good").unwrap();
        std::os::unix::fs::symlink("good.jar", dir.path().join("mods/alias.jar")).unwrap();
        std::os::unix::fs::symlink("absent", dir.path().join("mods/required.jar.disabled"))
            .unwrap();
        std::os::unix::fs::symlink("absent", dir.path().join("logs/unrelated")).unwrap();
        let (info, _, diagnostics) = scan_repair_oracle(dir.path()).await.unwrap();
        assert_eq!(info.files["/mods/alias.jar"].md5, md5_of(b"good"));
        assert_eq!(diagnostics.len(), 2);
        assert!(
            validate_oracle_diagnostics(&diagnostics, &universe(&["/mods/good.jar"]), false)
                .is_ok()
        );
        assert!(
            validate_oracle_diagnostics(&diagnostics, &universe(&["/mods/required.jar"]), false)
                .is_err()
        );
        assert!(
            validate_oracle_diagnostics(&diagnostics, &universe(&["/MODS/REQUIRED.JAR"]), true)
                .is_err()
        );
        assert!(
            scan_repair_oracle(&dir.path().join("missing-root"))
                .await
                .is_err()
        );
    }

    #[test]
    fn task08_oracle_ancestor_and_unattributable_root_faults_refuse() {
        let fault = |path: &str| OracleDiagnostic {
            path: Some(path.into()),
            message: "permission denied".into(),
        };
        assert!(
            validate_oracle_diagnostics(&[fault("/mods")], &universe(&["/mods/a.jar"]), false)
                .is_err()
        );
        assert!(validate_oracle_diagnostics(&[fault("/")], &universe(&[]), false).is_err());
        assert!(
            validate_oracle_diagnostics(&[fault("/logs")], &universe(&["/mods/a.jar"]), false)
                .is_ok()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn task08_fresh_required_dangling_path_is_error_not_verified_missing() {
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

        let (_, _, diagnostics) = scan_repair_oracle(dir.path()).await.unwrap();
        let message = validate_oracle_diagnostics(&diagnostics, &tracked, false)
            .unwrap_err()
            .to_string();
        assert!(message.contains("/resourcepacks"), "{message}");
        assert!(message.contains("Reconnect the drive"), "{message}");
        assert!(message.contains("remove the link"), "{message}");
    }

    /// Some Windows volumes (RAM disks, drives the Mount Manager does not
    /// map) cannot report a folder's final path. The repair scan still reads
    /// every file there, and still stops at a folder link that leads back
    /// into a folder it is in.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_repair_scan_works_where_folder_paths_cannot_be_resolved() {
        for root_resolves in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let data = dir.path().join("instance");
            std::fs::create_dir_all(data.join("mods/nested")).unwrap();
            std::fs::write(data.join("mods/a.jar"), b"a").unwrap();
            std::fs::write(data.join("mods/nested/b.jar"), b"b").unwrap();
            std::os::unix::fs::symlink(&data, data.join("mods/loop")).unwrap();
            if !root_resolves {
                unresolvable::add(&data);
            }
            for folder in ["mods", "mods/nested", "mods/loop"] {
                unresolvable::add(&data.join(folder));
            }
            let (info, scan, diagnostics) = scan_repair_oracle(&data).await.unwrap();
            assert_eq!(info.files.len(), 2, "{root_resolves}");
            assert_eq!(info.files["/mods/a.jar"].md5, md5_of(b"a"));
            assert_eq!(info.files["/mods/nested/b.jar"].md5, md5_of(b"b"));
            assert_eq!(scan.states.len(), 2, "{root_resolves}");
            assert_eq!(diagnostics.len(), 1, "{root_resolves}");
            assert_eq!(diagnostics[0].path.as_deref(), Some("/mods/loop"));
            assert_eq!(diagnostics[0].message, "directory link cycle");
        }
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

    // --- scan_instance_as_packinfo -----------------------------------------

    #[tokio::test]
    async fn keys_carry_a_leading_slash() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.jar"), b"hi").unwrap();

        let (packinfo, _disk_scan) = scan_instance_as_packinfo(dir.path()).await.unwrap();

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

        let (packinfo, _disk_scan) = scan_instance_as_packinfo(dir.path()).await.unwrap();

        assert!(packinfo.files.contains_key("/a.jar"));
        assert!(!packinfo.files.contains_key("/a.jar.disabled"));
    }

    #[tokio::test]
    async fn both_present_prefers_enabled_hashes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.jar"), b"enabled-bytes").unwrap();
        std::fs::write(dir.path().join("a.jar.disabled"), b"disabled-bytes").unwrap();

        let (packinfo, _disk_scan) = scan_instance_as_packinfo(dir.path()).await.unwrap();

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

        let (packinfo, _disk_scan) = scan_instance_as_packinfo(dir.path()).await.unwrap();

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

        let (packinfo, _disk_scan) = scan_instance_as_packinfo(dir.path()).await.unwrap();

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

        let (packinfo, _disk_scan) = scan_instance_as_packinfo(dir.path()).await.unwrap();

        let hashes = packinfo
            .files
            .get("/mods/sub/nested.jar")
            .expect("nested file must be found by the walk");
        assert_eq!(hashes.md5, md5_of(b"nested-bytes"));
    }

    // --- scan_instance_as_packinfo's paired DiskScan -----------------------

    #[tokio::test]
    async fn oracle_disk_scan_matches_a_direct_scan_disk_state_over_the_same_universe() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods")).unwrap();
        // Plain enabled-only file.
        std::fs::write(dir.path().join("mods/enabled.jar"), b"enabled-bytes").unwrap();
        // Plain disabled-only file (sole on-disk representation).
        std::fs::write(
            dir.path().join("mods/disabled.jar.disabled"),
            b"disabled-only-bytes",
        )
        .unwrap();
        // Coexisting enabled file + disabled twin.
        std::fs::write(dir.path().join("mods/twin.jar"), b"twin-enabled-bytes").unwrap();
        std::fs::write(
            dir.path().join("mods/twin.jar.disabled"),
            b"twin-disabled-bytes",
        )
        .unwrap();

        let (packinfo, oracle_scan) = scan_instance_as_packinfo(dir.path()).await.unwrap();

        let file_universe: BTreeSet<String> = packinfo.files.keys().cloned().collect();
        assert_eq!(
            file_universe,
            universe(&["/mods/enabled.jar", "/mods/disabled.jar", "/mods/twin.jar"])
        );

        // The oracle (`scan_instance_as_packinfo`'s own `DiskScan`) only ever
        // inserts a `states` entry for a path its walk actually found on
        // disk, in either spelling — a universe drawn solely from
        // `packinfo.files.keys()`, like `file_universe` above, can therefore
        // never contain a path absent from disk, since the oracle would
        // never have produced a key for one anyway. `absent` is added
        // directly to the universe passed to `scan_disk_state` below, which
        // — unlike the oracle — always inserts an explicit
        // `DiskState::Missing` entry for every universe path it's given.
        // The two maps are consequently never literally equal over this
        // wider universe; what both must actually agree on is the effective
        // lookup `apply_plan::plan` itself performs
        // (`disk.get(path).copied().unwrap_or(DiskState::Missing)`), so
        // that's what's compared below instead of raw map equality.
        const ABSENT: &str = "/mods/absent.jar";
        let mut universe_with_absent = file_universe.clone();
        universe_with_absent.insert(ABSENT.to_string());

        let direct_scan = scan_disk_state(dir.path(), &universe_with_absent)
            .await
            .unwrap();

        assert!(
            !oracle_scan.states.contains_key(ABSENT),
            "the oracle must report no states entry at all for a path absent from disk, got {:?}",
            oracle_scan.states.get(ABSENT)
        );
        assert_eq!(
            direct_scan.states.get(ABSENT).copied(),
            Some(apply_plan::DiskState::Missing),
            "scan_disk_state must report an explicit Missing for a universe path absent from disk"
        );

        for path in &universe_with_absent {
            let oracle_effective = oracle_scan
                .states
                .get(path)
                .copied()
                .unwrap_or(apply_plan::DiskState::Missing);
            let direct_effective = direct_scan
                .states
                .get(path)
                .copied()
                .unwrap_or(apply_plan::DiskState::Missing);
            assert_eq!(
                oracle_effective, direct_effective,
                "effective disk state (the oracle's `.get(path).copied().unwrap_or(Missing)`, \
                 matching apply_plan::plan's own lookup) must match a direct scan_disk_state \
                 for {path}"
            );
        }
        assert_eq!(
            oracle_scan.coexisting_disabled_twin_md5, direct_scan.coexisting_disabled_twin_md5,
            "the oracle-derived coexisting-twin md5s must exactly match a direct scan_disk_state"
        );
    }

    #[tokio::test]
    async fn oracle_disk_scan_reports_a_saves_file_present_without_hashing_it() {
        // `/saves` is excluded from the returned `PackInfo` (never hashed —
        // see this function's own doc), but its DiskScan companion still
        // needs to know a save exists at all, since `apply_plan::plan`'s
        // `/saves` branch can fall through to normal reconciliation for a
        // path `old` never tracked (see that function's own doc).
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("saves/world")).unwrap();
        std::fs::write(dir.path().join("saves/world/level.dat"), b"save-bytes").unwrap();

        let (packinfo, oracle_scan) = scan_instance_as_packinfo(dir.path()).await.unwrap();

        assert!(
            !packinfo.files.contains_key("/saves/world/level.dat"),
            "a /saves file must never be hashed into the returned PackInfo"
        );
        assert!(
            matches!(
                oracle_scan.states.get("/saves/world/level.dat"),
                Some(apply_plan::DiskState::Present { .. })
            ),
            "a /saves file's mere existence must still surface as Present in the paired \
             DiskScan, got {:?}",
            oracle_scan.states.get("/saves/world/level.dat")
        );
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
    async fn task03_case_probe_preserves_aged_user_notes() {
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

    #[tokio::test]
    async fn task03_oracle_many_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("mods")).unwrap();
        for n in 0..512 {
            std::fs::write(dir.path().join(format!("mods/{n}.jar")), vec![42; 32768]).unwrap();
        }
        let (info, _) = scan_instance_as_packinfo(dir.path()).await.unwrap();
        assert_eq!(info.files.len(), 512);
    }
}
