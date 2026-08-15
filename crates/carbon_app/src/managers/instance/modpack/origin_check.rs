//! Modrinth pack-origin prover: for an instance's untracked files, proves
//! ("origin-checks") whether each one was ever shipped by *some* version of
//! the instance's Modrinth pack — including versions other than the one
//! currently installed, since a downgrade's "extra" files often came from a
//! newer version the user previously ran.
//!
//! [`match_against_index`] is the pure core: given a set of untracked files'
//! sha512 hashes and one parsed `.mrpack` index (plus its overrides,
//! separately hashed by the caller), it says which untracked paths that
//! index proves. Everything else in this module is the impure shell that
//! feeds it: downloading and parsing candidate `.mrpack` archives — the
//! installed version first, then every other version newest-published-first
//! (see [`move_current_version_first`]) — and assembling the results into
//! [`OriginResults`].

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, bail};
use carbon_platforms::modrinth::{
    project::ProjectVersionsFilters,
    search::ProjectID,
    version::{ModpackIndex, Version, VersionFile},
};
use sha2::{Digest, Sha512};
use tokio::task::spawn_blocking;

use crate::api::keys::instance::GET_REPAIR_PREVIEW;
use crate::api::translation::Translation;
use crate::domain::instance::InstanceId;
use crate::domain::instance::info::{Modpack, ModrinthModpack};
use crate::domain::vtask::VisualTaskId;
use crate::managers::AppInner;
use crate::managers::ManagerRef;
use crate::managers::instance::{InstanceManager, InvalidInstanceIdError};
use crate::managers::minecraft::modrinth::{
    MAX_HASHED_ENTRY_BYTES, is_symlink_mode, secure_path_join,
};
use crate::managers::vtask::{TaskState, VisualTask};

use super::OriginVerdict;

/// Ceiling on the total decompressed bytes [`parse_mrpack`] will hash across
/// *all* of one archive's override entries combined. [`MAX_HASHED_ENTRY_BYTES`]
/// bounds each entry independently (a decompression-bomb guard — no single
/// entry can blow up unboundedly); this bounds the sum, which per-entry
/// budgeting alone doesn't: an archive with many entries each individually
/// within the per-entry limit could otherwise still cost entries × 64 MiB of
/// hashing work. This is deliberately generous (1 GiB) and is a stall guard
/// against a pathological archive with an enormous number of override
/// entries, not a correctness bound — a real modpack's overrides are nowhere
/// near this large in aggregate, so hitting it in practice would itself be
/// a sign of an abusive or corrupt archive, and the entries beyond the cap
/// are skipped (warned, not hashed) exactly like an individually-oversized
/// entry already is, rather than blocking the whole origin-check pass.
const MAX_TOTAL_HASHED_BYTES: u64 = 1024 * 1024 * 1024;

/// Manager-side cache entry for one instance — see
/// [`crate::managers::instance::InstanceManager::origin_checks`]. Replaced
/// wholesale by each completed (or failed-but-partial) run of
/// [`ManagerRef::check_pack_origin`](super::super::InstanceManager); never
/// incrementally merged with a previous run's results.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OriginResults {
    pub verdicts: HashMap<String, OriginVerdict>,
}

/// Pure matcher: given the untracked files still needing a verdict (path ->
/// sha512) and one parsed `.mrpack`, returns the subset of paths that
/// `.mrpack` proves — via either its `modrinth.index.json` `files[]` list or
/// its (separately hashed by the caller) `overrides/`/`client-overrides/`
/// entries.
///
/// A path matches only when BOTH the path and the sha512 agree; a path that
/// merely exists in the index/overrides under a different hash is never
/// matched (that's a different file that happens to share a name — the
/// untracked file on disk still isn't proven). Every key in the returned set
/// is guaranteed to be a key of `untracked`.
pub fn match_against_index(
    untracked: &HashMap<String, [u8; 64]>,
    index: &ModpackIndex,
    override_hashes: &HashMap<String, [u8; 64]>,
) -> HashSet<String> {
    if untracked.is_empty() {
        return HashSet::new();
    }

    let mut matched = HashSet::new();

    // `ModrinthFile::path` is relative to the instance root with no leading
    // slash ("mods/x.jar"); `untracked`'s keys are packinfo-style ("/mods/x.jar").
    for file in &index.files {
        let key = format!("/{}", file.path);
        let Some(&hash) = untracked.get(&key) else {
            continue;
        };

        // Decoded byte comparison rather than a string compare: sidesteps
        // hex case (Modrinth always sends lowercase, but nothing enforces
        // that) and a malformed hash from the index is simply never a match
        // rather than a panic.
        let mut file_hash = [0u8; 64];
        if hex::decode_to_slice(&file.hashes.sha512, &mut file_hash).is_ok() && file_hash == hash {
            matched.insert(key);
        }
    }

    // `override_hashes` is already keyed in the same packinfo-style form.
    for (key, hash) in override_hashes {
        if let Some(&untracked_hash) = untracked.get(key) {
            if *hash == untracked_hash {
                matched.insert(key.clone());
            }
        }
    }

    matched
}

/// For every `matched` path: records the verdict `version` proves (see
/// below) and removes it from `remaining`, so the caller's next iteration
/// only asks about files that still lack a verdict. Never clobbers an
/// existing `verdicts` entry for a path — unreachable in the real loop
/// (a path is removed from `remaining` the moment it's first proven, so it
/// can never be offered to [`match_against_index`] again), but kept as an
/// explicit invariant here rather than relying on that staying true forever.
fn record_matches(
    verdicts: &mut HashMap<String, OriginVerdict>,
    remaining: &mut HashMap<String, [u8; 64]>,
    matched: HashSet<String>,
    version: &Version,
    current_version_id: &str,
) {
    let verdict = if version.id == current_version_id {
        OriginVerdict::CurrentVersion
    } else {
        OriginVerdict::ShippedIn {
            version_id: version.id.clone(),
            version_name: version.name.clone(),
        }
    };

    for path in matched {
        remaining.remove(&path);
        verdicts.entry(path).or_insert_with(|| verdict.clone());
    }
}

/// Every key in `all_untracked` without an existing `verdicts` entry becomes
/// [`OriginVerdict::Unknown`] — the exhaustion case: no version (of however
/// many were actually checked before this run stopped, successfully or not)
/// proved that file. Never overwrites an existing verdict.
fn fill_unmatched_as_unknown(
    verdicts: &mut HashMap<String, OriginVerdict>,
    all_untracked: impl Iterator<Item = String>,
) {
    for path in all_untracked {
        verdicts.entry(path).or_insert(OriginVerdict::Unknown);
    }
}

/// The file a version's download should verify against: the entry flagged
/// `primary`, or the first file when none is flagged. `None` for a version
/// with an empty `files` list — deliberately not the more obvious
/// `files.iter().find(|f| f.primary).unwrap_or(&files[0])`, which indexes
/// `files[0]` unconditionally (`Option::unwrap_or`'s argument is evaluated
/// eagerly regardless of the `Option` it's called on) and would panic on
/// exactly the empty-list case this function guards against.
fn primary_file(files: &[VersionFile]) -> Option<&VersionFile> {
    if files.is_empty() {
        return None;
    }
    Some(files.iter().find(|f| f.primary).unwrap_or(&files[0]))
}

/// Sorts `versions` newest-`date_published`-first, in place. Modrinth's API
/// is documented to already return versions in this order; this is a
/// defensive re-sort so the newest-first guarantee
/// [`run_check_pack_origin`]'s early-exit relies on doesn't silently depend
/// on that continuing to hold.
fn sort_versions_newest_first(versions: &mut [Version]) {
    versions.sort_by(|a, b| b.date_published.cmp(&a.date_published));
}

/// Moves the entry whose id is `current_version_id` (if present) to the
/// front of `versions`, leaving every other entry in its existing relative
/// order behind it — call after [`sort_versions_newest_first`], so "every
/// other entry" is still newest-first; only the current version (which may
/// sit anywhere in that order, not necessarily first) jumps to the front.
///
/// This is a correctness fix, not a tie-break preference: [`record_matches`]
/// permanently claims a path the instant *any* version proves it and removes
/// it from further consideration (`remaining`), so whichever version is
/// checked first for a given path wins that claim forever. If a newer
/// version happened to ship the exact same bytes at the exact same path as
/// the currently installed version — an unchanged shared mod jar is the
/// common case — and were checked first, it would permanently label that
/// path `ShippedIn` an old/other version, even though the file may genuinely
/// still be required by the *current* install (e.g. packinfo simply failed
/// to record it). That mislabels a file the user's current install still
/// needs as safe-to-clean-up leftover cruft. Checking the current version
/// first guarantees its claim always wins that race, so a file the current
/// version ships is never mislabeled as belonging only to some other one.
fn move_current_version_first(versions: &mut Vec<Version>, current_version_id: &str) {
    if let Some(pos) = versions.iter().position(|v| v.id == current_version_id) {
        let current = versions.remove(pos);
        versions.insert(0, current);
    }
}

/// Decodes a hex sha512 digest (as Modrinth's API sends it) into raw bytes.
fn decode_sha512_hex(hex_str: &str) -> anyhow::Result<[u8; 64]> {
    let mut buf = [0u8; 64];
    hex::decode_to_slice(hex_str, &mut buf)
        .with_context(|| format!("{hex_str:?} is not a valid sha512 hex digest"))?;
    Ok(buf)
}

/// Rebuilds a packinfo-style key (`/mods/x.jar`) from a zip entry's path
/// already stripped of its `overrides/`/`client-overrides/` prefix by the
/// caller. `None` for anything that isn't a plain run of `Normal` path
/// components — belt-and-suspenders on top of the `enclosed_name`/
/// [`secure_path_join`] guards [`parse_mrpack`] already applies before ever
/// calling this, not the primary defense.
fn packinfo_key_from_relative(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?),
            _ => return None,
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!("/{}", parts.join("/")))
}

/// Reads all of `reader`, erroring out instead of hashing more than `limit`
/// bytes. The override-hashing analogue of
/// [`crate::managers::minecraft::modrinth::copy_bounded`], which guards a
/// decompression bomb the same way but copies into a [`std::io::Write`] sink
/// instead of a digest. Returns the number of bytes actually read; unlike
/// `copy_bounded`'s callers, [`parse_mrpack`] gives every entry the same
/// fresh `limit` rather than threading a shrinking budget across entries —
/// this only ever streams through a fixed-size buffer, so nothing is held in
/// memory proportional to `limit` in the first place.
fn hash_bounded<R: std::io::Read>(reader: &mut R, limit: u64) -> anyhow::Result<(u64, [u8; 64])> {
    let mut hasher = Sha512::new();
    let mut buf = [0u8; 64 * 1024];
    let mut total = 0u64;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > limit {
            bail!("Archive entry exceeds the {limit} byte decompressed size limit");
        }
        hasher.update(&buf[..n]);
    }

    Ok((total, hasher.finalize().into()))
}

/// Opens `path` and sha512-hashes its full contents, chunked so the whole
/// file is never buffered in memory at once — mirrors `disk_scan.rs`'s
/// `probe_md5`. Used both to hash untracked instance files and to verify (or
/// decide to reuse) a downloaded `.mrpack`.
async fn hash_file_sha512(path: &Path) -> anyhow::Result<[u8; 64]> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha512::new();

    carbon_scheduler::buffered_digest(&mut file, |chunk| {
        hasher.update(&chunk);
    })
    .await?;

    Ok(hasher.finalize().into())
}

/// Opens a downloaded `.mrpack`, parses its `modrinth.index.json`, and hashes
/// every `overrides/`/`client-overrides/` entry into a packinfo-style key ->
/// sha512 map ([`match_against_index`]'s `override_hashes` input). Blocking
/// zip work happens on a blocking thread, mirroring
/// [`crate::managers::minecraft::modrinth::prepare_modpack_from_mrpack`]'s
/// own archive-opening pattern; it also reuses that same function's
/// zip-slip defense (the crate-provided `enclosed_name` guard plus
/// [`secure_path_join`]) even though nothing here is ever written to disk —
/// a zip-slip path must never even be *hashed* under an attacker-chosen key.
///
/// Thin wrapper over [`parse_mrpack_with_budgets`] fixing the per-entry
/// budget at [`MAX_HASHED_ENTRY_BYTES`] and the archive-wide one at
/// [`MAX_TOTAL_HASHED_BYTES`] — the split exists only so tests can exercise
/// both budgets with small injected limits instead of needing real
/// multi-ten/hundred-megabyte archives to prove them.
async fn parse_mrpack(path: &Path) -> anyhow::Result<(ModpackIndex, HashMap<String, [u8; 64]>)> {
    parse_mrpack_with_budgets(path, MAX_HASHED_ENTRY_BYTES, MAX_TOTAL_HASHED_BYTES).await
}

/// See [`parse_mrpack`]. `entry_budget` is the fresh [`hash_bounded`] limit
/// given to *every* override entry independently — not a running total
/// shared across the archive. `hash_bounded` streams each entry through a
/// fixed 64 KiB buffer and never holds more than that in memory regardless
/// of the entry's size, so nothing is gained by shrinking later entries'
/// allowances to make room for earlier ones; doing so only made a
/// legitimately large pack (many override files individually well under the
/// limit, summing past it) silently lose hash coverage for whichever
/// entries happened to be enumerated last.
///
/// `total_budget` is the separate, archive-wide cap on bytes actually
/// hashed across every entry combined (see [`MAX_TOTAL_HASHED_BYTES`]):
/// once reached, remaining entries are skipped (warned, not hashed) rather
/// than continuing to spend unbounded total work on one archive.
async fn parse_mrpack_with_budgets(
    path: &Path,
    entry_budget: u64,
    total_budget: u64,
) -> anyhow::Result<(ModpackIndex, HashMap<String, [u8; 64]>)> {
    let path = path.to_path_buf();

    spawn_blocking(move || {
        let file =
            std::fs::File::open(&path).with_context(|| format!("opening archive {path:?}"))?;
        let mut archive = zip::ZipArchive::new(file).context("reading archive as zip")?;

        let index: ModpackIndex = {
            let index_file = archive
                .by_name("modrinth.index.json")
                .context("archive has no modrinth.index.json")?;
            serde_json::from_reader(index_file).context("parsing modrinth.index.json")?
        };

        let mut override_hashes = HashMap::new();
        let mut total_hashed_bytes: u64 = 0;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;

            let Some(enclosed) = entry.enclosed_name() else {
                continue;
            };

            let stripped = enclosed
                .strip_prefix("overrides")
                .or_else(|_| enclosed.strip_prefix("client-overrides"));
            let Ok(stripped) = stripped else {
                continue;
            };

            if entry.name().ends_with('/') {
                continue;
            }
            if is_symlink_mode(entry.unix_mode()) {
                continue;
            }
            // Defense in depth on top of `enclosed_name` — mirrors the same
            // double guard `prepare_modpack_from_mrpack` applies before
            // joining an override path onto a real directory. Nothing is
            // joined onto a real path here, so any root works as the probe.
            if secure_path_join(Path::new("/"), stripped).is_err() {
                continue;
            }

            let Some(key) = packinfo_key_from_relative(stripped) else {
                continue;
            };

            // Stall guard, checked before spending any work on this entry:
            // once the archive-wide total is already at/over budget, every
            // remaining entry is skipped rather than hashed — see
            // `MAX_TOTAL_HASHED_BYTES`'s own doc for why this is a separate
            // concern from the per-entry `entry_budget` above.
            if total_hashed_bytes >= total_budget {
                tracing::warn!(
                    "origin check: skipping override entry {} — archive-wide hashed byte cap \
                     ({total_budget} bytes) already reached",
                    enclosed.display()
                );
                continue;
            }

            match hash_bounded(&mut entry, entry_budget) {
                Ok((copied, hash)) => {
                    total_hashed_bytes += copied;
                    override_hashes.insert(key, hash);
                }
                Err(e) => {
                    tracing::warn!(
                        "origin check: skipping oversized override entry {}: {e}",
                        enclosed.display()
                    );
                }
            }
        }

        Ok::<_, anyhow::Error>((index, override_hashes))
    })
    .await
    .context("archive parsing task panicked")?
}

/// Downloads (or reuses an already-correct cached copy of) `file` to `dest`
/// via `carbon_net::download_multiple`, the same download primitive the rest
/// of the launcher uses for game/mod/modpack files — **not**
/// `app.reqwest_client`, which is a small-JSON-API client with a hard 60s
/// *total* timeout and no size cap (see its own doc comment in
/// `iridium_client.rs`: "Large file downloads run through a separate client
/// in carbon_net that intentionally has no overall timeout"). A `.mrpack`
/// can be tens of megabytes and take longer than 60s on a slow connection;
/// using the wrong client made large packs time out and silently fall back
/// to `Unknown` exactly where the checker mattered most.
///
/// `carbon_net` also owns the integrity checking here: it
/// verifies both `file.size` and `file.hashes.sha512` *during* the download
/// (streaming, not after-the-fact) and only ever renames its `.part` file
/// into place at `dest` once both check out — so by the time this returns
/// `Ok`, `dest` is already proven to match the API-declared hash, and a
/// failed/mismatched attempt never leaves a partial file at `dest` (only a
/// `.part` sibling, which `carbon_net` itself cleans up on error). It also
/// skips the network entirely when `dest` already holds a file matching
/// `file.size`/`file.hashes.sha512` — the "immutable cache" the task brief
/// describes, keyed by this content-addressed path, needs no extra code
/// here to get.
///
/// Does not clean up `dest` on failure — the caller
/// ([`download_and_match`]) owns that uniformly, for every failure path
/// after this point, as defense in depth on top of `carbon_net`'s own
/// cleanup (see that function's docs).
async fn download_verified(file: &VersionFile, dest: &Path) -> anyhow::Result<()> {
    let downloadable = carbon_net::Downloadable::new(file.url.clone(), dest)
        .with_checksum(Some(carbon_net::Checksum::Sha512(
            file.hashes.sha512.clone(),
        )))
        .with_size(u64::from(file.size));

    carbon_net::download_multiple(
        &[downloadable],
        carbon_net::DownloadOptions::builder()
            .concurrency(1)
            .build(),
    )
    .await
    .with_context(|| format!("downloading {}", file.url))?;

    Ok(())
}

/// Downloads one version's primary file, parses it, and matches it against
/// `remaining`. The temp path is content-addressed (named after the
/// *expected* sha512), and is unconditionally removed before this function
/// returns — on every exit, success or failure, at every step (download,
/// parse) — so nothing but the verdict map this feeds into ever persists
/// across versions or runs; see [`download_verified`] for the download
/// step's own safety properties.
async fn download_and_match(
    temp_root: &Path,
    file: &VersionFile,
    remaining: &HashMap<String, [u8; 64]>,
) -> anyhow::Result<HashSet<String>> {
    let expected_sha512 = decode_sha512_hex(&file.hashes.sha512).with_context(|| {
        format!(
            "version file {:?} has an invalid sha512 hash",
            file.filename
        )
    })?;

    let temp_path = temp_root.join(format!(
        "origin-check-{}.mrpack",
        hex::encode(expected_sha512)
    ));

    if let Err(e) = download_verified(file, &temp_path).await {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(e);
    }

    let parsed = parse_mrpack(&temp_path).await;
    let _ = tokio::fs::remove_file(&temp_path).await;
    let (index, override_hashes) = parsed?;

    Ok(match_against_index(remaining, &index, &override_hashes))
}

/// Writes `verdicts` into the instance's `origin_checks` entry (replacing
/// whatever was there — each run's result is a fresh snapshot, never merged
/// with the previous one) and nudges the frontend to re-read the repair
/// preview, which is where these verdicts actually surface (see
/// `super::origin_verdict_for`). `args` is a *partial* object (only the
/// `instance` key) rather than the full `RepairPreviewArgs` shape so the
/// frontend's partial query-key matching invalidates the cached preview for
/// this instance regardless of which `re_enable_disabled` value it was
/// queried with — see `GET_REPAIR_PREVIEW`'s route doc for why a full-object
/// filter would miss the other variant.
async fn store_and_invalidate(
    app: &Arc<AppInner>,
    instance_id: InstanceId,
    verdicts: HashMap<String, OriginVerdict>,
) {
    app.instance_manager()
        .origin_checks
        .write()
        .await
        .insert(instance_id, OriginResults { verdicts });

    app.invalidate(
        GET_REPAIR_PREVIEW,
        Some(serde_json::json!({ "instance": instance_id.0 })),
    );
}

/// Background body of [`ManagerRef::check_pack_origin`]; split out to a free
/// function because it must be `'static` to hand to `tokio::spawn`, which a
/// borrow-carrying `ManagerRef` can't be — the same reason `export_gdlauncher`
/// and friends are free functions taking `Arc<AppInner>` rather than manager
/// methods.
///
/// Algorithm: hash every currently-untracked file, then walk the project's
/// versions — the *currently installed* version first (see
/// [`move_current_version_first`]), then every other version
/// newest-published-first — downloading and matching each one's primary file
/// until either every untracked file has a verdict or the versions run out.
/// The installed version's matches get [`OriginVerdict::CurrentVersion`];
/// any other version's get [`OriginVerdict::ShippedIn`]. Checking the
/// installed version first is load-bearing, not cosmetic: [`record_matches`]
/// permanently claims a path the instant any version proves it, so if a
/// newer version were checked first and happened to ship the exact same
/// bytes at the exact same path, it would permanently win a claim that
/// should have gone to the installed version instead. Whatever is left
/// unmatched when the loop ends — for any reason, including the one
/// fatal-error path below — becomes [`OriginVerdict::Unknown`].
///
/// Errors: a failure to download/verify/parse *one* version's archive is
/// logged and that version is skipped, never fatal to the run (matches never
/// found there might still be found in an older or newer version). The one
/// fatal error is the versions-list request itself failing, since without it
/// there is nothing left to check against at all; even then, whatever
/// verdicts exist so far (hashed-but-unchecked untracked files, finalized to
/// `Unknown`) are still stored before the task is marked failed.
///
/// Cancellation: nothing currently aborts this task's `JoinHandle` — there is
/// no "cancel" affordance wired up for it, only the ordinary "drop a
/// finished/failed task from the list" flow every [`VisualTask`] already has
/// (see its `Drop` impl and [`VisualTask::fail`]). If the *process* exits
/// mid-run, whatever this run had matched so far is simply lost: results are
/// written to `origin_checks` exactly once, at the very end (success, or the
/// fatal-error path) — never incrementally per version — and any in-flight
/// temp archive is swept on next startup by
/// [`carbon_rt_path::RuntimePath::get_temp`]'s `cleanup_all`.
async fn run_check_pack_origin(
    app: Arc<AppInner>,
    task: VisualTask,
    instance_id: InstanceId,
    shortpath: String,
    project_id: String,
    current_version_id: String,
) {
    let runtime_path = app.settings_manager().runtime_path.clone();
    let instance_path = runtime_path.get_instances().get_instance_path(&shortpath);
    let instance_root = instance_path.get_root();
    let data_path = instance_path.get_data_path();

    let packinfo = match tokio::fs::read_to_string(instance_root.join("packinfo.json")).await {
        Ok(text) => {
            match super::packinfo::parse_packinfo(&text).context("while parsing packinfo json") {
                Ok(p) => p,
                Err(e) => {
                    store_and_invalidate(&app, instance_id, HashMap::new()).await;
                    task.fail(e).await;
                    return;
                }
            }
        }
        // No recorded packinfo at all: nothing to compare against, so there
        // is nothing untracked to prove — mirrors `repair_preview`'s own
        // "no packinfo -> nothing to report" handling.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => super::packinfo::PackInfo {
            files: HashMap::new(),
        },
        Err(e) => {
            store_and_invalidate(&app, instance_id, HashMap::new()).await;
            task.fail(e.into()).await;
            return;
        }
    };

    let all_files = super::walk_packinfo_scoped_files(&data_path, &packinfo, None).await;
    let raw_untracked: HashMap<String, PathBuf> = all_files
        .into_iter()
        .filter(|(key, _)| !packinfo.files.contains_key(key.as_str()))
        .collect();

    let mut untracked: HashMap<String, [u8; 64]> = HashMap::new();
    for (key, path) in &raw_untracked {
        match hash_file_sha512(path).await {
            Ok(hash) => {
                untracked.insert(key.clone(), hash);
            }
            Err(e) => {
                tracing::warn!("origin check: failed to hash untracked file {key}: {e:#}");
            }
        }
    }

    let mut verdicts: HashMap<String, OriginVerdict> = HashMap::new();

    if !untracked.is_empty() {
        let versions_resp = app
            .modplatforms_manager()
            .modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID(project_id),
                // Deliberately unfiltered: a downgrade's untracked leftovers
                // can come from a version that targeted a different
                // Minecraft version or loader than the instance's current one.
                game_versions: None,
                loaders: None,
                offset: None,
                limit: None,
            })
            .await;

        match versions_resp {
            Ok(response) => {
                let mut versions = response.0;
                sort_versions_newest_first(&mut versions);
                // Must run after the sort: pulls the installed version to the
                // front so it always wins a claim race against a newer
                // version shipping the same bytes at the same path — see
                // `move_current_version_first`'s own docs.
                move_current_version_first(&mut versions, &current_version_id);

                let subtask = task.subtask(Translation::InstanceTaskCheckPackOriginVersions);
                task.edit(|data| data.state = TaskState::KnownProgress)
                    .await;
                let total = (versions.len() as u32).max(1);

                let temp_root = runtime_path.get_temp().to_path();
                if let Err(e) = tokio::fs::create_dir_all(&temp_root).await {
                    tracing::warn!("origin check: failed to prepare temp dir: {e:#}");
                }

                let mut remaining = untracked.clone();

                for (i, version) in versions.iter().enumerate() {
                    if remaining.is_empty() {
                        break;
                    }

                    subtask.update_items(i as u32, total);

                    let Some(file) = primary_file(&version.files) else {
                        continue;
                    };

                    match download_and_match(&temp_root, file, &remaining).await {
                        Ok(matched) => {
                            record_matches(
                                &mut verdicts,
                                &mut remaining,
                                matched,
                                version,
                                &current_version_id,
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "origin check: skipping version {} ({}): {e:#}",
                                version.id,
                                version.name
                            );
                        }
                    }
                }

                subtask.complete_items();
            }
            Err(e) => {
                fill_unmatched_as_unknown(&mut verdicts, untracked.keys().cloned());
                store_and_invalidate(&app, instance_id, verdicts).await;
                task.fail(e.context("fetching modrinth project versions"))
                    .await;
                return;
            }
        }
    }

    fill_unmatched_as_unknown(&mut verdicts, untracked.keys().cloned());
    store_and_invalidate(&app, instance_id, verdicts).await;
}

impl ManagerRef<'_, InstanceManager> {
    /// Kicks off a background [`VisualTask`] that, for every currently
    /// untracked file in `instance_id`'s data directory, tries to prove it
    /// was shipped by *some* published version of the instance's Modrinth
    /// pack — not just the version currently installed, since the files a
    /// downgrade leaves behind usually came from a *newer* version the user
    /// previously ran. See [`run_check_pack_origin`] for the algorithm and
    /// [`InstanceManager::origin_checks`] for where results land; that
    /// stored [`OriginResults`] is what `repair_preview`'s
    /// `origin_verdict_for` reads.
    ///
    /// Only Modrinth packs can be checked — CurseForge exposes no
    /// equivalent "every published version of this project" listing this
    /// checker could walk — so a CurseForge-modpack instance is refused
    /// outright, before any task is even created.
    pub async fn check_pack_origin(self, instance_id: InstanceId) -> anyhow::Result<VisualTaskId> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;
        let data = instance.data()?;

        let Some(modpack_info) = data.config.modpack.clone() else {
            bail!("Instance does not have an associated modpack to check");
        };

        let (project_id, current_version_id) = match modpack_info.modpack {
            Modpack::Modrinth(ModrinthModpack {
                project_id,
                version_id,
            }) => (project_id, version_id),
            Modpack::Curseforge(_) => {
                bail!("pack origin checking is only available for Modrinth packs")
            }
        };

        let shortpath = instance.shortpath.clone();
        let instance_name = data.config.name.clone();
        drop(instances);

        let task = VisualTask::new(Translation::InstanceTaskCheckPackOrigin {
            name: instance_name,
        });
        let task_id = self.app.task_manager().spawn_task(&task).await;

        let app = self.app.clone();
        tokio::spawn(run_check_pack_origin(
            app,
            task,
            instance_id,
            shortpath,
            project_id,
            current_version_id,
        ));

        Ok(task_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carbon_platforms::modrinth::version::{
        Hashes, ModpackIndex, ModrinthFile, ModrinthGame, ModrinthPackDependencies,
    };

    fn sha(byte: u8) -> [u8; 64] {
        [byte; 64]
    }

    fn empty_index() -> ModpackIndex {
        index_with_files(vec![])
    }

    fn index_with_files(files: Vec<(&str, [u8; 64])>) -> ModpackIndex {
        ModpackIndex {
            format_version: 1,
            game: ModrinthGame::Minecraft,
            version_id: "v1".to_string(),
            name: "Test Pack".to_string(),
            summary: None,
            files: files
                .into_iter()
                .map(|(path, sha512)| ModrinthFile {
                    path: path.to_string(),
                    hashes: Hashes {
                        sha512: hex::encode(sha512),
                        sha1: String::new(),
                        others: HashMap::new(),
                    },
                    env: None,
                    downloads: vec![],
                    file_size: 0,
                })
                .collect(),
            dependencies: ModrinthPackDependencies {
                minecraft: None,
                forge: None,
                neoforge: None,
                fabric_loader: None,
                quilt_loader: None,
            },
        }
    }

    #[test]
    fn matches_by_path_and_sha512_via_index_files() {
        let untracked = HashMap::from([("/mods/a.jar".to_string(), sha(1))]);
        let index = index_with_files(vec![("mods/a.jar", sha(1))]);

        let matched = match_against_index(&untracked, &index, &HashMap::new());

        assert_eq!(matched, HashSet::from(["/mods/a.jar".to_string()]));
    }

    #[test]
    fn does_not_match_when_path_matches_but_hash_differs() {
        let untracked = HashMap::from([("/mods/a.jar".to_string(), sha(1))]);
        // Same path, different content (a same-named file from an unrelated source).
        let index = index_with_files(vec![("mods/a.jar", sha(2))]);

        let matched = match_against_index(&untracked, &index, &HashMap::new());

        assert!(
            matched.is_empty(),
            "a path match with a differing hash must never count as proof, got {matched:?}"
        );
    }

    #[test]
    fn matches_via_override_hashes_map() {
        let untracked = HashMap::from([("/config/settings.cfg".to_string(), sha(3))]);
        let override_hashes = HashMap::from([("/config/settings.cfg".to_string(), sha(3))]);

        let matched = match_against_index(&untracked, &empty_index(), &override_hashes);

        assert_eq!(matched, HashSet::from(["/config/settings.cfg".to_string()]));
    }

    #[test]
    fn override_hash_mismatch_does_not_match() {
        let untracked = HashMap::from([("/config/settings.cfg".to_string(), sha(3))]);
        let override_hashes = HashMap::from([("/config/settings.cfg".to_string(), sha(4))]);

        let matched = match_against_index(&untracked, &empty_index(), &override_hashes);

        assert!(matched.is_empty());
    }

    #[test]
    fn empty_untracked_returns_empty_regardless_of_index_or_overrides() {
        let index = index_with_files(vec![("mods/a.jar", sha(1))]);
        let override_hashes = HashMap::from([("/config/settings.cfg".to_string(), sha(3))]);

        let matched = match_against_index(&HashMap::new(), &index, &override_hashes);

        assert!(matched.is_empty());
    }

    #[test]
    fn index_entries_absent_from_untracked_are_ignored() {
        // Sanity: the index having extra files nobody asked about must never
        // panic or otherwise misbehave, and must never appear in the result
        // (the result is always a subset of `untracked`'s keys).
        let untracked = HashMap::from([("/mods/a.jar".to_string(), sha(1))]);
        let index = index_with_files(vec![("mods/a.jar", sha(1)), ("mods/b.jar", sha(9))]);

        let matched = match_against_index(&untracked, &index, &HashMap::new());

        assert_eq!(matched, HashSet::from(["/mods/a.jar".to_string()]));
    }

    #[test]
    fn combines_files_and_override_matches_into_one_union() {
        let untracked = HashMap::from([
            ("/mods/a.jar".to_string(), sha(1)),
            ("/config/settings.cfg".to_string(), sha(3)),
            ("/mods/unmatched.jar".to_string(), sha(5)),
        ]);
        let index = index_with_files(vec![("mods/a.jar", sha(1))]);
        let override_hashes = HashMap::from([("/config/settings.cfg".to_string(), sha(3))]);

        let matched = match_against_index(&untracked, &index, &override_hashes);

        assert_eq!(
            matched,
            HashSet::from([
                "/mods/a.jar".to_string(),
                "/config/settings.cfg".to_string()
            ])
        );
    }

    // --- test builders for `Version`/`VersionFile` -------------------------

    fn test_version(id: &str, date_published: &str, files: Vec<VersionFile>) -> Version {
        Version {
            name: format!("Version {id}"),
            version_number: id.to_string(),
            changelog: None,
            dependencies: vec![],
            game_versions: vec!["1.20.1".to_string()],
            version_type: carbon_platforms::modrinth::version::VersionType::Release,
            loaders: vec!["fabric".to_string()],
            featured: false,
            status: None,
            requested_status: None,
            id: id.to_string(),
            project_id: "test-project".to_string(),
            author_id: "test-author".to_string(),
            date_published: date_published
                .parse()
                .expect("test fixture must be a valid RFC3339 timestamp"),
            downloads: 0,
            files,
        }
    }

    fn test_file(filename: &str, primary: bool) -> VersionFile {
        VersionFile {
            hashes: Hashes {
                sha512: "0".repeat(128),
                sha1: String::new(),
                others: HashMap::new(),
            },
            url: format!("https://example.invalid/{filename}"),
            filename: filename.to_string(),
            primary,
            size: 0,
            file_type: None,
        }
    }

    // --- record_matches -----------------------------------------------------

    #[test]
    fn record_matches_assigns_current_version_verdict_when_version_is_current() {
        let mut verdicts = HashMap::new();
        let mut remaining = HashMap::from([
            ("/mods/a.jar".to_string(), sha(1)),
            ("/mods/b.jar".to_string(), sha(2)),
        ]);
        let version = test_version("cur", "2024-01-01T00:00:00Z", vec![]);
        let matched = HashSet::from(["/mods/a.jar".to_string()]);

        record_matches(&mut verdicts, &mut remaining, matched, &version, "cur");

        assert_eq!(
            verdicts.get("/mods/a.jar"),
            Some(&OriginVerdict::CurrentVersion)
        );
        assert!(
            !remaining.contains_key("/mods/a.jar"),
            "a matched path must be removed from `remaining`"
        );
        assert!(
            remaining.contains_key("/mods/b.jar"),
            "an unmatched path must stay in `remaining`"
        );
    }

    #[test]
    fn record_matches_assigns_shipped_in_verdict_for_a_non_current_version() {
        let mut verdicts = HashMap::new();
        let mut remaining = HashMap::from([("/mods/a.jar".to_string(), sha(1))]);
        let version = test_version("old-version", "2023-01-01T00:00:00Z", vec![]);
        let matched = HashSet::from(["/mods/a.jar".to_string()]);

        record_matches(&mut verdicts, &mut remaining, matched, &version, "cur");

        assert_eq!(
            verdicts.get("/mods/a.jar"),
            Some(&OriginVerdict::ShippedIn {
                version_id: "old-version".to_string(),
                version_name: "Version old-version".to_string(),
            })
        );
    }

    #[test]
    fn record_matches_never_overwrites_an_existing_verdict() {
        let mut verdicts =
            HashMap::from([("/mods/a.jar".to_string(), OriginVerdict::CurrentVersion)]);
        let mut remaining = HashMap::new();
        let version = test_version("old-version", "2023-01-01T00:00:00Z", vec![]);
        let matched = HashSet::from(["/mods/a.jar".to_string()]);

        record_matches(&mut verdicts, &mut remaining, matched, &version, "cur");

        assert_eq!(
            verdicts.get("/mods/a.jar"),
            Some(&OriginVerdict::CurrentVersion)
        );
    }

    // --- fill_unmatched_as_unknown ------------------------------------------

    #[test]
    fn fill_unmatched_as_unknown_only_fills_missing_keys() {
        let mut verdicts =
            HashMap::from([("/mods/a.jar".to_string(), OriginVerdict::CurrentVersion)]);

        fill_unmatched_as_unknown(
            &mut verdicts,
            ["/mods/a.jar".to_string(), "/mods/b.jar".to_string()].into_iter(),
        );

        assert_eq!(
            verdicts.get("/mods/a.jar"),
            Some(&OriginVerdict::CurrentVersion),
            "an existing verdict must never be overwritten"
        );
        assert_eq!(verdicts.get("/mods/b.jar"), Some(&OriginVerdict::Unknown));
    }

    // --- primary_file ---------------------------------------------------------

    #[test]
    fn primary_file_prefers_the_flagged_entry() {
        let files = vec![test_file("a.jar", false), test_file("b.jar", true)];
        assert_eq!(
            primary_file(&files).map(|f| f.filename.as_str()),
            Some("b.jar")
        );
    }

    #[test]
    fn primary_file_falls_back_to_the_first_entry_when_none_is_flagged() {
        let files = vec![test_file("a.jar", false), test_file("b.jar", false)];
        assert_eq!(
            primary_file(&files).map(|f| f.filename.as_str()),
            Some("a.jar")
        );
    }

    #[test]
    fn primary_file_is_none_for_an_empty_file_list() {
        // Regression test: a naive `files.iter().find(|f| f.primary).unwrap_or(&files[0])`
        // indexes `files[0]` unconditionally and panics here.
        assert!(primary_file(&[]).is_none());
    }

    // --- sort_versions_newest_first ------------------------------------------

    #[test]
    fn sorts_versions_newest_first_even_when_input_is_out_of_order() {
        let mut versions = vec![
            test_version("old", "2024-01-01T00:00:00Z", vec![]),
            test_version("newest", "2024-06-01T00:00:00Z", vec![]),
            test_version("middle", "2024-03-01T00:00:00Z", vec![]),
        ];

        sort_versions_newest_first(&mut versions);

        assert_eq!(
            versions.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            vec!["newest", "middle", "old"]
        );
    }

    #[test]
    fn sort_versions_newest_first_is_a_no_op_on_an_already_sorted_list() {
        let mut versions = vec![
            test_version("newest", "2024-06-01T00:00:00Z", vec![]),
            test_version("old", "2024-01-01T00:00:00Z", vec![]),
        ];

        sort_versions_newest_first(&mut versions);

        assert_eq!(
            versions.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            vec!["newest", "old"]
        );
    }

    // --- move_current_version_first -----------------------------------------

    #[test]
    fn move_current_version_first_pulls_it_from_the_middle_to_the_front() {
        let mut versions = vec![
            test_version("newest", "2024-06-01T00:00:00Z", vec![]),
            test_version("current", "2024-03-01T00:00:00Z", vec![]),
            test_version("old", "2024-01-01T00:00:00Z", vec![]),
        ];

        move_current_version_first(&mut versions, "current");

        assert_eq!(
            versions.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            vec!["current", "newest", "old"],
            "the current version must jump to the front; every other entry \
             keeps its existing (newest-first) relative order behind it"
        );
    }

    #[test]
    fn move_current_version_first_is_a_no_op_when_already_first() {
        let mut versions = vec![
            test_version("current", "2024-06-01T00:00:00Z", vec![]),
            test_version("old", "2024-01-01T00:00:00Z", vec![]),
        ];

        move_current_version_first(&mut versions, "current");

        assert_eq!(
            versions.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            vec!["current", "old"]
        );
    }

    #[test]
    fn move_current_version_first_leaves_order_untouched_when_current_is_absent() {
        // e.g. the installed version was unpublished/deleted since install —
        // nothing to promote, and the list must be left exactly as sorted.
        let mut versions = vec![
            test_version("newest", "2024-06-01T00:00:00Z", vec![]),
            test_version("old", "2024-01-01T00:00:00Z", vec![]),
        ];

        move_current_version_first(&mut versions, "does-not-exist");

        assert_eq!(
            versions.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            vec!["newest", "old"]
        );
    }

    // --- regression: verdict precedence when versions overlap ---------------
    //
    // Simulates `run_check_pack_origin`'s own claim loop (sort, reorder,
    // then per-version match+record against a shrinking `remaining` set)
    // without touching the network: each "version's archive" is stood in for
    // by a hand-built `ModpackIndex`, fed through the real, pure
    // `match_against_index` + `record_matches` in the same order the real
    // loop would visit them.

    #[test]
    fn current_version_claims_a_file_a_newer_version_also_ships_unchanged() {
        let untracked = HashMap::from([("/mods/common.jar".to_string(), sha(1))]);

        let mut versions = vec![
            test_version("newer", "2024-06-01T00:00:00Z", vec![]),
            test_version("current", "2024-01-01T00:00:00Z", vec![]),
        ];
        sort_versions_newest_first(&mut versions);
        move_current_version_first(&mut versions, "current");
        assert_eq!(
            versions[0].id, "current",
            "sanity: the fix must put the installed version first, not the newer one"
        );

        // Both versions' real archives would ship the exact same file at the
        // exact same path with the exact same bytes — the scenario that
        // exposes the bug (an unchanged shared mod jar across versions).
        let shared_index = index_with_files(vec![("mods/common.jar", sha(1))]);

        let mut verdicts = HashMap::new();
        let mut remaining = untracked.clone();

        for version in &versions {
            if remaining.is_empty() {
                break;
            }
            let matched = match_against_index(&remaining, &shared_index, &HashMap::new());
            record_matches(&mut verdicts, &mut remaining, matched, version, "current");
        }

        assert_eq!(
            verdicts.get("/mods/common.jar"),
            Some(&OriginVerdict::CurrentVersion),
            "the currently-installed version must win the claim over a newer \
             version shipping the identical file — losing this claim mislabels \
             a file the current install may still need as safe-to-delete \
             leftover cruft from another version, got {verdicts:?}"
        );
    }

    #[test]
    fn newest_first_alone_without_reordering_loses_the_claim_to_a_newer_version() {
        // Same setup as the regression test above, but skipping
        // `move_current_version_first` — pinning down exactly what the bug
        // looked like before the fix, so a future refactor can't silently
        // resurrect it without this test catching it.
        let untracked = HashMap::from([("/mods/common.jar".to_string(), sha(1))]);

        let mut versions = vec![
            test_version("newer", "2024-06-01T00:00:00Z", vec![]),
            test_version("current", "2024-01-01T00:00:00Z", vec![]),
        ];
        sort_versions_newest_first(&mut versions);
        // Deliberately no `move_current_version_first` call here.

        let shared_index = index_with_files(vec![("mods/common.jar", sha(1))]);

        let mut verdicts = HashMap::new();
        let mut remaining = untracked.clone();

        for version in &versions {
            if remaining.is_empty() {
                break;
            }
            let matched = match_against_index(&remaining, &shared_index, &HashMap::new());
            record_matches(&mut verdicts, &mut remaining, matched, version, "current");
        }

        assert_eq!(
            verdicts.get("/mods/common.jar"),
            Some(&OriginVerdict::ShippedIn {
                version_id: "newer".to_string(),
                version_name: "Version newer".to_string(),
            }),
            "documents the pre-fix bug: plain newest-first order lets the \
             newer version win the claim race"
        );
    }

    // --- decode_sha512_hex ----------------------------------------------------

    #[test]
    fn decode_sha512_hex_round_trips_a_valid_hash() {
        let bytes = sha(7);
        let encoded = hex::encode(bytes);
        assert_eq!(decode_sha512_hex(&encoded).unwrap(), bytes);
    }

    #[test]
    fn decode_sha512_hex_rejects_a_too_short_string() {
        assert!(decode_sha512_hex("abcd").is_err());
    }

    #[test]
    fn decode_sha512_hex_rejects_non_hex_characters() {
        assert!(decode_sha512_hex(&"z".repeat(128)).is_err());
    }

    // --- packinfo_key_from_relative -------------------------------------------

    #[test]
    fn packinfo_key_from_relative_builds_a_leading_slash_key() {
        assert_eq!(
            packinfo_key_from_relative(Path::new("mods/x.jar")),
            Some("/mods/x.jar".to_string())
        );
    }

    #[test]
    fn packinfo_key_from_relative_handles_a_single_segment() {
        assert_eq!(
            packinfo_key_from_relative(Path::new("options.txt")),
            Some("/options.txt".to_string())
        );
    }

    #[test]
    fn packinfo_key_from_relative_rejects_an_empty_path() {
        assert_eq!(packinfo_key_from_relative(Path::new("")), None);
    }

    #[test]
    fn packinfo_key_from_relative_rejects_a_parent_dir_component() {
        // Defense in depth: the real caller never reaches this with a `..`
        // component (secure_path_join already rejected it), but this must
        // still refuse to build a key from one rather than silently letting
        // it through.
        assert_eq!(packinfo_key_from_relative(Path::new("../etc/passwd")), None);
    }

    // --- hash_bounded -----------------------------------------------------

    #[test]
    fn hash_bounded_hashes_data_within_the_limit() {
        let data = b"hello world";
        let (n, hash) = hash_bounded(&mut &data[..], 1024).unwrap();
        assert_eq!(n, data.len() as u64);
        assert_eq!(hash.as_slice(), Sha512::digest(data).as_slice());
    }

    #[test]
    fn hash_bounded_rejects_data_over_the_limit() {
        let data = vec![0u8; 1024];
        assert!(hash_bounded(&mut &data[..], 100).is_err());
    }

    #[test]
    fn hash_bounded_accepts_data_exactly_at_the_limit() {
        let data = vec![0u8; 100];
        let (n, hash) = hash_bounded(&mut &data[..], 100).unwrap();
        assert_eq!(n, 100);
        assert_eq!(hash.as_slice(), Sha512::digest(&data).as_slice());
    }

    // --- hash_file_sha512 (local filesystem only, no network) --------------

    #[tokio::test]
    async fn hash_file_sha512_matches_a_known_digest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"hello").unwrap();

        let hash = hash_file_sha512(&path).await.unwrap();

        assert_eq!(hash.as_slice(), Sha512::digest(b"hello").as_slice());
    }

    #[tokio::test]
    async fn hash_file_sha512_errors_on_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            hash_file_sha512(&dir.path().join("missing.txt"))
                .await
                .is_err()
        );
    }

    // --- parse_mrpack (synthetic local archive, no network) -----------------

    #[tokio::test]
    async fn parse_mrpack_extracts_index_and_hashes_overrides_and_client_overrides() {
        use std::io::Write as _;

        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("test.mrpack");

        {
            let file = std::fs::File::create(&archive_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);

            zip.start_file(
                "modrinth.index.json",
                zip::write::FileOptions::<()>::default(),
            )
            .unwrap();
            zip.write_all(
                br#"{"formatVersion":1,"game":"minecraft","versionId":"v1","name":"Test","files":[],"dependencies":{}}"#,
            )
            .unwrap();

            zip.start_file(
                "overrides/config/settings.cfg",
                zip::write::FileOptions::<()>::default(),
            )
            .unwrap();
            zip.write_all(b"override-bytes").unwrap();

            zip.start_file(
                "client-overrides/mods/extra.jar",
                zip::write::FileOptions::<()>::default(),
            )
            .unwrap();
            zip.write_all(b"client-override-bytes").unwrap();

            // A non-overrides entry must never be hashed as one.
            zip.start_file("README.md", zip::write::FileOptions::<()>::default())
                .unwrap();
            zip.write_all(b"readme").unwrap();

            zip.finish().unwrap();
        }

        let (index, override_hashes) = parse_mrpack(&archive_path).await.unwrap();

        assert_eq!(index.name, "Test");
        assert_eq!(
            override_hashes
                .get("/config/settings.cfg")
                .map(|h| h.as_slice()),
            Some(Sha512::digest(b"override-bytes").as_slice())
        );
        assert_eq!(
            override_hashes.get("/mods/extra.jar").map(|h| h.as_slice()),
            Some(Sha512::digest(b"client-override-bytes").as_slice())
        );
        assert_eq!(
            override_hashes.len(),
            2,
            "only the two overrides/client-overrides entries may be hashed, got {override_hashes:?}"
        );
    }

    #[tokio::test]
    async fn parse_mrpack_errors_on_a_non_zip_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-zip.mrpack");
        std::fs::write(&path, b"not a zip archive").unwrap();

        assert!(parse_mrpack(&path).await.is_err());
    }

    #[tokio::test]
    async fn parse_mrpack_gives_every_override_entry_its_own_full_budget() {
        // Real-world shape this pins: three override entries each under
        // MAX_HASHED_ENTRY_BYTES (64 MiB) individually, but summing to
        // nearly double it (e.g. three 40 MiB entries in a pack bundling
        // large resource/shader packs). A budget shared across entries
        // (subtracting each entry's size from what's left for the next)
        // would hash the first and reject the second and third as
        // "oversized" purely because the *running total* crossed the limit —
        // even though no single entry does. Scaled down to a small
        // test-only `entry_budget` via `parse_mrpack_with_budgets` rather
        // than real 40 MiB entries so this stays fast: the archive and
        // hashing work here are the same code path regardless of the
        // budget's magnitude. `total_budget` is passed generously large —
        // the archive-wide aggregate cap is a separate concern, its own
        // dedicated test below.
        use std::io::Write as _;

        const ENTRY_BUDGET: u64 = 1000;
        // Each entry is under ENTRY_BUDGET on its own; three of them sum to
        // well past it.
        let entry_data = vec![0xABu8; 700];

        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("multi-overrides.mrpack");

        {
            let file = std::fs::File::create(&archive_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);

            zip.start_file(
                "modrinth.index.json",
                zip::write::FileOptions::<()>::default(),
            )
            .unwrap();
            zip.write_all(
                br#"{"formatVersion":1,"game":"minecraft","versionId":"v1","name":"Test","files":[],"dependencies":{}}"#,
            )
            .unwrap();

            for name in ["a.bin", "b.bin", "c.bin"] {
                zip.start_file(
                    format!("overrides/{name}"),
                    zip::write::FileOptions::<()>::default(),
                )
                .unwrap();
                zip.write_all(&entry_data).unwrap();
            }

            zip.finish().unwrap();
        }

        let (_index, override_hashes) =
            parse_mrpack_with_budgets(&archive_path, ENTRY_BUDGET, u64::MAX)
                .await
                .unwrap();

        let expected_hash = Sha512::digest(&entry_data);
        for name in ["a.bin", "b.bin", "c.bin"] {
            let key = format!("/{name}");
            assert_eq!(
                override_hashes.get(&key).map(|h| h.as_slice()),
                Some(expected_hash.as_slice()),
                "entry {key} must be hashed on its own {ENTRY_BUDGET}-byte budget, got {:?}",
                override_hashes.keys().collect::<Vec<_>>()
            );
        }
        assert_eq!(
            override_hashes.len(),
            3,
            "all three entries must be hashed, none skipped as oversized due to a shared budget"
        );
    }

    #[tokio::test]
    async fn parse_mrpack_stops_hashing_once_the_aggregate_cap_is_reached() {
        // The per-entry test above proves each entry gets its own full
        // budget; this proves the archive-wide total is still bounded
        // *somewhere* — an unbounded number of individually-small entries
        // could otherwise cost unbounded total hashing work. Three entries,
        // each within `ENTRY_BUDGET` on their own, with `TOTAL_BUDGET` set
        // to allow exactly the first two combined: the first two must be
        // hashed, the third skipped (warned, not hashed) rather than
        // rejected as "oversized" — it never reaches `hash_bounded` at all.
        use std::io::Write as _;

        const ENTRY_BUDGET: u64 = 1000;
        const TOTAL_BUDGET: u64 = 800; // room for exactly two 400-byte entries
        let entry_data = vec![0xCDu8; 400];

        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("aggregate-cap.mrpack");

        {
            let file = std::fs::File::create(&archive_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);

            zip.start_file(
                "modrinth.index.json",
                zip::write::FileOptions::<()>::default(),
            )
            .unwrap();
            zip.write_all(
                br#"{"formatVersion":1,"game":"minecraft","versionId":"v1","name":"Test","files":[],"dependencies":{}}"#,
            )
            .unwrap();

            // Zip entries are enumerated in the order they were written, and
            // `parse_mrpack_with_budgets` walks the archive by index, so
            // "a.bin"/"b.bin" land within budget and "c.bin" is the one that
            // overflows it.
            for name in ["a.bin", "b.bin", "c.bin"] {
                zip.start_file(
                    format!("overrides/{name}"),
                    zip::write::FileOptions::<()>::default(),
                )
                .unwrap();
                zip.write_all(&entry_data).unwrap();
            }

            zip.finish().unwrap();
        }

        let (_index, override_hashes) =
            parse_mrpack_with_budgets(&archive_path, ENTRY_BUDGET, TOTAL_BUDGET)
                .await
                .unwrap();

        let expected_hash = Sha512::digest(&entry_data);
        for name in ["a.bin", "b.bin"] {
            let key = format!("/{name}");
            assert_eq!(
                override_hashes.get(&key).map(|h| h.as_slice()),
                Some(expected_hash.as_slice()),
                "entries within the aggregate cap must still be hashed, got {:?}",
                override_hashes.keys().collect::<Vec<_>>()
            );
        }
        assert!(
            !override_hashes.contains_key("/c.bin"),
            "an entry beyond the aggregate cap must be skipped, not hashed, got {:?}",
            override_hashes.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            override_hashes.len(),
            2,
            "exactly the two entries within the aggregate cap must be hashed"
        );
    }
}
