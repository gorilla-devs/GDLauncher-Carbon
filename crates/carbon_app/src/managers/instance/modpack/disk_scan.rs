//! Produces [`apply_plan::DiskState`] from the real filesystem for a given
//! universe of packinfo-style paths, and takes a full disk snapshot of an
//! instance's data directory as a [`PackInfo`] for the repair skip-oracle.
//!
//! Neither function consults any pack metadata — they only look at what is
//! actually on disk right now. Callers combine their output with `old`/
//! `target` packinfos in [`apply_plan::plan`](super::apply_plan::plan).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use md5::Md5;
use sha2::{Digest, Sha512};

use crate::util::NormalizedWalkdir;

use super::apply_plan;
use super::packinfo::{FileHashes, PackInfo};

/// [`scan_disk_state`]'s result: every path's ordinary
/// [`apply_plan::DiskState`], plus — only for a path currently `Present`
/// under its bare spelling — the md5 of a `.disabled` twin that coexists
/// with it right now, when one does. `DiskState::Disabled` already carries
/// a twin's md5 for the twin-only case (bare spelling absent); this covers
/// the shape `DiskState` alone can't express, both spellings present at
/// once. That shape only ever arises mid-way through an interrupted
/// disabled [`apply_plan::PlanAction::Replace`] — `run/modpack.rs::execute_plan`
/// renames the staged bytes into the twin spelling *before* removing the
/// stale bare copy, so a crash or a locked-file removal failure between the
/// two leaves exactly this. [`apply_plan::decide_version_change`]'s
/// pristine-unstaged-differs arm is the sole consumer, using it to
/// recognize and finish that interrupted state on resume instead of
/// erroring on it forever (staging is consumed either way and is never
/// re-populated on resume).
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

    let results = futures::future::join_all(futures)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

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

/// Process-wide, so two concurrent [`probe_case_insensitive`] calls in the
/// same process (e.g. a repair preview refetch racing a real staging apply
/// against the same instance) never generate the same probe filename.
static PROBE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A leftover `.gdl-case-probe*` (either casing) is only ever cleaned up by
/// [`probe_case_insensitive`] itself deciding it's old enough to be a crash
/// remnant rather than another call's file still in flight — comfortably
/// above the microsecond-scale window a single write+stat+remove takes, and
/// nowhere near how quickly two independent callers could plausibly race
/// each other.
const STALE_PROBE_AGE: Duration = Duration::from_secs(60);

/// Probes whether `dir`'s filesystem folds path case together, by writing a
/// uniquely-named marker file and checking whether an all-different-case
/// spelling of that same unique name resolves to it too. Best-effort: both
/// spellings of the probe file are always removed again before returning,
/// regardless of the outcome or which one (if either) actually got written.
/// On any I/O error other than the expected "not found" from the
/// case-sensitive stat — including a failure to write the probe file in the
/// first place — falls back to the platform default: Windows and macOS are
/// case-insensitive by default, every other target isn't.
///
/// The probe name is unique per call (`.gdl-case-probe-<pid>-<nanos>-<call
/// counter>`), not a fixed name: two callers can legitimately probe the same
/// data dir concurrently (`repair_preview` is a query the frontend refetches
/// on invalidation and can overlap a real `process_modpack_staging` run), and
/// a fixed name meant one call's cleanup landing between the other's write
/// and stat could report a false "case-sensitive" — silently reintroducing
/// the exact Delete this whole mechanism exists to prevent. Before writing
/// its own probe, this also sweeps `dir` for stale `.gdl-case-probe*`
/// leftovers (either casing) older than [`STALE_PROBE_AGE`] — a crashed
/// earlier probe's file, left behind because the process died between the
/// write and the removal below — since an old fixed-name leftover would
/// otherwise make a genuinely case-SENSITIVE filesystem report insensitive
/// forever (only the exact spelling a normal exit wrote is ever cleaned up).
/// The age threshold is what keeps that sweep from also deleting a
/// *concurrent, still in-flight* call's own probe file.
pub async fn probe_case_insensitive(dir: &Path) -> bool {
    cleanup_stale_probes(dir).await;

    let unique = format!(
        ".gdl-case-probe-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
        PROBE_COUNTER.fetch_add(1, Ordering::Relaxed),
    );
    let probe = dir.join(&unique);
    let folded = dir.join(unique.to_ascii_uppercase());

    let outcome = async {
        tokio::fs::write(&probe, b"gdl-case-probe").await?;
        match tokio::fs::metadata(&folded).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }
    .await;

    // Both spellings, best-effort, on every exit path: on an insensitive
    // filesystem they're the same physical file and either call removes it,
    // but removing both keeps this correct even if this probe's own write
    // landed under the "wrong" one of the two names for some reason, and
    // costs nothing extra on a sensitive filesystem where at most one of the
    // two ever existed.
    let _ = tokio::fs::remove_file(&probe).await;
    let _ = tokio::fs::remove_file(&folded).await;

    outcome.unwrap_or_else(|_: std::io::Error| cfg!(any(windows, target_os = "macos")))
}

/// Removes any `.gdl-case-probe*` entry (either casing) in `dir` whose mtime
/// is older than [`STALE_PROBE_AGE`] — see [`probe_case_insensitive`]'s own
/// doc for why this exists and why it's age-gated rather than unconditional.
/// Best-effort throughout: a directory that can't be read, an entry whose
/// metadata can't be stat'd, or a removal that fails is silently skipped —
/// this is hygiene for a hazard that's already merely theoretical once probe
/// names are unique, never something worth failing the probe over.
async fn cleanup_stale_probes(dir: &Path) {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(_) => return,
    };

    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(_) => break,
        };

        let Some(name) = entry.file_name().to_str().map(str::to_ascii_lowercase) else {
            continue;
        };
        if !name.starts_with(".gdl-case-probe") {
            continue;
        }

        let is_stale = match entry.metadata().await.and_then(|m| m.modified()) {
            Ok(modified) => SystemTime::now()
                .duration_since(modified)
                .map(|age| age >= STALE_PROBE_AGE)
                // `modified` in the future (clock skew) is never "stale".
                .unwrap_or(false),
            // Can't even stat it — leave it alone rather than guess.
            Err(_) => false,
        };

        if is_stale {
            let _ = tokio::fs::remove_file(entry.path()).await;
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
        if apply_plan::is_saves_path(relative_path) || relative_path.starts_with("/.install_audit")
        {
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
    async fn probe_case_insensitive_removes_a_stale_leftover_probe_of_either_casing() {
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
            !stale_lower.exists(),
            "a stale leftover probe (crash remnant) must be swept before probing"
        );
        assert!(
            !stale_upper.exists(),
            "a stale leftover probe must be swept regardless of its casing"
        );
    }

    #[tokio::test]
    async fn probe_case_insensitive_does_not_remove_a_fresh_same_prefixed_file() {
        // Pins the race-avoidance mechanism directly, without needing actual
        // concurrency: a file matching the probe's own naming prefix but
        // freshly written (as a genuinely concurrent caller's in-flight
        // probe file would be) must survive another call's cleanup sweep —
        // only entries older than `STALE_PROBE_AGE` are ever removed by it.
        let dir = tempfile::tempdir().unwrap();
        let concurrent = dir.path().join(".gdl-case-probe-concurrent-simulated");
        std::fs::write(&concurrent, b"in-flight-from-another-call").unwrap();

        let _ = probe_case_insensitive(dir.path()).await;

        assert!(
            concurrent.exists(),
            "a fresh, still-in-flight-looking probe file must not be swept as stale"
        );
    }
}
